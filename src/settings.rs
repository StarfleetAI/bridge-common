// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::types::models::Provider;

const DEFAULT_EMBEDDINGS_MODEL: &str = "sentence-transformers/all-MiniLM-L6-v2";
const DEFAULT_MODEL: &str = "OpenAI/gpt-4-turbo";
const DEFAULT_EXECUTION_STEPS_LIMIT: i64 = 12;
const DEFAULT_PLANNING_DEPTH_LIMIT: u8 = 5;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Embeddings {
    #[serde(default = "default_embeddings_model")]
    pub model: String,
}

fn default_embeddings_model() -> String {
    DEFAULT_EMBEDDINGS_MODEL.to_string()
}

impl Default for Embeddings {
    fn default() -> Self {
        Self {
            model: DEFAULT_EMBEDDINGS_MODEL.to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tasks {
    pub execution_concurrency: u16,
    #[serde(default = "default_planning_depth_limit")]
    pub planning_depth_limit: u8,
}

impl Default for Tasks {
    fn default() -> Self {
        Self {
            execution_concurrency: 1,
            planning_depth_limit: DEFAULT_PLANNING_DEPTH_LIMIT,
        }
    }
}

fn default_planning_depth_limit() -> u8 {
    DEFAULT_PLANNING_DEPTH_LIMIT
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Agents {
    #[serde(default = "default_execution_steps_limit")]
    pub execution_steps_limit: i64,
}

fn default_execution_steps_limit() -> i64 {
    DEFAULT_EXECUTION_STEPS_LIMIT
}

impl Default for Agents {
    fn default() -> Self {
        Self {
            execution_steps_limit: DEFAULT_EXECUTION_STEPS_LIMIT,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    #[serde(default = "default_model")]
    pub default_model: String,
    #[serde(default)]
    pub api_keys: BTreeMap<Provider, String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub agents: Agents,
    #[serde(default)]
    pub embeddings: Embeddings,
    #[serde(default)]
    pub tasks: Tasks,
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> std::result::Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

fn default_model() -> String {
    DEFAULT_MODEL.to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_model: DEFAULT_MODEL.to_string(),
            api_keys: BTreeMap::new(),
            agents: Agents::default(),
            embeddings: Embeddings::default(),
            tasks: Tasks::default(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to parse settings: {0}")]
    JsonDeserialization(serde_json::Error),
}

impl TryFrom<Value> for Settings {
    type Error = Error;

    fn try_from(value: Value) -> std::result::Result<Self, Self::Error> {
        serde_json::from_value(value).map_err(Self::Error::JsonDeserialization)
    }
}
