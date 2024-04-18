// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use chrono::{DateTime, Utc};

pub struct Agent {
    pub id: i32,
    pub company_id: i32,
    pub name: String,
    pub description: String,
    pub system_message: String,
    pub is_enabled: bool,
    pub is_code_interpreter_enabled: bool,
    pub is_web_browser_enabled: bool,
    pub execution_steps_limit: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
