// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::clients::openai::Function;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Ability {
    pub id: i32,
    pub company_id: i32,
    pub name: String,
    pub description: String,
    pub code: String,
    pub parameters_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Ability {
    /// Constructs virtual ability for a function.
    ///
    /// # Panics
    ///
    /// Panics if `parameters_json` cannot be serialized.
    #[must_use]
    pub fn for_fn(description: &str, parameters_json: &Value) -> Self {
        Self {
            description: description.to_string(),
            parameters_json: parameters_json.clone(),

            ..Default::default()
        }
    }

    #[must_use]
    pub fn function(&self) -> Function {
        serde_json::from_value(self.parameters_json.clone()).unwrap_or_default()
    }
}
