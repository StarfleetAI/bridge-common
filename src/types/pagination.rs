// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Pagination {
    pub page: i64,
    pub per_page: i64,
}
