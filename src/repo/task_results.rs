// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use chrono::Utc;
use markdown::to_html;
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, Executor, Postgres};
use uuid::Uuid;

use crate::types::{
    task_results::{Kind, TaskResult},
    Result,
};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct CreateParams {
    pub agent_id: Uuid,
    pub task_id: Uuid,
    pub kind: Kind,
    pub data: String,
}

/// Create task result.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn create<'a, E>(
    executor: E,
    company_id: Uuid,
    params: CreateParams,
) -> Result<TaskResult>
where
    E: Executor<'a, Database = Postgres>,
{
    let now = Utc::now();

    Ok(query_as!(
        TaskResult,
        r#"
        INSERT INTO task_results (company_id, agent_id, task_id, kind, data, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $6)
        RETURNING *
        "#,
        company_id,
        params.agent_id,
        params.task_id,
        params.kind as Kind,
        params.data,
        now,
    )
    .fetch_one(executor)
    .await?)
}

/// List task results by task id
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn list<'a, E>(executor: E, company_id: Uuid, task_id: Uuid) -> Result<Vec<TaskResult>>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_as!(
        TaskResult,
        "SELECT * FROM task_results WHERE company_id = $1 AND task_id = $2 ORDER BY id ASC",
        company_id,
        task_id
    )
    .fetch_all(executor)
    .await?)
}

/// Get text data by task result id
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn get_text_data<'a, E>(executor: E, company_id: Uuid, id: Uuid) -> Result<String>
where
    E: Executor<'a, Database = Postgres>,
{
    let task_result = query!(
        "SELECT data FROM task_results WHERE company_id = $1 AND id = $2",
        company_id,
        id
    )
    .fetch_one(executor)
    .await?;

    // Safely render markdown in a result as an untrusted input.
    let serialized_data = to_html(task_result.data.as_str());
    Ok(serialized_data)
}

/// Delete task results by task id
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn delete_for_task<'a, E>(executor: E, company_id: Uuid, task_id: Uuid) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    query!(
        "DELETE FROM task_results WHERE company_id = $1 AND task_id = $2",
        company_id,
        task_id
    )
    .execute(executor)
    .await?;

    Ok(())
}
