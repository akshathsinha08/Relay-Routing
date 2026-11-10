use std::collections::BTreeMap;

use rusqlite::{params, OptionalExtension};

use crate::database::models::UsageSummary;
use crate::database::{new_id, now, Db};
use crate::error::AppError;

impl Db {
    pub fn get_settings(&self) -> Result<BTreeMap<String, String>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT key, value FROM settings ORDER BY key")?;
            let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
            let mut map = BTreeMap::new();
            for row in rows {
                let (k, v) = row?;
                map.insert(k, v);
            }
            Ok(map)
        })
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, AppError> {
        self.with_conn(|conn| {
            conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()
        })
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), AppError> {
        if key.trim().is_empty() {
            return Err(AppError::invalid("setting key is required"));
        }
        if value.len() > 100_000 {
            return Err(AppError::invalid("setting value is too large"));
        }
        let ts = now();
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                params![key, value, ts],
            )
        })?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_usage(
        &self,
        provider_id: Option<&str>,
        account_id: Option<&str>,
        model: Option<&str>,
        conversation_id: Option<&str>,
        task_id: Option<&str>,
        input_tokens: i64,
        output_tokens: i64,
    ) -> Result<(), AppError> {
        let id = new_id();
        let ts = now();
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO usage_records (id, provider_id, account_id, model, conversation_id, task_id,
                 input_tokens, output_tokens, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    id,
                    provider_id,
                    account_id,
                    model,
                    conversation_id,
                    task_id,
                    input_tokens,
                    output_tokens,
                    ts
                ],
            )
        })?;
        Ok(())
    }

    pub fn usage_summary(&self) -> Result<Vec<UsageSummary>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT provider_id, model, COUNT(*), COALESCE(SUM(input_tokens),0),
                        COALESCE(SUM(output_tokens),0)
                 FROM usage_records GROUP BY provider_id, model
                 ORDER BY COALESCE(SUM(input_tokens+output_tokens),0) DESC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(UsageSummary {
                    provider_id: row.get(0)?,
                    model: row.get(1)?,
                    requests: row.get(2)?,
                    input_tokens: row.get(3)?,
                    output_tokens: row.get(4)?,
                })
            })?;
            rows.collect()
        })
    }
}
