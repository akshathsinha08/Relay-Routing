use std::time::Instant;

use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use crate::providers::{
    emit_or_cancel, normalize_base, response_error, ChatRequest, Completion, Emit, ModelInfo,
    ProviderConfig, ProviderError, StreamEvent,
};

pub async fn health(
    client: &reqwest::Client,
    config: &ProviderConfig,
) -> Result<(String, u64), ProviderError> {
    let started = Instant::now();
    let url = format!("{}/api/version", normalize_base(&config.base_url));
    let response = client.get(url).send().await?;
    let latency = started.elapsed().as_millis() as u64;
    if !response.status().is_success() {
        return Err(response_error(response).await);
    }
    let value: Value = response.json().await?;
    let version = value
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    Ok((version, latency))
}

pub async fn list_models(
    client: &reqwest::Client,
    config: &ProviderConfig,
) -> Result<Vec<ModelInfo>, ProviderError> {
    let url = format!("{}/api/tags", normalize_base(&config.base_url));
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(response_error(response).await);
    }
    let value: Value = response.json().await?;
    let models = value
        .get("models")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(models
        .into_iter()
        .map(|model| {
            let details = model.get("details").cloned().unwrap_or(Value::Null);
            let name = model
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| model.get("model").and_then(Value::as_str))
                .unwrap_or("unknown")
                .to_string();
            let capabilities = model
                .get("capabilities")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            ModelInfo {
                id: name.clone(),
                name,
                size_bytes: model.get("size").and_then(Value::as_i64),
                family: details
                    .get("family")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                parameter_size: details
                    .get("parameter_size")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                quantization: details
                    .get("quantization_level")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                capabilities,
                modified_at: model
                    .get("modified_at")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            }
        })
        .collect())
}

fn request_body(request: &ChatRequest, stream: bool) -> Value {
    let mut messages: Vec<Value> = Vec::with_capacity(request.messages.len() + 1);
    if let Some(system) = &request.system {
        if !system.trim().is_empty() {
            messages.push(json!({ "role": "system", "content": system }));
        }
    }
    messages.extend(request.messages.iter().map(|message| {
        let images: Vec<String> = message
            .images
            .iter()
            .map(|image| image.data_base64.clone())
            .collect();
        let mut value = json!({ "role": message.role, "content": message.content });
        if !images.is_empty() {
            value["images"] = json!(images);
        }
        value
    }));
    let mut body = json!({
        "model": request.model,
        "messages": messages,
        "stream": stream,
    });
    let mut options = serde_json::Map::new();
    if let Some(temperature) = request.temperature {
        options.insert("temperature".into(), json!(temperature));
    }
    if let Some(max_tokens) = request.max_tokens {
        options.insert("num_predict".into(), json!(max_tokens));
    }
    if !options.is_empty() {
        body["options"] = Value::Object(options);
    }
    if let Some(keep_alive) = &request.keep_alive {
        body["keep_alive"] = json!(keep_alive);
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
    let url = format!("{}/api/chat", normalize_base(&config.base_url));
    let response = tokio::select! {
        _ = cancel.cancelled() => return Err(ProviderError::Cancelled),
        result = client.post(url).json(&request_body(&request, true)).send() => result?,
    };
    if !response.status().is_success() {
        return Err(response_error(response).await);
    }

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut done = false;
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
            let line = buffer[..position].trim().to_string();
            buffer.drain(..=position);
            if line.is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(&line)
                .map_err(|error| ProviderError::BadResponse(error.to_string()))?;
            if let Some(error) = value.get("error").and_then(Value::as_str) {
                return Err(ProviderError::BadResponse(error.to_string()));
            }
            if let Some(message) = value.get("message") {
                if let Some(thinking) = message.get("thinking").and_then(Value::as_str) {
                    if !thinking.is_empty() {
                        emit_or_cancel(emit, StreamEvent::Reasoning { content: thinking.into() })?;
                    }
                }
                if let Some(content) = message.get("content").and_then(Value::as_str) {
                    if !content.is_empty() {
                        emit_or_cancel(emit, StreamEvent::Delta { content: content.into() })?;
                    }
                }
            }
            if value.get("done").and_then(Value::as_bool) == Some(true) {
                let input_tokens = value.get("prompt_eval_count").and_then(Value::as_i64);
                let output_tokens = value.get("eval_count").and_then(Value::as_i64);
                if input_tokens.is_some() || output_tokens.is_some() {
                    emit_or_cancel(
                        emit,
                        StreamEvent::Usage {
                            input_tokens,
                            output_tokens,
                        },
                    )?;
                }
                let finish_reason = value
                    .get("done_reason")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                emit_or_cancel(emit, StreamEvent::Done { finish_reason })?;
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
    let url = format!("{}/api/chat", normalize_base(&config.base_url));
    let response = client.post(url).json(&request_body(&request, false)).send().await?;
    if !response.status().is_success() {
        return Err(response_error(response).await);
    }
    let value: Value = response.json().await?;
    if let Some(error) = value.get("error").and_then(Value::as_str) {
        return Err(ProviderError::BadResponse(error.to_string()));
    }
    let message = value.get("message").cloned().unwrap_or(Value::Null);
    Ok(Completion {
        content: message
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        reasoning: message
            .get("thinking")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(str::to_string),
        input_tokens: value.get("prompt_eval_count").and_then(Value::as_i64),
        output_tokens: value.get("eval_count").and_then(Value::as_i64),
        finish_reason: value
            .get("done_reason")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}
