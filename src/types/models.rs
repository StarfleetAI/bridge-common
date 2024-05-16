// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/";
const GROQ_API_URL: &str = "https://api.groq.com/openai/v1/";

#[derive(
    Serialize, Deserialize, Debug, sqlx::Type, Default, PartialEq, Eq, Clone, Ord, PartialOrd,
)]
pub enum Provider {
    #[default]
    OpenAI,
    Groq,
}

impl From<String> for Provider {
    fn from(s: String) -> Self {
        match s.as_str() {
            "Groq" => Provider::Groq,
            _ => Provider::OpenAI,
        }
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Model {
    // Unique identifier of the model
    pub id: Uuid,
    // Unique identifier of the company
    pub company_id: Uuid,
    // Provider of the model
    pub provider: Provider,
    // Name of the model (e.g. `gpt-4-turbo-preview`)
    pub name: String,
    // Context window size
    pub context_length: i32,
    // Maximum new tokens model can generate
    pub max_tokens: i64,
    // If model can take text input
    pub text_in: bool,
    // If model can generate text output
    pub text_out: bool,
    // If model can take image input
    pub image_in: bool,
    // If model can generate image output
    pub image_out: bool,
    // If model can take audio input
    pub audio_in: bool,
    // If model can generate audio output
    pub audio_out: bool,
    // If model has function calling capabilities
    pub function_calling: bool,
    // Base URL for the model's API. Leave empty to use provider's default
    pub api_url: Option<String>,
    // API key for the API. Leave empty to use provider's default
    pub api_key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Model {
    #[must_use]
    pub fn api_url_or_default(&self) -> &str {
        match self.api_url {
            Some(ref url) => url,
            None => match self.provider {
                Provider::OpenAI => OPENAI_API_URL,
                Provider::Groq => GROQ_API_URL,
            },
        }
    }
}
