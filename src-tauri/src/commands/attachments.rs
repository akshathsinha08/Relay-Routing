use std::sync::Arc;

use base64::Engine as _;
use tauri::State;

use super::ctx;
use crate::database::models::Attachment;
use crate::error::AppError;
use crate::security::paths::ensure_within;
use crate::services::attachments as service;
use crate::state::AppContext;

#[tauri::command]
pub fn import_attachments(
    state: State<'_, Arc<AppContext>>,
    project_id: Option<String>,
    conversation_id: Option<String>,
    paths: Vec<String>,
) -> Result<Vec<Attachment>, AppError> {
    let context = ctx(&state);
    service::import_files(
        &context,
        project_id.as_deref(),
        conversation_id.as_deref(),
        &paths,
    )
}

#[tauri::command]
pub fn delete_attachment(
    state: State<'_, Arc<AppContext>>,
    id: String,
) -> Result<(), AppError> {
    let context = ctx(&state);
    let attachment = context.db.get_attachment(&id)?;
    context.db.delete_attachment(&id)?;
    service::remove_file(&context, &attachment);
    Ok(())
}

#[tauri::command]
pub fn attachment_preview(
    state: State<'_, Arc<AppContext>>,
    id: String,
) -> Result<Option<String>, AppError> {
    let context = ctx(&state);
    let attachment = context.db.get_attachment(&id)?;
    if attachment.kind != "image" || attachment.size > 12 * 1024 * 1024 {
        return Ok(None);
    }
    let path = service::verified_path(&context, &attachment)?;
    let bytes = std::fs::read(path)?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(Some(format!("data:{};base64,{}", attachment.mime, encoded)))
}

#[tauri::command]
pub fn open_attachment(
    state: State<'_, Arc<AppContext>>,
    id: String,
) -> Result<(), AppError> {
    let context = ctx(&state);
    let attachment = context.db.get_attachment(&id)?;
    let path = ensure_within(&context.paths.attachments_dir, std::path::Path::new(&attachment.path))?;
    std::process::Command::new("open")
        .arg(path)
        .status()
        .map_err(|error| AppError::internal(format!("cannot open attachment: {error}")))?;
    Ok(())
}
