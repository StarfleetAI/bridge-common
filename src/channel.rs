// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use serde::Serialize;

use crate::types::{chats::Chat, messages::Message, task_results::TaskResult, tasks::Task, Result};

#[derive(Serialize, Debug)]
#[serde(tag = "event", content = "data")]
pub enum Event<'a> {
    ChatUpdated(&'a Chat),
    MessageCreated(&'a Message),
    MessageUpdated(&'a Message),
    TaskCreated(&'a Task),
    TaskUpdated(&'a Task),
    TaskResultCreated(&'a TaskResult),
}

#[async_trait]
pub trait Emitter {
    // TODO: maybe use Option<i32> instead of i32
    async fn emit<'a>(&self, user_id: i32, event: Event<'a>) -> Result<()>;
}

pub type Channel = Box<dyn Emitter + Send + Sync>;
