use std::time::Instant;

use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use crate::providers::{
    emit_or_cancel, normalize_base, response_error, ChatRequest, Completion, Emit, ModelInfo,
    ProviderConfig, ProviderError, StreamEvent,
};

const ANTHROPIC_VERSION: &str = "2023-06-01";

fn require_key(config: &ProviderConfig) -> Result<&str, ProviderError> {
    config
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .ok_or_else(|| ProviderError::NotConfigured("add an Anthropic API key for this account".into()))
}

fn endpoint(config: &ProviderConfig, path: &str) -> String {
    format!("{}{}", normalize_base(&config.base_url), path)
}

pub async fn health(
    client: &reqwest::Client,
    config: &ProviderConfig,
) -> Result<(String, u64), ProviderError> {
    let key = require_key(config)?;
    let started = Instant::now();
    let response = client
        .get(endpoint(config, "/v1/models?limit=1"))
        .header("x-api-key", key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .send()
        .await?;
    let latency = started.elapsed().as_millis() as u64;
    if !response.status().is_success() {
        return Err(response_error(response).await);
    }
    Ok(("Anthropic Messages API".to_string(), latency))
}

pub async fn list_models(
    client: &reqwest::Client,
    config: &ProviderConfig,
) -> Result<Vec<ModelInfo>, ProviderError> {
    let key = require_key(config)?;
    let response = client
        .get(endpoint(config, "/v1/models?limit=100"))
        .header("x-api-key", key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(response_error(response).await);
    }
    let value: Value = response.json().await?;
    let models = value
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(models
        .into_iter()
        .map(|model| {
            let id = model
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            ModelInfo {
                name: model
                    .get("display_name")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| id.clone()),
                id,
                size_bytes: None,
                family: Some("Claude".into()),
                parameter_size: None,
                quantization: None,
                capabilities: vec!["completion".into(), "vision".into()],
                modified_at: model
                    .get("created_at")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            }
        })
        .collect())
}

fn request_body(request: &ChatRequest, stream: bool) -> Value {
    let mut system = request
        .system
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string);
    let mut messages: Vec<Value> = Vec::new();
    for message in &request.messages {
        if message.role == "system" {
            if !message.content.trim().is_empty() {
                system = Some(match system {
                    Some(existing) => format!("{existing}\n\n{}", message.content),
                    None => message.content.clone(),
                });
            }
            continue;
        }
        let mut blocks: Vec<Value> = Vec::new();
        for image in &message.images {
            blocks.push(json!({
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": image.mime,
                    "data": image.data_base64
                }
            }));
        }
        if !message.content.is_empty() {
            blocks.push(json!({ "type": "text", "text": message.content }));
        }
        if blocks.is_empty() {
            continue;
        }
        if let Some(last) = messages.last_mut() {
            if last.get("role").and_then(Value::as_str) == Some(message.role.as_str()) {
                if let Some(existing) = last.get_mut("content").and_then(Value::as_array_mut) {
                    existing.extend(blocks);
                    continue;
                }
            }
        }
        messages.push(json!({ "role": message.role, "content": blocks }));
    }

    let mut body = json!({
        "model": request.model,
        "max_tokens": request.max_tokens.unwrap_or(4096),
        "messages": messages,
        "stream": stream,
    });
    if let Some(system) = system {
        body["system"] = json!(system);
    }
    if let Some(temperature) = request.temperature {
        body["temperature"] = json!(temperature);
    }
    body
}

pub async fn stream_chat(
    client: &reqwest::Client,
    config: &ProviderConfig,
    request: ChatRequest,
    cancel: CancellationToken,
    emit: &mut Emit,
) -> Result<(), ProviderError> {
    let key = require_key(config)?.to_string();
    let url = endpoint(config, "/v1/messages");
    let response = tokio::select! {
        _ = cancel.cancelled() => return Err(ProviderError::Cancelled),
        result = client
            .post(url)
            .header("x-api-key", key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request_body(&request, true))
            .send() => result?,
    };
    if !response.status().is_success() {
        return Err(response_error(response).await);
    }

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut done = false;
    let mut input_tokens: Option<i64> = None;
    let mut output_tokens: Option<i64> = None;

    loop {
        let next = tokio::select! {
            _ = cancel.cancelled() => return Err(ProviderError::Cancelled),
            item = stream.next() => item,
        };
        let Some(chunk) = next else {
            break;
        };
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(position) = buffer.find('\n') {
            let line = buffer[..position].trim_end_matches('\r').to_string();
            buffer.drain(..=position);
            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data.is_empty() || data == "[DONE]" {
                continue;
            }
            let value: Value = serde_json::from_str(data)
                .map_err(|error| ProviderError::BadResponse(error.to_string()))?;
            let event_type = value.get("type").and_then(Value::as_str).unwrap_or("");
            match event_type {
                "message_start" => {
                    input_tokens = value
                        .get("message")
                        .and_then(|m| m.get("usage"))
                        .and_then(|u| u.get("input_tokens"))
                        .and_then(Value::as_i64);
                }
                "content_block_delta" => {
                    let delta = value.get("delta").cloned().unwrap_or(Value::Null);
                    match delta.get("type").and_then(Value::as_str) {
                        Some("text_delta") => {
                            if let Some(text) = delta.get("text").and_then(Value::as_str) {
                                if !text.is_empty() {
                                    emit_or_cancel(
                                        emit,
                                        StreamEvent::Delta { content: text.into() },
                                    )?;
                                }
                            }
                        }
                        Some("thinking_delta") => {
                            if let Some(text) = delta.get("thinking").and_then(Value::as_str) {
                                if !text.is_empty() {
                                    emit_or_cancel(
                                        emit,
                                        StreamEvent::Reasoning { content: text.into() },
                                    )?;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                "message_delta" => {
                    output_tokens = value
                        .get("usage")
                        .and_then(|u| u.get("output_tokens"))
                        .and_then(Value::as_i64)
                        .or(output_tokens);
                    let finish_reason = value
                        .get("delta")
                        .and_then(|d| d.get("stop_reason"))
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    emit_or_cancel(
                        emit,
                        StreamEvent::Usage {
                            input_tokens,
                            output_tokens,
                        },
                    )?;
                    if finish_reason.is_some() {
                        emit_or_cancel(emit, StreamEvent::Done { finish_reason })?;
                        done = true;
                    }
                }
                "message_stop" => {
                    if !done {
                        emit_or_cancel(emit, StreamEvent::Done { finish_reason: None })?;
                        done = true;
                    }
                }
                "error" => {
                    let message = value
                        .get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(Value::as_str)
                        .unwrap_or("Anthropic streaming error")
                        .to_string();
                    return Err(ProviderError::BadResponse(message));
                }
                _ => {}
            }
        }
    }

    if !done {
        emit_or_cancel(emit, StreamEvent::Done { finish_reason: None })?;
    }
    Ok(())
}

pub async fn complete(
    client: &reqwest::Client,
    config: &ProviderConfig,
    request: ChatRequest,
) -> Result<Completion, ProviderError> {
    let key = require_key(config)?.to_string();
    let response = client
        .post(endpoint(config, "/v1/messages"))
        .header("x-api-key", key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(&request_body(&request, false))
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(response_error(response).await);
    }
    let value: Value = response.json().await?;
    let content = value
        .get("content")
        .and_then(Value::as_array)
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|block| block.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    let usage = value.get("usage").cloned().unwrap_or(Value::Null);
    Ok(Completion {
        content,
        reasoning: None,
        input_tokens: usage.get("input_tokens").and_then(Value::as_i64),
        output_tokens: usage.get("output_tokens").and_then(Value::as_i64),
        finish_reason: value
            .get("stop_reason")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}
