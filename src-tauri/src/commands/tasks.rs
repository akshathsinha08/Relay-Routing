use std::sync::Arc;

use tauri::State;

use super::ctx;
use crate::database::models::{Artifact, Task, TaskDetail};
use crate::database::tasks::NewTask;
use crate::error::AppError;
use crate::security::paths::ensure_within;
use crate::state::AppContext;
use crate::tasks::engine;

#[tauri::command]
pub fn list_tasks(
    state: State<'_, Arc<AppContext>>,
    project_id: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<Task>, AppError> {
    ctx(&state)
        .db
        .list_tasks(project_id.as_deref(), limit.unwrap_or(100).clamp(1, 500))
}

#[tauri::command]
pub fn get_task_detail(
    state: State<'_, Arc<AppContext>>,
    task_id: String,
) -> Result<TaskDetail, AppError> {
    ctx(&state).db.task_detail(&task_id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_task(
    state: State<'_, Arc<AppContext>>,
    name: String,
    instructions: String,
    project_id: Option<String>,
    conversation_id: Option<String>,
    provider_id: Option<String>,
    account_id: Option<String>,
    model: Option<String>,
) -> Result<Task, AppError> {
    let context = ctx(&state);
    let name = name.trim();
    let instructions = instructions.trim();
    if name.is_empty() {
        return Err(AppError::invalid("task name is required"));
    }
    if instructions.is_empty() {
        return Err(AppError::invalid("task instructions are required"));
    }
    context.db.create_task(NewTask {
        project_id,
        conversation_id,
        name: name.to_string(),
        instructions: instructions.to_string(),
        provider_id,
        account_id,
        model,
    })
}

#[tauri::command]
pub fn start_task(
    state: State<'_, Arc<AppContext>>,
    task_id: String,
) -> Result<Task, AppError> {
    let context = ctx(&state);
    let task = context.db.get_task(&task_id)?;
    if task.status == "failed" {
        context.db.increment_task_retry(&task_id)?;
    }
    engine::start(context, task_id)
}

#[tauri::command]
pub fn pause_task(
    state: State<'_, Arc<AppContext>>,
    task_id: String,
) -> Result<Task, AppError> {
    engine::pause(&ctx(&state), &task_id)
}

#[tauri::command]
pub fn cancel_task(
    state: State<'_, Arc<AppContext>>,
    task_id: String,
) -> Result<Task, AppError> {
    engine::cancel(&ctx(&state), &task_id)
}

#[tauri::command]
pub fn resume_task(
    state: State<'_, Arc<AppContext>>,
    task_id: String,
) -> Result<Task, AppError> {
    let context = ctx(&state);
    let task = context.db.get_task(&task_id)?;
    if !matches!(task.status.as_str(), "paused" | "failed" | "queued") {
        return Err(AppError::invalid("this task cannot be resumed in its current state"));
    }
    if task.status == "failed" {
        context.db.increment_task_retry(&task_id)?;
    }
    engine::start(context, task_id)
}

#[tauri::command]
pub fn delete_task(state: State<'_, Arc<AppContext>>, task_id: String) -> Result<(), AppError> {
    let context = ctx(&state);
    context.db.get_task(&task_id)?;
    if context.is_task_running(&task_id) {
        context.cancel_task(&task_id);
    }
    context.db.delete_task(&task_id)
}

#[tauri::command]
pub fn task_artifacts(
    state: State<'_, Arc<AppContext>>,
    task_id: String,
) -> Result<Vec<Artifact>, AppError> {
    ctx(&state).db.list_artifacts_for_task(&task_id)
}

#[tauri::command]
pub fn open_artifact(state: State<'_, Arc<AppContext>>, id: String) -> Result<(), AppError> {
    let context = ctx(&state);
    let artifact = context.db.get_artifact(&id)?;
    let path = ensure_within(
        &context.paths.artifacts_dir,
        std::path::Path::new(&artifact.path),
    )?;
    std::process::Command::new("open")
        .arg(path)
        .status()
        .map_err(|error| AppError::internal(format!("cannot open artifact: {error}")))?;
    Ok(())
}
