// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use std::fmt::{Display, Formatter};

use chrono::{DateTime, Utc};
use markdown::to_html;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use uuid::Uuid;

use crate::clients::openai::ToolCalls;

#[derive(Serialize, Deserialize, Debug, sqlx::Type, Default, PartialEq, Clone, Copy)]
pub enum Role {
    #[default]
    System,
    User,
    Assistant,
    Tool,
    CodeInterpreter,
}

impl Display for Role {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<String> for Role {
    fn from(role: String) -> Self {
        match role.as_str() {
            "System" => Role::System,
            "Assistant" => Role::Assistant,
            "Tool" => Role::Tool,
            "CodeInterpreter" => Role::CodeInterpreter,
            _ => Role::User,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, sqlx::Type, PartialEq, Default, Clone, Copy)]
pub enum Status {
    #[default]
    Writing,
    WaitingForToolCall,
    Completed,
    Failed,
    ToolCallDenied,
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<String> for Status {
    fn from(status: String) -> Self {
        match status.as_str() {
            "Writing" => Status::Writing,
            "WaitingForToolCall" => Status::WaitingForToolCall,
            "Failed" => Status::Failed,
            "ToolCallDenied" => Status::ToolCallDenied,
            _ => Status::Completed,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, sqlx::FromRow)]
pub struct Message {
    pub id: Uuid,
    pub company_id: Uuid,
    pub chat_id: Uuid,
    pub agent_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub status: Status,
    pub role: Role,
    #[serde(serialize_with = "serialize_content")]
    pub content: Option<String>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub tool_calls: Option<Value>,
    pub tool_call_id: Option<String>,
    pub is_self_reflection: bool,
    pub is_internal_tool_output: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Message {
    #[must_use]
    pub fn tool_calls(&self) -> ToolCalls {
        match self.tool_calls {
            Some(ref tool_calls) => match serde_json::from_value(tool_calls.clone()) {
                Ok(tool_calls) => tool_calls,
                Err(_) => ToolCalls::default(),
            },
            None => ToolCalls::default(),
        }
    }
}

/// Safely render markdown in a message as an untrusted user input.
fn serialize_content<S>(
    content: &Option<String>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&to_html(content.as_ref().unwrap_or(&String::new())))
}
