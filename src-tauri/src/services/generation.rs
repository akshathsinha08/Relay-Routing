use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::ipc::Channel;
use tokio_util::sync::CancellationToken;

use crate::database::models::{Conversation, Message};
use crate::error::AppError;
use crate::providers::{
    self, ChatMessage, ChatRequest, ImagePart, ProviderError, StreamEvent,
};
use crate::services::accounts::{self, Target};
use crate::services::attachments;
use crate::state::AppContext;

const MAX_ATTEMPTS: usize = 2;
const RETRY_DELAY: Duration = Duration::from_millis(500);
const FLUSH_INTERVAL: Duration = Duration::from_secs(1);
const FLUSH_CHARS: usize = 2048;

fn setting_i64(ctx: &AppContext, key: &str, fallback: i64) -> i64 {
    ctx.db
        .get_setting(key)
        .ok()
        .flatten()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(fallback)
}

fn setting_string(ctx: &AppContext, key: &str) -> Option<String> {
    ctx.db
        .get_setting(key)
        .ok()
        .flatten()
        .filter(|value| !value.trim().is_empty())
}

fn auto_title(ctx: &AppContext, conversation: &Conversation, content: &str) {
    if conversation.title != "New conversation" {
        return;
    }
    let source = content
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("New conversation");
    let mut title: String = source.trim().chars().take(64).collect();
    if title.is_empty() {
        title = "New conversation".into();
    }
    let _ = ctx.db.rename_conversation(&conversation.id, &title);
}

fn compose_system(ctx: &AppContext, conversation: &Conversation) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(prompt) = setting_string(ctx, "appSystemPrompt") {
        parts.push(prompt);
    }
    if let Some(project_id) = &conversation.project_id {
        if let Ok(project) = ctx.db.get_project(project_id) {
            if !project.instructions.trim().is_empty() {
                parts.push(project.instructions.trim().to_string());
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

fn history(
    ctx: &AppContext,
    conversation_id: &str,
    before_seq: i64,
    partial: &str,
) -> Result<Vec<ChatMessage>, AppError> {
    let limit = setting_i64(ctx, "contextMessageLimit", 30).clamp(2, 200);
    let views = ctx.db.list_messages(conversation_id)?;
    let mut messages: Vec<ChatMessage> = Vec::new();
    for view in views {
        if view.message.seq >= before_seq {
            continue;
        }
        if view.message.status == "error" && view.message.content.trim().is_empty() {
            continue;
        }
        let role = match view.message.role.as_str() {
            "user" => "user",
            "assistant" => "assistant",
            _ => continue,
        };
        let mut text = view.message.content.clone();
        if role == "user" {
            text.push_str(&attachments::text_excerpts(&view.attachments));
        }
        if text.trim().is_empty() {
            continue;
        }
        messages.push(ChatMessage::text(role, text));
    }
    let start = messages.len().saturating_sub(limit as usize);
    let mut messages = messages.split_off(start);
    while messages.first().map(|message| message.role.as_str()) == Some("assistant") {
        messages.remove(0);
    }
    if !partial.trim().is_empty() {
        messages.push(ChatMessage::text("assistant", partial));
    }
    Ok(messages)
}

fn combine(first: &str, second: &str) -> String {
    match (first.trim().is_empty(), second.trim().is_empty()) {
        (true, true) => String::new(),
        (true, false) => second.to_string(),
        (false, true) => first.to_string(),
        (false, false) => format!("{first}\n{second}"),
    }
}

fn append_tokens(total: Option<i64>, next: Option<i64>) -> Option<i64> {
    match (total, next) {
        (Some(a), Some(b)) => Some(a + b),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

struct AttemptBuffer {
    content: String,
    reasoning: String,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    last_flush: Instant,
    dirty_chars: usize,
}

impl AttemptBuffer {
    fn new() -> Self {
        Self {
            content: String::new(),
            reasoning: String::new(),
            input_tokens: None,
            output_tokens: None,
            last_flush: Instant::now(),
            dirty_chars: 0,
        }
    }
}

fn apply_event(state: &mut AttemptBuffer, event: &StreamEvent) {
    match event {
        StreamEvent::Delta { content } => {
            state.dirty_chars += content.len();
            state.content.push_str(content);
        }
        StreamEvent::Reasoning { content } => {
            if !state.reasoning.is_empty() {
                state.reasoning.push('\n');
            }
            state.reasoning.push_str(content);
        }
        StreamEvent::Usage {
            input_tokens,
            output_tokens,
        } => {
            if input_tokens.is_some() {
                state.input_tokens = *input_tokens;
            }
            if output_tokens.is_some() {
                state.output_tokens = *output_tokens;
            }
        }
        _ => {}
    }
}

async fn retry_delay(cancel: &CancellationToken) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(RETRY_DELAY) => true,
        _ = cancel.cancelled() => false,
    }
}

#[allow(clippy::too_many_arguments)]
async fn run(
    ctx: Arc<AppContext>,
    conversation: Conversation,
    assistant_id: String,
    generation_id: String,
    targets: Vec<Target>,
    channel: Channel<StreamEvent>,
    images: Vec<ImagePart>,
    cancel: CancellationToken,
) -> Result<Message, AppError> {
    let assistant = ctx.db.get_message(&assistant_id)?;
    let mut committed_content = assistant.content.clone();
    let mut committed_reasoning = assistant.reasoning.clone().unwrap_or_default();
    let mut total_input = assistant.input_tokens;
    let mut total_output = assistant.output_tokens;
    let mut run_input: Option<i64> = None;
    let mut run_output: Option<i64> = None;

    ctx.db.start_generation(&generation_id)?;

    let mut attempts = 0usize;
    loop {
        let target = targets[attempts % targets.len()].clone();
        attempts += 1;
        let attempt = ctx.db.create_attempt(
            &generation_id,
            &target.provider_id,
            &target.account_id,
            &target.model,
        )?;
        ctx.db.set_message_target(
            &assistant_id,
            &target.provider_id,
            &target.account_id,
            &target.model,
        )?;
        let segment = ctx.db.create_segment(
            &assistant_id,
            Some(&generation_id),
            Some(&attempt.id),
            Some(&target.provider_id),
            Some(&target.account_id),
            Some(&target.model),
        )?;

        let _ = channel.send(StreamEvent::AttemptStarted {
            generation_id: generation_id.clone(),
            attempt_no: attempt.attempt_no,
            provider_id: target.provider_id.clone(),
            account_id: target.account_id.clone(),
            model: target.model.clone(),
        });
        let _ = channel.send(StreamEvent::SegmentStarted {
            message_id: assistant_id.clone(),
            segment_id: segment.id.clone(),
            seq: segment.seq,
        });

        let mut messages = history(&ctx, &conversation.id, assistant.seq, &committed_content)?;
        if !images.is_empty() {
            let user_index = messages
                .iter()
                .rposition(|message| message.role == "user")
                .unwrap_or(0);
            if let Some(message) = messages.get_mut(user_index) {
                message.images = images.clone();
            }
        }
        let request = ChatRequest {
            model: target.model.clone(),
            messages,
            system: compose_system(&ctx, &conversation),
            temperature: None,
            max_tokens: Some(setting_i64(&ctx, "maxOutputTokens", 4096)),
            keep_alive: setting_string(&ctx, "ollamaKeepAlive"),
        };

        let buffer = Arc::new(Mutex::new(AttemptBuffer::new()));
        let shared = buffer.clone();
        let sender = channel.clone();
        let db = ctx.db.clone();
        let message_id = assistant_id.clone();
        let segment_id = segment.id.clone();
        let base_content = committed_content.clone();
        let base_reasoning = committed_reasoning.clone();
        let attempt_base_input = append_tokens(total_input, run_input);
        let attempt_base_output = append_tokens(total_output, run_output);
        let mut emit = move |event: StreamEvent| {
            let flush = if let Ok(mut state) = shared.lock() {
                apply_event(&mut state, &event);
                if state.dirty_chars >= FLUSH_CHARS || state.last_flush.elapsed() >= FLUSH_INTERVAL {
                    state.last_flush = Instant::now();
                    state.dirty_chars = 0;
                    Some((
                        state.content.clone(),
                        combine(&base_reasoning, &state.reasoning),
                        state.input_tokens,
                        state.output_tokens,
                    ))
                } else {
                    None
                }
            } else {
                None
            };
            if let Some((content, reasoning, input, output)) = flush {
                let _ = db.update_segment(
                    &segment_id,
                    Some(&content),
                    Some(&reasoning),
                    Some("streaming"),
                    input,
                    output,
                );
                let _ = db.update_message(
                    &message_id,
                    Some(&format!("{base_content}{content}")),
                    Some("streaming"),
                    None,
                    Some(&reasoning),
                    append_tokens(attempt_base_input, input),
                    append_tokens(attempt_base_output, output),
                );
            }
            sender.send(event).is_ok()
        };

        let outcome =
            providers::stream_chat_for(target.kind, &ctx.http, &target.config, request, cancel.clone(), &mut emit)
                .await;
        drop(emit);

        let (content, reasoning, input_tokens, output_tokens) = {
            let state = buffer
                .lock()
                .map_err(|_| AppError::internal("generation buffer unavailable"))?;
            (
                state.content.clone(),
                state.reasoning.clone(),
                state.input_tokens,
                state.output_tokens,
            )
        };

        match outcome {
            Ok(()) => {
                let final_content = format!("{committed_content}{content}");
                let final_reasoning = combine(&committed_reasoning, &reasoning);
                let final_reasoning = final_reasoning.trim();
                let reasoning_ref = if final_reasoning.is_empty() {
                    None
                } else {
                    Some(final_reasoning)
                };
                run_input = append_tokens(run_input, input_tokens);
                run_output = append_tokens(run_output, output_tokens);
                total_input = append_tokens(total_input, run_input);
                total_output = append_tokens(total_output, run_output);
                ctx.db.update_segment(
                    &segment.id,
                    Some(&content),
                    reasoning_ref,
                    Some("complete"),
                    input_tokens,
                    output_tokens,
                )?;
                ctx.db.finish_attempt(
                    &attempt.id,
                    "completed",
                    None,
                    None,
                    None,
                    input_tokens,
                    output_tokens,
                )?;
                ctx.db.complete_message(
                    &assistant_id,
                    &final_content,
                    reasoning_ref,
                    total_input,
                    total_output,
                )?;
                ctx.db
                    .finish_generation(&generation_id, "completed", None, None)?;
                let _ = ctx.db.insert_usage(
                    Some(&target.provider_id),
                    Some(&target.account_id),
                    Some(&target.model),
                    Some(&conversation.id),
                    Some(&generation_id),
                    run_input.unwrap_or(0),
                    run_output.unwrap_or(0),
                );
                return ctx.db.get_message(&assistant_id);
            }
            Err(ProviderError::Cancelled) => {
                ctx.db.update_segment(
                    &segment.id,
                    Some(&content),
                    Some(&reasoning),
                    Some("interrupted"),
                    input_tokens,
                    output_tokens,
                )?;
                ctx.db.finish_attempt(
                    &attempt.id,
                    "cancelled",
                    Some("cancelled"),
                    Some("Generation cancelled"),
                    None,
                    input_tokens,
                    output_tokens,
                )?;
                ctx.db
                    .finish_generation(&generation_id, "cancelled", Some("cancelled"), Some("Generation cancelled"))?;
                run_input = append_tokens(run_input, input_tokens);
                run_output = append_tokens(run_output, output_tokens);
                ctx.db.update_message(
                    &assistant_id,
                    Some(&format!("{committed_content}{content}")),
                    Some("cancelled"),
                    Some("Generation cancelled"),
                    Some(&combine(&committed_reasoning, &reasoning)),
                    append_tokens(total_input, run_input),
                    append_tokens(total_output, run_output),
                )?;
                return ctx.db.get_message(&assistant_id);
            }
            Err(error) => {
                let retryable = accounts::is_retryable(&error);
                let has_content = !content.trim().is_empty() || !reasoning.trim().is_empty();
                ctx.db.update_segment(
                    &segment.id,
                    Some(&content),
                    Some(&reasoning),
                    Some(if has_content { "interrupted" } else { "failed" }),
                    input_tokens,
                    output_tokens,
                )?;
                ctx.db.finish_attempt(
                    &attempt.id,
                    "failed",
                    Some(error.code()),
                    Some(&error.to_string()),
                    None,
                    input_tokens,
                    output_tokens,
                )?;

                if retryable && attempts < MAX_ATTEMPTS {
                    if !content.is_empty() {
                        committed_content.push_str(&content);
                    }
                    committed_reasoning = combine(&committed_reasoning, &reasoning);
                    run_input = append_tokens(run_input, input_tokens);
                    run_output = append_tokens(run_output, output_tokens);
                    ctx.db.update_message(
                        &assistant_id,
                        Some(&committed_content),
                        Some("streaming"),
                        None,
                        Some(&committed_reasoning),
                        append_tokens(total_input, run_input),
                        append_tokens(total_output, run_output),
                    )?;
                    let _ = channel.send(StreamEvent::RetryScheduled {
                        attempt_no: attempt.attempt_no + 1,
                        delay_ms: RETRY_DELAY.as_millis() as u64,
                        code: error.code().to_string(),
                        message: error.to_string(),
                    });
                    if !retry_delay(&cancel).await {
                        ctx.db.finish_generation(
                            &generation_id,
                            "cancelled",
                            Some("cancelled"),
                            Some("Generation cancelled"),
                        )?;
                        ctx.db.update_message(
                            &assistant_id,
                            Some(&committed_content),
                            Some("cancelled"),
                            Some("Generation cancelled"),
                            Some(&committed_reasoning),
                            append_tokens(total_input, run_input),
                            append_tokens(total_output, run_output),
                        )?;
                        return ctx.db.get_message(&assistant_id);
                    }
                    continue;
                }

                committed_content.push_str(&content);
                committed_reasoning = combine(&committed_reasoning, &reasoning);
                run_input = append_tokens(run_input, input_tokens);
                run_output = append_tokens(run_output, output_tokens);
                total_input = append_tokens(total_input, run_input);
                total_output = append_tokens(total_output, run_output);
                let reason = error.to_string();
                ctx.db.update_message(
                    &assistant_id,
                    Some(&committed_content),
                    Some("error"),
                    None,
                    Some(&committed_reasoning),
                    total_input,
                    total_output,
                )?;
                ctx.db.mark_message_interrupted(&assistant_id, &reason)?;
                let status = if committed_content.trim().is_empty() {
                    "failed"
                } else {
                    "interrupted"
                };
                ctx.db
                    .finish_generation(&generation_id, status, Some(error.code()), Some(&reason))?;
                let _ = ctx.db.insert_usage(
                    Some(&target.provider_id),
                    Some(&target.account_id),
                    Some(&target.model),
                    Some(&conversation.id),
                    Some(&generation_id),
                    run_input.unwrap_or(0),
                    run_output.unwrap_or(0),
                );
                let _ = channel.send(StreamEvent::Interrupted {
                    message_id: assistant_id.clone(),
                    reason: reason.clone(),
                    can_continue: true,
                });
                return Err(AppError::new(error.code(), reason));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn send_message(
    ctx: Arc<AppContext>,
    conversation_id: String,
    content: String,
    attachment_ids: Vec<String>,
    channel: Channel<StreamEvent>,
) -> Result<Message, AppError> {
    let conversation = ctx.db.get_conversation(&conversation_id)?;
    let trimmed = content.trim_end().to_string();
    if trimmed.trim().is_empty() && attachment_ids.is_empty() {
        return Err(AppError::invalid("message is empty"));
    }
    let targets = accounts::targets_for(&ctx, &conversation)?;

    let user_message = ctx.db.insert_message(
        &conversation_id,
        "user",
        &trimmed,
        "complete",
        conversation.provider_id.as_deref(),
        conversation.account_id.as_deref(),
        conversation.model.as_deref(),
    )?;
    if !attachment_ids.is_empty() {
        ctx.db.attach_to_message(&attachment_ids, &user_message.id)?;
    }
    auto_title(&ctx, &conversation, &trimmed);

    let assistant = ctx.db.insert_message(
        &conversation_id,
        "assistant",
        "",
        "streaming",
        conversation.provider_id.as_deref(),
        conversation.account_id.as_deref(),
        conversation.model.as_deref(),
    )?;
    let generation = ctx.db.create_generation(
        &conversation_id,
        &user_message.id,
        &assistant.id,
        conversation.provider_id.as_deref(),
        conversation.account_id.as_deref(),
        conversation.model.as_deref(),
    )?;

    let user_attachments = ctx.db.list_attachments_for_message(&user_message.id)?;
    let images = match attachments::image_parts(&ctx, &user_attachments) {
        Ok(images) => images,
        Err(error) => {
            let message = error.to_string();
            ctx.db.mark_message_interrupted(&assistant.id, &message)?;
            let _ = ctx.db.finish_generation(
                &generation.id,
                "failed",
                Some(error.code.as_str()),
                Some(&message),
            );
            return Err(error);
        }
    };

    let cancel = ctx.register_chat(&assistant.id);
    if channel
        .send(StreamEvent::Started {
            message_id: assistant.id.clone(),
            user_message_id: user_message.id.clone(),
        })
        .is_err()
    {
        ctx.finish_chat(&assistant.id);
        ctx.db.update_message(
            &assistant.id,
            Some(""),
            Some("cancelled"),
            Some("The interface disconnected before the response started"),
            None,
            None,
            None,
        )?;
        let _ = ctx.db.finish_generation(
            &generation.id,
            "cancelled",
            Some("cancelled"),
            Some("The interface disconnected before the response started"),
        );
        return ctx.db.get_message(&assistant.id);
    }

    let result = run(
        ctx.clone(),
        conversation,
        assistant.id.clone(),
        generation.id.clone(),
        targets,
        channel,
        images,
        cancel,
    )
    .await;
    ctx.finish_chat(&assistant.id);
    result
}

pub async fn continue_generation(
    ctx: Arc<AppContext>,
    conversation_id: String,
    message_id: String,
    channel: Channel<StreamEvent>,
) -> Result<Message, AppError> {
    let conversation = ctx.db.get_conversation(&conversation_id)?;
    let assistant = ctx.db.get_message(&message_id)?;
    if assistant.conversation_id != conversation_id {
        return Err(AppError::invalid("message does not belong to this conversation"));
    }
    if assistant.role != "assistant" {
        return Err(AppError::invalid("only assistant responses can be continued"));
    }
    if assistant.status == "streaming" {
        return Err(AppError::invalid("this response is already generating"));
    }

    let existing = ctx.db.latest_generation_for_message(&message_id)?;
    let generation = match existing {
        Some(generation) if generation.status == "completed" => {
            return Err(AppError::invalid("this response is already complete"));
        }
        Some(generation) => generation,
        None => {
            let user_message = ctx
                .db
                .list_messages(&conversation_id)?
                .into_iter()
                .map(|view| view.message)
                .rfind(|message| message.role == "user" && message.seq < assistant.seq)
                .ok_or_else(|| AppError::invalid("no user message to continue from"))?;
            ctx.db.create_generation(
                &conversation_id,
                &user_message.id,
                &assistant.id,
                conversation.provider_id.as_deref(),
                conversation.account_id.as_deref(),
                conversation.model.as_deref(),
            )?
        }
    };

    let targets = accounts::targets_for(&ctx, &conversation)?;
    ctx.db.restart_message(&assistant.id)?;
    let cancel = ctx.register_chat(&assistant.id);
    if channel
        .send(StreamEvent::Started {
            message_id: assistant.id.clone(),
            user_message_id: generation.user_message_id.clone(),
        })
        .is_err()
    {
        ctx.finish_chat(&assistant.id);
        ctx.db.update_message(
            &assistant.id,
            None,
            Some("error"),
            None,
            None,
            None,
            None,
        )?;
        ctx.db.mark_message_interrupted(
            &assistant.id,
            "The interface disconnected before the response resumed",
        )?;
        return ctx.db.get_message(&assistant.id);
    }

    let result = run(
        ctx.clone(),
        conversation,
        assistant.id.clone(),
        generation.id,
        targets,
        channel,
        Vec::new(),
        cancel,
    )
    .await;
    ctx.finish_chat(&assistant.id);
    result
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    use tauri::ipc::{Channel, InvokeResponseBody};

    use super::*;
    use crate::database::Db;
    use crate::providers::StreamEvent;
    use crate::state::{AppContext, AppPaths};

    fn paths(root: &Path) -> AppPaths {
        AppPaths {
            data_dir: root.to_path_buf(),
            db_path: root.join("db.sqlite3"),
            artifacts_dir: root.join("artifacts"),
            attachments_dir: root.join("attachments"),
            logs_dir: root.join("logs"),
        }
    }

    fn mock_server(responses: Vec<(u16, String)>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
        let address = listener.local_addr().expect("mock address");
        std::thread::spawn(move || {
            for (status, body) in responses {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut request = [0u8; 8192];
                let _ = stream.read(&mut request);
                let reason = if status == 200 { "OK" } else { "Error" };
                let response = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/x-ndjson\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        format!("http://{address}")
    }

    fn collect() -> (Channel<StreamEvent>, Arc<Mutex<Vec<String>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = events.clone();
        let channel = Channel::new(move |body: InvokeResponseBody| {
            if let InvokeResponseBody::Json(json) = body {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json) {
                    if let Some(kind) = value.get("type").and_then(|value| value.as_str()) {
                        sink.lock().unwrap().push(kind.to_string());
                    }
                }
            }
            Ok(())
        });
        (channel, events)
    }

    async fn setup(base_url: &str) -> (Arc<AppContext>, crate::database::models::Conversation) {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.keep();
        let db = Arc::new(Db::open_in_memory().expect("db"));
        db.update_provider("ollama", None, Some(base_url), None)
            .expect("provider");
        let conversation = db
            .create_conversation(
                None,
                Some("New conversation"),
                Some("ollama"),
                Some("ollama-default"),
                Some("mock-model"),
            )
            .expect("conversation");
        let ctx = AppContext::new(db, paths(&root));
        (ctx, conversation)
    }

    #[tokio::test]
    async fn retries_a_transient_failure_and_completes_the_response() {
        let base_url = mock_server(vec![
            (500, "temporary\n".into()),
            (
                200,
                "{\"message\":{\"thinking\":\"why\",\"content\":\"hello\"},\"done\":false}\n\
                 {\"message\":{\"content\":\" world\"},\"done\":false}\n\
                 {\"done\":true,\"done_reason\":\"stop\",\"prompt_eval_count\":3,\"eval_count\":2}\n"
                    .into(),
            ),
        ]);
        let (ctx, conversation) = setup(&base_url).await;
        let (channel, events) = collect();

        let message = send_message(
            ctx.clone(),
            conversation.id.clone(),
            "hello".into(),
            Vec::new(),
            channel,
        )
        .await
        .expect("generation succeeds after retry");

        assert_eq!(message.content, "hello world");
        assert_eq!(message.reasoning.as_deref(), Some("why"));
        assert_eq!(message.status, "complete");
        assert_eq!(message.input_tokens, Some(3));
        assert_eq!(message.output_tokens, Some(2));

        let generation = ctx
            .db
            .latest_generation_for_message(&message.id)
            .expect("generation lookup")
            .expect("generation");
        assert_eq!(generation.status, "completed");
        assert_eq!(generation.attempt_count, 2);
        let segments = ctx.db.list_segments(&message.id).expect("segments");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[1].status, "complete");

        let events = events.lock().unwrap();
        assert_eq!(events.iter().filter(|kind| *kind == "attemptStarted").count(), 2);
        assert!(events.iter().any(|kind| kind == "retryScheduled"));
        assert!(!events.iter().any(|kind| kind == "interrupted"));
    }

    #[tokio::test]
    async fn continues_an_interrupted_response_in_the_same_generation() {
        let base_url = mock_server(vec![(
            200,
            "{\"message\":{\"content\":\"second\"},\"done\":false}\n\
             {\"done\":true,\"done_reason\":\"stop\",\"prompt_eval_count\":4,\"eval_count\":1}\n"
                .into(),
        )]);
        let (ctx, conversation) = setup(&base_url).await;
        let user = ctx
            .db
            .insert_message(
                &conversation.id,
                "user",
                "hello",
                "complete",
                Some("ollama"),
                Some("ollama-default"),
                Some("mock-model"),
            )
            .expect("user");
        let assistant = ctx
            .db
            .insert_message(
                &conversation.id,
                "assistant",
                "first ",
                "error",
                Some("ollama"),
                Some("ollama-default"),
                Some("mock-model"),
            )
            .expect("assistant");
        ctx.db
            .mark_message_interrupted(&assistant.id, "connection reset")
            .expect("interrupt");
        let generation = ctx
            .db
            .create_generation(
                &conversation.id,
                &user.id,
                &assistant.id,
                Some("ollama"),
                Some("ollama-default"),
                Some("mock-model"),
            )
            .expect("generation");
        ctx.db
            .finish_generation(&generation.id, "interrupted", Some("network"), Some("reset"))
            .expect("finish generation");
        let (channel, _) = collect();

        let message = continue_generation(
            ctx.clone(),
            conversation.id.clone(),
            assistant.id.clone(),
            channel,
        )
        .await
        .expect("continuation succeeds");

        assert_eq!(message.content, "first second");
        assert_eq!(message.status, "complete");
        assert!(!message.interrupted);
        assert_eq!(message.input_tokens, Some(4));
        let latest = ctx
            .db
            .latest_generation_for_message(&assistant.id)
            .expect("generation lookup")
            .expect("generation");
        assert_eq!(latest.id, generation.id);
        assert_eq!(latest.status, "completed");
        assert_eq!(latest.attempt_count, 1);
    }
}
