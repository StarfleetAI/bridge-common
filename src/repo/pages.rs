// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, Executor, Postgres};
use uuid::Uuid;

use crate::types::{
    pages::{Page, ShortPage},
    Result,
};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct CreateParams {
    pub title: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct UpdateParams {
    pub title: String,
    pub text: String,
}

/// Create page.
///
/// # Errors
///
/// Returns error if there was a problem while creating page.
pub async fn create<'a, E>(executor: E, company_id: Uuid, params: CreateParams) -> Result<Page>
where
    E: Executor<'a, Database = Postgres>,
{
    let current_datetime = Utc::now();

    Ok(query_as!(
        Page,
        r#"
        INSERT INTO pages (company_id, title, text, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $4)
        RETURNING *
        "#,
        company_id,
        params.title,
        params.text,
        current_datetime
    )
    .fetch_one(executor)
    .await?)
}

/// List all pages.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
pub async fn list<'a, E>(executor: E, company_id: Uuid) -> Result<Vec<ShortPage>>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_as!(
        ShortPage,
        "SELECT id, title, created_at, updated_at FROM pages WHERE company_id = $1",
        company_id
    )
    .fetch_all(executor)
    .await?)
}

/// Get page by id.
///
/// # Errors
///
/// Returns error if there was a problem while fetching page.
pub async fn get<'a, E>(executor: E, company_id: Uuid, id: Uuid) -> Result<Page>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_as!(
        Page,
        "SELECT * FROM pages WHERE company_id = $1 AND id = $2",
        company_id,
        id
    )
    .fetch_one(executor)
    .await?)
}

/// Update page text.
///
/// # Errors
///
/// Returns error if there was a problem while updating page text.
pub async fn update<'a, E>(
    executor: E,
    company_id: Uuid,
    id: Uuid,
    data: UpdateParams,
) -> Result<Page>
where
    E: Executor<'a, Database = Postgres>,
{
    let current_datetime = Utc::now();

    Ok(query_as!(
        Page,
        r#"
        UPDATE pages
        SET title = $3, text = $4, updated_at = $5
        WHERE company_id = $1 AND id = $2
        RETURNING *
        "#,
        company_id,
        id,
        data.title,
        data.text,
        current_datetime
    )
    .fetch_one(executor)
    .await?)
}

/// Delete page.
///
/// # Errors
///
/// Returns error if there was a problem while deleting page.
pub async fn delete<'a, E>(executor: E, company_id: Uuid, id: Uuid) -> Result<()>
where
    E: Executor<'a, Database = Postgres>,
{
    query!(
        "DELETE FROM pages WHERE company_id = $1 AND id = $2",
        company_id,
        id
    )
    .execute(executor)
    .await?;

    Ok(())
}
