// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use chrono::Utc;
use sqlx::{query, query_as, Executor, Postgres};
use uuid::Uuid;

use crate::types::{agents::Agent, Result};

pub struct CreateParams {
    pub name: String,
    pub description: String,
    pub system_message: String,
    pub is_code_interpreter_enabled: bool,
    pub is_web_browser_enabled: bool,
}

pub struct UpdateParams {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub system_message: String,
    pub is_code_interpreter_enabled: bool,
    pub is_web_browser_enabled: bool,
}

/// List all agents.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn list<'a, E>(executor: E, company_id: Uuid) -> Result<Vec<Agent>>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_as!(
        Agent,
        "SELECT * FROM agents WHERE company_id = $1 ORDER BY id ASC",
        company_id
    )
    .fetch_all(executor)
    .await?)
}

/// List enabled agents.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn list_enabled<'a, E>(executor: E, company_id: Uuid) -> Result<Vec<Agent>>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_as!(
        Agent,
        "SELECT * FROM agents WHERE company_id = $1 AND is_enabled = TRUE ORDER BY id ASC",
        company_id
    )
    .fetch_all(executor)
    .await?)
}

/// Get agent by id.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn get<'a, E>(executor: E, company_id: Uuid, id: Uuid) -> Result<Agent>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_as!(
        Agent,
        "SELECT * FROM agents WHERE company_id = $1 AND id = $2 LIMIT 1",
        company_id,
        id
    )
    .fetch_one(executor)
    .await?)
}

/// Get agent by id_int.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn get_by_id_int<'a, E>(executor: E, company_id: Uuid, id_int: i32) -> Result<Agent>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_as!(
        Agent,
        "SELECT * FROM agents WHERE company_id = $1 AND id_int = $2 LIMIT 1",
        company_id,
        id_int
    )
    .fetch_one(executor)
    .await?)
}

/// Get agent by chat id.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn get_for_chat<'a, E>(executor: E, company_id: Uuid, chat_id: Uuid) -> Result<Agent>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_as!(
        Agent,
        r#"
        SELECT agents.*
        FROM agents
        INNER JOIN agents_chats ON agents.id = agents_chats.agent_id
        WHERE agents.company_id = $1 AND agents_chats.chat_id = $2
        LIMIT 1
        "#,
        company_id,
        chat_id
    )
    .fetch_one(executor)
    .await?)
}

/// Get agent.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn create<'a, E>(executor: E, company_id: Uuid, params: CreateParams) -> Result<Agent>
where
    E: Executor<'a, Database = Postgres>,
{
    let now = Utc::now();

    Ok(query_as!(
        Agent,
        r#"
        INSERT INTO agents (
            company_id, name, description, system_message,
            created_at, updated_at, is_code_interpreter_enabled, is_web_browser_enabled
        )
        VALUES ($1, $2, $3, $4, $5, $5, $6, $7)
        RETURNING *
        "#,
        company_id,
        params.name,
        params.description,
        params.system_message,
        now,
        params.is_code_interpreter_enabled,
        params.is_web_browser_enabled,
    )
    .fetch_one(executor)
    .await?)
}

/// Update agent.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn update<'a, E>(executor: E, company_id: Uuid, params: UpdateParams) -> Result<Agent>
where
    E: Executor<'a, Database = Postgres>,
{
    let now = Utc::now();

    Ok(query_as!(
        Agent,
        r#"
        UPDATE agents
        SET
            name = $3, description = $4, system_message = $5, updated_at = $6,
            is_code_interpreter_enabled = $7, is_web_browser_enabled = $8
        WHERE company_id = $1 AND id = $2
        RETURNING *
        "#,
        company_id,
        params.id,
        params.name,
        params.description,
        params.system_message,
        now,
        params.is_code_interpreter_enabled,
        params.is_web_browser_enabled,
    )
    .fetch_one(executor)
    .await?)
}

/// Update `is_enabled` field for agent by id.
///
/// # Errors
///
/// Returns error if agent with given id does not exist.
/// Returns error if any error occurs while accessing database.
pub async fn update_is_enabled<'a, E>(
    executor: E,
    company_id: Uuid,
    id: Uuid,
    is_enabled: bool,
) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    query!(
        "UPDATE agents SET is_enabled = $3 WHERE company_id = $1 AND id = $2",
        company_id,
        id,
        is_enabled
    )
    .execute(executor)
    .await?;

    Ok(())
}

/// Delete agent.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn delete<'a, E>(executor: E, company_id: Uuid, id: Uuid) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    query!(
        "DELETE FROM agents WHERE company_id = $1 AND id = $2",
        company_id,
        id
    )
    .execute(executor)
    .await?;

    Ok(())
}
