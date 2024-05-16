// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use uuid::Uuid;

pub struct AgentAbility {
    pub company_id: Uuid,
    pub agent_id: Uuid,
    pub ability_id: Uuid,
}
