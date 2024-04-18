// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use std::{
    fmt::{self, Display, Formatter},
    path::PathBuf,
};

use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::Result;

#[derive(Serialize, Deserialize, Debug, sqlx::Type, PartialEq, Default, Clone, Copy)]
pub enum Status {
    /// Task is in draft and has not been selected for execution yet.
    #[default]
    Draft,
    /// Task is selected for execution.
    ToDo,
    /// Task is currently being executed.
    InProgress,
    /// Task is waiting for a user input.
    WaitingForUser,
    /// Task is completed.
    Done,
    /// Task execution failed.
    Failed,
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<String> for Status {
    fn from(status: String) -> Self {
        match status.as_str() {
            "ToDo" => Status::ToDo,
            "InProgress" => Status::InProgress,
            "WaitingForUser" => Status::WaitingForUser,
            "Done" => Status::Done,
            "Failed" => Status::Failed,
            _ => Status::Draft,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Task {
    pub id: i32,
    pub company_id: i32,
    pub user_id: i32,
    pub agent_id: i32,
    /// Chat from which this task was created.
    pub origin_chat_id: Option<i32>,
    /// Chat from which this task is being controlled (between the user and the Bridge).
    pub control_chat_id: Option<i32>,
    /// Chat in which this task is being executed (between the Bridge and the agent).
    pub execution_chat_id: Option<i32>,
    pub title: String,
    pub summary: String,
    pub status: Status,
    /// Task's parent ids in a form of `1/2/3`. `None` for root tasks.
    pub ancestry: Option<String>,
    pub ancestry_level: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    /// Returns parent id of the task.
    ///
    /// # Errors
    ///
    /// Returns error if there was a problem while parsing parent id.
    pub fn parent_id(&self) -> Result<Option<i32>> {
        Ok(match self.ancestry {
            Some(ref ancestry) => {
                let segment = ancestry
                    .split('/')
                    .last()
                    .context("No segments found in ancestry")?;

                Some(
                    segment.parse::<i32>().with_context(|| {
                        "Failed to parse parent id from ancestry segment {segment}"
                    })?,
                )
            }
            None => None,
        })
    }

    /// Returns parent ids of the task.
    ///
    /// # Errors
    ///
    /// Returns error if there was a problem while parsing parent ids.
    pub fn parent_ids(&self) -> Result<Option<Vec<i32>>> {
        Ok(match self.ancestry {
            Some(ref ancestry) => Some(
                ancestry
                    .split('/')
                    .map(|segment| {
                        segment.parse::<i32>().with_context(|| {
                            "Failed to parse parent id from ancestry segment {segment}"
                        })
                    })
                    .collect::<std::result::Result<Vec<i32>, _>>()?,
            ),
            None => None,
        })
    }

    #[must_use]
    pub fn children_ancestry(&self) -> String {
        match self.ancestry {
            Some(ref ancestry) => format!("{}/{}", ancestry, self.id),
            None => self.id.to_string(),
        }
    }

    /// Returns workdir for the task.
    ///
    /// # Errors
    ///
    /// Returns error if there was a problem while building workdir path.
    pub async fn workdir(&self, root: &PathBuf) -> Result<PathBuf> {
        let dir = format!(
            "wd-task-{}",
            self.workdir_id().context("Failed to get workdir ID")?
        );

        Ok(root.join(dir))
    }

    fn workdir_id(&self) -> Result<i32> {
        Ok(match self.ancestry {
            Some(ref ancestry) => ancestry
                .split('/')
                .collect::<Vec<_>>()
                .first()
                .context("No segments found in ancestry")?
                .parse::<i32>()
                .context("Failed to parse workdir id")?,
            None => self.id,
        })
    }
}
