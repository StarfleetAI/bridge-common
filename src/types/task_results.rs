// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(
    Serialize, Deserialize, Debug, sqlx::Type, Default, PartialEq, Eq, Clone, Copy, Ord, PartialOrd,
)]
pub enum Kind {
    #[default]
    Text,
    Url,
}

impl From<String> for Kind {
    fn from(kind: String) -> Self {
        match kind.as_str() {
            "Url" => Kind::Url,
            _ => Kind::Text,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskResult {
    pub id: Uuid,
    pub company_id: Uuid,
    pub agent_id: Uuid,
    pub task_id: Uuid,
    pub kind: Kind,
    pub data: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
