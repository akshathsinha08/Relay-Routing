use rusqlite::{params, OptionalExtension};

use crate::database::models::{Account, Provider};
use crate::database::{new_id, now, Db};
use crate::error::AppError;

fn map_provider(row: &rusqlite::Row<'_>) -> rusqlite::Result<Provider> {
    Ok(Provider {
        id: row.get(0)?,
        kind: row.get(1)?,
        name: row.get(2)?,
        base_url: row.get(3)?,
        enabled: row.get::<_, i64>(4)? != 0,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn map_account(row: &rusqlite::Row<'_>) -> rusqlite::Result<Account> {
    Ok(Account {
        id: row.get(0)?,
        provider_id: row.get(1)?,
        label: row.get(2)?,
        auth_kind: row.get(3)?,
        status: row.get(4)?,
        status_message: row.get(5)?,
        metadata_json: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        has_secret: false,
    })
}

const PROVIDER_SELECT: &str =
    "SELECT id, kind, name, base_url, enabled, created_at, updated_at FROM providers";
const ACCOUNT_SELECT: &str = "SELECT id, provider_id, label, auth_kind, status, status_message,
    metadata_json, created_at, updated_at FROM accounts";

impl Db {
    pub fn list_providers(&self) -> Result<Vec<Provider>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(&format!("{} ORDER BY name", PROVIDER_SELECT))?;
            let rows = stmt.query_map([], map_provider)?;
            rows.collect()
        })
    }

    pub fn get_provider(&self, id: &str) -> Result<Provider, AppError> {
        self.with_conn(|conn| {
            conn.query_row(
                &format!("{} WHERE id = ?1", PROVIDER_SELECT),
                [id],
                map_provider,
            )
            .optional()
        })?
        .ok_or_else(|| AppError::not_found("provider not found"))
    }

    pub fn update_provider(
        &self,
        id: &str,
        name: Option<&str>,
        base_url: Option<&str>,
        enabled: Option<bool>,
    ) -> Result<Provider, AppError> {
        if let Some(url) = base_url {
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                return Err(AppError::invalid("base URL must start with http:// or https://"));
            }
        }
        let ts = now();
        let changed = self.with_conn(|conn| {
            conn.execute(
                "UPDATE providers SET name = COALESCE(?2, name), base_url = COALESCE(?3, base_url),
                 enabled = COALESCE(?4, enabled), updated_at = ?5 WHERE id = ?1",
                params![
                    id,
                    name,
                    base_url.map(|u| u.trim_end_matches('/')),
                    enabled.map(i64::from),
                    ts
                ],
            )
        })?;
        if changed == 0 {
            return Err(AppError::not_found("provider not found"));
        }
        self.get_provider(id)
    }

    pub fn list_accounts(&self, provider_id: Option<&str>) -> Result<Vec<Account>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(&format!(
                "{} WHERE (?1 IS NULL OR provider_id = ?1) ORDER BY created_at",
                ACCOUNT_SELECT
            ))?;
            let rows = stmt.query_map([provider_id], map_account)?;
            rows.collect()
        })
    }

    pub fn get_account(&self, id: &str) -> Result<Account, AppError> {
        self.with_conn(|conn| {
            conn.query_row(&format!("{} WHERE id = ?1", ACCOUNT_SELECT), [id], map_account)
                .optional()
        })?
        .ok_or_else(|| AppError::not_found("account not found"))
    }

    pub fn create_account(
        &self,
        provider_id: &str,
        label: &str,
        auth_kind: &str,
        status: &str,
    ) -> Result<Account, AppError> {
        let label = label.trim();
        if label.is_empty() {
            return Err(AppError::invalid("account label is required"));
        }
        if !matches!(auth_kind, "none" | "api_key") {
            return Err(AppError::invalid("unsupported authentication type"));
        }
        let id = new_id();
        let ts = now();
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO accounts (id, provider_id, label, auth_kind, status, metadata_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, '{}', ?6, ?6)",
                params![id, provider_id, label, auth_kind, status, ts],
            )
        })?;
        self.get_account(&id)
    }

    pub fn update_account(
        &self,
        id: &str,
        label: Option<&str>,
        status: Option<&str>,
        status_message: Option<&str>,
    ) -> Result<Account, AppError> {
        let ts = now();
        let changed = self.with_conn(|conn| {
            conn.execute(
                "UPDATE accounts SET label = COALESCE(?2, label), status = COALESCE(?3, status),
                 status_message = ?4, updated_at = ?5 WHERE id = ?1",
                params![id, label, status, status_message, ts],
            )
        })?;
        if changed == 0 {
            return Err(AppError::not_found("account not found"));
        }
        self.get_account(id)
    }

    pub fn delete_account(&self, id: &str) -> Result<(), AppError> {
        let changed = self.with_conn(|conn| conn.execute("DELETE FROM accounts WHERE id = ?1", [id]))?;
        if changed == 0 {
            return Err(AppError::not_found("account not found"));
        }
        Ok(())
    }

}
