// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Application(#[from] anyhow::Error),

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Abilities(#[from] crate::abilities::Error),
    #[error(transparent)]
    Database(#[from] crate::database::Error),
    #[error("feedback channel error: {0}")]
    Channel(anyhow::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    Browser(#[from] crate::browser::Error),
    #[error(transparent)]
    Docker(#[from] crate::docker::Error),
    #[error("embeddings error: {0}")]
    Embeddings(#[from] crate::embeddings::Error),
    #[error(transparent)]
    Executor(#[from] crate::task_executor::Error),
    #[error(transparent)]
    Messages(#[from] crate::messages::Error),
    #[error(transparent)]
    Models(#[from] crate::models::Error),
    #[error(transparent)]
    Pages(#[from] crate::pages::Error),
    #[error(transparent)]
    Planner(#[from] crate::task_planner::Error),
    #[error(transparent)]
    Settings(#[from] crate::settings::Error),
    #[error(transparent)]
    WebBrowsing(#[from] crate::tools::web_browsing::Error),
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(format!("{self:#}").as_str())
    }
}
