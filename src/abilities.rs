// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use anyhow::{anyhow, Context};
use askama::Template;
use sqlx::{Pool, Postgres};
use tokio::{fs, spawn};
use tracing::{debug, trace};
use uuid::Uuid;

use crate::{
    channel::Channel,
    clients::openai::{Function, Tool, ToolCall},
    docker,
    repo::{self, messages::CreateParams},
    types::{
        abilities::Ability,
        messages::{Message, Role, Status},
        Result,
    },
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("error joining tokio tasks")]
    TokioJoin(#[from] tokio::task::JoinError),
    #[error("ability is used by agents")]
    IsUsedByAgents,
}

#[derive(Template)]
#[template(path = "python/get_function_definition.py", escape = "none")]
struct GetFunctionDefinitionTemplate<'a> {
    code: &'a str,
}

#[derive(Template)]
#[template(path = "python/call_tools.py", escape = "none")]
struct CallToolsTemplate<'a> {
    code: &'a str,
    tool_call: &'a str,
}

/// Get function definition by its code.
///
/// # Errors
///
/// Returns error if there was a problem when determining function parameters.
// TODO: work correctly if there are imports in the code
pub async fn get_function_definition(code: &str) -> Result<Function> {
    let template = GetFunctionDefinitionTemplate { code };

    // TODO: seems a little bit inefficient to run a container only to get a function definition.
    //       Consider using some Python parser library to get type hints on a Rust side.
    let output = docker::run_python_code(
        &template
            .render()
            .context("Failed to render `get_function_definition` script")?,
        None,
    )
    .await?;

    debug!("Function definition script output: {:?}", output);

    let tool: Tool = serde_json::from_str(&output)
        .with_context(|| "Failed to parse function definition script output")?;

    Ok(tool.function)
}

/// Preprocess code: trim leading and trailing whitespaces around the code, remove trailing whitespaces
/// from each line.
#[must_use]
pub fn preprocess_code(code: &str) -> String {
    let mut result = String::new();

    for line in code.lines() {
        result.push_str(line.trim_end());
        result.push('\n');
    }

    result.trim().to_string()
}

/// Executes tool calls for the message.
///
/// # Errors
///
/// Will return an error if there was a problem while executing tool calls.
pub async fn execute_for_message(
    pool: &Pool<Postgres>,
    channel: &Channel,
    cid: Uuid,
    uid: Uuid,
    workdir_root: &PathBuf,
    message: &Message,
) -> Result<()> {
    // Load agent abilities
    let abilities = match message.agent_id {
        Some(agent_id) => repo::abilities::list_for_agent(pool, cid, agent_id).await?,
        None => return Err(anyhow!("Agent is not set for the message").into()),
    };

    let tool_calls = message.tool_calls();
    if tool_calls.is_empty() {
        return Err(anyhow!("Tool calls are not set for the message").into());
    };

    let mut handles = Vec::with_capacity(tool_calls.len());
    for tool_call in tool_calls.iter() {
        // Skip internal tool calls
        if tool_call.function.name.starts_with("sfai_") {
            continue;
        }

        let abilities = abilities.clone();
        let workdir_root = workdir_root.clone();
        let msg = message.clone();
        let tc = tool_call.clone();

        let handle = spawn(async move {
            let output = execute(&abilities, &workdir_root, &msg, &tc).await?;
            // Wrap output in a code block
            //
            // TODO: This is a temporary solution. It's better to wrap it on before markdown-2-html
            //       processing, but it requires writing custom Serializer for Message.
            let output = format!("```\n{output}\n```");
            Ok::<_, anyhow::Error>(CreateParams {
                chat_id: msg.chat_id,
                status: Status::Completed,
                role: Role::Tool,
                content: Some(output),
                tool_call_id: Some(tc.id),

                ..Default::default()
            })
        });

        handles.push(handle);
    }

    for handle in handles {
        let params = handle.await.map_err(Error::TokioJoin)??;
        let results_message = repo::messages::create(pool, cid, params).await?;

        // Emit event
        channel
            .emit(
                uid,
                &crate::channel::Event::MessageCreated(&results_message),
            )
            .await?;
    }

    // Mark message as completed
    repo::messages::update_status(pool, uid, message.id, Status::Completed).await?;

    Ok(())
}

/// Execute abilities code.
///
/// # Errors
///
/// Will return an error if the script can't be written, executed or removed.
pub async fn execute(
    abilities: &[Ability],
    workdir_root: &PathBuf,
    message: &Message,
    tool_call: &ToolCall,
) -> Result<String> {
    debug!(
        "Executing tool call `{}` for message `{}`",
        tool_call.id, message.id
    );

    // Join the abilities code into one string
    let code = abilities
        .iter()
        .map(|ability| ability.code.as_str())
        .collect::<Vec<&str>>()
        .join("\n\n");

    let workdir_name = format!("wd-{}", message.chat_id);

    // Build workdir path
    let mut workdir = PathBuf::new();
    workdir.push(workdir_root);
    workdir.push(workdir_name);

    trace!("Workdir: {:?}", workdir);

    if !workdir.exists() {
        fs::create_dir_all(&workdir)
            .await
            .with_context(|| "Failed to create workdir")?;
    }

    let tool_call_string =
        serde_json::to_string(&tool_call).with_context(|| "Failed to serialize tool call")?;

    let script_name = format!("tc-{}-{}.py", message.id, tool_call.id);
    let call_tools_template = CallToolsTemplate {
        code: &code,
        tool_call: &tool_call_string,
    };
    let content = call_tools_template
        .render()
        .with_context(|| "Failed to render `call_tools` script")?;

    trace!("Script name: {}", script_name);
    trace!("Script content: {}", content);

    // Write script to workdir
    let mut script_path = workdir.clone();
    script_path.push(&script_name);
    trace!("Script path: {:?}", script_path);

    fs::write(&script_path, content)
        .await
        .with_context(|| "Failed to write script to workdir")?;

    // Run script
    let output = docker::run_python_script(&workdir, &script_name).await;

    // Delete script
    fs::remove_file(&script_path)
        .await
        .with_context(|| "Failed to remove script from workdir")?;

    output
}
