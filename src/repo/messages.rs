// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{query, query_as, query_scalar, Executor, Postgres};

use crate::messages::Error;
use crate::types::{
    messages::{Message, Role, Status},
    Result,
};

pub struct RawContent {
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct CreateParams {
    pub chat_id: i32,
    pub agent_id: Option<i32>,
    pub status: Status,
    pub role: Role,
    pub content: Option<String>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub tool_calls: Option<Value>,
    pub tool_call_id: Option<String>,
    pub is_self_reflection: bool,
    pub is_internal_tool_output: bool,
}

#[derive(Debug, Default)]
pub struct ListParams {
    pub chat_id: i32,
}

#[derive(Debug, Default)]
pub struct UpdateWithCompletionResultParams {
    pub id: i64,
    pub status: Status,
    pub content: Option<String>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub tool_calls: Option<Value>,
}

/// List all messages.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn list<'a, E>(executor: E, company_id: i32, params: ListParams) -> Result<Vec<Message>>
where
    E: Executor<'a, Database = Postgres>,
{
    let messages = query_as!(
        Message,
        r#"
        SELECT *
        FROM messages
        WHERE company_id = $1 AND chat_id = $2
        ORDER BY id ASC
        "#,
        company_id,
        params.chat_id,
    )
    .fetch_all(executor)
    .await?;

    Ok(messages)
}

/// Create message.
///
/// # Errors
///
/// Returns error if there was a problem while creating message.
pub async fn create<'a, E>(executor: E, company_id: i32, params: CreateParams) -> Result<Message>
where
    E: Executor<'a, Database = Postgres>,
{
    let now = Utc::now();
    Ok(query_as!(
        Message,
        r#"
        INSERT INTO messages (
            company_id, chat_id, agent_id, status,
            role, content, prompt_tokens, completion_tokens,
            tool_calls, tool_call_id, created_at, updated_at,
            is_self_reflection, is_internal_tool_output
        ) VALUES (
            $1, $2, $3, $4,
            $5, $6, $7, $8,
            $9, $10, $11, $11,
            $12, $13
        ) RETURNING *
        "#,
        company_id,
        params.chat_id,
        params.agent_id,
        params.status.to_string(),
        params.role.to_string(),
        params.content,
        params.prompt_tokens,
        params.completion_tokens,
        params.tool_calls,
        params.tool_call_id,
        now,
        params.is_self_reflection,
        params.is_internal_tool_output,
    )
    .fetch_one(executor)
    .await?)
}

/// Create multiple messages in a one request.
///
/// # Errors
///
/// Returns error if there was a problem while creating message.
pub async fn create_multiple<'a, E>(
    executor: E,
    company_id: i32,
    params: Vec<CreateParams>,
) -> Result<Vec<Message>>
where
    E: Executor<'a, Database = Postgres>,
{
    let mut company_ids = Vec::with_capacity(params.len());
    let mut chat_ids = Vec::with_capacity(params.len());
    let mut agent_ids = Vec::with_capacity(params.len());
    let mut statuses = Vec::with_capacity(params.len());
    let mut roles = Vec::with_capacity(params.len());
    let mut contents = Vec::with_capacity(params.len());
    let mut prompt_tokens = Vec::with_capacity(params.len());
    let mut completion_tokens = Vec::with_capacity(params.len());
    let mut tool_calls = Vec::with_capacity(params.len());
    let mut tool_call_ids = Vec::with_capacity(params.len());

    let now = Utc::now();

    let created_at = vec![now; params.len()];
    let updated_at = vec![now; params.len()];
    let is_self_reflection = vec![false; params.len()];
    let is_internal_tool_output = vec![false; params.len()];

    for param in params {
        company_ids.push(company_id);
        chat_ids.push(param.chat_id);
        agent_ids.push(param.agent_id);
        statuses.push(param.status.to_string());
        roles.push(param.role.to_string());
        contents.push(param.content);
        prompt_tokens.push(param.prompt_tokens);
        completion_tokens.push(param.completion_tokens);
        tool_calls.push(param.tool_calls);
        tool_call_ids.push(param.tool_call_id);
    }

    Ok(query_as!(
        Message,
        r#"
        INSERT INTO messages (
            company_id, chat_id, agent_id, status,
            role, content, prompt_tokens, completion_tokens,
            tool_calls, tool_call_id, created_at, updated_at,
            is_self_reflection, is_internal_tool_output
        )
        SELECT * FROM unnest(
            $1::INTEGER[], $2::INTEGER[], $3::INTEGER[], $4::TEXT[],
            $5::TEXT[], $6::TEXT[], $7::INTEGER[], $8::INTEGER[],
            $9::JSONB[], $10::TEXT[], $11::TIMESTAMPTZ[], $12::TIMESTAMPTZ[],
            $13::BOOLEAN[], $14::BOOLEAN[]
        ) RETURNING *
        "#,
        &company_ids,
        &chat_ids,
        &agent_ids as &[Option<i32>],
        &statuses,
        &roles,
        &contents as &[Option<String>],
        &prompt_tokens as &[Option<i32>],
        &completion_tokens as &[Option<i32>],
        &tool_calls as &[Option<Value>],
        &tool_call_ids as &[Option<String>],
        &created_at,
        &updated_at,
        &is_self_reflection,
        &is_internal_tool_output,
    )
    .fetch_all(executor)
    .await?)
}

/// Get message by id.
///
/// # Errors
///
/// Returns error if there was a problem while fetching message.
pub async fn get<'a, E>(executor: E, company_id: i32, id: i64) -> Result<Message>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_as!(
        Message,
        "SELECT * FROM messages WHERE company_id = $1 AND id = $2",
        company_id,
        id
    )
    .fetch_one(executor)
    .await?)
}

/// Get last message id.
///
/// # Errors
///
/// Returns error if there was a problem while fetching last message id.
pub async fn get_last_message_id<'a, E>(
    executor: E,
    company_id: i32,
    chat_id: i32,
) -> Result<Option<i64>>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_scalar!(
        "SELECT MAX(id) FROM messages WHERE company_id = $1 AND chat_id = $2",
        company_id,
        chat_id
    )
    .fetch_one(executor)
    .await?)
}

/// Get last message for chat.
///
/// # Errors
///
/// Returns error if there was a problem while fetching last message.
pub async fn get_last_message<'a, E>(
    executor: E,
    company_id: i32,
    chat_id: i32,
) -> Result<Option<Message>>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_as!(
        Message,
        r#"
        SELECT *
        FROM messages
        WHERE company_id = $1 AND chat_id = $2
        ORDER BY id DESC
        LIMIT 1
        "#,
        company_id,
        chat_id,
    )
    .fetch_optional(executor)
    .await?)
}

/// Get the number of messages from the Assistant role that are not `is_internal_tool_output`..
///
/// # Errors
///
/// Returns error if there was a problem while counting of messages.
pub async fn get_execution_steps_count<'a, E>(
    executor: E,
    company_id: i32,
    chat_id: i32,
) -> Result<i64>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_scalar!(
        r#"
        SELECT count(*) as msg_count
        FROM messages
        WHERE company_id = $1 AND chat_id = $2
        AND role = $3
        AND is_internal_tool_output IS FALSE
        "#,
        company_id,
        chat_id,
        Role::Assistant.to_string(),
    )
    .fetch_one(executor)
    .await?
    .unwrap_or_default())
}

/// Get last non self-reflection agent message for chat.
///
/// # Errors
///
/// Returns error if there was a problem while fetching last non self-reflection message.
pub async fn get_last_non_self_reflection_message<'a, E>(
    executor: E,
    company_id: i32,
    chat_id: i32,
) -> Result<Option<Message>>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_as!(
        Message,
        r#"
        SELECT * FROM messages
        WHERE
            company_id = $1 AND
            chat_id = $2 AND
            is_self_reflection = FALSE AND
            role = $3
        ORDER BY id DESC LIMIT 1
        "#,
        company_id,
        chat_id,
        Role::Assistant.to_string(),
    )
    .fetch_optional(executor)
    .await?)
}

/// Update message status.
///
/// # Errors
///
/// Returns error if there was a problem while updating message status.
pub async fn update_status<'a, E>(
    executor: E,
    company_id: i32,
    id: i64,
    status: Status,
) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    query!(
        "UPDATE messages SET status = $3 WHERE company_id = $1 AND id = $2",
        company_id,
        id,
        status.to_string()
    )
    .execute(executor)
    .await?;

    Ok(())
}

/// Update message tool call id.
///
/// # Errors
///
/// Returns error if there was a problem while updating message tool call id.
pub async fn update_tool_call_id<'a, E>(
    executor: E,
    company_id: i32,
    id: i64,
    tool_call_id: &str,
) -> Result<Message>
where
    E: Executor<'a, Database = Postgres>,
{
    let now = Utc::now();

    Ok(query_as!(
        Message,
        r#"
        UPDATE messages
        SET tool_call_id = $3, updated_at = $4
        WHERE company_id = $1 AND id = $2
        RETURNING *
        "#,
        company_id,
        id,
        tool_call_id,
        now
    )
    .fetch_one(executor)
    .await?)
}

/// Update assistant message with completion result.
///
/// # Errors
///
/// Returns error if there was a problem while updating assistant message.
pub async fn update_with_completion_result<'a, E>(
    executor: E,
    company_id: i32,
    params: UpdateWithCompletionResultParams,
) -> Result<Message>
where
    E: Executor<'a, Database = Postgres>,
{
    let now = Utc::now();

    Ok(query_as!(
        Message,
        r#"
        UPDATE messages
        SET
            status = $3,
            content = $4,
            prompt_tokens = $5,
            completion_tokens = $6,
            tool_calls = $7,
            updated_at = $8
        WHERE company_id = $1 AND id = $2
        RETURNING *
        "#,
        company_id,
        params.id,
        params.status.to_string(),
        params.content,
        params.prompt_tokens,
        params.completion_tokens,
        params.tool_calls,
        now
    )
    .fetch_one(executor)
    .await?)
}

/// Delete message.
///
/// # Errors
///
/// Returns error if there was a problem while deleting message.
pub async fn delete<'a, E>(executor: E, company_id: i32, id: i64) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    query!(
        "DELETE FROM messages WHERE company_id = $1 AND id = $2",
        company_id,
        id
    )
    .execute(executor)
    .await?;

    Ok(())
}

/// Update message content.
///
/// # Errors
///
/// Returns error if there was a problem while updating message content.
pub async fn update_message_content<'a, E>(
    executor: E,
    company_id: i32,
    id: i64,
    content: &str,
) -> Result<Message>
where
    E: Executor<'a, Database = Postgres>,
{
    let now = Utc::now();

    Ok(query_as!(
        Message,
        r#"
        UPDATE messages
        SET content = $3, updated_at = $4
        WHERE company_id = $1 AND id = $2
        RETURNING *
        "#,
        company_id,
        id,
        content,
        now
    )
    .fetch_one(executor)
    .await?)
}

/// Transitions messages from one status to another.
///
/// # Errors
///
/// Returns error if there was a problem while updating messages.
pub async fn transition_all<'a, E>(executor: E, from: Status, to: Status) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    let now = Utc::now();
    query!(
        "UPDATE messages
         SET status = $1, updated_at = $3
         WHERE status = $2",
        to.to_string(),
        from.to_string(),
        now
    )
    .execute(executor)
    .await
    .with_context(|| format!("Failed to set `{from}` messages to `{to}`"))?;

    Ok(())
}

/// Delete messages for chat.
///
/// # Errors
///
/// Returns error if there was a problem while deleting messages.
pub async fn delete_for_chat<'a, E>(executor: E, company_id: i32, chat_id: i32) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    query!(
        "DELETE FROM messages WHERE company_id = $1 AND chat_id = $2",
        company_id,
        chat_id
    )
    .execute(executor)
    .await?;

    Ok(())
}

/// Create tool call denied
///
/// # Errors
///
/// Returns error if there was a problem while creating message.
pub async fn create_tool_call_denied<'a, E>(
    executor: E,
    company_id: i32,
    message: &Message,
) -> Result<Vec<Message>>
where
    E: Executor<'a, Database = Postgres>,
{
    let tool_calls = message.tool_calls();

    if tool_calls.is_empty() {
        return Err(Error::NoToolCallsFound.into());
    }

    let mut messages = Vec::with_capacity(tool_calls.len());
    for tool_call in tool_calls.iter() {
        messages.push(CreateParams {
            chat_id: message.chat_id,
            status: Status::ToolCallDenied,
            role: Role::Tool,
            content: Some("Tool call denied".to_string()),
            tool_call_id: Some(tool_call.id.clone()),

            ..Default::default()
        });
    }

    create_multiple(executor, company_id, messages).await
}
