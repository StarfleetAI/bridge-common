// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use std::fmt::{self, Display, Formatter};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, sqlx::Type, Default, PartialEq, Clone)]
pub enum Kind {
    #[default]
    Direct,
    Control,
    Execution,
}

impl Display for Kind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<String> for Kind {
    fn from(kind: String) -> Self {
        match kind.as_str() {
            "Control" => Kind::Control,
            "Execution" => Kind::Execution,
            _ => Kind::Direct,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Chat {
    pub id: Uuid,
    pub company_id: Uuid,
    pub model_id: Option<Uuid>,
    pub title: String,
    pub is_pinned: bool,
    pub kind: Kind,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
