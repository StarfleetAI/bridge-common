// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Context};
use serde_json::Value;
use sqlx::{Pool, Postgres};
use tracing::{debug, error, instrument, trace, warn};
use uuid::Uuid;

use crate::{
    channel::{Channel, Event},
    clients::{
        self,
        openai::{
            Client, CreateChatCompletionRequest, FunctionCall, Tool, ToolCall, ToolCalls, ToolType,
        },
    },
    errors, messages,
    repo::{
        self,
        messages::{ListParams, UpdateWithCompletionResultParams},
    },
    types::{
        abilities::Ability,
        messages::{Message, Role, Status},
        models::Model,
        Result,
    },
};

const CHUNK_SEPARATOR: &str = "\n\n";
const DONE_CHUNK: &str = "data: [DONE]";

#[derive(Debug, Default)]
pub struct CreateCompletionParams {
    pub messages_pre: Option<Vec<Message>>,
    pub messages_post: Option<Vec<Message>>,
    pub abilities: Option<Vec<Ability>>,
    pub is_self_reflection: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to get completion")]
    FailedToGetCompletion,
}

/// Does the whole chat completion routine.
// TODO: refactor this function.
#[instrument(skip(pool, channel, params, model, api_key, user_agent))]
#[allow(clippy::too_many_lines)]
pub async fn create_completion(
    pool: &Pool<Postgres>,
    channel: &Channel,
    cid: Uuid,
    uid: Uuid,
    chat_id: Uuid,
    params: CreateCompletionParams,
    model: &Model,
    api_key: &str,
    user_agent: &str,
) -> Result<()> {
    debug!("Getting chat completion");

    let mut tx = pool.begin().await.context("Failed to begin transaction")?;

    let mut messages = repo::messages::list(&mut *tx, cid, ListParams { chat_id }).await?;

    if let Some(messages_pre) = params.messages_pre {
        messages = messages_pre.into_iter().chain(messages).collect();
    }

    if let Some(messages_post) = params.messages_post {
        messages = messages.into_iter().chain(messages_post).collect();
    }

    trace!("Messages so far: {:?}", messages);

    // Get current agent.
    let agent = repo::agents::get_for_chat(&mut *tx, cid, chat_id).await?;
    let agent_abilities = repo::abilities::list_for_agent(&mut *tx, cid, agent.id).await?;
    let abilities = match params.abilities {
        Some(abilities) => abilities.into_iter().chain(agent_abilities).collect(),
        None => agent_abilities,
    };

    let req_messages = messages
        .into_iter()
        .map(crate::clients::openai::Message::try_from)
        .collect::<std::result::Result<Vec<_>, _>>()?;

    // Insert dummy message to chat.
    let mut message = repo::messages::create(
        &mut *tx,
        cid,
        repo::messages::CreateParams {
            chat_id,
            agent_id: Some(agent.id),
            status: Status::Writing,
            role: Role::Assistant,
            is_self_reflection: params.is_self_reflection,
            ..Default::default()
        },
    )
    .await
    .context("Failed to insert dummy assistant message")?;

    tx.commit().await.context("Failed to commit transaction")?;

    channel.emit(uid, &Event::MessageCreated(&message)).await?;

    let tools = match construct_tools(abilities).await {
        Ok(tools) => tools,
        Err(err) => {
            fail_message(pool, channel, uid, &mut message).await?;

            return Err(err);
        }
    };

    debug!("Tools: {:?}", tools);

    // Send request to LLM
    let client = Client::new(api_key, model.api_url_or_default(), user_agent);

    create_completion_stream(
        pool,
        channel,
        cid,
        uid,
        req_messages,
        &mut message,
        tools,
        model,
        client,
    )
    .await?;

    if message.status == Status::Writing {
        fail_message(pool, channel, uid, &mut message).await?;

        return Err(anyhow!("Failed to get completion").into());
    }

    Ok(())
}

#[allow(dead_code)]
async fn create_completion_sync<'a>(
    pool: &Pool<Postgres>,
    channel: &Channel,
    cid: Uuid,
    uid: Uuid,
    messages: Vec<clients::openai::Message>,
    message: &'a mut Message,
    tools: Option<Vec<Tool>>,
    model: &'a Model,
    client: Client,
) -> Result<()> {
    let response = match client
        .create_chat_completion(CreateChatCompletionRequest {
            model: &model.name,
            messages,
            tools,
            ..Default::default()
        })
        .await
        .context("Failed to create chat completion")
    {
        Ok(response) => response,
        Err(err) => {
            fail_message(pool, channel, uid, message).await?;

            return Err(err.into());
        }
    };

    let choice = response.choices.first().context("Failed to get choice")?;

    if let clients::openai::Message::Assistant {
        content,
        tool_calls,
        ..
    } = &choice.message
    {
        message.content = content.clone();
        message.tool_calls = tool_calls.clone();

        message.status = match message.tool_calls().is_empty() {
            false => Status::WaitingForToolCall,
            true => Status::Completed,
        };

        if let Err(err) = repo::messages::update_with_completion_result(
            pool,
            cid,
            UpdateWithCompletionResultParams {
                id: message.id,
                status: message.status,
                content: message.content.clone(),
                tool_calls: message.tool_calls.clone(),
                ..Default::default()
            },
        )
        .await
        .context("Failed to update assistant message")
        {
            fail_message(pool, channel, uid, message).await?;

            return Err(err.into());
        };

        if let Err(err) = channel.emit(uid, &Event::MessageUpdated(&message)).await {
            warn!("Failed to emit `MessageUpdate` event: {}", err);
        }
    } else {
        fail_message(pool, channel, uid, message).await?;

        return Err(anyhow!("Unexpected message type").into());
    }

    Ok(())
}

async fn create_completion_stream<'a>(
    pool: &Pool<Postgres>,
    channel: &Channel,
    cid: Uuid,
    uid: Uuid,
    messages: Vec<clients::openai::Message>,
    message: &'a mut Message,
    tools: Option<Vec<Tool>>,
    model: &'a Model,
    client: Client,
) -> Result<()> {
    let mut response = match client
        .create_chat_completion_stream(CreateChatCompletionRequest {
            model: &model.name,
            messages,
            stream: true,
            tools,
        })
        .await
        .context("Failed to create chat completion")
    {
        Ok(response) => response,
        Err(err) => {
            fail_message(pool, channel, uid, message).await?;

            return Err(err.into());
        }
    };

    let mut chunk_remainder = String::new();

    while let Some(chunk) = match response.chunk().await.context("Failed to get chunk") {
        Ok(chunk) => chunk,
        Err(err) => {
            fail_message(pool, channel, uid, message).await?;

            return Err(err.into());
        }
    } {
        // TODO: come up with a more efficient way to split chunks.
        chunk_remainder.push_str(&String::from_utf8_lossy(&chunk));
        let chunk = chunk_remainder.clone();
        chunk_remainder = String::new();
        debug!("RAW chunk: {:?}", chunk);

        let chunks = chunk
            .split(CHUNK_SEPARATOR)
            .map(str::trim)
            .filter(|chunk| !chunk.is_empty())
            .collect::<Vec<&str>>();

        for chunk in chunks {
            if chunk == DONE_CHUNK {
                let mut tool_calls = message.tool_calls();

                message.status = match tool_calls.is_empty() {
                    false => Status::WaitingForToolCall,
                    true => Status::Completed,
                };

                // Cleanup tool calls arguments due to newlines in JSON values causing issues.
                if !tool_calls.is_empty() {
                    for tool_call in &mut tool_calls.0 {
                        tool_call.function.arguments =
                            cleanup_json_string_newlines(&tool_call.function.arguments);
                    }

                    message.tool_calls = Some(serde_json::json!(tool_calls));
                }

                if let Err(err) = repo::messages::update_with_completion_result(
                    pool,
                    cid,
                    UpdateWithCompletionResultParams {
                        id: message.id,
                        status: message.status,
                        content: message.content.clone(),
                        prompt_tokens: None,
                        completion_tokens: None,
                        tool_calls: message.tool_calls.clone(),
                    },
                )
                .await
                .context("Failed to update assistant message")
                {
                    fail_message(pool, channel, uid, message).await?;

                    return Err(err.into());
                };
            } else {
                match apply_completion_chunk(message, chunk) {
                    Err(errors::Error::Messages(
                        messages::Error::ChunkDeserialization(_)
                        | messages::Error::NoValidChunkPrefix,
                    )) => {
                        // TODO: might be incomplete chunk, but might, as well, be an error. Handle this properly.
                        debug!("Error parsing chunk, might be incomplete, pushing to remainder");
                        chunk_remainder = chunk.to_string();
                    }
                    Err(err) => {
                        fail_message(pool, channel, uid, message).await?;

                        return Err(err);
                    }
                    _ => {}
                };
            }

            if let Err(err) = channel.emit(uid, &Event::MessageUpdated(&message)).await {
                warn!("Failed to emit `MessageUpdate` event: {}", err);
            };
        }
    }

    Ok(())
}

/// Constructs tools from abilities.
///
/// # Errors
///
/// Returns error if there was a problem while constructing tools.
pub async fn construct_tools(abilities: Vec<Ability>) -> Result<Option<Vec<Tool>>> {
    let mut tools = None;

    if !abilities.is_empty() {
        tools = Some(
            abilities
                .into_iter()
                .map(|ability| {
                    let mut function = ability.function();
                    function.description = Some(ability.description);

                    Ok(Tool {
                        type_: "function".to_string(),
                        function,
                    })
                })
                .collect::<Result<Vec<Tool>>>()
                .with_context(|| "Failed to construct tools")?,
        );
    }

    Ok(tools)
}

async fn fail_message(
    pool: &Pool<Postgres>,
    channel: &Channel,
    uid: Uuid,
    message: &mut Message,
) -> Result<()> {
    repo::messages::update_status(pool, message.company_id, message.id, Status::Failed).await?;
    message.status = Status::Failed;

    channel.emit(uid, &Event::MessageUpdated(&message)).await?;

    Ok(())
}

#[allow(clippy::too_many_lines)]
#[instrument(skip(message))]
fn apply_completion_chunk(message: &mut Message, chunk: &str) -> Result<()> {
    debug!("Applying completion chunk");
    let mut current_tool_call = None;

    let tool_calls = message.tool_calls();

    if !tool_calls.is_empty() {
        current_tool_call = tool_calls.0.into_iter().last();
        trace!("Last tool call: {:?}", current_tool_call);
    }

    let completion: Value = serde_json::from_str(
        chunk
            .trim()
            .strip_prefix("data: ")
            .ok_or(messages::Error::NoValidChunkPrefix)?,
    )
    .map_err(messages::Error::ChunkDeserialization)?;

    if let Some(choices) = completion.get("choices") {
        trace!("Choices: {:?}", choices);

        if let Some(delta) = choices[0].get("delta") {
            trace!("Delta: {:?}", delta);

            match delta.get("content") {
                Some(content) if content.is_string() => {
                    trace!("Content: {:?}", content);

                    message.content = Some(match &message.content {
                        Some(existed) => {
                            existed.to_owned()
                                + content.as_str().context("Failed to get content as str")?
                        }
                        None => content
                            .as_str()
                            .context("Failed to get content as str")?
                            .to_string(),
                    });
                }
                _ => {}
            }

            match delta.get("tool_calls") {
                Some(tool_calls) if tool_calls.is_array() => {
                    trace!("Tool call: {:?}", tool_calls[0]);

                    if current_tool_call.is_none() || tool_calls[0].get("id").is_some() {
                        trace!("Current tool call: {:?}", current_tool_call);
                        trace!("Get id: {:?}", tool_calls[0].get("id"));
                        current_tool_call = Some(ToolCall {
                            id: String::new(),
                            type_: ToolType::Function,
                            function: FunctionCall {
                                name: String::new(),
                                arguments: String::new(),
                            },
                        });
                    }

                    if let Some(id) = tool_calls[0].get("id") {
                        trace!("ID: {:?}", id);

                        current_tool_call
                            .as_mut()
                            .context("Failed to get tool call")?
                            .id
                            .push_str(id.as_str().context("Failed to get id as str")?);
                    }

                    if let Some(function) = tool_calls[0].get("function") {
                        trace!("Function: {:?}", function);

                        if let Some(name) = function.get("name") {
                            trace!("Name: {:?}", name);

                            current_tool_call
                                .as_mut()
                                .context("Failed to get tool call")?
                                .function
                                .name
                                .push_str(name.as_str().context("Failed to get name as str")?);
                        }

                        if let Some(arguments) = function.get("arguments") {
                            trace!("Arguments: {:?}", arguments);

                            current_tool_call
                                .as_mut()
                                .context("Failed to get tool call")?
                                .function
                                .arguments
                                .push_str(
                                    arguments
                                        .as_str()
                                        .context("Failed to get arguments as str")?,
                                );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    if let Some(tool_call) = current_tool_call {
        let tool_calls = message.tool_calls();
        let tool_calls = match tool_calls.is_empty() {
            true => vec![tool_call],
            false => {
                let mut tc = tool_calls.0.into_iter().collect::<Vec<ToolCall>>();
                if tool_call.id == tc.last().context("Last tool call is somehow None")?.id {
                    tc.pop();
                }

                tc.push(tool_call);

                tc
            }
        };

        trace!("Resulting tool calls: {:?}", tool_calls);

        message.tool_calls = Some(
            serde_json::json!(ToolCalls(tool_calls))
                .to_string()
                .parse()
                .context("Failed to parse tool calls")?,
        );
    }

    Ok(())
}

// This function is used to remove newlines from the JSON struct. It should not alter the keys or the values, only the newlines between them.
fn cleanup_json_string_newlines(json_str: &str) -> String {
    let mut new_json_str = String::with_capacity(json_str.len());
    let mut in_quotes = false;
    let mut last_char = ' ';

    for c in json_str.chars() {
        if c == '"' && last_char != '\\' {
            in_quotes = !in_quotes;
        }

        if c == '\n' {
            if in_quotes {
                new_json_str.push_str("\\n");
                last_char = c;
            }

            continue;
        }

        new_json_str.push(c);
        last_char = c;
    }

    new_json_str.trim().replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_json_string_newlines() {
        let json_str = r#"[{"id":"call_qSoLU7GYixJU7OLXKJxGdBGz","type":"function","function":{"name":"sfai_provide_text_result","arguments":"{\n\"text\": \"In Vue 3, the 'ref' keyword is used in the composition API to create \\\"reac\ntive\\\" references. While regular JavaScript variables won't be reactive inside Vue's templating system, `ref` creates a reactive and mutable object that can be used to keep track of changes in your Vue component. \n\nA ref is defined as follows:\n```javascript\nimport { ref } from 'vue'\n\nconst myVar = ref('initial value')\n```\nYou would access a ref value with `.value`:\n```javascript\nconsole.log(myVar.value)\n```\n\nOne practical example is if we wanted a button click to increment a counter:\n```javascript\nimport { ref } from 'vue'\n\nconst counter = ref(0)\n\n// In your method\nconst increment = () => {\n  counter.value += 1\n}\n\nexport default {\n  setup() {\n    return { counter , increment }\n  }\n}\n```\nIn this scenario, anytime `counter.value` is updated, Vue.js would be aware of the changes and re-render as needed. 'ref' is useful to track stateful values throughout your Vue application.\",\n\"is_done\": true\n} \n"}}]"#;
        let tool_calls: Vec<ToolCall> =
            serde_json::from_str(json_str).expect("Failed to parse tool call");

        let expected = r#"{"text": "In Vue 3, the 'ref' keyword is used in the composition API to create \"reac\ntive\" references. While regular JavaScript variables won't be reactive inside Vue's templating system, `ref` creates a reactive and mutable object that can be used to keep track of changes in your Vue component. \n\nA ref is defined as follows:\n```javascript\nimport { ref } from 'vue'\n\nconst myVar = ref('initial value')\n```\nYou would access a ref value with `.value`:\n```javascript\nconsole.log(myVar.value)\n```\n\nOne practical example is if we wanted a button click to increment a counter:\n```javascript\nimport { ref } from 'vue'\n\nconst counter = ref(0)\n\n// In your method\nconst increment = () => {\n  counter.value += 1\n}\n\nexport default {\n  setup() {\n    return { counter , increment }\n  }\n}\n```\nIn this scenario, anytime `counter.value` is updated, Vue.js would be aware of the changes and re-render as needed. 'ref' is useful to track stateful values throughout your Vue application.","is_done": true}"#;

        assert_eq!(
            cleanup_json_string_newlines(&tool_calls[0].function.arguments),
            expected
        );
    }
}
