use std::collections::HashMap;

use rusqlite::{params, OptionalExtension};

use crate::database::models::Attachment;
use crate::database::{new_id, now, Db};
use crate::error::AppError;

const SELECT: &str = "SELECT id, project_id, conversation_id, message_id, name, mime, size, path,
    sha256, kind, text_content, created_at FROM attachments";

fn map_attachment(row: &rusqlite::Row<'_>) -> rusqlite::Result<Attachment> {
    Ok(Attachment {
        id: row.get(0)?,
        project_id: row.get(1)?,
        conversation_id: row.get(2)?,
        message_id: row.get(3)?,
        name: row.get(4)?,
        mime: row.get(5)?,
        size: row.get(6)?,
        path: row.get(7)?,
        sha256: row.get(8)?,
        kind: row.get(9)?,
        text_content: row.get(10)?,
        created_at: row.get(11)?,
    })
}

pub struct NewAttachment<'a> {
    pub project_id: Option<&'a str>,
    pub conversation_id: Option<&'a str>,
    pub message_id: Option<&'a str>,
    pub name: &'a str,
    pub mime: &'a str,
    pub size: i64,
    pub path: &'a str,
    pub sha256: &'a str,
    pub kind: &'a str,
    pub text_content: Option<&'a str>,
}

impl Db {
    pub fn insert_attachment(&self, new: NewAttachment<'_>) -> Result<Attachment, AppError> {
        let id = new_id();
        let ts = now();
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO attachments (id, project_id, conversation_id, message_id, name, mime,
                 size, path, sha256, kind, text_content, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    id,
                    new.project_id,
                    new.conversation_id,
                    new.message_id,
                    new.name,
                    new.mime,
                    new.size,
                    new.path,
                    new.sha256,
                    new.kind,
                    new.text_content,
                    ts
                ],
            )
        })?;
        self.get_attachment(&id)
    }

    pub fn get_attachment(&self, id: &str) -> Result<Attachment, AppError> {
        self.with_conn(|conn| {
            conn.query_row(&format!("{} WHERE id = ?1", SELECT), [id], map_attachment)
                .optional()
        })?
        .ok_or_else(|| AppError::not_found("attachment not found"))
    }

    pub fn attachments_for_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<HashMap<String, Vec<Attachment>>, AppError> {
        let items: Vec<Attachment> = self.with_conn(|conn| {
            let mut stmt = conn.prepare(&format!(
                "{} WHERE conversation_id = ?1 AND message_id IS NOT NULL ORDER BY created_at",
                SELECT
            ))?;
            let rows = stmt.query_map([conversation_id], map_attachment)?;
            rows.collect()
        })?;
        let mut grouped: HashMap<String, Vec<Attachment>> = HashMap::new();
        for item in items {
            if let Some(message_id) = item.message_id.clone() {
                grouped.entry(message_id).or_default().push(item);
            }
        }
        Ok(grouped)
    }

    pub fn list_attachments_for_message(&self, message_id: &str) -> Result<Vec<Attachment>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(&format!(
                "{} WHERE message_id = ?1 ORDER BY created_at",
                SELECT
            ))?;
            let rows = stmt.query_map([message_id], map_attachment)?;
            rows.collect()
        })
    }

    pub fn list_project_attachments(&self, project_id: &str) -> Result<Vec<Attachment>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(&format!(
                "{} WHERE project_id = ?1 AND message_id IS NULL ORDER BY created_at DESC",
                SELECT
            ))?;
            let rows = stmt.query_map([project_id], map_attachment)?;
            rows.collect()
        })
    }

    pub fn attach_to_message(&self, ids: &[String], message_id: &str) -> Result<(), AppError> {
        self.with_conn(|conn| {
            let tx = conn.transaction()?;
            for id in ids {
                tx.execute(
                    "UPDATE attachments SET message_id = ?2 WHERE id = ?1 AND message_id IS NULL",
                    params![id, message_id],
                )?;
            }
            tx.commit()?;
            Ok(())
        })
    }

    pub fn delete_attachment(&self, id: &str) -> Result<Attachment, AppError> {
        let attachment = self.get_attachment(id)?;
        self.with_conn(|conn| conn.execute("DELETE FROM attachments WHERE id = ?1", [id]))?;
        Ok(attachment)
    }
}
