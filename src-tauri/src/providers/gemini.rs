use std::time::Instant;

use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use crate::providers::{
    emit_or_cancel, normalize_base, response_error, ChatRequest, Completion, Emit, ModelInfo,
    ProviderConfig, ProviderError, StreamEvent,
};

fn require_key(config: &ProviderConfig) -> Result<&str, ProviderError> {
    config
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .ok_or_else(|| {
            ProviderError::NotConfigured("add a Google Gemini API key for this account".into())
        })
}

fn endpoint(config: &ProviderConfig, path: &str) -> String {
    format!("{}{}", normalize_base(&config.base_url), path)
}

fn model_path(model: &str) -> String {
    model.trim().trim_start_matches("models/").to_string()
}

pub async fn health(
    client: &reqwest::Client,
    config: &ProviderConfig,
) -> Result<(String, u64), ProviderError> {
    let key = require_key(config)?;
    let started = Instant::now();
    let response = client
        .get(endpoint(config, "/v1beta/models?pageSize=1"))
        .header("x-goog-api-key", key)
        .send()
        .await?;
    let latency = started.elapsed().as_millis() as u64;
    if !response.status().is_success() {
        return Err(response_error(response).await);
    }
    Ok(("Gemini API".to_string(), latency))
}

pub async fn list_models(
    client: &reqwest::Client,
    config: &ProviderConfig,
) -> Result<Vec<ModelInfo>, ProviderError> {
    let key = require_key(config)?;
    let response = client
        .get(endpoint(config, "/v1beta/models?pageSize=1000"))
        .header("x-goog-api-key", key)
        .send()
        .await?;
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
        .filter(|model| {
            model
                .get("supportedGenerationMethods")
                .and_then(Value::as_array)
                .map(|methods| methods.iter().any(|m| m.as_str() == Some("generateContent")))
                .unwrap_or(true)
        })
        .map(|model| {
            let id = model
                .get("name")
                .and_then(Value::as_str)
                .map(model_path)
                .unwrap_or_else(|| "unknown".to_string());
            ModelInfo {
                name: model
                    .get("displayName")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| id.clone()),
                id,
                size_bytes: None,
                family: Some("Gemini".into()),
                parameter_size: None,
                quantization: None,
                capabilities: vec!["completion".into(), "vision".into()],
                modified_at: None,
            }
        })
        .collect())
}

fn request_body(request: &ChatRequest) -> Value {
    let mut system = request
        .system
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string);
    let mut contents: Vec<Value> = Vec::new();
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
        let role = if message.role == "assistant" {
            "model"
        } else {
            "user"
        };
        let mut parts: Vec<Value> = Vec::new();
        if !message.content.is_empty() {
            parts.push(json!({ "text": message.content }));
        }
        for image in &message.images {
            parts.push(json!({
                "inlineData": {
                    "mimeType": image.mime,
                    "data": image.data_base64
                }
            }));
        }
        if parts.is_empty() {
            continue;
        }
        contents.push(json!({ "role": role, "parts": parts }));
    }

    let mut body = json!({ "contents": contents });
    if let Some(system) = system {
        body["systemInstruction"] = json!({ "parts": [{ "text": system }] });
    }
    let mut generation = serde_json::Map::new();
    if let Some(temperature) = request.temperature {
        generation.insert("temperature".into(), json!(temperature));
    }
    if let Some(max_tokens) = request.max_tokens {
        generation.insert("maxOutputTokens".into(), json!(max_tokens));
    }
    if !generation.is_empty() {
        body["generationConfig"] = Value::Object(generation);
    }
    body
}

fn emit_parts(parts: &[Value], emit: &mut Emit) -> Result<(), ProviderError> {
    for part in parts {
        let Some(text) = part.get("text").and_then(Value::as_str) else {
            continue;
        };
        if text.is_empty() {
            continue;
        }
        if part.get("thought").and_then(Value::as_bool) == Some(true) {
            emit_or_cancel(emit, StreamEvent::Reasoning { content: text.into() })?;
        } else {
            emit_or_cancel(emit, StreamEvent::Delta { content: text.into() })?;
        }
    }
    Ok(())
}

pub async fn stream_chat(
    client: &reqwest::Client,
    config: &ProviderConfig,
    request: ChatRequest,
    cancel: CancellationToken,
    emit: &mut Emit,
) -> Result<(), ProviderError> {
    let key = require_key(config)?.to_string();
    let url = endpoint(
        config,
        &format!(
            "/v1beta/models/{}:streamGenerateContent?alt=sse",
            model_path(&request.model)
        ),
    );
    let response = tokio::select! {
        _ = cancel.cancelled() => return Err(ProviderError::Cancelled),
        result = client
            .post(url)
            .header("x-goog-api-key", key)
            .header("content-type", "application/json")
            .json(&request_body(&request))
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
            if let Some(error) = value.get("error") {
                let message = error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("Gemini streaming error")
                    .to_string();
                return Err(ProviderError::BadResponse(message));
            }
            if let Some(usage) = value.get("usageMetadata") {
                input_tokens = usage
                    .get("promptTokenCount")
                    .and_then(Value::as_i64)
                    .or(input_tokens);
                output_tokens = usage
                    .get("candidatesTokenCount")
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
            if let Some(candidate) = value
                .get("candidates")
                .and_then(Value::as_array)
                .and_then(|candidates| candidates.first())
            {
                if let Some(parts) = candidate
                    .get("content")
                    .and_then(|content| content.get("parts"))
                    .and_then(Value::as_array)
                {
                    emit_parts(parts, emit)?;
                }
                if let Some(finish_reason) = candidate
                    .get("finishReason")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                {
                    emit_or_cancel(
                        emit,
                        StreamEvent::Done {
                            finish_reason: Some(finish_reason),
                        },
                    )?;
                    done = true;
                }
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
    let url = endpoint(
        config,
        &format!(
            "/v1beta/models/{}:generateContent",
            model_path(&request.model)
        ),
    );
    let response = client
        .post(url)
        .header("x-goog-api-key", key)
        .header("content-type", "application/json")
        .json(&request_body(&request))
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(response_error(response).await);
    }
    let value: Value = response.json().await?;
    let candidate = value
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|candidates| candidates.first())
        .cloned()
        .unwrap_or(Value::Null);
    let parts = candidate
        .get("content")
        .and_then(|content| content.get("parts"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut content = String::new();
    let mut reasoning = String::new();
    for part in &parts {
        let Some(text) = part.get("text").and_then(Value::as_str) else {
            continue;
        };
        if part.get("thought").and_then(Value::as_bool) == Some(true) {
            reasoning.push_str(text);
        } else {
            content.push_str(text);
        }
    }
    let usage = value.get("usageMetadata").cloned().unwrap_or(Value::Null);
    Ok(Completion {
        content,
        reasoning: (!reasoning.is_empty()).then_some(reasoning),
        input_tokens: usage.get("promptTokenCount").and_then(Value::as_i64),
        output_tokens: usage.get("candidatesTokenCount").and_then(Value::as_i64),
        finish_reason: candidate
            .get("finishReason")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}
