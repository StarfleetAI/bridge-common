// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Context};
use chrono::Utc;
use sqlx::{query, query_as, query_scalar, Executor, Postgres};

use crate::types::{
    pagination::Pagination,
    tasks::{Status, Task},
    Result,
};

#[derive(Debug, Default)]
pub struct CreateParams<'a> {
    pub agent_id: i32,
    /// Chat from which this task was created.
    pub origin_chat_id: Option<i32>,
    pub title: &'a str,
    pub summary: Option<&'a str>,
    pub status: Status,
    /// Task's parent ids in a form of `1/2/3`. `None` for root tasks.
    pub ancestry: Option<&'a str>,
}

pub struct UpdateParams<'a> {
    pub id: i32,
    pub title: &'a str,
    pub summary: &'a str,
    pub agent_id: i32,
}

/// Gets root task for execution.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn get_root_for_execution<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
) -> Result<Option<Task>> {
    Ok(query_as!(
        Task,
        r#"
        SELECT *
        FROM tasks
        WHERE ancestry IS NULL
        AND company_id = $1 AND status = $2
        ORDER BY created_at ASC
        LIMIT 1
        "#,
        company_id,
        Status::ToDo.to_string(),
    )
    .fetch_optional(executor)
    .await?)
}

/// Gets child task for execution.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn get_child_for_execution<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    ancestry: &'a str,
) -> Result<Option<Task>> {
    Ok(query_as!(
        Task,
        r#"
        SELECT *
        FROM tasks
        WHERE company_id = $1 AND ancestry = $2
        AND status != $3
        ORDER BY created_at ASC
        LIMIT 1
        "#,
        company_id,
        ancestry,
        Status::Done.to_string(),
    )
    .fetch_optional(executor)
    .await?)
}

/// List all tasks.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn list_roots<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    pagination: Pagination,
) -> Result<Vec<Task>> {
    if pagination.page < 1 {
        return Err(anyhow!("`page` number must be greater than 0").into());
    }

    if pagination.per_page < 1 {
        return Err(anyhow!("`per_page` number must be greater than 0").into());
    }

    let offset = (pagination.page - 1) * pagination.per_page;

    Ok(query_as!(
        Task,
        r#"
        SELECT *
        FROM tasks
        WHERE company_id = $1 AND ancestry IS NULL
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
        company_id,
        pagination.per_page,
        offset,
    )
    .fetch_all(executor)
    .await?)
}

/// List root tasks by status
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn list_roots_by_status<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    status: Status,
    pagination: Pagination,
) -> Result<Vec<Task>> {
    if pagination.page < 1 {
        return Err(anyhow!("`page` number must be greater than 0").into());
    }

    if pagination.per_page < 1 {
        return Err(anyhow!("`per_page` number must be greater than 0").into());
    }

    let offset = (pagination.page - 1) * pagination.per_page;

    Ok(query_as!(
        Task,
        r#"
        SELECT *
        FROM tasks
        WHERE company_id = $1 AND ancestry IS NULL AND status = $2
        ORDER BY created_at DESC
        LIMIT $3 OFFSET $4
        "#,
        company_id,
        status.to_string(),
        pagination.per_page,
        offset,
    )
    .fetch_all(executor)
    .await?)
}

/// List all children tasks for given task.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn list_all_children<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    ancestry: &'a str,
) -> Result<Vec<Task>> {
    let like_ancestry = format!("{ancestry}/%");

    Ok(query_as!(
        Task,
        r#"
        SELECT *
        FROM tasks
        WHERE company_id = $1 AND ancestry = $2 OR ancestry LIKE $3
        ORDER BY created_at ASC
        "#,
        company_id,
        ancestry,
        like_ancestry,
    )
    .fetch_all(executor)
    .await?)
}

/// Returns count of all children tasks for given task's ancestry.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn get_all_children_count<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    task: &Task,
) -> Result<i64> {
    let ancestry = task.children_ancestry();
    let like_ancestry = format!("{ancestry}/%");

    let count: i64 = query_scalar!(
        r#"
        SELECT COUNT(*)
        FROM tasks
        WHERE company_id = $1 AND ancestry = $2 OR ancestry LIKE $3
        "#,
        company_id,
        ancestry,
        like_ancestry,
    )
    .fetch_one(executor)
    .await?
    .unwrap_or_default();

    Ok(count)
}

/// Returns true if all sibling tasks are done.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn is_all_siblings_done<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    task: &Task,
) -> Result<bool> {
    let count: i64 = query_scalar!(
        r#"
        SELECT COUNT(*) FROM tasks
        WHERE company_id = $1 AND ancestry = $2 AND status != $3
        "#,
        company_id,
        task.ancestry,
        Status::Done.to_string(),
    )
    .fetch_one(executor)
    .await?
    .unwrap_or_default();

    Ok(count == 0)
}

/// List direct children tasks for given task.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn list_direct_children<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    task: &Task,
) -> Result<Vec<Task>> {
    let ancestry = task.children_ancestry();

    Ok(query_as!(
        Task,
        "SELECT * FROM tasks WHERE company_id = $1 AND ancestry = $2 ORDER BY created_at ASC",
        company_id,
        ancestry,
    )
    .fetch_all(executor)
    .await?)
}

/// Create new task.
///
/// # Errors
///
/// Returns error if there was a problem while inserting new task.
pub async fn create<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    params: CreateParams<'a>,
) -> Result<Task> {
    let now = Utc::now();

    let ancestry_level = match params.ancestry {
        Some(ancestry) => {
            let count = ancestry.split('/').count();

            match count.try_into() {
                Ok(ancestry_level) => ancestry_level,
                Err(_) => return Err(anyhow!("Too many ancestors").into()),
            }
        }
        None => 0,
    };

    Ok(query_as!(
        Task,
        r#"
        INSERT INTO tasks (
            company_id, agent_id, origin_chat_id,
            title, summary, status,
            ancestry, ancestry_level, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)
        RETURNING *
        "#,
        company_id,
        params.agent_id,
        params.origin_chat_id,
        params.title,
        params.summary,
        params.status.to_string(),
        params.ancestry,
        ancestry_level,
        now,
    )
    .fetch_one(executor)
    .await?)
}

/// Update task title or/and summary by id
///
/// # Errors
///
/// Returns error if there was a problem while updating task.
pub async fn update<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    params: UpdateParams<'a>,
) -> Result<Task> {
    let now = Utc::now();

    let task = query_as!(
        Task,
        r#"
        UPDATE tasks
        SET
            title = COALESCE($3, title),
            summary = COALESCE($4, summary),
            agent_id = $5,
            updated_at = $6
        WHERE company_id = $1 AND id = $2
        RETURNING *"#,
        company_id,
        params.id,
        params.title,
        params.summary,
        params.agent_id,
        now,
    )
    .fetch_one(executor)
    .await?;

    Ok(task)
}

/// Update task status by id.
///
/// # Errors
///
/// Returns error if there was a problem while updating task status.
pub async fn update_status<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    id: i32,
    status: Status,
) -> Result<Task> {
    let now = Utc::now();
    let task = query_as!(
        Task,
        r#"
        UPDATE tasks
        SET
            status = $3,
            updated_at = $4
        WHERE company_id = $1 AND id = $2
        RETURNING *
        "#,
        company_id,
        id,
        status.to_string(),
        now,
    )
    .fetch_one(executor)
    .await?;

    Ok(task)
}

/// Get task by execution chat id.
///
/// # Errors
///
/// Returns error if there was a problem while fetching task.
pub async fn get_by_execution_chat_id<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    execution_chat_id: i32,
) -> Result<Task> {
    let task = query_as!(
        Task,
        "SELECT * FROM tasks WHERE company_id = $1 ANd execution_chat_id = $2",
        company_id,
        execution_chat_id,
    )
    .fetch_one(executor)
    .await?;

    Ok(task)
}

/// Update task execution chat id by id.
///
/// # Errors
///
/// Returns error if there was a problem while updating task execution chat id.
pub async fn update_execution_chat_id<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    id: i32,
    execution_chat_id: i32,
) -> Result<()> {
    let now = Utc::now();
    query!(
        r#"
        UPDATE tasks
        SET
            execution_chat_id = $3,
            updated_at = $4
        WHERE company_id = $1 AND id = $2
        "#,
        company_id,
        id,
        execution_chat_id,
        now,
    )
    .execute(executor)
    .await?;

    Ok(())
}

/// Revise task by id.
///
/// # Errors
///
/// Returns error if there was a problem while revising task.
pub async fn revise<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    id: i32,
) -> Result<Task> {
    update_status(executor, company_id, id, Status::Draft).await
}

/// Execute task by id.
///
/// # Errors
///
/// Returns error if there was a problem while executing task.
pub async fn execute<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    id: i32,
) -> Result<Task> {
    update_status(executor, company_id, id, Status::ToDo).await
}

/// Start task by id.
///
/// # Errors
///
/// Returns error if there was a problem while starting task.
pub async fn start_progress<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    id: i32,
) -> Result<Task> {
    update_status(executor, company_id, id, Status::InProgress).await
}

/// Marks task as waiting for user input by id.
///
/// # Errors
///
/// Returns error if there was a problem while marking task as waiting for user input.
pub async fn wait_for_user<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    id: i32,
) -> Result<Task> {
    update_status(executor, company_id, id, Status::WaitingForUser).await
}

/// Fail task by id.
///
/// # Errors
///
/// Returns error if there was a problem while failing task.
pub async fn fail<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    id: i32,
) -> Result<Task> {
    update_status(executor, company_id, id, Status::Failed).await
}

/// Complete task by id.
///
/// # Errors
///
/// Returns error if there was a problem while completing task.
pub async fn complete<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    id: i32,
) -> Result<Task> {
    update_status(executor, company_id, id, Status::Done).await
}

/// Get task by id.
///
/// # Errors
///
/// Returns error if there was a problem while fetching task.
pub async fn get<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    id: i32,
) -> Result<Task> {
    Ok(query_as!(
        Task,
        "SELECT * FROM tasks WHERE company_id = $1 AND id = $2",
        company_id,
        id
    )
    .fetch_one(executor)
    .await?)
}

/// Delete task by id.
///
/// # Errors
///
/// Returns error if there was a problem while deleting task.
pub async fn delete<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    id: i32,
) -> Result<()> {
    query!(
        "DELETE FROM tasks WHERE company_id = $1 AND id = $2",
        company_id,
        id
    )
    .execute(executor)
    .await?;

    Ok(())
}

/// Delete child tasks by parent id and ancestry.
///
/// # Errors
///
/// Returns error if there was a problem while deleting tasks.
pub async fn delete_children<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    id: i32,
    ancestry: Option<&'a str>,
) -> Result<()> {
    let children_ancestry = if let Some(ancestry) = ancestry {
        format!("{ancestry}/{id}/%")
    } else {
        format!("{id}/%")
    };

    query!(
        "DELETE FROM tasks WHERE company_id = $1 AND ancestry LIKE $2",
        company_id,
        children_ancestry
    )
    .execute(executor)
    .await?;

    Ok(())
}

/// Delete tasks from chat.
///
/// # Errors
///
/// Returns error if there was a problem while deleting `tasks` records.
pub async fn delete_for_chat<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    chat_id: i32,
) -> Result<()> {
    query!(
        "DELETE FROM tasks WHERE company_id = $1 AND (origin_chat_id = $2 OR control_chat_id = $2 OR execution_chat_id = $2)",
        company_id,
        chat_id
    )
        .execute(executor)
        .await?;

    Ok(())
}

/// Transitions tasks from one status to another.
///
/// # Errors
///
/// Returns error if there was a problem while updating messages.
pub async fn transition_all<'a, E>(executor: E, from: Status, to: Status) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    query!(
        "UPDATE tasks SET status = $1 WHERE status = $2",
        to.to_string(),
        from.to_string()
    )
    .execute(executor)
    .await?;

    Ok(())
}

/// Assigns tasks to agent by id.
///
/// # Errors
///
/// Returns error if there was a problem while assigning tasks to agent.
pub async fn assign<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    task_id: i32,
    agent_id: i32,
) -> Result<()> {
    query!(
        "UPDATE tasks SET agent_id = $3 WHERE company_id = $1 AND id = $2",
        company_id,
        task_id,
        agent_id,
    )
    .execute(executor)
    .await?;

    Ok(())
}

/// Get total number of tasks by status
///
/// # Errors
///
/// Returns error if there was a problem while fetching tasks count.
pub async fn get_total_number_by_status<'a, E: Executor<'a, Database = Postgres>>(
    executor: E,
    company_id: i32,
    status: Status,
) -> Result<i32> {
    let count = query_scalar!(
        "SELECT COUNT(*) FROM tasks WHERE company_id = $1 AND status = $2",
        company_id,
        status.to_string()
    )
    .fetch_one(executor)
    .await?
    .unwrap_or_default();

    Ok(i32::try_from(count).context("Failed to convert tasks count to i32")?)
}
