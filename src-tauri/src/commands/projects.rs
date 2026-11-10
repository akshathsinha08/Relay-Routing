use std::sync::Arc;

use tauri::State;

use super::ctx;
use crate::database::models::{Artifact, Attachment, Project};
use crate::error::AppError;
use crate::state::AppContext;

#[tauri::command]
pub fn list_projects(
    state: State<'_, Arc<AppContext>>,
    include_archived: Option<bool>,
) -> Result<Vec<Project>, AppError> {
    ctx(&state)
        .db
        .list_projects(include_archived.unwrap_or(false))
}

#[tauri::command]
pub fn get_project(
    state: State<'_, Arc<AppContext>>,
    id: String,
) -> Result<Project, AppError> {
    ctx(&state).db.get_project(&id)
}

#[tauri::command]
pub fn create_project(
    state: State<'_, Arc<AppContext>>,
    name: String,
    description: Option<String>,
    instructions: Option<String>,
) -> Result<Project, AppError> {
    ctx(&state).db.create_project(
        &name,
        description.as_deref().unwrap_or(""),
        instructions.as_deref().unwrap_or(""),
    )
}

#[tauri::command]
pub fn update_project(
    state: State<'_, Arc<AppContext>>,
    id: String,
    name: Option<String>,
    description: Option<String>,
    instructions: Option<String>,
    archived: Option<bool>,
) -> Result<Project, AppError> {
    ctx(&state).db.update_project(
        &id,
        name.as_deref(),
        description.as_deref(),
        instructions.as_deref(),
        archived,
    )
}

#[tauri::command]
pub fn delete_project(state: State<'_, Arc<AppContext>>, id: String) -> Result<(), AppError> {
    ctx(&state).db.delete_project(&id)
}

#[tauri::command]
pub fn list_project_attachments(
    state: State<'_, Arc<AppContext>>,
    project_id: String,
) -> Result<Vec<Attachment>, AppError> {
    ctx(&state).db.list_project_attachments(&project_id)
}

#[tauri::command]
pub fn list_project_artifacts(
    state: State<'_, Arc<AppContext>>,
    project_id: String,
) -> Result<Vec<Artifact>, AppError> {
    ctx(&state).db.list_artifacts_for_project(&project_id)
}
