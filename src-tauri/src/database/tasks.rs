use rusqlite::{params, OptionalExtension};

use crate::database::models::{
    Artifact, Checkpoint, ExecutionLog, Task, TaskDetail, TaskStep,
};
use crate::database::{new_id, now, Db};
use crate::error::AppError;

pub struct NewTask {
    pub project_id: Option<String>,
    pub conversation_id: Option<String>,
    pub name: String,
    pub instructions: String,
    pub provider_id: Option<String>,
    pub account_id: Option<String>,
    pub model: Option<String>,
}

const TASK_SELECT: &str = "SELECT t.id, t.project_id, p.name, t.conversation_id, t.name,
    t.instructions, t.status, t.provider_id, t.account_id, t.model, t.current_step, t.error,
    t.retry_count, t.created_at, t.updated_at, t.started_at, t.completed_at
    FROM tasks t LEFT JOIN projects p ON p.id = t.project_id";

const STEP_SELECT: &str = "SELECT id, task_id, seq, title, instruction, status, output, error,
    started_at, completed_at FROM task_steps";

fn map_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get(0)?,
        project_id: row.get(1)?,
        project_name: row.get(2)?,
        conversation_id: row.get(3)?,
        name: row.get(4)?,
        instructions: row.get(5)?,
        status: row.get(6)?,
        provider_id: row.get(7)?,
        account_id: row.get(8)?,
        model: row.get(9)?,
        current_step: row.get(10)?,
        error: row.get(11)?,
        retry_count: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        started_at: row.get(15)?,
        completed_at: row.get(16)?,
    })
}

fn map_step(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskStep> {
    Ok(TaskStep {
        id: row.get(0)?,
        task_id: row.get(1)?,
        seq: row.get(2)?,
        title: row.get(3)?,
        instruction: row.get(4)?,
        status: row.get(5)?,
        output: row.get(6)?,
        error: row.get(7)?,
        started_at: row.get(8)?,
        completed_at: row.get(9)?,
    })
}

fn map_checkpoint(row: &rusqlite::Row<'_>) -> rusqlite::Result<Checkpoint> {
    Ok(Checkpoint {
        id: row.get(0)?,
        task_id: row.get(1)?,
        seq: row.get(2)?,
        state_json: row.get(3)?,
        created_at: row.get(4)?,
    })
}

fn map_artifact(row: &rusqlite::Row<'_>) -> rusqlite::Result<Artifact> {
    Ok(Artifact {
        id: row.get(0)?,
        task_id: row.get(1)?,
        project_id: row.get(2)?,
        name: row.get(3)?,
        path: row.get(4)?,
        mime: row.get(5)?,
        size: row.get(6)?,
        sha256: row.get(7)?,
        created_at: row.get(8)?,
    })
}

fn map_log(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExecutionLog> {
    Ok(ExecutionLog {
        id: row.get(0)?,
        task_id: row.get(1)?,
        level: row.get(2)?,
        message: row.get(3)?,
        data_json: row.get(4)?,
        created_at: row.get(5)?,
    })
}

impl Db {
    pub fn list_tasks(&self, project_id: Option<&str>, limit: i64) -> Result<Vec<Task>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(&format!(
                "{} WHERE (?1 IS NULL OR t.project_id = ?1)
                 ORDER BY CASE t.status WHEN 'running' THEN 0 WHEN 'queued' THEN 1 ELSE 2 END,
                          t.updated_at DESC LIMIT ?2",
                TASK_SELECT
            ))?;
            let rows = stmt.query_map(params![project_id, limit], map_task)?;
            rows.collect()
        })
    }

    pub fn get_task(&self, id: &str) -> Result<Task, AppError> {
        self.with_conn(|conn| {
            conn.query_row(&format!("{} WHERE t.id = ?1", TASK_SELECT), [id], map_task)
                .optional()
        })?
        .ok_or_else(|| AppError::not_found("task not found"))
    }

    pub fn create_task(&self, new: NewTask) -> Result<Task, AppError> {
        if new.name.trim().is_empty() {
            return Err(AppError::invalid("task name is required"));
        }
        if new.instructions.trim().is_empty() {
            return Err(AppError::invalid("task instructions are required"));
        }
        let id = new_id();
        let ts = now();
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO tasks (id, project_id, conversation_id, name, instructions, status,
                 provider_id, account_id, model, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'queued', ?6, ?7, ?8, ?9, ?9)",
                params![
                    id,
                    new.project_id,
                    new.conversation_id,
                    new.name.trim(),
                    new.instructions,
                    new.provider_id,
                    new.account_id,
                    new.model,
                    ts
                ],
            )
        })?;
        self.get_task(&id)
    }

    pub fn update_task_status(
        &self,
        id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<Task, AppError> {
        if !matches!(
            status,
            "queued" | "running" | "waiting_for_input" | "paused" | "completed" | "failed" | "cancelled"
        ) {
            return Err(AppError::invalid("invalid task status"));
        }
        let ts = now();
        let changed = self.with_conn(|conn| {
            conn.execute(
                "UPDATE tasks SET status = ?2, error = ?3, updated_at = ?4,
                 started_at = CASE WHEN ?2 = 'running' AND started_at IS NULL THEN ?4 ELSE started_at END,
                 completed_at = CASE WHEN ?2 IN ('completed','failed','cancelled') THEN ?4 ELSE NULL END
                 WHERE id = ?1",
                params![id, status, error, ts],
            )
        })?;
        if changed == 0 {
            return Err(AppError::not_found("task not found"));
        }
        self.get_task(id)
    }

    pub fn update_task_target(
        &self,
        id: &str,
        provider_id: Option<&str>,
        account_id: Option<&str>,
        model: Option<&str>,
    ) -> Result<Task, AppError> {
        let changed = self.with_conn(|conn| {
            conn.execute(
                "UPDATE tasks SET provider_id = ?2, account_id = ?3, model = ?4, updated_at = ?5
                 WHERE id = ?1",
                params![id, provider_id, account_id, model, now()],
            )
        })?;
        if changed == 0 {
            return Err(AppError::not_found("task not found"));
        }
        self.get_task(id)
    }

    pub fn set_task_current_step(&self, id: &str, step: i64) -> Result<(), AppError> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE tasks SET current_step = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, step, now()],
            )
        })?;
        Ok(())
    }

    pub fn increment_task_retry(&self, id: &str) -> Result<(), AppError> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE tasks SET retry_count = retry_count + 1, updated_at = ?2 WHERE id = ?1",
                params![id, now()],
            )
        })?;
        Ok(())
    }

    pub fn delete_task(&self, id: &str) -> Result<(), AppError> {
        let changed = self.with_conn(|conn| conn.execute("DELETE FROM tasks WHERE id = ?1", [id]))?;
        if changed == 0 {
            return Err(AppError::not_found("task not found"));
        }
        Ok(())
    }

    pub fn insert_steps(
        &self,
        task_id: &str,
        steps: &[(String, String)],
    ) -> Result<Vec<TaskStep>, AppError> {
        if steps.is_empty() {
            return Err(AppError::invalid("a task needs at least one step"));
        }
        if steps.len() > 60 {
            return Err(AppError::invalid("too many task steps"));
        }
        self.with_conn(|conn| {
            let tx = conn.transaction()?;
            let existing: i64 = tx.query_row(
                "SELECT COUNT(*) FROM task_steps WHERE task_id = ?1",
                [task_id],
                |row| row.get(0),
            )?;
            if existing > 0 {
                return Err(rusqlite::Error::InvalidQuery);
            }
            for (index, (title, instruction)) in steps.iter().enumerate() {
                tx.execute(
                    "INSERT INTO task_steps (id, task_id, seq, title, instruction, status)
                     VALUES (?1, ?2, ?3, ?4, ?5, 'queued')",
                    params![new_id(), task_id, index as i64 + 1, title, instruction],
                )?;
            }
            tx.commit()?;
            Ok(())
        })?;
        self.list_steps(task_id)
    }

    pub fn list_steps(&self, task_id: &str) -> Result<Vec<TaskStep>, AppError> {
        self.with_conn(|conn| {
            let mut stmt =
                conn.prepare(&format!("{} WHERE task_id = ?1 ORDER BY seq", STEP_SELECT))?;
            let rows = stmt.query_map([task_id], map_step)?;
            rows.collect()
        })
    }

    pub fn update_step_status(
        &self,
        task_id: &str,
        seq: i64,
        status: &str,
        output: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), AppError> {
        if !matches!(status, "queued" | "running" | "completed" | "failed" | "skipped") {
            return Err(AppError::invalid("invalid step status"));
        }
        let ts = now();
        let changed = self.with_conn(|conn| {
            conn.execute(
                "UPDATE task_steps SET status = ?3, output = ?4, error = ?5,
                 started_at = CASE WHEN ?3 = 'running' AND started_at IS NULL THEN ?6 ELSE started_at END,
                 completed_at = CASE WHEN ?3 IN ('completed','failed','skipped') THEN ?6 ELSE NULL END
                 WHERE task_id = ?1 AND seq = ?2",
                params![task_id, seq, status, output, error, ts],
            )
        })?;
        if changed == 0 {
            return Err(AppError::not_found("task step not found"));
        }
        Ok(())
    }


    pub fn insert_checkpoint(&self, task_id: &str, state_json: &str) -> Result<Checkpoint, AppError> {
        let id = new_id();
        let ts = now();
        self.with_conn(|conn| {
            let tx = conn.transaction()?;
            let seq: i64 = tx.query_row(
                "SELECT COALESCE(MAX(seq), 0) + 1 FROM checkpoints WHERE task_id = ?1",
                [task_id],
                |row| row.get(0),
            )?;
            tx.execute(
                "INSERT INTO checkpoints (id, task_id, seq, state_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, task_id, seq, state_json, ts],
            )?;
            tx.commit()?;
            Ok(())
        })?;
        self.get_checkpoint(&id)
    }

    pub fn get_checkpoint(&self, id: &str) -> Result<Checkpoint, AppError> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, task_id, seq, state_json, created_at FROM checkpoints WHERE id = ?1",
                [id],
                map_checkpoint,
            )
            .optional()
        })?
        .ok_or_else(|| AppError::not_found("checkpoint not found"))
    }

    pub fn list_checkpoints(&self, task_id: &str) -> Result<Vec<Checkpoint>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, task_id, seq, state_json, created_at FROM checkpoints
                 WHERE task_id = ?1 ORDER BY seq DESC",
            )?;
            let rows = stmt.query_map([task_id], map_checkpoint)?;
            rows.collect()
        })
    }

    pub fn add_log(
        &self,
        task_id: &str,
        level: &str,
        message: &str,
        data_json: Option<&str>,
    ) -> Result<(), AppError> {
        if !matches!(level, "debug" | "info" | "warn" | "error") {
            return Err(AppError::invalid("invalid log level"));
        }
        let id = new_id();
        let ts = now();
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO execution_logs (id, task_id, level, message, data_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![id, task_id, level, message, data_json, ts],
            )
        })?;
        Ok(())
    }

    pub fn list_logs(&self, task_id: &str, limit: i64) -> Result<Vec<ExecutionLog>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, task_id, level, message, data_json, created_at FROM execution_logs
                 WHERE task_id = ?1 ORDER BY created_at DESC LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![task_id, limit], map_log)?;
            let mut out: Vec<ExecutionLog> = rows.collect::<rusqlite::Result<_>>()?;
            out.reverse();
            Ok(out)
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_artifact(
        &self,
        task_id: Option<&str>,
        project_id: Option<&str>,
        name: &str,
        path: &str,
        mime: &str,
        size: i64,
        sha256: &str,
    ) -> Result<Artifact, AppError> {
        let id = new_id();
        let ts = now();
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO artifacts (id, task_id, project_id, name, path, mime, size, sha256, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![id, task_id, project_id, name, path, mime, size, sha256, ts],
            )
        })?;
        self.get_artifact(&id)
    }

    pub fn get_artifact(&self, id: &str) -> Result<Artifact, AppError> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, task_id, project_id, name, path, mime, size, sha256, created_at
                 FROM artifacts WHERE id = ?1",
                [id],
                map_artifact,
            )
            .optional()
        })?
        .ok_or_else(|| AppError::not_found("artifact not found"))
    }

    pub fn list_artifacts_for_task(&self, task_id: &str) -> Result<Vec<Artifact>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, task_id, project_id, name, path, mime, size, sha256, created_at
                 FROM artifacts WHERE task_id = ?1 ORDER BY created_at DESC",
            )?;
            let rows = stmt.query_map([task_id], map_artifact)?;
            rows.collect()
        })
    }

    pub fn list_artifacts_for_project(&self, project_id: &str) -> Result<Vec<Artifact>, AppError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, task_id, project_id, name, path, mime, size, sha256, created_at
                 FROM artifacts WHERE project_id = ?1 ORDER BY created_at DESC",
            )?;
            let rows = stmt.query_map([project_id], map_artifact)?;
            rows.collect()
        })
    }

    pub fn task_detail(&self, task_id: &str) -> Result<TaskDetail, AppError> {
        let task = self.get_task(task_id)?;
        Ok(TaskDetail {
            steps: self.list_steps(task_id)?,
            checkpoints: self.list_checkpoints(task_id)?,
            logs: self.list_logs(task_id, 200)?,
            artifacts: self.list_artifacts_for_task(task_id)?,
            task,
        })
    }
}
