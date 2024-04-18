// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

pub mod abilities;
pub mod agent_abilities;
pub mod agents;
pub mod agents_chats;
pub mod chats;
pub mod messages;
pub mod models;
pub mod pages;
pub mod pagination;
pub mod task_results;
pub mod tasks;

pub type Result<T> = std::result::Result<T, crate::errors::Error>;
