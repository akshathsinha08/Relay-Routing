use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tokio_util::sync::CancellationToken;

use crate::database::Db;
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub artifacts_dir: PathBuf,
    pub attachments_dir: PathBuf,
    pub logs_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve(app: &AppHandle) -> Result<Self, AppError> {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| AppError::internal(format!("cannot resolve app data directory: {error}")))?;
        let paths = Self {
            db_path: data_dir.join("relayrouting.sqlite3"),
            artifacts_dir: data_dir.join("artifacts"),
            attachments_dir: data_dir.join("attachments"),
            logs_dir: data_dir.join("logs"),
            data_dir,
        };
        for dir in [
            &paths.data_dir,
            &paths.artifacts_dir,
            &paths.attachments_dir,
            &paths.logs_dir,
        ] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(paths)
    }
}

pub struct AppContext {
    pub db: Arc<Db>,
    pub http: reqwest::Client,
    pub paths: AppPaths,
    chat_cancels: Mutex<HashMap<String, CancellationToken>>,
    task_cancels: Mutex<HashMap<String, CancellationToken>>,
    task_pauses: Mutex<HashSet<String>>,
}

impl AppContext {
    pub fn new(db: Arc<Db>, paths: AppPaths) -> Arc<Self> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .user_agent(concat!("RelayRouting/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build HTTP client");
        Arc::new(Self {
            db,
            http,
            paths,
            chat_cancels: Mutex::new(HashMap::new()),
            task_cancels: Mutex::new(HashMap::new()),
            task_pauses: Mutex::new(HashSet::new()),
        })
    }

    pub fn register_chat(&self, id: &str) -> CancellationToken {
        let token = CancellationToken::new();
        if let Ok(mut map) = self.chat_cancels.lock() {
            map.insert(id.to_string(), token.clone());
        }
        token
    }

    pub fn finish_chat(&self, id: &str) {
        if let Ok(mut map) = self.chat_cancels.lock() {
            map.remove(id);
        }
    }

    pub fn cancel_chat(&self, id: &str) -> bool {
        if let Ok(map) = self.chat_cancels.lock() {
            if let Some(token) = map.get(id) {
                token.cancel();
                return true;
            }
        }
        false
    }

    pub fn register_task(&self, id: &str) -> Option<CancellationToken> {
        let mut map = self.task_cancels.lock().ok()?;
        if map.contains_key(id) {
            return None;
        }
        let token = CancellationToken::new();
        map.insert(id.to_string(), token.clone());
        Some(token)
    }

    pub fn finish_task(&self, id: &str) {
        if let Ok(mut map) = self.task_cancels.lock() {
            map.remove(id);
        }
        self.clear_pause(id);
    }

    pub fn cancel_task(&self, id: &str) -> bool {
        if let Ok(map) = self.task_cancels.lock() {
            if let Some(token) = map.get(id) {
                token.cancel();
                return true;
            }
        }
        false
    }

    pub fn is_task_running(&self, id: &str) -> bool {
        self.task_cancels
            .lock()
            .map(|map| map.contains_key(id))
            .unwrap_or(false)
    }

    pub fn request_pause(&self, id: &str) {
        if let Ok(mut set) = self.task_pauses.lock() {
            set.insert(id.to_string());
        }
    }

    pub fn clear_pause(&self, id: &str) {
        if let Ok(mut set) = self.task_pauses.lock() {
            set.remove(id);
        }
    }

    pub fn is_paused(&self, id: &str) -> bool {
        self.task_pauses
            .lock()
            .map(|set| set.contains(id))
            .unwrap_or(false)
    }
}
