// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use sqlx::{Pool, Postgres};
use tracing::warn;

use crate::{
    repo,
    settings::Settings,
    types::{chats::Chat, models::Model, Result},
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("default model `{1}` (cid: {0}) is not found in the database")]
    DefaultModelNotFound(i32, String),
}

/// Get model for a given chat.
///
/// If chat has a model assigned, it will be loaded. Otherwise, default model will be loaded.
///
/// # Errors
///
/// Returns error if there was a problem while accessing database.
/// Returns error if default model is not found in the database.
pub async fn get_for_chat(
    pool: &Pool<Postgres>,
    cid: i32,
    settings: &Settings,
    chat: &Chat,
) -> Result<Model> {
    if let Some(model_id) = chat.model_id {
        if let Ok(model) = repo::models::get(pool, cid, model_id).await {
            return Ok(model);
        }

        warn!(
            "Model `{}` for chat `{}` is not found in the database. Continuing with a default model",
            model_id, chat.id
        );
    }

    match repo::models::get_by_full_name(pool, cid, &settings.default_model).await? {
        Some(model) => Ok(model),
        None => Err(Error::DefaultModelNotFound(cid, settings.default_model.clone()).into()),
    }
}

pub async fn get_default(pool: &Pool<Postgres>, cid: i32, settings: &Settings) -> Result<Model> {
    match repo::models::get_by_full_name(pool, cid, &settings.default_model).await? {
        Some(model) => Ok(model),
        None => Err(Error::DefaultModelNotFound(cid, settings.default_model.clone()).into()),
    }
}
