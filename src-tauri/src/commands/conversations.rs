use std::path::PathBuf;
use std::sync::Arc;

use tauri::ipc::Channel;
use tauri::State;

use super::ctx;
use crate::database::models::{Conversation, Message, MessageView, SearchHit};
use crate::error::AppError;
use crate::providers::StreamEvent;
use crate::services::{chat, generation};
use crate::state::AppContext;

#[tauri::command]
pub fn list_conversations(
    state: State<'_, Arc<AppContext>>,
    project_id: Option<String>,
    include_archived: Option<bool>,
    query: Option<String>,
) -> Result<Vec<Conversation>, AppError> {
    ctx(&state).db.list_conversations(
        project_id.as_deref(),
        include_archived.unwrap_or(false),
        query.as_deref(),
    )
}

#[tauri::command]
pub fn get_conversation(
    state: State<'_, Arc<AppContext>>,
    id: String,
) -> Result<Conversation, AppError> {
    ctx(&state).db.get_conversation(&id)
}

#[tauri::command]
pub fn create_conversation(
    state: State<'_, Arc<AppContext>>,
    project_id: Option<String>,
    title: Option<String>,
    provider_id: Option<String>,
    account_id: Option<String>,
    model: Option<String>,
) -> Result<Conversation, AppError> {
    ctx(&state).db.create_conversation(
        project_id.as_deref(),
        title.as_deref(),
        provider_id.as_deref(),
        account_id.as_deref(),
        model.as_deref(),
    )
}

#[tauri::command]
pub fn rename_conversation(
    state: State<'_, Arc<AppContext>>,
    id: String,
    title: String,
) -> Result<Conversation, AppError> {
    ctx(&state).db.rename_conversation(&id, &title)
}

#[tauri::command]
pub fn set_conversation_flags(
    state: State<'_, Arc<AppContext>>,
    id: String,
    pinned: Option<bool>,
    archived: Option<bool>,
) -> Result<Conversation, AppError> {
    ctx(&state).db.set_conversation_flags(&id, pinned, archived)
}

#[tauri::command]
pub fn set_conversation_target(
    state: State<'_, Arc<AppContext>>,
    id: String,
    provider_id: String,
    account_id: Option<String>,
    model: String,
) -> Result<Conversation, AppError> {
    ctx(&state)
        .db
        .set_conversation_target(&id, Some(&provider_id), account_id.as_deref(), Some(&model))
}

#[tauri::command]
pub fn move_conversation(
    state: State<'_, Arc<AppContext>>,
    id: String,
    project_id: Option<String>,
) -> Result<Conversation, AppError> {
    ctx(&state).db.move_conversation(&id, project_id.as_deref())
}

#[tauri::command]
pub fn delete_conversation(state: State<'_, Arc<AppContext>>, id: String) -> Result<(), AppError> {
    ctx(&state).db.delete_conversation(&id)
}

#[tauri::command]
pub fn list_messages(
    state: State<'_, Arc<AppContext>>,
    conversation_id: String,
) -> Result<Vec<MessageView>, AppError> {
    ctx(&state).db.list_messages(&conversation_id)
}

#[tauri::command]
pub fn search_messages(
    state: State<'_, Arc<AppContext>>,
    query: String,
    limit: Option<i64>,
) -> Result<Vec<SearchHit>, AppError> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    ctx(&state).db.search_messages(query, limit.unwrap_or(50).clamp(1, 200))
}

#[tauri::command]
pub fn export_conversation(
    state: State<'_, Arc<AppContext>>,
    conversation_id: String,
    destination: String,
) -> Result<String, AppError> {
    let markdown = ctx(&state)
        .db
        .export_conversation_markdown(&conversation_id)?;
    let mut path = PathBuf::from(&destination);
    if !path.is_absolute() {
        return Err(AppError::invalid("choose an absolute destination path"));
    }
    if path.extension().is_none() {
        path.set_extension("md");
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, markdown.as_bytes())?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn send_message(
    state: State<'_, Arc<AppContext>>,
    conversation_id: String,
    content: String,
    attachment_ids: Option<Vec<String>>,
    channel: Channel<StreamEvent>,
) -> Result<Message, AppError> {
    let context = ctx(&state);
    chat::send_message(
        context,
        conversation_id,
        content,
        attachment_ids.unwrap_or_default(),
        channel,
    )
    .await
}

#[tauri::command]
pub async fn continue_generation(
    state: State<'_, Arc<AppContext>>,
    conversation_id: String,
    message_id: String,
    channel: Channel<StreamEvent>,
) -> Result<Message, AppError> {
    generation::continue_generation(ctx(&state), conversation_id, message_id, channel).await
}

#[tauri::command]
pub fn cancel_stream(state: State<'_, Arc<AppContext>>, message_id: String) -> Result<bool, AppError> {
    Ok(ctx(&state).cancel_chat(&message_id))
}
