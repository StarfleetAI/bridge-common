// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use chrono::Utc;
use sqlx::{query, query_as, Executor, Postgres};

use crate::types::{
    chats::{Chat, Kind},
    Result,
};

/// List all chats.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn list<'a, E>(executor: E, company_id: i32, is_pinned: Option<bool>) -> Result<Vec<Chat>>
where
    E: Executor<'a, Database = Postgres>,
{
    if let Some(is_pinned) = is_pinned {
        return Ok(query_as!(
            Chat,
            r#"
            SELECT *
            FROM chats
            WHERE
                company_id = $1 AND
                is_pinned = $2 AND
                kind = $3
            ORDER BY updated_at DESC
            "#,
            company_id,
            is_pinned,
            Kind::Direct.to_string()
        )
        .fetch_all(executor)
        .await?);
    }

    Ok(query_as!(
        Chat,
        "SELECT * FROM chats WHERE company_id = $1 AND kind = $2 ORDER BY id DESC",
        company_id,
        Kind::Direct.to_string()
    )
    .fetch_all(executor)
    .await?)
}

/// Get chat by id.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn get<'a, E>(executor: E, company_id: i32, id: i32) -> Result<Chat>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_as!(
        Chat,
        "SELECT * FROM chats WHERE company_id = $1 AND id = $2 LIMIT 1",
        company_id,
        id
    )
    .fetch_one(executor)
    .await?)
}

/// Delete chat by id.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn delete<'a, E>(executor: E, company_id: i32, id: i32) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    query!(
        "DELETE FROM chats WHERE company_id = $1 AND id = $2",
        company_id,
        id
    )
    .execute(executor)
    .await
    .with_context(|| "Failed to delete chat")?;

    Ok(())
}

/// Create chat.
///
/// # Errors
///
/// Returns error if there was a problem while creating chat.
pub async fn create<'a, E>(executor: E, company_id: i32, kind: Kind) -> Result<Chat>
where
    E: Executor<'a, Database = Postgres>,
{
    let now = Utc::now();

    Ok(query_as!(
        Chat,
        "INSERT INTO chats (company_id, kind, created_at, updated_at) VALUES ($1, $2, $3, $3) RETURNING *",
        company_id,
        kind.to_string(),
        now
    )
        .fetch_one(executor)
        .await?)
}

/// Update chat title by id.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database or if the chat with the given ID does not exist.
pub async fn update_title<'a, E>(executor: E, company_id: i32, id: i32, title: &str) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    let now = Utc::now();
    query!(
        "UPDATE chats SET title = $1, updated_at = $2 WHERE company_id = $3 AND id = $4",
        title,
        now,
        company_id,
        id
    )
    .execute(executor)
    .await
    .with_context(|| format!("Failed to update title for chat with id: {id}"))?;

    Ok(())
}

/// Toggle chat is pinned status by id.
///
/// # Errors
///
/// Returns error if the chat with the given ID does not exist.
pub async fn toggle_is_pinned<'a, E>(executor: E, company_id: i32, id: i32) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    query!(
        "UPDATE chats SET is_pinned = NOT is_pinned WHERE company_id = $1 AND id = $2",
        company_id,
        id
    )
    .execute(executor)
    .await
    .with_context(|| format!("Failed to toggle pin status for chat with id: {id}"))?;

    Ok(())
}

/// Change chat model by id
///
/// # Errors
///
/// Return error if the chat with the given ID does not exist.
pub async fn update_model_id<'a, E>(
    executor: E,
    company_id: i32,
    id: i32,
    model_id: Option<i32>,
) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    let now = Utc::now();
    query!(
        "UPDATE chats SET model_id = $3, updated_at = $4 WHERE company_id = $1 AND id = $2",
        company_id,
        id,
        model_id,
        now
    )
    .execute(executor)
    .await
    .with_context(|| format!("Failed to change model for chat with id: {id}"))?;

    Ok(())
}
