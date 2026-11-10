use std::collections::HashMap;

use rusqlite::{params, OptionalExtension};

use crate::database::models::{Generation, GenerationAttempt, MessageSegment};
use crate::database::{new_id, now, Db};
use crate::error::AppError;

const GENERATION_SELECT: &str = "SELECT id, conversation_id, user_message_id, assistant_message_id,
    status, requested_provider_id, requested_account_id, requested_model, attempt_count,
    error_code, error_message, started_at, finished_at, created_at, updated_at FROM generations";

const ATTEMPT_SELECT: &str = "SELECT id, generation_id, attempt_no, provider_id, account_id, model,
    status, error_code, error_message, finish_reason, input_tokens, output_tokens, started_at,
    finished_at, created_at, updated_at FROM generation_attempts";

const SEGMENT_SELECT: &str = "SELECT id, message_id, generation_id, attempt_id, seq, content,
    reasoning, status, provider_id, account_id, model, input_tokens, output_tokens, created_at,
    updated_at FROM message_segments";

fn map_generation(row: &rusqlite::Row<'_>) -> rusqlite::Result<Generation> {
    Ok(Generation {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        user_message_id: row.get(2)?,
        assistant_message_id: row.get(3)?,
        status: row.get(4)?,
        requested_provider_id: row.get(5)?,
        requested_account_id: row.get(6)?,
        requested_model: row.get(7)?,
        attempt_count: row.get(8)?,
        error_code: row.get(9)?,
        error_message: row.get(10)?,
        started_at: row.get(11)?,
        finished_at: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn map_attempt(row: &rusqlite::Row<'_>) -> rusqlite::Result<GenerationAttempt> {
    Ok(GenerationAttempt {
        id: row.get(0)?,
        generation_id: row.get(1)?,
        attempt_no: row.get(2)?,
        provider_id: row.get(3)?,
        account_id: row.get(4)?,
        model: row.get(5)?,
        status: row.get(6)?,
        error_code: row.get(7)?,
        error_message: row.get(8)?,
        finish_reason: row.get(9)?,
        input_tokens: row.get(10)?,
        output_tokens: row.get(11)?,
        started_at: row.get(12)?,
        finished_at: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn map_segment(row: &rusqlite::Row<'_>) -> rusqlite::Result<MessageSegment> {
    Ok(MessageSegment {
        id: row.get(0)?,
        message_id: row.get(1)?,
        generation_id: row.get(2)?,
        attempt_id: row.get(3)?,
        seq: row.get(4)?,
        content: row.get(5)?,
        reasoning: row.get(6)?,
        status: row.get(7)?,
        provider_id: row.get(8)?,
        account_id: row.get(9)?,
        model: row.get(10)?,
        input_tokens: row.get(11)?,
        output_tokens: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

impl Db {
    #[allow(clippy::too_many_arguments)]
    pub fn create_generation(
        &self,
        conversation_id: &str,
        user_message_id: &str,
        assistant_message_id: &str,
        provider_id: Option<&str>,
        account_id: Option<&str>,
        model: Option<&str>,
    ) -> Result<Generation, AppError> {
        let id = new_id();
        let ts = now();
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO generations (id, conversation_id, user_message_id,
                 assistant_message_id, status, requested_provider_id, requested_account_id,
                 requested_model, attempt_count, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?7, 0, ?8, ?8)",
                params![
                    id,
                    conversation_id,
                    user_message_id,
                    assistant_message_id,
                    provider_id,
                    account_id,
                    model,
                    ts
                ],
            )
        })?;
        self.get_generation(&id)
    }

    pub fn get_generation(&self, id: &str) -> Result<Generation, AppError> {
        self.with_conn(|conn| {
            conn.query_row(
                &format!("{} WHERE id = ?1", GENERATION_SELECT),
                [id],
                map_generation,
            )
            .optional()
        })?
        .ok_or_else(|| AppError::not_found("generation not found"))
    }

    pub fn latest_generation_for_message(
        &self,
        assistant_message_id: &str,
    ) -> Result<Option<Generation>, AppError> {
        self.with_conn(|conn| {
            conn.query_row(
                &format!(
                    "{} WHERE assistant_message_id = ?1 ORDER BY created_at DESC LIMIT 1",
                    GENERATION_SELECT
                ),
                [assistant_message_id],
                map_generation,
            )
            .optional()
        })
    }

    pub fn start_generation(&self, id: &str) -> Result<(), AppError> {
        let ts = now();
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE generations SET status = 'streaming',
                 started_at = COALESCE(started_at, ?2), finished_at = NULL, error_code = NULL,
                 error_message = NULL, updated_at = ?2 WHERE id = ?1",
                params![id, ts],
            )
        })?;
        Ok(())
    }

    pub fn finish_generation(
        &self,
        id: &str,
        status: &str,
        error_code: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<(), AppError> {
        let ts = now();
        let changed = self.with_conn(|conn| {
            conn.execute(
                "UPDATE generations SET status = ?2, error_code = ?3, error_message = ?4,
                 finished_at = ?5, updated_at = ?5 WHERE id = ?1",
                params![id, status, error_code, error_message, ts],
            )
        })?;
        if changed == 0 {
            return Err(AppError::not_found("generation not found"));
        }
        Ok(())
    }

    pub fn create_attempt(
        &self,
        generation_id: &str,
        provider_id: &str,
        account_id: &str,
        model: &str,
    ) -> Result<GenerationAttempt, AppError> {
        let id = new_id();
        let ts = now();
        self.with_conn(|conn| {
            let tx = conn.transaction()?;
            let attempt_no: i64 = tx.query_row(
                "SELECT COALESCE(MAX(attempt_no), 0) + 1 FROM generation_attempts
                 WHERE generation_id = ?1",
                [generation_id],
                |row| row.get(0),
            )?;
            tx.execute(
                "INSERT INTO generation_attempts (id, generation_id, attempt_no, provider_id,
                 account_id, model, status, started_at, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'streaming', ?7, ?7, ?7)",
                params![id, generation_id, attempt_no, provider_id, account_id, model, ts],
            )?;
            tx.execute(
                "UPDATE generations SET attempt_count = ?2, updated_at = ?3 WHERE id = ?1",
                params![generation_id, attempt_no, ts],
            )?;
            tx.commit()?;
            Ok(())
        })?;
        self.get_attempt(&id)
    }

    pub fn get_attempt(&self, id: &str) -> Result<GenerationAttempt, AppError> {
        self.with_conn(|conn| {
            conn.query_row(
                &format!("{} WHERE id = ?1", ATTEMPT_SELECT),
                [id],
                map_attempt,
            )
            .optional()
        })?
        .ok_or_else(|| AppError::not_found("generation attempt not found"))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn finish_attempt(
        &self,
        id: &str,
        status: &str,
        error_code: Option<&str>,
        error_message: Option<&str>,
        finish_reason: Option<&str>,
        input_tokens: Option<i64>,
        output_tokens: Option<i64>,
    ) -> Result<(), AppError> {
        let ts = now();
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE generation_attempts SET status = ?2, error_code = ?3,
                 error_message = ?4, finish_reason = ?5, input_tokens = COALESCE(?6, input_tokens),
                 output_tokens = COALESCE(?7, output_tokens), finished_at = ?8, updated_at = ?8
                 WHERE id = ?1",
                params![
                    id,
                    status,
                    error_code,
                    error_message,
                    finish_reason,
                    input_tokens,
                    output_tokens,
                    ts
                ],
            )
        })?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_segment(
        &self,
        message_id: &str,
        generation_id: Option<&str>,
        attempt_id: Option<&str>,
        provider_id: Option<&str>,
        account_id: Option<&str>,
        model: Option<&str>,
    ) -> Result<MessageSegment, AppError> {
        let id = new_id();
        let ts = now();
        self.with_conn(|conn| {
            let tx = conn.transaction()?;
            let seq: i64 = tx.query_row(
                "SELECT COALESCE(MAX(seq), 0) + 1 FROM message_segments WHERE message_id = ?1",
                [message_id],
                |row| row.get(0),
            )?;
            tx.execute(
                "INSERT INTO message_segments (id, message_id, generation_id, attempt_id, seq,
                 content, status, provider_id, account_id, model, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, '', 'streaming', ?6, ?7, ?8, ?9, ?9)",
                params![
                    id,
                    message_id,
                    generation_id,
                    attempt_id,
                    seq,
                    provider_id,
                    account_id,
                    model,
                    ts
                ],
            )?;
            tx.commit()?;
            Ok(())
        })?;
        self.get_segment(&id)
    }

    pub fn get_segment(&self, id: &str) -> Result<MessageSegment, AppError> {
        self.with_conn(|conn| {
            conn.query_row(
                &format!("{} WHERE id = ?1", SEGMENT_SELECT),
                [id],
                map_segment,
            )
            .optional()
        })?
        .ok_or_else(|| AppError::not_found("message segment not found"))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_segment(
        &self,
        id: &str,
        content: Option<&str>,
        reasoning: Option<&str>,
        status: Option<&str>,
        input_tokens: Option<i64>,
        output_tokens: Option<i64>,
    ) -> Result<(), AppError> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE message_segments SET content = COALESCE(?2, content),
                 reasoning = COALESCE(?3, reasoning), status = COALESCE(?4, status),
                 input_tokens = COALESCE(?5, input_tokens),
                 output_tokens = COALESCE(?6, output_tokens), updated_at = ?7 WHERE id = ?1",
                params![id, content, reasoning, status, input_tokens, output_tokens, now()],
            )
        })?;
        Ok(())
    }

    pub fn list_segments(&self, message_id: &str) -> Result<Vec<MessageSegment>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(&format!(
                "{} WHERE message_id = ?1 ORDER BY seq ASC",
                SEGMENT_SELECT
            ))?;
            let rows = stmt.query_map([message_id], map_segment)?;
            rows.collect()
        })
    }

    pub fn segments_for_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<HashMap<String, Vec<MessageSegment>>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(&format!(
                "{} WHERE message_id IN (SELECT id FROM messages WHERE conversation_id = ?1)
                 ORDER BY message_id, seq ASC",
                SEGMENT_SELECT
            ))?;
            let rows = stmt.query_map([conversation_id], map_segment)?;
            let mut grouped: HashMap<String, Vec<MessageSegment>> = HashMap::new();
            for row in rows {
                let segment = row?;
                grouped
                    .entry(segment.message_id.clone())
                    .or_default()
                    .push(segment);
            }
            Ok(grouped)
        })
    }

    /// Preserve partial output and mark anything left mid-stream as resumable.
    pub fn recover_interrupted_generations(&self) -> Result<usize, AppError> {
        let ts = now();
        self.with_conn(|conn| {
            let tx = conn.transaction()?;
            tx.execute(
                "UPDATE message_segments SET status = 'interrupted', updated_at = ?1
                 WHERE status = 'streaming'",
                [&ts],
            )?;
            tx.execute(
                "UPDATE generation_attempts SET status = 'interrupted',
                 error_code = 'generation_interrupted',
                 error_message = 'Interrupted before completion', finished_at = ?1, updated_at = ?1
                 WHERE status = 'streaming'",
                [&ts],
            )?;
            let changed = tx.execute(
                "UPDATE generations SET status = 'interrupted',
                 error_code = 'generation_interrupted',
                 error_message = 'Interrupted before completion', finished_at = ?1, updated_at = ?1
                 WHERE status IN ('pending', 'streaming')",
                [&ts],
            )?;
            tx.execute(
                "UPDATE messages SET status = 'error', error = NULL, interrupted = 1,
                 interruption_reason = 'Generation interrupted before completion', updated_at = ?1
                 WHERE status = 'streaming'",
                [&ts],
            )?;
            tx.commit()?;
            Ok(changed)
        })
    }
}
