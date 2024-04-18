// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use chrono::Utc;
use serde_json::json;
use sqlx::{query, query_as, Executor, Postgres};

use crate::{
    clients::openai::Function,
    types::{abilities::Ability, Result},
};

pub struct CreateParams {
    pub name: String,
    pub description: String,
    pub code: String,
    pub parameters_json: Function,
}

pub struct UpdateParams {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub code: String,
    pub parameters_json: Function,
}

/// List abilities for agent.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn list_for_agent<'a, E>(
    executor: E,
    company_id: i32,
    agent_id: i32,
) -> Result<Vec<Ability>>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_as!(
        Ability,
        r#"
        SELECT abilities.*
        FROM abilities
        INNER JOIN agent_abilities ON abilities.id = agent_abilities.ability_id
        WHERE abilities.company_id = $1 AND agent_abilities.agent_id = $2
        "#,
        company_id,
        agent_id
    )
    .fetch_all(executor)
    .await?)
}

/// List all abilities.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn list<'a, E>(executor: E, company_id: i32) -> Result<Vec<Ability>>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_as!(
        Ability,
        "SELECT * FROM abilities WHERE company_id = $1 ORDER BY id DESC",
        company_id
    )
    .fetch_all(executor)
    .await?)
}

/// Create ability.
///
/// # Errors
///
/// Returns error if there was a problem while creating ability.
pub async fn create<'a, E>(executor: E, company_id: i32, params: CreateParams) -> Result<Ability>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_as!(
        Ability,
        r#"
        INSERT INTO abilities (company_id, name, description, code, parameters_json, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $6)
        RETURNING *
        "#,
        company_id,
        params.name,
        params.description,
        params.code,
        json!(params.parameters_json),
        Utc::now()
    )
        .fetch_one(executor)
        .await?)
}

/// Update ability.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn update<'a, E>(executor: E, company_id: i32, params: UpdateParams) -> Result<Ability>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_as!(
        Ability,
        r#"
        UPDATE abilities
        SET name = $3, description = $4, code = $5, parameters_json = $6, updated_at = $7
        WHERE company_id = $1 AND id = $2
        RETURNING *
        "#,
        company_id,
        params.id,
        params.name,
        params.description,
        params.code,
        json!(params.parameters_json),
        Utc::now()
    )
    .fetch_one(executor)
    .await?)
}

/// Delete ability.
///
/// # Errors
///
/// Returns error if there was a problem while deleting ability.
pub async fn delete<'a, E>(executor: E, company_id: i32, id: i32) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    query!(
        "DELETE FROM abilities WHERE company_id = $1 AND id = $2",
        company_id,
        id
    )
    .execute(executor)
    .await
    .with_context(|| "Failed to delete ability")?;

    Ok(())
}
