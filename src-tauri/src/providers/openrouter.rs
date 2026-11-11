use std::time::Instant;

use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use crate::providers::{
    emit_or_cancel, normalize_base, response_error, ChatRequest, Completion, Emit, ModelInfo,
    ProviderConfig, ProviderError, StreamEvent,
};

const TITLE: &str = "Relay Routing";

fn require_key(config: &ProviderConfig) -> Result<&str, ProviderError> {
    config
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .ok_or_else(|| {
            ProviderError::NotConfigured("add an OpenRouter API key for this account".into())
        })
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
        .get(endpoint(config, "/v1/key"))
        .header("authorization", format!("Bearer {key}"))
        .header("x-title", TITLE)
        .send()
        .await?;
    let latency = started.elapsed().as_millis() as u64;
    if !response.status().is_success() {
        return Err(response_error(response).await);
    }
    Ok(("OpenRouter API".to_string(), latency))
}

pub async fn list_models(
    client: &reqwest::Client,
    config: &ProviderConfig,
) -> Result<Vec<ModelInfo>, ProviderError> {
    let key = require_key(config)?;
    let response = client
        .get(endpoint(config, "/v1/models"))
        .header("authorization", format!("Bearer {key}"))
        .header("x-title", TITLE)
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
        .filter_map(|model| {
            let id = model.get("id").and_then(Value::as_str)?.to_string();
            let mut capabilities = vec!["completion".to_string()];
            let modalities = model
                .get("architecture")
                .and_then(|a| a.get("input_modalities"))
                .and_then(Value::as_array);
            if let Some(modalities) = modalities {
                if modalities.iter().any(|m| m.as_str() == Some("image")) {
                    capabilities.push("vision".into());
                }
            }
            Some(ModelInfo {
                name: model
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| id.clone()),
                family: model
                    .get("architecture")
                    .and_then(|a| a.get("modality"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                size_bytes: None,
                parameter_size: None,
                quantization: None,
                capabilities,
                modified_at: None,
                id,
            })
        })
        .collect())
}

fn content_value(message: &crate::providers::ChatMessage) -> Value {
    if message.images.is_empty() {
        return json!(message.content);
    }
    let mut parts: Vec<Value> = Vec::new();
    if !message.content.is_empty() {
        parts.push(json!({ "type": "text", "text": message.content }));
    }
    for image in &message.images {
        parts.push(json!({
            "type": "image_url",
            "image_url": { "url": format!("data:{};base64,{}", image.mime, image.data_base64) }
        }));
    }
    json!(parts)
}

fn request_body(request: &ChatRequest, stream: bool) -> Value {
    let mut messages: Vec<Value> = Vec::with_capacity(request.messages.len() + 1);
    if let Some(system) = &request.system {
        if !system.trim().is_empty() {
            messages.push(json!({ "role": "system", "content": system }));
        }
    }
    for message in &request.messages {
        if message.role == "system" {
            if !message.content.trim().is_empty() {
                messages.push(json!({ "role": "system", "content": message.content }));
            }
            continue;
        }
        let role = if message.role == "assistant" {
            "assistant"
        } else {
            "user"
        };
        messages.push(json!({ "role": role, "content": content_value(message) }));
    }
    let mut body = json!({
        "model": request.model,
        "messages": messages,
        "stream": stream,
    });
    if stream {
        body["stream_options"] = json!({ "include_usage": true });
    }
    if let Some(temperature) = request.temperature {
        body["temperature"] = json!(temperature);
    }
    if let Some(max_tokens) = request.max_tokens {
        body["max_tokens"] = json!(max_tokens);
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
    let url = endpoint(config, "/v1/chat/completions");
    let response = tokio::select! {
        _ = cancel.cancelled() => return Err(ProviderError::Cancelled),
        result = client
            .post(url)
            .header("authorization", format!("Bearer {key}"))
            .header("x-title", TITLE)
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
            if data.is_empty() {
                continue;
            }
            if data == "[DONE]" {
                if !done {
                    emit_or_cancel(emit, StreamEvent::Done { finish_reason: None })?;
                    done = true;
                }
                continue;
            }
            let value: Value = serde_json::from_str(data)
                .map_err(|error| ProviderError::BadResponse(error.to_string()))?;
            if let Some(error) = value.get("error") {
                let message = error
                    .get("message")
                    .and_then(Value::as_str)
                    .or_else(|| error.as_str())
                    .unwrap_or("OpenRouter streaming error")
                    .to_string();
                return Err(ProviderError::BadResponse(message));
            }
            if let Some(usage) = value.get("usage").filter(|u| !u.is_null()) {
                input_tokens = usage
                    .get("prompt_tokens")
                    .and_then(Value::as_i64)
                    .or(input_tokens);
                output_tokens = usage
                    .get("completion_tokens")
                    .and_then(Value::as_i64)
                    .or(output_tokens);
                emit_or_cancel(
                    emit,
                    StreamEvent::Usage {
                        input_tokens,
                        output_tokens,
                    },
                )?;
            }
            let choice = value
                .get("choices")
                .and_then(Value::as_array)
                .and_then(|choices| choices.first())
                .cloned()
                .unwrap_or(Value::Null);
            let delta = choice.get("delta").cloned().unwrap_or(Value::Null);
            if let Some(reasoning) = delta
                .get("reasoning")
                .or_else(|| delta.get("reasoning_content"))
                .and_then(Value::as_str)
            {
                if !reasoning.is_empty() {
                    emit_or_cancel(emit, StreamEvent::Reasoning { content: reasoning.into() })?;
                }
            }
            if let Some(content) = delta.get("content").and_then(Value::as_str) {
                if !content.is_empty() {
                    emit_or_cancel(emit, StreamEvent::Delta { content: content.into() })?;
                }
            }
            if let Some(finish_reason) = choice
                .get("finish_reason")
                .and_then(Value::as_str)
                .map(str::to_string)
            {
                emit_or_cancel(emit, StreamEvent::Done { finish_reason: Some(finish_reason) })?;
                done = true;
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
        .post(endpoint(config, "/v1/chat/completions"))
        .header("authorization", format!("Bearer {key}"))
        .header("x-title", TITLE)
        .header("content-type", "application/json")
        .json(&request_body(&request, false))
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(response_error(response).await);
    }
    let value: Value = response.json().await?;
    let choice = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .cloned()
        .unwrap_or(Value::Null);
    let message = choice.get("message").cloned().unwrap_or(Value::Null);
    let usage = value.get("usage").cloned().unwrap_or(Value::Null);
    Ok(Completion {
        content: message
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        reasoning: message
            .get("reasoning")
            .or_else(|| message.get("reasoning_content"))
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(str::to_string),
        input_tokens: usage.get("prompt_tokens").and_then(Value::as_i64),
        output_tokens: usage.get("completion_tokens").and_then(Value::as_i64),
        finish_reason: choice
            .get("finish_reason")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}
