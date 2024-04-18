// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use tracing::{instrument, trace};

use crate::types::messages::Role;
use crate::{
    clients::{
        self,
        openai::{Client, CreateChatCompletionRequest},
    },
    types::{messages::Message, models::Model, Result},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("too few messages: {0}")]
    TooFewMessages(usize),
    #[error("chat has no user or assistant messages")]
    NoSuitableMessages,
    #[error("last message in the chat is not from assistant")]
    LastMessageNotFromAssistant,
    #[error("failed to create chat completion: {0}")]
    FailedToCreateChatCompletion(String),
    #[error("received empty response from LLM")]
    EmptyLLMResponseReceived,
    #[error("unexpected response from LLM")]
    UnexpectedResponse,
    #[error("tool calls are not an array")]
    ToolCallsNotArray,
    #[error("failed to convert message to OpenAI message")]
    OpenAIConversionError(#[from] anyhow::Error),
    #[error("chunk deserialization error: {0}")]
    ChunkDeserialization(#[from] serde_json::Error),
    #[error("no valid chunk prefix found")]
    NoValidChunkPrefix,
    #[error("no tool calls found in message")]
    NoToolCallsFound,
    #[error("tool call has no `id`")]
    NoToolCallId,
}

#[instrument(skip(messages, model, api_key, user_agent))]
pub async fn generate_chat_title(
    messages: Vec<Message>,
    model: &Model,
    api_key: &str,
    user_agent: &str,
) -> Result<String> {
    if messages.len() < 3 {
        return Err(Error::TooFewMessages(messages.len()).into());
    }

    let user_message = messages.iter().find(|message| message.role == Role::User);
    let assistant_message_without_tools = messages
        .iter()
        .find(|message| message.role == Role::Assistant && message.tool_calls().is_empty());

    if user_message.is_none() || assistant_message_without_tools.is_none() {
        return Err(Error::NoSuitableMessages.into());
    }

    let last_message = messages.last().unwrap();

    if last_message.role != Role::Assistant {
        return Err(Error::LastMessageNotFromAssistant.into());
    }

    let mut req_messages = messages
        .into_iter()
        .filter(|message| {
            message.role == Role::User
                || message.role == Role::System
                || (message.role == Role::Assistant && message.tool_calls().is_empty())
        })
        .map(clients::openai::Message::try_from)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Error::OpenAIConversionError)?;

    trace!("Messages so far: {:?}", req_messages);

    req_messages.push(clients::openai::Message::User {
        content: "Provide a short title for the current conversation (4-6 words). Your response must only contain the chat title and nothing else.".to_string(),
        name: None,
    });

    // Send request to LLM
    let client = Client::new(api_key, model.api_url_or_default(), user_agent);
    let response = client
        .create_chat_completion(CreateChatCompletionRequest {
            model: &model.name,
            messages: req_messages,
            ..Default::default()
        })
        .await
        .map_err(|e| Error::FailedToCreateChatCompletion(e.to_string()))?;

    let mut title = match &response.choices[0].message {
        crate::clients::openai::Message::Assistant { content, .. } => match content {
            Some(title) => title,
            _ => return Err(Error::EmptyLLMResponseReceived.into()),
        },
        _ => return Err(Error::UnexpectedResponse.into()),
    }
    .to_string();

    // Clean up title
    if title.starts_with('"') && title.ends_with('"') {
        title = title
            .trim_start_matches('"')
            .trim_end_matches('"')
            .to_string();
    }

    Ok(title)
}
