pub mod attachments;
pub mod conversations;
pub mod generations;
pub mod models;
pub mod projects;
pub mod providers;
pub mod settings;
pub mod tasks;

use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use rusqlite::Connection;

use crate::error::AppError;

const MIGRATIONS: &[(i64, &str)] = &[
    (
        1,
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations/0001_init.sql")),
    ),
    (
        2,
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations/0002_generations.sql")),
    ),
    (
        3,
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations/0003_cloud_providers.sql")),
    ),
];

pub fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub struct Db {
    pub(crate) conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        Self::configure(&conn)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        db.seed()?;
        Ok(db)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, AppError> {
        let conn = Connection::open_in_memory()?;
        Self::configure(&conn)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        db.seed()?;
        Ok(db)
    }

    fn configure(conn: &Connection) -> Result<(), AppError> {
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;",
        )?;
        Ok(())
    }

    fn migrate(&self) -> Result<(), AppError> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|_| AppError::internal("database lock poisoned"))?;
        let current: i64 = guard.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        let latest = MIGRATIONS.iter().map(|(version, _)| *version).max().unwrap_or(0);
        if current >= latest {
            return Ok(());
        }
        // Table-rebuilding migrations require foreign keys to be off, and this
        // pragma is a no-op inside a transaction, so toggle it around the run.
        guard.execute_batch("PRAGMA foreign_keys = OFF")?;
        let outcome = apply_migrations(&mut guard, current);
        guard.execute_batch("PRAGMA foreign_keys = ON")?;
        outcome?;
        let violations: i64 = guard.query_row(
            "SELECT COUNT(*) FROM pragma_foreign_key_check",
            [],
            |row| row.get(0),
        )?;
        if violations > 0 {
            return Err(AppError::internal(format!(
                "database migration left {violations} foreign key violation(s)"
            )));
        }
        Ok(())
    }

    fn seed(&self) -> Result<(), AppError> {
        let now = now();
        let mut guard = self
            .conn
            .lock()
            .map_err(|_| AppError::internal("database lock poisoned"))?;
        let tx = guard.transaction()?;
        tx.execute(
            "INSERT OR IGNORE INTO providers (id, kind, name, base_url, enabled, created_at, updated_at)
             VALUES ('ollama', 'ollama', 'Ollama', 'http://127.0.0.1:11434', 1, ?1, ?1)",
            [&now],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO providers (id, kind, name, base_url, enabled, created_at, updated_at)
             VALUES ('anthropic', 'anthropic', 'Anthropic', 'https://api.anthropic.com', 1, ?1, ?1)",
            [&now],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO providers (id, kind, name, base_url, enabled, created_at, updated_at)
             VALUES ('openrouter', 'openrouter', 'OpenRouter', 'https://openrouter.ai/api', 1, ?1, ?1)",
            [&now],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO providers (id, kind, name, base_url, enabled, created_at, updated_at)
             VALUES ('gemini', 'gemini', 'Google Gemini', 'https://generativelanguage.googleapis.com', 1, ?1, ?1)",
            [&now],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO accounts
             (id, provider_id, label, auth_kind, status, metadata_json, created_at, updated_at)
             VALUES ('ollama-default', 'ollama', 'Local Ollama', 'none', 'configured', '{}', ?1, ?1)",
            [&now],
        )?;
        for (key, value) in [
            ("theme", "system"),
            ("sendOnEnter", "true"),
            ("contextMessageLimit", "30"),
            ("maxOutputTokens", "4096"),
            ("appSystemPrompt", ""),
            ("onboardingComplete", "false"),
            ("taskMaxSteps", "8"),
            ("ollamaKeepAlive", "10m"),
        ] {
            tx.execute(
                "INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![key, value, now],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn with_conn<T>(
        &self,
        f: impl FnOnce(&mut Connection) -> rusqlite::Result<T>,
    ) -> Result<T, AppError> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|_| AppError::internal("database lock poisoned"))?;
        f(&mut guard).map_err(AppError::from)
    }

    /// Mark interrupted work as recoverable rather than silently losing it.
    pub fn recover_interrupted_work(&self) -> Result<usize, AppError> {
        let now = now();
        self.with_conn(|conn| {
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE task_steps SET status = 'queued', error = NULL
                 WHERE status = 'running'",
                [],
            )?;
            let changed = tx.execute(
                "UPDATE tasks SET status = 'paused', error = 'Recovered after an unexpected shutdown',
                 updated_at = ?1 WHERE status IN ('running', 'queued')",
                [&now],
            )?;
            tx.commit()?;
            Ok(changed)
        })
    }

    /// Create a consistent SQLite backup using VACUUM INTO.
    pub fn backup_to(&self, dir: &Path) -> Result<crate::database::models::BackupInfo, AppError> {
        std::fs::create_dir_all(dir)?;
        let stamp = Utc::now().format("%Y%m%d-%H%M%S");
        let dest = dir.join(format!("claude-multi-{stamp}.db"));
        if dest.exists() {
            return Err(AppError::invalid("a backup with this timestamp already exists"));
        }
        let sql = format!("VACUUM INTO '{}'", dest.to_string_lossy().replace('\'', "''"));
        self.with_conn(|conn| conn.execute_batch(&sql))?;
        let size = std::fs::metadata(&dest)?.len();
        Ok(crate::database::models::BackupInfo {
            path: dest.to_string_lossy().to_string(),
            size_bytes: size,
            created_at: now(),
        })
    }

    pub fn health_check(&self) -> Result<(), AppError> {
        self.with_conn(|conn| conn.query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0)))?;
        Ok(())
    }
}

fn apply_migrations(conn: &mut Connection, current: i64) -> Result<(), AppError> {
    for (version, sql) in MIGRATIONS {
        if *version <= current {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", *version)?;
        tx.commit()?;
        tracing::info!(version, "applied database migration");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_and_seeds() {
        let db = Db::open_in_memory().expect("open db");
        let providers = db.list_providers().expect("providers");
        assert_eq!(providers.len(), 4);
        let kinds: Vec<&str> = providers.iter().map(|p| p.kind.as_str()).collect();
        assert!(kinds.contains(&"openrouter"));
        assert!(kinds.contains(&"gemini"));
        let accounts = db.list_accounts(None).expect("accounts");
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].id, "ollama-default");
        let version: i64 = db
            .with_conn(|conn| conn.query_row("PRAGMA user_version", [], |row| row.get(0)))
            .unwrap();
        assert_eq!(version, 3);
        db.health_check().expect("health");
    }

    #[test]
    fn recovery_marks_generations_interrupted() {
        let db = Db::open_in_memory().unwrap();
        let conversation = db
            .create_conversation(None, Some("t"), Some("ollama"), Some("ollama-default"), Some("m"))
            .unwrap();
        let user = db
            .insert_message(&conversation.id, "user", "hello", "complete", None, None, None)
            .unwrap();
        let assistant = db
            .insert_message(&conversation.id, "assistant", "partial", "streaming", None, None, None)
            .unwrap();
        let generation = db
            .create_generation(
                &conversation.id,
                &user.id,
                &assistant.id,
                Some("ollama"),
                Some("ollama-default"),
                Some("m"),
            )
            .unwrap();
        db.start_generation(&generation.id).unwrap();
        let attempt = db
            .create_attempt(&generation.id, "ollama", "ollama-default", "m")
            .unwrap();
        db.create_segment(
            &assistant.id,
            Some(&generation.id),
            Some(&attempt.id),
            Some("ollama"),
            Some("ollama-default"),
            Some("m"),
        )
        .unwrap();
        assert_eq!(db.recover_interrupted_generations().unwrap(), 1);
        let recovered = db.get_message(&assistant.id).unwrap();
        assert!(recovered.interrupted);
        assert_eq!(recovered.status, "error");
        assert_eq!(db.get_generation(&generation.id).unwrap().status, "interrupted");
        assert_eq!(db.list_segments(&assistant.id).unwrap()[0].status, "interrupted");
    }

    #[test]
    fn recovery_marks_running_tasks_paused() {
        let db = Db::open_in_memory().unwrap();
        let task = db
            .create_task(crate::database::tasks::NewTask {
                project_id: None,
                conversation_id: None,
                name: "t".into(),
                instructions: "do".into(),
                provider_id: Some("ollama".into()),
                account_id: Some("ollama-default".into()),
                model: Some("m".into()),
            })
            .unwrap();
        db.update_task_status(&task.id, "running", None).unwrap();
        let recovered = db.recover_interrupted_work().unwrap();
        assert_eq!(recovered, 1);
        assert_eq!(db.get_task(&task.id).unwrap().status, "paused");
    }
}
