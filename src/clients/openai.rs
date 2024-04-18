// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, ops::Deref};

use anyhow::Context;
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use crate::types::Result;

pub struct Client {
    pub api_key: String,
    pub api_url: String,
    pub user_agent: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "role")]
pub enum Message {
    #[serde(rename = "system")]
    System {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    #[serde(rename = "user")]
    User {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    #[serde(rename = "assistant")]
    Assistant {
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Value>,
    },
    #[serde(rename = "tool")]
    Tool {
        content: String,
        tool_call_id: String,
    },
}

impl Message {
    #[must_use]
    pub fn tool_calls(&self) -> ToolCalls {
        match self {
            Message::Assistant { tool_calls, .. } => match tool_calls {
                Some(tool_calls) => match serde_json::from_value(tool_calls.clone()) {
                    Ok(tool_calls) => tool_calls,
                    Err(_) => ToolCalls::default(),
                },
                None => ToolCalls::default(),
            },
            _ => ToolCalls::default(),
        }
    }
}

impl TryFrom<crate::types::messages::Message> for Message {
    type Error = anyhow::Error;

    fn try_from(
        message: crate::types::messages::Message,
    ) -> std::result::Result<Self, Self::Error> {
        Ok(match message.role {
            crate::types::messages::Role::System => Message::System {
                content: message
                    .content
                    .with_context(|| "Failed to get message content")?,
                name: None,
            },
            crate::types::messages::Role::User => Message::User {
                content: message
                    .content
                    .with_context(|| "Failed to get message content")?,
                name: None,
            },
            crate::types::messages::Role::CodeInterpreter => Message::User {
                content: message
                    .content
                    .with_context(|| "Failed to get message content")?,
                name: Some("Code-Interpreter".to_string()),
            },
            crate::types::messages::Role::Assistant => Message::Assistant {
                content: message.content,
                name: None,
                tool_calls: message.tool_calls,
            },
            crate::types::messages::Role::Tool => Message::Tool {
                content: message
                    .content
                    .with_context(|| "Failed to get message content")?,
                tool_call_id: message
                    .tool_call_id
                    .with_context(|| "Failed to get tool call id")?,
            },
        })
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct ToolCalls(pub Vec<ToolCall>);

impl Deref for ToolCalls {
    type Target = Vec<ToolCall>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: ToolType,
    pub function: FunctionCall,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ToolType {
    #[serde(rename = "function")]
    Function,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tool {
    #[serde(rename = "type")]
    pub type_: String,
    pub function: Function,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Function {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<FunctionParameters>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FunctionParameters {
    #[serde(rename = "type")]
    pub type_: String,
    pub properties: HashMap<String, FunctionPropertyValue>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FunctionPropertyValue {
    #[serde(rename = "type")]
    pub type_: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<FunctionParameters>,
}

#[derive(Debug, Serialize, Default)]
pub struct CreateChatCompletionRequest<'a> {
    pub model: &'a str,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    pub stream: bool,
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub object: String,
    pub created: u32,
    pub model: String,
    pub system_fingerprint: Option<String>,
    pub choices: Vec<ChunkChoice>,
}

#[derive(Debug, Deserialize)]
pub struct ChunkChoice {
    pub index: u32,
    pub delta: Message,
    pub finish_reason: Option<String>,
    pub logprobs: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletion {
    pub created: u32,
    pub id: String,
    pub model: String,
    pub object: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub finish_reason: String,
    pub index: u32,
    pub message: Message,
    pub logprobs: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub completion_tokens: u32,
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

impl<'a> Client {
    #[must_use]
    pub fn new(api_key: &'a str, api_url: &'a str, user_agent: &'a str) -> Self {
        Self {
            api_key: api_key.to_string(),
            api_url: api_url.to_string(),
            user_agent: user_agent.to_string(),
        }
    }

    /// Creates a streaming chat completion.
    ///
    /// # Errors
    ///
    /// Returns error if there was a problem while making the API call.
    pub async fn create_chat_completion_stream(
        &self,
        mut request: CreateChatCompletionRequest<'_>,
    ) -> Result<Response> {
        request.stream = true;

        self.post_stream("chat/completions", &request).await
    }

    /// Creates a chat completion.
    ///
    /// # Errors
    ///
    /// Returns error if there was a problem while making the API call.
    pub async fn create_chat_completion(
        &self,
        request: CreateChatCompletionRequest<'_>,
    ) -> Result<ChatCompletion> {
        self.post("chat/completions", &request).await
    }

    /// Sends a stream POST request, returns the response for further processing.
    ///
    /// # Errors
    ///
    /// Returns error if there was a problem while sending the request or
    /// deserializing the response.
    pub async fn post_stream<B>(&self, endpoint: &str, body: B) -> Result<Response>
    where
        B: serde::Serialize,
    {
        let url = format!("{}{endpoint}", self.api_url);
        let client = reqwest::Client::new();

        let body =
            serde_json::to_value(body).with_context(|| "Failed to serialize request body")?;

        debug!("Inference API request: {:?}", body.to_string());

        Ok(client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("User-Agent", self.user_agent.clone())
            .json(&body)
            .send()
            .await
            .with_context(|| "Failed to send request")?)
    }

    /// Sends a POST request, deserializes the response to the given type.
    ///
    /// # Errors
    ///
    /// Returns error if there was a problem while sending the request or
    /// deserializing the response.
    pub async fn post<T, B>(&self, endpoint: &str, body: B) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize,
    {
        let url = format!("{}{endpoint}", self.api_url);
        let client = reqwest::Client::new();

        let body =
            serde_json::to_value(body).with_context(|| "Failed to serialize request body")?;
        debug!("Inference API request: {:?}", body.to_string());

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("User-Agent", self.user_agent.clone())
            .json(&body)
            .send()
            .await
            .with_context(|| "Failed to send request")?
            .text()
            .await
            .with_context(|| "Failed to get response text")?;

        debug!("Inference API response: {:?}", response);

        Ok(serde_json::from_str(&response)?)
    }
}
