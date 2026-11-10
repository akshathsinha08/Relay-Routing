use std::sync::Arc;

use tauri::ipc::Channel;

use crate::database::models::Message;
use crate::error::AppError;
use crate::providers::StreamEvent;
use crate::services::generation;
use crate::state::AppContext;

pub async fn send_message(
    ctx: Arc<AppContext>,
    conversation_id: String,
    content: String,
    attachment_ids: Vec<String>,
    channel: Channel<StreamEvent>,
) -> Result<Message, AppError> {
    generation::send_message(ctx, conversation_id, content, attachment_ids, channel).await
}
