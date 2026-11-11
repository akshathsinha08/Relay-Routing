use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub base_url: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub provider_id: String,
    pub label: String,
    pub auth_kind: String,
    pub status: String,
    pub status_message: Option<String>,
    pub metadata_json: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub has_secret: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub archived: bool,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub conversation_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub title: String,
    pub pinned: bool,
    pub archived: bool,
    pub provider_id: Option<String>,
    pub account_id: Option<String>,
    pub model: Option<String>,
    pub summary: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_message_at: Option<String>,
    pub last_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub seq: i64,
    pub role: String,
    pub content: String,
    pub reasoning: Option<String>,
    pub status: String,
    pub error: Option<String>,
    pub provider_id: Option<String>,
    pub account_id: Option<String>,
    pub model: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub interrupted: bool,
    pub interruption_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub segments: Vec<MessageSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageSegment {
    pub id: String,
    pub message_id: String,
    pub generation_id: Option<String>,
    pub attempt_id: Option<String>,
    pub seq: i64,
    pub content: String,
    pub reasoning: Option<String>,
    pub status: String,
    pub provider_id: Option<String>,
    pub account_id: Option<String>,
    pub model: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Generation {
    pub id: String,
    pub conversation_id: String,
    pub user_message_id: String,
    pub assistant_message_id: String,
    pub status: String,
    pub requested_provider_id: Option<String>,
    pub requested_account_id: Option<String>,
    pub requested_model: Option<String>,
    pub attempt_count: i64,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationAttempt {
    pub id: String,
    pub generation_id: String,
    pub attempt_no: i64,
    pub provider_id: String,
    pub account_id: String,
    pub model: String,
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub finish_reason: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageView {
    pub message: Message,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: String,
    pub project_id: Option<String>,
    pub conversation_id: Option<String>,
    pub message_id: Option<String>,
    pub name: String,
    pub mime: String,
    pub size: i64,
    pub path: String,
    pub sha256: String,
    pub kind: String,
    pub text_content: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub conversation_id: Option<String>,
    pub name: String,
    pub instructions: String,
    pub status: String,
    pub provider_id: Option<String>,
    pub account_id: Option<String>,
    pub model: Option<String>,
    pub current_step: i64,
    pub error: Option<String>,
    pub retry_count: i64,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStep {
    pub id: String,
    pub task_id: String,
    pub seq: i64,
    pub title: String,
    pub instruction: String,
    pub status: String,
    pub output: Option<String>,
    pub error: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Checkpoint {
    pub id: String,
    pub task_id: String,
    pub seq: i64,
    pub state_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub id: String,
    pub task_id: Option<String>,
    pub project_id: Option<String>,
    pub name: String,
    pub path: String,
    pub mime: String,
    pub size: i64,
    pub sha256: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionLog {
    pub id: String,
    pub task_id: String,
    pub level: String,
    pub message: String,
    pub data_json: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDetail {
    pub task: Task,
    pub steps: Vec<TaskStep>,
    pub checkpoints: Vec<Checkpoint>,
    pub logs: Vec<ExecutionLog>,
    pub artifacts: Vec<Artifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub conversation_id: String,
    pub conversation_title: String,
    pub message_id: String,
    pub role: String,
    pub snippet: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    pub provider_id: Option<String>,
    pub model: Option<String>,
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderHealth {
    pub ok: bool,
    pub message: String,
    pub latency_ms: u64,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountTest {
    pub ok: bool,
    pub message: String,
    pub latency_ms: u64,
    pub model_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub path: String,
    pub size_bytes: u64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub identifier: String,
    pub platform: String,
    pub data_dir: String,
    pub db_path: String,
    pub log_dir: String,
    pub artifacts_dir: String,
}
