// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use sqlx::{query_as, Executor, Postgres};
use tracing::{instrument, trace};

use crate::types::{models::Model, Result};

/// Get model by ID.
///
/// # Errors
///
/// Returns error if there was a problem while fetching model.
// TODO: filter results by company_id
#[instrument(skip(executor))]
pub async fn get<'a, E>(executor: E, company_id: i32, id: i32) -> Result<Model>
where
    E: Executor<'a, Database = Postgres>,
{
    trace!("Getting model by id");

    Ok(query_as!(
        Model,
        "SELECT * FROM models WHERE company_id = $1 AND id = $2",
        company_id,
        id,
    )
    .fetch_one(executor)
    .await?)
}

/// Get model by full name (`provider/name`).
///
/// # Errors
///
/// Returns error if there was a problem while fetching model.
// TODO: filter results by company_id
#[instrument(skip(executor))]
pub async fn get_by_full_name<'a, E>(
    executor: E,
    company_id: i32,
    full_name: &str,
) -> Result<Option<Model>>
where
    E: Executor<'a, Database = Postgres>,
{
    trace!("Getting model by full name");
    let (provider, name) = full_name.split_once('/').context("Invalid model name")?;

    Ok(query_as!(
        Model,
        "SELECT * FROM models WHERE company_id = $1 AND provider = $2 AND name = $3",
        company_id,
        provider,
        name
    )
    .fetch_optional(executor)
    .await?)
}

/// List models
///
/// # Errors
///
/// Returns error if there was a problem while fetching models.
#[instrument(skip(executor))]
pub async fn list<'a, E>(executor: E, company_id: i32) -> Result<Vec<Model>>
where
    E: Executor<'a, Database = Postgres>,
{
    Ok(query_as!(
        Model,
        "SELECT * FROM models WHERE company_id = $1",
        company_id
    )
    .fetch_all(executor)
    .await?)
}
