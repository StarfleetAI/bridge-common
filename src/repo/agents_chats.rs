// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use anyhow::Context;
use sqlx::{query, query_as, Executor, Postgres};
use uuid::Uuid;

use crate::types::{agents_chats::AgentsChat, Result};

/// List all agents for all chats.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn list<'a, E>(executor: E, company_id: Uuid) -> Result<HashMap<Uuid, Vec<Uuid>>>
where
    E: Executor<'a, Database = Postgres>,
{
    // TODO: this is incredibly inefficient when there are many chats. Consider using
    //       a different approach.
    let rows: Vec<AgentsChat> = query_as!(
        AgentsChat,
        "SELECT * FROM agents_chats WHERE company_id = $1",
        company_id
    )
    .fetch_all(executor)
    .await
    .with_context(|| "Failed to fetch agents for chat")?;

    let mut chat_agents: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

    for row in rows {
        chat_agents
            .entry(row.chat_id)
            .or_default()
            .push(row.agent_id);
    }

    Ok(chat_agents)
}

/// Add agent to chat.
///
/// # Errors
///
/// Returns error if there was a problem while creating `agents_chats` record.
pub async fn create<'a, E>(
    executor: E,
    company_id: Uuid,
    agent_id: Uuid,
    chat_id: Uuid,
) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    query!(
        "INSERT INTO agents_chats (company_id, agent_id, chat_id) VALUES ($1, $2, $3)",
        company_id,
        agent_id,
        chat_id
    )
    .execute(executor)
    .await
    .with_context(|| "Failed to create `agents_chats` record")?;

    Ok(())
}

/// Remove agents from chat.
///
/// # Errors
///
/// Returns error if there was a problem while deleting `agents_chats` records.
pub async fn delete_for_chat<'a, E>(executor: E, company_id: Uuid, chat_id: Uuid) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    query!(
        "DELETE FROM agents_chats WHERE company_id = $1 AND chat_id = $2",
        company_id,
        chat_id
    )
    .execute(executor)
    .await
    .with_context(|| "Failed to delete `agents_chats` records")?;

    Ok(())
}
