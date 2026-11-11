pub mod anthropic;
pub mod gemini;
pub mod ollama;
pub mod openrouter;

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum ProviderError {
    Http { status: u16, message: String },
    Auth(String),
    RateLimited(String),
    NotConfigured(String),
    BadResponse(String),
    Cancelled,
    Network(String),
}

impl ProviderError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Http { .. } => "provider_http_error",
            Self::Auth(_) => "provider_auth_error",
            Self::RateLimited(_) => "provider_rate_limited",
            Self::NotConfigured(_) => "provider_not_configured",
            Self::BadResponse(_) => "provider_bad_response",
            Self::Cancelled => "cancelled",
            Self::Network(_) => "provider_network_error",
        }
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http { status, message } => write!(f, "provider returned HTTP {status}: {message}"),
            Self::Auth(m) => write!(f, "authentication failed: {m}"),
            Self::RateLimited(m) => write!(f, "rate limited: {m}"),
            Self::NotConfigured(m) => write!(f, "provider not configured: {m}"),
            Self::BadResponse(m) => write!(f, "unexpected provider response: {m}"),
            Self::Cancelled => write!(f, "request cancelled"),
            Self::Network(m) => write!(f, "network error: {m}"),
        }
    }
}

impl std::error::Error for ProviderError {}

impl From<reqwest::Error> for ProviderError {
    fn from(value: reqwest::Error) -> Self {
        Self::Network(value.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub base_url: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImagePart {
    pub mime: String,
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub images: Vec<ImagePart>,
}

impl ChatMessage {
    pub fn text(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            images: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub system: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
    pub keep_alive: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StreamEvent {
    #[serde(rename_all = "camelCase")]
    Started {
        message_id: String,
        user_message_id: String,
    },
    Delta {
        content: String,
    },
    Reasoning {
        content: String,
    },
    #[serde(rename_all = "camelCase")]
    AttemptStarted {
        generation_id: String,
        attempt_no: i64,
        provider_id: String,
        account_id: String,
        model: String,
    },
    #[serde(rename_all = "camelCase")]
    SegmentStarted {
        message_id: String,
        segment_id: String,
        seq: i64,
    },
    #[serde(rename_all = "camelCase")]
    RetryScheduled {
        attempt_no: i64,
        delay_ms: u64,
        code: String,
        message: String,
    },
    #[serde(rename_all = "camelCase")]
    Interrupted {
        message_id: String,
        reason: String,
        can_continue: bool,
    },
    #[serde(rename_all = "camelCase")]
    Usage {
        input_tokens: Option<i64>,
        output_tokens: Option<i64>,
    },
    #[serde(rename_all = "camelCase")]
    Done {
        finish_reason: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub size_bytes: Option<i64>,
    pub family: Option<String>,
    pub parameter_size: Option<String>,
    pub quantization: Option<String>,
    pub capabilities: Vec<String>,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Completion {
    pub content: String,
    pub reasoning: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub finish_reason: Option<String>,
}

pub type Emit = dyn FnMut(StreamEvent) -> bool + Send;

pub(crate) fn normalize_base(base_url: &str) -> String {
    base_url.trim().trim_end_matches('/').to_string()
}

pub(crate) async fn response_error(response: reqwest::Response) -> ProviderError {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let message = extract_error_message(&body).unwrap_or_else(|| {
        let trimmed = body.trim();
        if trimmed.is_empty() {
            status
                .canonical_reason()
                .unwrap_or("provider request failed")
                .to_string()
        } else {
            trimmed.chars().take(600).collect()
        }
    });
    match status.as_u16() {
        401 | 403 => ProviderError::Auth(message),
        429 => ProviderError::RateLimited(message),
        code => ProviderError::Http {
            status: code,
            message,
        },
    }
}

pub(crate) fn extract_error_message(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let error = value.get("error")?;
    if let Some(text) = error.as_str() {
        return Some(text.to_string());
    }
    error
        .get("message")
        .and_then(|m| m.as_str())
        .map(str::to_string)
}

pub(crate) fn emit_or_cancel(
    emit: &mut Emit,
    event: StreamEvent,
) -> Result<(), ProviderError> {
    if emit(event) {
        Ok(())
    } else {
        Err(ProviderError::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Ollama,
    Anthropic,
    OpenRouter,
    Gemini,
}

impl Kind {
    pub fn parse(value: &str) -> Result<Self, ProviderError> {
        match value {
            "ollama" => Ok(Self::Ollama),
            "anthropic" => Ok(Self::Anthropic),
            "openrouter" => Ok(Self::OpenRouter),
            "gemini" => Ok(Self::Gemini),
            other => Err(ProviderError::BadResponse(format!(
                "unsupported provider kind: {other}"
            ))),
        }
    }
}

pub async fn health_for(
    kind: Kind,
    client: &reqwest::Client,
    config: &ProviderConfig,
) -> Result<(String, u64), ProviderError> {
    match kind {
        Kind::Ollama => ollama::health(client, config).await,
        Kind::Anthropic => anthropic::health(client, config).await,
        Kind::OpenRouter => openrouter::health(client, config).await,
        Kind::Gemini => gemini::health(client, config).await,
    }
}

pub async fn list_models_for(
    kind: Kind,
    client: &reqwest::Client,
    config: &ProviderConfig,
) -> Result<Vec<ModelInfo>, ProviderError> {
    match kind {
        Kind::Ollama => ollama::list_models(client, config).await,
        Kind::Anthropic => anthropic::list_models(client, config).await,
        Kind::OpenRouter => openrouter::list_models(client, config).await,
        Kind::Gemini => gemini::list_models(client, config).await,
    }
}

pub async fn stream_chat_for(
    kind: Kind,
    client: &reqwest::Client,
    config: &ProviderConfig,
    request: ChatRequest,
    cancel: tokio_util::sync::CancellationToken,
    emit: &mut Emit,
) -> Result<(), ProviderError> {
    match kind {
        Kind::Ollama => ollama::stream_chat(client, config, request, cancel, emit).await,
        Kind::Anthropic => anthropic::stream_chat(client, config, request, cancel, emit).await,
        Kind::OpenRouter => {
            openrouter::stream_chat(client, config, request, cancel, emit).await
        }
        Kind::Gemini => gemini::stream_chat(client, config, request, cancel, emit).await,
    }
}

pub async fn complete_for(
    kind: Kind,
    client: &reqwest::Client,
    config: &ProviderConfig,
    request: ChatRequest,
) -> Result<Completion, ProviderError> {
    match kind {
        Kind::Ollama => ollama::complete(client, config, request).await,
        Kind::Anthropic => anthropic::complete(client, config, request).await,
        Kind::OpenRouter => openrouter::complete(client, config, request).await,
        Kind::Gemini => gemini::complete(client, config, request).await,
    }
}
