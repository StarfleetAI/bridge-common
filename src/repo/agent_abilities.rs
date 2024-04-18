// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use sqlx::{query, query_as, query_scalar, Executor, Postgres};

use crate::types::{agent_abilities::AgentAbility, Result};

/// List all agent abilities.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn list<'a, E>(executor: E, company_id: i32) -> Result<Vec<AgentAbility>>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_as!(
        AgentAbility,
        "SELECT * FROM agent_abilities WHERE company_id = $1",
        company_id
    )
    .fetch_all(executor)
    .await?)
}

/// Create agent ability.
///
/// # Errors
///
/// Returns error if there was a problem while creating agent ability.
pub async fn create<'a, E>(
    executor: E,
    company_id: i32,
    agent_id: i32,
    ability_id: i32,
) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    query!(
        "INSERT INTO agent_abilities (company_id, agent_id, ability_id) VALUES ($1, $2, $3)",
        company_id,
        agent_id,
        ability_id
    )
    .execute(executor)
    .await?;

    Ok(())
}

/// Delete agent ability.
///
/// # Errors
///
/// Returns error if there was a problem while deleting agent ability.
pub async fn delete_for_agent<'a, E>(executor: E, company_id: i32, agent_id: i32) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    query!(
        "DELETE FROM agent_abilities WHERE company_id = $1 AND agent_id = $2",
        company_id,
        agent_id
    )
    .execute(executor)
    .await?;

    Ok(())
}

/// Get agents count for ability.
///
/// # Errors
///
/// Returns error if there was a problem while fetching agents count for ability.
pub async fn get_agents_count<'a, E>(executor: E, company_id: i32, ability_id: i32) -> Result<i64>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_scalar!(
        "SELECT COUNT(*) FROM agent_abilities WHERE company_id = $1 AND ability_id = $2",
        company_id,
        ability_id
    )
    .fetch_one(executor)
    .await?
    .unwrap_or_default())
}
