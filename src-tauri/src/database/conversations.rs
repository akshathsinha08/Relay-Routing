use rusqlite::{params, OptionalExtension};

use crate::database::models::{Conversation, Message, MessageView, SearchHit};
use crate::database::{new_id, now, Db};
use crate::error::AppError;

const CONVERSATION_SELECT: &str = "SELECT c.id, c.project_id, p.name, c.title, c.pinned, c.archived,
    c.provider_id, c.account_id, c.model, c.summary, c.created_at, c.updated_at, c.last_message_at,
    (SELECT m.content FROM messages m WHERE m.conversation_id = c.id ORDER BY m.seq DESC LIMIT 1)
    FROM conversations c LEFT JOIN projects p ON p.id = c.project_id";

const MESSAGE_SELECT: &str = "SELECT id, conversation_id, seq, role, content, reasoning, status, error,
    provider_id, account_id, model, input_tokens, output_tokens, interrupted, interruption_reason,
    created_at, updated_at FROM messages";

fn map_conversation(row: &rusqlite::Row<'_>) -> rusqlite::Result<Conversation> {
    Ok(Conversation {
        id: row.get(0)?,
        project_id: row.get(1)?,
        project_name: row.get(2)?,
        title: row.get(3)?,
        pinned: row.get::<_, i64>(4)? != 0,
        archived: row.get::<_, i64>(5)? != 0,
        provider_id: row.get(6)?,
        account_id: row.get(7)?,
        model: row.get(8)?,
        summary: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
        last_message_at: row.get(12)?,
        last_message: row.get(13)?,
    })
}

fn map_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<Message> {
    Ok(Message {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        seq: row.get(2)?,
        role: row.get(3)?,
        content: row.get(4)?,
        reasoning: row.get(5)?,
        status: row.get(6)?,
        error: row.get(7)?,
        provider_id: row.get(8)?,
        account_id: row.get(9)?,
        model: row.get(10)?,
        input_tokens: row.get(11)?,
        output_tokens: row.get(12)?,
        interrupted: row.get::<_, i64>(13)? != 0,
        interruption_reason: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
        segments: Vec::new(),
    })
}

impl Db {
    pub fn list_conversations(
        &self,
        project_id: Option<&str>,
        include_archived: bool,
        query: Option<&str>,
    ) -> Result<Vec<Conversation>, AppError> {
        self.with_conn(|conn| {
            let sql = format!(
                "{} WHERE (?1 IS NULL OR c.project_id = ?1)
                   AND (?2 = 1 OR c.archived = 0)
                   AND (?3 IS NULL OR c.title LIKE '%' || ?3 || '%'
                        OR EXISTS (SELECT 1 FROM messages m
                                   WHERE m.conversation_id = c.id AND m.content LIKE '%' || ?3 || '%'))
                 ORDER BY c.pinned DESC, COALESCE(c.last_message_at, c.created_at) DESC
                 LIMIT 500",
                CONVERSATION_SELECT
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(
                params![project_id, i64::from(include_archived), query],
                map_conversation,
            )?;
            rows.collect()
        })
    }

    pub fn get_conversation(&self, id: &str) -> Result<Conversation, AppError> {
        self.with_conn(|conn| {
            conn.query_row(
                &format!("{} WHERE c.id = ?1", CONVERSATION_SELECT),
                [id],
                map_conversation,
            )
            .optional()
        })?
        .ok_or_else(|| AppError::not_found("conversation not found"))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_conversation(
        &self,
        project_id: Option<&str>,
        title: Option<&str>,
        provider_id: Option<&str>,
        account_id: Option<&str>,
        model: Option<&str>,
    ) -> Result<Conversation, AppError> {
        let id = new_id();
        let ts = now();
        let title = title.map(str::trim).filter(|t| !t.is_empty()).unwrap_or("New conversation");
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO conversations (id, project_id, title, pinned, archived, provider_id,
                 account_id, model, created_at, updated_at) VALUES (?1, ?2, ?3, 0, 0, ?4, ?5, ?6, ?7, ?7)",
                params![id, project_id, title, provider_id, account_id, model, ts],
            )
        })?;
        self.get_conversation(&id)
    }

    pub fn rename_conversation(&self, id: &str, title: &str) -> Result<Conversation, AppError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(AppError::invalid("conversation title cannot be empty"));
        }
        let changed = self.with_conn(|conn| {
            conn.execute(
                "UPDATE conversations SET title = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, title, now()],
            )
        })?;
        if changed == 0 {
            return Err(AppError::not_found("conversation not found"));
        }
        self.get_conversation(id)
    }

    pub fn set_conversation_flags(
        &self,
        id: &str,
        pinned: Option<bool>,
        archived: Option<bool>,
    ) -> Result<Conversation, AppError> {
        let changed = self.with_conn(|conn| {
            conn.execute(
                "UPDATE conversations SET pinned = COALESCE(?2, pinned),
                 archived = COALESCE(?3, archived), updated_at = ?4 WHERE id = ?1",
                params![id, pinned.map(i64::from), archived.map(i64::from), now()],
            )
        })?;
        if changed == 0 {
            return Err(AppError::not_found("conversation not found"));
        }
        self.get_conversation(id)
    }

    pub fn set_conversation_target(
        &self,
        id: &str,
        provider_id: Option<&str>,
        account_id: Option<&str>,
        model: Option<&str>,
    ) -> Result<Conversation, AppError> {
        let changed = self.with_conn(|conn| {
            conn.execute(
                "UPDATE conversations SET provider_id = ?2, account_id = ?3, model = ?4,
                 updated_at = ?5 WHERE id = ?1",
                params![id, provider_id, account_id, model, now()],
            )
        })?;
        if changed == 0 {
            return Err(AppError::not_found("conversation not found"));
        }
        self.get_conversation(id)
    }

    pub fn move_conversation(&self, id: &str, project_id: Option<&str>) -> Result<Conversation, AppError> {
        let changed = self.with_conn(|conn| {
            conn.execute(
                "UPDATE conversations SET project_id = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, project_id, now()],
            )
        })?;
        if changed == 0 {
            return Err(AppError::not_found("conversation not found"));
        }
        self.get_conversation(id)
    }

    pub fn delete_conversation(&self, id: &str) -> Result<(), AppError> {
        let changed =
            self.with_conn(|conn| conn.execute("DELETE FROM conversations WHERE id = ?1", [id]))?;
        if changed == 0 {
            return Err(AppError::not_found("conversation not found"));
        }
        Ok(())
    }

    pub fn list_messages(&self, conversation_id: &str) -> Result<Vec<MessageView>, AppError> {
        let messages: Vec<Message> = self.with_conn(|conn| {
            let mut stmt = conn.prepare(&format!(
                "{} WHERE conversation_id = ?1 ORDER BY seq ASC",
                MESSAGE_SELECT
            ))?;
            let rows = stmt.query_map([conversation_id], map_message)?;
            rows.collect()
        })?;
        let mut attachments = self.attachments_for_conversation(conversation_id)?;
        let mut segments = self.segments_for_conversation(conversation_id)?;
        let mut views = Vec::with_capacity(messages.len());
        for mut message in messages {
            let owned: Vec<_> = attachments
                .remove(&message.id)
                .unwrap_or_default()
                .into_iter()
                .collect();
            message.segments = segments.remove(&message.id).unwrap_or_default();
            views.push(MessageView {
                message,
                attachments: owned,
            });
        }
        Ok(views)
    }

    pub fn get_message(&self, id: &str) -> Result<Message, AppError> {
        let mut message = self
            .with_conn(|conn| {
                conn.query_row(&format!("{} WHERE id = ?1", MESSAGE_SELECT), [id], map_message)
                    .optional()
            })?
            .ok_or_else(|| AppError::not_found("message not found"))?;
        message.segments = self.list_segments(id)?;
        Ok(message)
    }


    #[allow(clippy::too_many_arguments)]
    pub fn insert_message(
        &self,
        conversation_id: &str,
        role: &str,
        content: &str,
        status: &str,
        provider_id: Option<&str>,
        account_id: Option<&str>,
        model: Option<&str>,
    ) -> Result<Message, AppError> {
        if !matches!(role, "system" | "user" | "assistant") {
            return Err(AppError::invalid("invalid message role"));
        }
        let id = new_id();
        let ts = now();
        self.with_conn(|conn| {
            let tx = conn.transaction()?;
            let seq: i64 = tx.query_row(
                "SELECT COALESCE(MAX(seq), 0) + 1 FROM messages WHERE conversation_id = ?1",
                [conversation_id],
                |row| row.get(0),
            )?;
            tx.execute(
                "INSERT INTO messages (id, conversation_id, seq, role, content, status,
                 provider_id, account_id, model, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                params![
                    id,
                    conversation_id,
                    seq,
                    role,
                    content,
                    status,
                    provider_id,
                    account_id,
                    model,
                    ts
                ],
            )?;
            tx.execute(
                "UPDATE conversations SET last_message_at = ?2, updated_at = ?2 WHERE id = ?1",
                params![conversation_id, ts],
            )?;
            tx.commit()?;
            Ok(())
        })?;
        self.get_message(&id)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_message(
        &self,
        id: &str,
        content: Option<&str>,
        status: Option<&str>,
        error: Option<&str>,
        reasoning: Option<&str>,
        input_tokens: Option<i64>,
        output_tokens: Option<i64>,
    ) -> Result<(), AppError> {
        let changed = self.with_conn(|conn| {
            conn.execute(
                "UPDATE messages SET content = COALESCE(?2, content), status = COALESCE(?3, status),
                 error = ?4, reasoning = COALESCE(?5, reasoning),
                 input_tokens = COALESCE(?6, input_tokens), output_tokens = COALESCE(?7, output_tokens),
                 updated_at = ?8 WHERE id = ?1",
                params![id, content, status, error, reasoning, input_tokens, output_tokens, now()],
            )
        })?;
        if changed == 0 {
            return Err(AppError::not_found("message not found"));
        }
        Ok(())
    }





    pub fn mark_message_interrupted(&self, id: &str, reason: &str) -> Result<(), AppError> {
        let changed = self.with_conn(|conn| {
            conn.execute(
                "UPDATE messages SET status = 'error', error = ?2, interrupted = 1,
                 interruption_reason = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, reason, now()],
            )
        })?;
        if changed == 0 {
            return Err(AppError::not_found("message not found"));
        }
        Ok(())
    }

    pub fn set_message_target(
        &self,
        id: &str,
        provider_id: &str,
        account_id: &str,
        model: &str,
    ) -> Result<(), AppError> {
        let changed = self.with_conn(|conn| {
            conn.execute(
                "UPDATE messages SET provider_id = ?2, account_id = ?3, model = ?4,
                 updated_at = ?5 WHERE id = ?1",
                params![id, provider_id, account_id, model, now()],
            )
        })?;
        if changed == 0 {
            return Err(AppError::not_found("message not found"));
        }
        Ok(())
    }

    pub fn restart_message(&self, id: &str) -> Result<(), AppError> {
        let changed = self.with_conn(|conn| {
            conn.execute(
                "UPDATE messages SET status = 'streaming', error = NULL, updated_at = ?2
                 WHERE id = ?1",
                params![id, now()],
            )
        })?;
        if changed == 0 {
            return Err(AppError::not_found("message not found"));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn complete_message(
        &self,
        id: &str,
        content: &str,
        reasoning: Option<&str>,
        input_tokens: Option<i64>,
        output_tokens: Option<i64>,
    ) -> Result<(), AppError> {
        let changed = self.with_conn(|conn| {
            conn.execute(
                "UPDATE messages SET content = ?2, status = 'complete', error = NULL,
                 reasoning = ?3, input_tokens = COALESCE(?4, input_tokens),
                 output_tokens = COALESCE(?5, output_tokens), interrupted = 0,
                 interruption_reason = NULL, updated_at = ?6 WHERE id = ?1",
                params![id, content, reasoning, input_tokens, output_tokens, now()],
            )
        })?;
        if changed == 0 {
            return Err(AppError::not_found("message not found"));
        }
        Ok(())
    }

    pub fn search_messages(&self, query: &str, limit: i64) -> Result<Vec<SearchHit>, AppError> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT c.id, c.title, m.id, m.role,
                        substr(m.content, MAX(1, instr(lower(m.content), lower(?1)) - 60), 220),
                        m.created_at
                 FROM messages m JOIN conversations c ON c.id = m.conversation_id
                 WHERE m.content LIKE '%' || ?1 || '%' AND m.role IN ('user', 'assistant')
                 ORDER BY m.created_at DESC LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![query, limit], |row| {
                Ok(SearchHit {
                    conversation_id: row.get(0)?,
                    conversation_title: row.get(1)?,
                    message_id: row.get(2)?,
                    role: row.get(3)?,
                    snippet: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?;
            rows.collect()
        })
    }

    /// Build a provider-neutral export document (Markdown).
    pub fn export_conversation_markdown(&self, conversation_id: &str) -> Result<String, AppError> {
        let conversation = self.get_conversation(conversation_id)?;
        let messages = self.list_messages(conversation_id)?;
        let mut out = String::new();
        out.push_str(&format!("# {}\n\n", conversation.title));
        if let Some(project) = &conversation.project_name {
            out.push_str(&format!("Project: {}\n\n", project));
        }
        out.push_str(&format!("Exported: {}\n\n---\n\n", now()));
        for view in messages {
            let m = view.message;
            let label = match m.role.as_str() {
                "user" => "User",
                "assistant" => "Assistant",
                _ => "System",
            };
            out.push_str(&format!("## {}\n\n{}\n\n", label, m.content));
            for attachment in view.attachments {
                out.push_str(&format!("_Attachment: {} ({})_\n\n", attachment.name, attachment.mime));
            }
        }
        Ok(out)
    }
}
