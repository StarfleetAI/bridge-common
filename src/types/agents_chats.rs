// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use uuid::Uuid;

pub struct AgentsChat {
    pub company_id: Uuid,
    pub agent_id: Uuid,
    pub chat_id: Uuid,
}
