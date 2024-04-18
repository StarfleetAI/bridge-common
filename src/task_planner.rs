// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use async_recursion::async_recursion;
use serde::Deserialize;
use serde_json::json;
use sqlx::{Pool, Postgres};
use tracing::info;

use crate::channel::{self, Channel};
use crate::chats::construct_tools;
use crate::clients::openai::{
    ChatCompletion, Client, CreateChatCompletionRequest, Message, ToolCalls,
};
use crate::repo;

use crate::repo::tasks::CreateParams;
use crate::settings::Settings;
use crate::types::abilities::Ability;
use crate::types::tasks::Task;
use crate::types::Result;

const PROMPT: &str = r#"You are a project manager with the objective of orchestrating task execution using your team effectively.

## Planning Guidelines

1. Ensure each task is a discrete, manageable unit of work. Avoid splitting broad concepts like "research" and "understanding", "writing" and "executing" scripts or "running a benchmark" and "analyzing results" into separate sub-tasks.
2. Assign each task to only one agent.
3. A task can have multiple sub-tasks.
4. Parent tasks have visibility over the outcomes of their sub-tasks.
5. Sub-tasks have visibility over the outcomes of their sibling tasks.
6. Tasks should be executed in a sequential manner.

## Examples

1. Simple tasks like writing a straight-forward script should not be divided into sub-tasks.
2. Complex tasks, such as those requiring internet data retrieval and script writing, should be split into two sub-tasks: data gathering and script development.
3. Straightforward queries like "tell me about Ruby on Rails" do not require planning. Avoid unnecessary task creation for such direct questions.
4. Try to keep the number of sub-tasks to a minimum to avoid task fragmentation.
5. Keep the number of nesting levels to a minimum.

## Additional Notes

1. Use the web browser sparingly to minimize user billing. Avoid researching well-known topics.
2. Eliminate "review" steps from tasks; the user will review the final results. Focus on creating meaningful, actionable tasks.
3. Plan at a single level of depth only.
4. Do not include tasks for delivering results like "save a file" or "provide a URL."
5. Keep task titles succinct and to the point.
6. When planning, you can safely assume that the working environment is set up correctly.
7. Task summary should have all the relevant information for the agent to complete the task, but avoid unnecessary details.

## Response Format

Approach each task methodically and devise a plan to achieve it. Respond with concise task titles and assigned agents only, omitting any additional explanations."#;

pub struct TaskPlanner<'a> {
    pool: &'a Pool<Postgres>,
    settings: &'a Settings,
    channel: &'a Channel,
    user_id: i32,
    user_agent: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct ExecutionPlanTask {
    pub title: String,
    pub summary: String,
    pub agent_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct ExecutionPlan {
    pub tasks: Vec<ExecutionPlanTask>,
}

#[derive(Debug, Deserialize)]
struct SfaiAssignToAgentArgs {
    agent_id: i32,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Planning is not available for tasks with status: {0:?}")]
    PlanningUnavailable(crate::types::tasks::Status),
    #[error("No tool call received from LLM")]
    NoToolCallReceived,
    #[error("Non-assistant message received from LLM")]
    NonAssistantMessage,
    #[error("cannot load model `{0}` from database")]
    CannotLoadModel(String),
    #[error("Empty plan received from LLM")]
    EmptyPlan,
}

impl<'a> TaskPlanner<'a> {
    #[must_use]
    pub fn new(
        pool: &'a Pool<Postgres>,
        channel: &'a Channel,
        settings: &'a Settings,
        user_id: i32,
        user_agent: &'a str,
    ) -> Self {
        Self {
            pool,
            settings,
            channel,
            user_id,
            user_agent,
        }
    }

    /// Plan task execution
    ///
    /// # Errors
    ///
    /// Returns error if planning is unavailable for the task status, or if there was a problem while planning the task execution.
    #[async_recursion]
    pub async fn plan(&self, task: &mut Task) -> Result<()> {
        match task.status {
            crate::types::tasks::Status::ToDo | crate::types::tasks::Status::InProgress => {
                return Err(Error::PlanningUnavailable(task.status).into())
            }
            _ => {}
        }

        info!("Planning task: {}", task.id);

        let messages = self.messages(task).await?;
        let tools = construct_tools(Self::abilities()).await?;

        let model = match repo::models::get_by_full_name(
            self.pool,
            task.company_id,
            &self.settings.default_model,
        )
        .await
        .context("Failed to get model")?
        {
            Some(model) => model,
            None => return Err(Error::CannotLoadModel(self.settings.default_model.clone()).into()),
        };

        let api_key = self
            .settings
            .api_keys
            .get(&model.provider)
            .with_context(|| format!("Failed to get api key for provider: {:?}", model.provider))?;

        // Send request to LLM
        let client = Client::new(api_key, model.api_url_or_default(), self.user_agent);
        let response = client
            .create_chat_completion(CreateChatCompletionRequest {
                model: &model.name,
                messages,
                stream: false,
                tools,
            })
            .await
            .context("Failed to create chat completion")?;

        let plan = Self::plan_from_response(&response, task)
            .context("Failed to plan a task execution")?
            .context("Empty plan received")?;

        if plan.tasks.is_empty() {
            // TODO: retry planning
            return Err(Error::EmptyPlan.into());
        }

        if plan.tasks.len() == 1 {
            task.agent_id = plan.tasks[0].agent_id;
            repo::tasks::assign(self.pool, task.company_id, task.id, task.agent_id).await?;

            self.channel
                .emit(self.user_id, channel::Event::TaskUpdated(&task))
                .await?;

            return Ok(());
        }

        let planning_depth_limit = self.settings.tasks.planning_depth_limit;

        if task.ancestry_level >= i32::from(planning_depth_limit) {
            info!(
                "The nesting level limit {} of tasks for scheduling has been reached. \
                Further planning of subtasks is stopped. \
                Current task id:{}, title:{}, ancestry_level:{}",
                planning_depth_limit, task.id, task.title, task.ancestry_level
            );
            return Ok(());
        }

        for sub_task in plan.tasks {
            let mut task = repo::tasks::create(
                self.pool,
                task.company_id,
                CreateParams {
                    title: &sub_task.title,
                    summary: Some(&sub_task.summary),
                    agent_id: sub_task.agent_id,
                    ancestry: Some(&task.children_ancestry()),
                    ..Default::default()
                },
            )
            .await?;

            self.channel
                .emit(self.user_id, channel::Event::TaskCreated(&task))
                .await?;

            // Plan sub-tasks
            self.plan(&mut task).await?;
        }

        Ok(())
    }

    fn assistant_message_tool_calls(response: &ChatCompletion) -> Result<ToolCalls> {
        let message = &response.choices[0].message;

        match &message {
            Message::Assistant { .. } => Ok(message.tool_calls()),
            _ => Err(Error::NonAssistantMessage.into()),
        }
    }

    fn plan_from_response(response: &ChatCompletion, task: &Task) -> Result<Option<ExecutionPlan>> {
        let tool_calls = Self::assistant_message_tool_calls(response)?;
        let mut plan = None;

        for tool_call in tool_calls.iter() {
            match tool_call.function.name.as_str() {
                "sfai_plan_task_execution" => {
                    plan = Some(
                        serde_json::from_str(&tool_call.function.arguments)
                            .context("Failed to parse plan")?,
                    );
                }
                "sfai_assign_to_agent" => {
                    let args: SfaiAssignToAgentArgs =
                        serde_json::from_str(&tool_call.function.arguments)
                            .context("Failed to parse `sfai_assign_to_agent` arguments")?;

                    plan = Some(ExecutionPlan {
                        tasks: vec![ExecutionPlanTask {
                            title: task.title.clone(),
                            summary: task.summary.clone(),
                            agent_id: args.agent_id,
                        }],
                    });
                }
                _ => {}
            }
        }

        Ok(plan)
    }

    async fn messages(&self, task: &Task) -> Result<Vec<Message>> {
        let agents = repo::agents::list_enabled(self.pool, task.company_id)
            .await
            .context("Failed to list agents")?
            .into_iter()
            .map(|agent| format!("- ID: {}. {}: {}", agent.id, agent.name, agent.description))
            .collect::<Vec<String>>();

        let agents = if agents.is_empty() {
            "No agents available".to_string()
        } else {
            agents.join("\n")
        };

        let summary = if task.summary.is_empty() {
            String::new()
        } else {
            format!("\n\n{}", task.summary)
        };

        Ok(vec![
            Message::System {
                content: PROMPT.to_string(),
                name: None,
            },
            Message::User {
                content: format!(
                    "## Available Agents\n\n{}\n\n## Task: {}{}\n\n## Attachments\n\nNo attachments provided.",
                    agents,
                    task.title,
                    summary
                ),
                name: None,
            },
        ])
    }

    fn abilities() -> Vec<Ability> {
        vec![
            Ability::for_fn(
                "No plan required. Assign task to an agent",
                &json!({
                    "name": "sfai_assign_to_agent",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "agent_id": {
                                "type": "integer",
                                "description": "ID of the agent to assign the task to"
                            }
                        }
                    }
                }),
            ),
            Ability::for_fn(
                "Plan task execution",
                &json!({
                    "name": "sfai_plan_task_execution",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "tasks": {
                                "type": "array",
                                "description": "List of planned sub-tasks",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "title": {
                                            "type": "string",
                                            "description": "Task title"
                                        },
                                        "summary": {
                                            "type": "string",
                                            "description": "Task summary"
                                        },
                                        "agent_id": {
                                            "type": "integer",
                                            "description": "ID of the agent to assign the task to"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }),
            ),
        ]
    }
}
