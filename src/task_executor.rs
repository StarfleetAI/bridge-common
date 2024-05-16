// Copyright 2024 StarfleetAI
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use anyhow::{anyhow, Context};
use askama::Template;
use serde::Deserialize;
use serde_json::json;
use sqlx::{Pool, Postgres};
use tokio::fs;
use tracing::{debug, error, info, instrument};
use uuid::Uuid;

use crate::channel::{self, Channel};
use crate::clients::openai::{ToolCall, ToolCalls};
use crate::repo::{self, messages::CreateParams};
use crate::settings::Settings;
use crate::types::Result;
use crate::types::{
    abilities::Ability,
    agents::Agent,
    chats::{Chat, Kind},
    messages::{Message, Role},
    tasks::{Status, Task},
};
use crate::{
    chats::{self, CreateCompletionParams},
    docker,
};
use crate::{models, types};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no root tasks to execute")]
    NoRootTasks,
    #[error("chat #{0} is not an execution chat")]
    NotAnExecutionChat(Uuid),
    #[error("failed to render template: {0}")]
    TemplateRender(#[from] askama::Error),
}

pub struct TaskExecutor<'a> {
    pub pool: &'a Pool<Postgres>,
    pub channel: &'a Channel,
    pub settings: &'a Settings,
    pub workdir_root: PathBuf,
    pub user_agent: String,
}

impl TaskExecutor<'_> {
    #[instrument(skip_all)]
    pub async fn execute_root_task(&self, cid: Uuid) -> Result<()> {
        let mut task = match self.get_root_task_for_execution(cid).await {
            Ok(Some(task)) => task,
            Ok(None) => return Err(Error::NoRootTasks.into()),
            Err(err) => return Err(err),
        };

        // TODO: get the uid from task author
        let uid = Uuid::new_v4();
        self.channel
            .emit(uid, &channel::Event::TaskUpdated(&task))
            .await?;

        info!("Root task for execution: #{}. {}", task.id, task.title);

        let children_count = repo::tasks::get_all_children_count(self.pool, cid, &task).await?;

        if children_count > 0 {
            info!("Executing children tasks for root task #{}.", task.id);
            self.execute_children_task_tree(cid, uid, &mut task).await?;

            return Ok(());
        }

        info!("Executing root task #{}", task.id);

        match self.execute_task(cid, uid, &mut task).await {
            Ok(status) => {
                debug!(
                    "No errors. Transitioning root task #{} to status: {:?}",
                    task.id, status
                );

                let task = repo::tasks::update_status(self.pool, cid, task.id, status).await?;
                self.channel
                    .emit(uid, &channel::Event::TaskUpdated(&task))
                    .await?;

                Ok(())
            }
            Err(err) => {
                let task = repo::tasks::fail(self.pool, cid, task.id).await?;
                self.channel
                    .emit(uid, &channel::Event::TaskUpdated(&task))
                    .await?;

                Err(err)
            }
        }
    }

    #[instrument(skip_all)]
    async fn get_task_execution_chat(&self, cid: Uuid, task: &Task) -> Result<Chat> {
        if let Some(chat_id) = task.execution_chat_id {
            match repo::chats::get(self.pool, cid, chat_id).await {
                Ok(chat) if chat.kind == Kind::Execution => Ok(chat),
                Ok(_) => Err(Error::NotAnExecutionChat(chat_id).into()),
                Err(err) => Err(err),
            }
        } else {
            let chat = repo::chats::create(self.pool, cid, Kind::Execution).await?;
            repo::tasks::update_execution_chat_id(self.pool, cid, task.id, chat.id).await?;
            repo::agents_chats::create(self.pool, cid, task.agent_id, chat.id).await?;

            Ok(chat)
        }
    }

    #[instrument(skip_all)]
    async fn get_root_task_for_execution(&self, cid: Uuid) -> Result<Option<Task>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin transaction")?;

        let Some(mut task) = repo::tasks::get_root_for_execution(&mut *tx, cid).await? else {
            tx.commit().await.context("failed to commit transaction")?;

            return Ok(None);
        };

        if task.status != Status::ToDo {
            tx.commit().await.context("failed to commit transaction")?;

            return Ok(None);
        }

        repo::tasks::start_progress(&mut *tx, cid, task.id).await?;
        task.status = Status::InProgress;

        tx.commit().await.context("failed to commit transaction")?;

        Ok(Some(task))
    }

    #[instrument(skip_all)]
    async fn get_child_task_for_execution(&self, cid: Uuid, parent: &Task) -> Result<Option<Task>> {
        let mut children_tasks =
            repo::tasks::list_all_children(self.pool, cid, &parent.children_ancestry())
                .await
                .context("failed to list children")?;

        let mut tree = TaskTree {
            root: (*parent).clone(),
            children: Vec::new(),
        };

        sort_task_tree(&mut children_tasks);
        collect_children(&mut tree, &mut children_tasks)?;

        if let Some(task) = find_execution_candidate(&tree) {
            return Ok(Some(
                repo::tasks::start_progress(self.pool, cid, task.id).await?,
            ));
        }

        Ok(None)
    }

    async fn execute_children_task_tree(
        &self,
        cid: Uuid,
        uid: Uuid,
        parent: &mut Task,
    ) -> Result<()> {
        info!("Executing children tasks tree for task #{}", parent.id);

        while let Some(mut child) = match self.get_child_task_for_execution(cid, parent).await {
            Ok(task) => task,
            Err(err) => {
                repo::tasks::fail(self.pool, cid, parent.id).await?;
                self.fail_parent_tasks(cid, uid, parent).await?;

                return Err(err);
            }
        } {
            info!("Executing child task #{}: {}", child.id, child.title);

            // TODO: seems counterintuitive to emit the task update here, since it was updated in the
            //       `get_child_task_for_execution` function. Consider code reorganization.
            self.channel
                .emit(uid, &channel::Event::TaskUpdated(&child))
                .await?;

            match self.execute_task(cid, uid, &mut child).await {
                Ok(_) => {
                    info!("Child task #{} is done", child.id);
                    repo::tasks::complete(self.pool, cid, child.id).await?;

                    // Complete parent task if all siblings are done
                    if repo::tasks::is_all_siblings_done(self.pool, cid, &child).await? {
                        info!(
                        "All siblings are done for the parent task #{}, marking it as `Done` as well",
                        parent.id
                    );

                        let task = repo::tasks::complete(
                            self.pool,
                            cid,
                            child
                                .parent_id()?
                                .context("parent_id is not set for the child task")?,
                        )
                        .await?;

                        self.channel
                            .emit(uid, &channel::Event::TaskUpdated(&task))
                            .await?;
                    }
                }
                Err(err) => {
                    repo::tasks::fail(self.pool, cid, child.id).await?;
                    self.fail_parent_tasks(cid, uid, &child).await?;

                    return Err(err);
                }
            }
        }

        Ok(())
    }

    async fn fail_parent_tasks(&self, cid: Uuid, uid: Uuid, child: &Task) -> Result<()> {
        if let Some(parent_ids) = child.parent_ids()? {
            for parent_id in parent_ids {
                let task = repo::tasks::fail(self.pool, cid, parent_id).await?;
                self.channel
                    .emit(uid, &channel::Event::TaskUpdated(&task))
                    .await?;
            }
        }

        Ok(())
    }

    #[instrument(skip_all)]
    async fn execute_task(&self, cid: Uuid, uid: Uuid, task: &mut Task) -> Result<Status> {
        info!("Executing task #{}: {}", task.id, task.title);

        let chat = self.get_task_execution_chat(cid, task).await?;

        task.execution_chat_id = Some(chat.id);

        self.channel
            .emit(uid, &channel::Event::TaskUpdated(&task))
            .await?;

        loop {
            match repo::messages::get_last_message(self.pool, cid, chat.id).await? {
                Some(message) => match message.role {
                    Role::CodeInterpreter | Role::Tool | Role::User => {
                        self.send_to_agent(cid, uid, chat.id, task).await?;
                    }
                    Role::Assistant => {
                        let tc = message.tool_calls();

                        match tc.len() {
                            0 if message.is_self_reflection => {
                                self.send_to_agent(cid, uid, chat.id, task).await?;
                            }
                            0 => {
                                let content = message.content.clone().unwrap_or_default();
                                match parse_code_blocks(&content) {
                                    Ok(code_blocks) if !code_blocks.is_empty() => {
                                        self.sfai_code_interpreter(cid, uid, &message, task)
                                            .await?;
                                    }
                                    _ => self.self_reflect(cid, uid, chat.id, task).await?,
                                }
                            }
                            _ => {
                                // I acknowledge, that this is weird to pass `tool_calls` alongside the `message`, but why not since it's already unpacked from `Option`?
                                match self.call_tools(cid, uid, &message, tc, task).await {
                                    Ok(maybe_new_status) => {
                                        self.complete_message(cid, uid, &message).await?;

                                        if let Some(new_status) = maybe_new_status {
                                            return Ok(new_status);
                                        }
                                    }
                                    Err(err) => {
                                        self.fail_message(cid, uid, &message).await?;
                                        return Err(err);
                                    }
                                }
                            }
                        }
                    }
                    Role::System => {
                        return Err(
                            anyhow!("unexpected system message in the execution chat").into()
                        );
                    }
                },
                None => self.send_to_agent(cid, uid, chat.id, task).await?,
            }
        }
    }

    /// Call tools.
    ///
    /// Returns optional new task status. This is useful when the task execution is finished and the
    /// task status should be updated. For example, when the LLM marks the task as `Done`.
    #[instrument(skip_all)]
    async fn call_tools(
        &self,
        cid: Uuid,
        uid: Uuid,
        message: &Message,
        tool_calls: ToolCalls,
        task: &Task,
    ) -> Result<Option<Status>> {
        let mut new_status = None;

        // Call task management tools
        for tool_call in tool_calls.iter() {
            if let Some(status) = match tool_call.function.name.as_str() {
                "sfai_done" => {
                    self.sfai_done(cid, uid, message, task.id, &tool_call)
                        .await?
                }
                "sfai_fail" => self.sfai_fail(cid, message, &tool_call).await?,
                "sfai_wait_for_user" => self.sfai_wait_for_user(cid, message, &tool_call).await?,
                "sfai_code_interpreter" => {
                    self.sfai_code_interpreter(cid, uid, message, task).await?
                }
                _ => None,
            } {
                new_status = Some(status);
            }
        }

        // Call other tools
        // TODO: implement tool calls
        // crate::abilities::execute_for_message(message, app_handle).await?;

        Ok(new_status)
    }

    async fn sfai_wait_for_user(
        &self,
        cid: Uuid,
        message: &Message,
        tool_call: &ToolCall,
    ) -> Result<Option<Status>> {
        repo::messages::create(
            self.pool,
            cid,
            CreateParams {
                content: Some("```\nWaiting for user input\n```".to_string()),
                chat_id: message.chat_id,
                status: types::messages::Status::Completed,
                role: Role::Tool,
                tool_call_id: Some(tool_call.id.clone()),
                is_internal_tool_output: true,
                ..Default::default()
            },
        )
        .await?;

        Ok(Some(Status::WaitingForUser))
    }

    async fn sfai_fail(
        &self,
        cid: Uuid,
        message: &Message,
        tool_call: &ToolCall,
    ) -> Result<Option<Status>> {
        repo::messages::create(
            self.pool,
            cid,
            CreateParams {
                content: Some("```\nTask has been marked as failed\n```".to_string()),
                chat_id: message.chat_id,
                status: types::messages::Status::Completed,
                role: Role::Tool,
                tool_call_id: Some(tool_call.id.clone()),
                is_internal_tool_output: true,
                ..Default::default()
            },
        )
        .await?;

        Ok(Some(Status::Failed))
    }

    async fn sfai_code_interpreter(
        &self,
        cid: Uuid,
        uid: Uuid,
        message: &Message,
        task: &Task,
    ) -> Result<Option<Status>> {
        if let Some(result_message) =
            repo::messages::get_last_non_self_reflection_message(self.pool, cid, message.chat_id)
                .await?
        {
            let content = Some(match self.interpret_code(&result_message, task).await {
                Ok(out_lines) => out_lines.join("\n\n"),
                Err(err) => format!("Failed to interpret code: {err}"),
            });

            let out_message = repo::messages::create(
                self.pool,
                cid,
                CreateParams {
                    content,
                    chat_id: message.chat_id,
                    status: types::messages::Status::Completed,
                    role: Role::CodeInterpreter,
                    ..Default::default()
                },
            )
            .await?;

            self.channel
                .emit(uid, &channel::Event::MessageCreated(&out_message))
                .await?;
        }

        Ok(None)
    }

    async fn interpret_code(&self, message: &Message, task: &Task) -> Result<Vec<String>> {
        let code_blocks = match parse_code_blocks(match &message.content.as_ref() {
            Some(content) => content,
            None => return Ok(vec!["No content in the message to interpret".to_string()]),
        }) {
            Ok(code_blocks) => code_blocks,
            Err(err) => {
                return Ok(vec![format!(
                    "Failed to parse code blocks in the message: {err}"
                )])
            }
        };

        let mut lines = Vec::with_capacity(code_blocks.len());

        let workdir = task.workdir(&self.workdir_root).await?;

        for code_block in code_blocks {
            if code_block.filename.is_none() {
                let result = match code_block.language {
                    Language::Shell => docker::run_cmd(&code_block.code, Some(&workdir)).await?,
                    Language::Python => {
                        docker::run_python_code(&code_block.code, Some(&workdir)).await?
                    }
                    lang => {
                        format!("Error: language `{lang:?}` is not supported for code execution")
                    }
                };

                lines.push(format!("```\n{result}\n```"));
            } else if let Some(filename) = &code_block.filename {
                let mut workdir = match task.workdir(&self.workdir_root).await {
                    Ok(workdir) => workdir,
                    Err(err) => {
                        lines.push(format!("```\nFailed to get task workdir: {err}\n```"));
                        continue;
                    }
                };

                workdir.push(filename);

                match fs::write(&workdir, code_block.code).await {
                    Ok(()) => {
                        lines.push(format!("```\nFile `{filename}` has been saved\n```"));
                    }
                    Err(err) => {
                        lines.push(format!("```\nFailed to save file `{filename}`: {err}\n```"));
                    }
                }
            }
        }

        Ok(lines)
    }

    async fn sfai_done(
        &self,
        cid: Uuid,
        uid: Uuid,
        message: &Message,
        task_id: Uuid,
        tool_call: &ToolCall,
    ) -> Result<Option<Status>> {
        repo::messages::create(
            self.pool,
            cid,
            CreateParams {
                content: Some("```\nTask has been marked as done\n```".to_string()),
                chat_id: message.chat_id,
                status: types::messages::Status::Completed,
                role: Role::Tool,
                tool_call_id: Some(tool_call.id.clone()),
                is_internal_tool_output: true,
                ..Default::default()
            },
        )
        .await?;

        if let Some(result_message) =
            repo::messages::get_last_non_self_reflection_message(self.pool, cid, message.chat_id)
                .await?
        {
            let text = result_message.content.clone().unwrap_or_default();

            self.sfai_provide_text_result(
                cid,
                uid,
                &result_message,
                task_id,
                ProvideTextResultArgs {
                    text,
                    ..Default::default()
                },
            )
            .await?;
        }

        Ok(Some(Status::Done))
    }

    /// Provide a text result for the task.
    ///
    /// # Errors
    ///
    /// Returns an error if the tool call arguments cannot be parsed or the task result cannot be
    /// created.
    #[instrument(skip_all)]
    async fn sfai_provide_text_result(
        &self,
        cid: Uuid,
        uid: Uuid,
        message: &Message,
        task_id: Uuid,
        args: ProvideTextResultArgs,
    ) -> Result<Option<Status>> {
        let mut new_status = None;

        let task_result = repo::task_results::create(
            self.pool,
            cid,
            repo::task_results::CreateParams {
                agent_id: message
                    .agent_id
                    .context("Agent is not set for the message with a tool call")?,
                task_id,
                kind: types::task_results::Kind::Text,
                data: args.text,
            },
        )
        .await?;

        self.channel
            .emit(uid, &channel::Event::TaskResultCreated(&task_result))
            .await?;

        if args.is_done {
            new_status = Some(Status::Done);
        }

        self.channel
            .emit(uid, &channel::Event::MessageCreated(&message))
            .await?;

        Ok(new_status)
    }

    async fn complete_message(&self, cid: Uuid, uid: Uuid, message: &Message) -> Result<()> {
        repo::messages::update_status(
            self.pool,
            cid,
            message.id,
            types::messages::Status::Completed,
        )
        .await?;

        let mut message = message.clone();
        message.status = types::messages::Status::Completed;

        self.channel
            .emit(uid, &channel::Event::MessageUpdated(&message))
            .await?;

        Ok(())
    }

    async fn fail_message(&self, cid: Uuid, uid: Uuid, message: &Message) -> Result<()> {
        repo::messages::update_status(self.pool, cid, message.id, types::messages::Status::Failed)
            .await?;

        let mut message = message.clone();
        message.status = types::messages::Status::Failed;

        self.channel
            .emit(uid, &channel::Event::MessageUpdated(&message))
            .await?;

        Ok(())
    }

    #[instrument(skip_all)]
    async fn send_to_agent(&self, cid: Uuid, uid: Uuid, chat_id: Uuid, task: &Task) -> Result<()> {
        let agent = repo::agents::get_for_chat(self.pool, cid, chat_id).await?;

        let model =
            match repo::models::get_by_full_name(self.pool, cid, &self.settings.default_model)
                .await?
            {
                Some(model) => model,
                None => {
                    return Err(models::Error::DefaultModelNotFound(
                        cid,
                        self.settings.default_model.clone(),
                    )
                    .into())
                }
            };

        // TODO: get the api key
        let api_key = "";

        chats::create_completion(
            self.pool,
            self.channel,
            cid,
            uid,
            chat_id,
            CreateCompletionParams {
                messages_pre: Some(execution_prelude(chat_id, task, &agent, false)?),
                ..Default::default()
            },
            &model,
            api_key,
            &self.user_agent,
        )
        .await?;

        Ok(())
    }

    #[instrument(skip_all)]
    async fn self_reflect(&self, cid: Uuid, uid: Uuid, chat_id: Uuid, task: &Task) -> Result<()> {
        let agent = repo::agents::get_for_chat(self.pool, cid, chat_id).await?;

        let message = SelfReflectionMessageTemplate {};
        let content = Some(message.render().map_err(Error::TemplateRender)?);

        let messages_post = vec![Message {
            chat_id,
            content,
            role: Role::User,
            ..Default::default()
        }];

        let model =
            match repo::models::get_by_full_name(self.pool, cid, &self.settings.default_model)
                .await?
            {
                Some(model) => model,
                None => {
                    return Err(models::Error::DefaultModelNotFound(
                        cid,
                        self.settings.default_model.clone(),
                    )
                    .into())
                }
            };

        // TODO: get the api key
        let api_key = "";

        chats::create_completion(
            self.pool,
            self.channel,
            cid,
            uid,
            chat_id,
            CreateCompletionParams {
                messages_pre: Some(execution_prelude(chat_id, task, &agent, true)?),
                messages_post: Some(messages_post),
                abilities: Some(internal_task_abilities()),
                is_self_reflection: true,
            },
            &model,
            api_key,
            &self.user_agent,
        )
        .await?;

        Ok(())
    }
}

#[derive(Deserialize, Debug, Default)]
pub struct ProvideTextResultArgs {
    pub text: String,
    pub is_done: bool,
}

fn internal_task_abilities() -> Vec<Ability> {
    // TODO: it's inefficient to use `Ability` here, since we're serializing parameters to JSON
    //       only to deserialize them back in `chats::create_completion`. Consider using [`Tool`]
    //       instead.
    //
    // TODO: It's also slightly inefficient to create these abilities on every iteration.
    //       Consider caching them or something.
    vec![
        Ability::for_fn("Mark current task as done", &json!({ "name": "sfai_done" })),
        Ability::for_fn(
            "Mark current task as failed",
            &json!({ "name": "sfai_fail" }),
        ),
        Ability::for_fn(
            "Wait for additional user input",
            &json!({ "name": "sfai_wait_for_user" }),
        ),
    ]
}

#[derive(Template)]
#[template(path = "task_executor/task_message.md", escape = "none")]
struct TaskMessageTemplate<'a> {
    task: &'a Task,
}

#[derive(Template)]
#[template(path = "task_executor/system_message.md", escape = "none")]
struct SystemMessageTemplate<'a> {
    agent: &'a Agent,
    is_self_reflection: bool,
}

#[derive(Template)]
#[template(path = "task_executor/self_reflection_message.md", escape = "none")]
struct SelfReflectionMessageTemplate {}

fn execution_prelude(
    chat_id: Uuid,
    task: &Task,
    agent: &Agent,
    is_self_reflection: bool,
) -> Result<Vec<Message>> {
    let system_message = SystemMessageTemplate {
        agent,
        is_self_reflection,
    };
    let task_message = TaskMessageTemplate { task };

    Ok(vec![
        Message {
            chat_id,
            role: Role::System,
            content: Some(system_message.render().map_err(Error::TemplateRender)?),
            ..Default::default()
        },
        Message {
            chat_id,
            role: Role::User,
            content: Some(task_message.render().map_err(Error::TemplateRender)?),
            ..Default::default()
        },
    ])
}

struct TaskTree {
    pub root: Task,
    pub children: Vec<TaskTree>,
}

fn find_execution_candidate(tree: &TaskTree) -> Option<&Task> {
    if !tree.children.is_empty() {
        for child in &tree.children {
            if let Some(task) = find_execution_candidate(child) {
                return Some(task);
            }
        }
    }

    match tree.root.status {
        Status::InProgress | Status::Done => None,
        Status::Draft | Status::ToDo | Status::WaitingForUser | Status::Failed => Some(&tree.root),
    }
}

fn collect_children(tree: &mut TaskTree, tasks: &mut Vec<Task>) -> Result<()> {
    for task in tasks.clone() {
        if task.parent_id()? == Some(tree.root.id) {
            tree.children.push(TaskTree {
                root: task.clone(),
                children: Vec::new(),
            });

            tasks.retain(|t| t.id != task.id);

            collect_children(tree.children.last_mut().unwrap(), tasks)?;
        }
    }

    Ok(())
}

fn sort_task_tree(tasks: &mut [Task]) {
    tasks.sort_by(|a, b| a.created_at.cmp(&b.created_at));
}

#[derive(Default, Debug)]
enum Language {
    #[default]
    Unknown,
    Shell,
    Markdown,
    Python,
    Other,
}

impl From<String> for Language {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "sh" | "shell" => Language::Shell,
            "markdown" | "md" => Language::Markdown,
            "python" => Language::Python,
            "" => Language::Unknown,
            _ => Language::Other,
        }
    }
}

#[derive(Default, Debug, PartialEq)]
enum CodeBlockAction {
    #[default]
    DoNothing,
    Execute,
    Save,
}

#[derive(Default)]
struct CodeBlock {
    pub code: String,
    pub language: Language,
    pub filename: Option<String>,
    pub action: CodeBlockAction,
}

fn parse_code_blocks(text: &str) -> Result<Vec<CodeBlock>> {
    let ast = markdown::to_mdast(text, &markdown::ParseOptions::default())
        .map_err(|err| anyhow!("Failed to parse markdown AST: {}", err))?;

    let mut code_blocks = Vec::new();
    let mut code_block = CodeBlock::default();

    for node in ast
        .children()
        .ok_or_else(|| anyhow!("Failed to get AST children"))?
    {
        match node {
            markdown::mdast::Node::BlockQuote(blockquote) => {
                if blockquote.children.len() != 1 {
                    continue;
                }

                let markdown::mdast::Node::Paragraph(paragraph) = &blockquote.children[0] else {
                    continue;
                };

                match paragraph.children.len() {
                    1 => {
                        if let markdown::mdast::Node::Text(text) = &paragraph.children[0] {
                            if text.value.to_lowercase().trim() != "execute" {
                                continue;
                            }

                            code_block.action = CodeBlockAction::Execute;
                        }
                    }
                    2 => {
                        if let markdown::mdast::Node::Text(text) = &paragraph.children[0] {
                            if text.value.to_lowercase().trim() != "save:" {
                                continue;
                            }

                            if let markdown::mdast::Node::InlineCode(ic) = &paragraph.children[1] {
                                code_block.filename = Some(ic.value.clone());
                                code_block.action = CodeBlockAction::Save;
                            }
                        }
                    }
                    _ => continue,
                }
            }
            markdown::mdast::Node::Code(code)
                if code_block.action != CodeBlockAction::DoNothing =>
            {
                code_block.code = code.value.clone();
                code_block.language = code.lang.clone().unwrap_or_default().into();
                code_blocks.push(code_block);
                code_block = CodeBlock::default();
            }
            _ => {}
        }
    }

    Ok(code_blocks)
}
