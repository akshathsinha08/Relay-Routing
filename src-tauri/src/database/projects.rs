use rusqlite::{params, OptionalExtension};

use crate::database::models::Project;
use crate::database::{new_id, now, Db};
use crate::error::AppError;

const SELECT: &str = "SELECT p.id, p.name, p.description, p.instructions, p.archived,
    p.created_at, p.updated_at,
    (SELECT COUNT(*) FROM conversations c WHERE c.project_id = p.id) AS conversation_count
    FROM projects p";

fn map_project(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        instructions: row.get(3)?,
        archived: row.get::<_, i64>(4)? != 0,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        conversation_count: row.get(7)?,
    })
}

impl Db {
    pub fn list_projects(&self, include_archived: bool) -> Result<Vec<Project>, AppError> {
        self.with_conn(|conn| {
            let sql = format!(
                "{} WHERE (?1 = 1 OR p.archived = 0) ORDER BY p.archived ASC, p.updated_at DESC",
                SELECT
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map([i64::from(include_archived)], map_project)?;
            rows.collect()
        })
    }

    pub fn get_project(&self, id: &str) -> Result<Project, AppError> {
        self.with_conn(|conn| {
            conn.query_row(&format!("{} WHERE p.id = ?1", SELECT), [id], map_project)
                .optional()
        })?
        .ok_or_else(|| AppError::not_found("project not found"))
    }

    pub fn create_project(
        &self,
        name: &str,
        description: &str,
        instructions: &str,
    ) -> Result<Project, AppError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::invalid("project name is required"));
        }
        let id = new_id();
        let ts = now();
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, description, instructions, archived, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 0, ?5, ?5)",
                params![id, name, description, instructions, ts],
            )
        })?;
        self.get_project(&id)
    }

    pub fn update_project(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        instructions: Option<&str>,
        archived: Option<bool>,
    ) -> Result<Project, AppError> {
        if let Some(name) = name {
            if name.trim().is_empty() {
                return Err(AppError::invalid("project name cannot be empty"));
            }
        }
        let ts = now();
        let changed = self.with_conn(|conn| {
            conn.execute(
                "UPDATE projects SET
                    name = COALESCE(?2, name),
                    description = COALESCE(?3, description),
                    instructions = COALESCE(?4, instructions),
                    archived = COALESCE(?5, archived),
                    updated_at = ?6
                 WHERE id = ?1",
                params![
                    id,
                    name.map(str::trim),
                    description,
                    instructions,
                    archived.map(i64::from),
                    ts
                ],
            )
        })?;
        if changed == 0 {
            return Err(AppError::not_found("project not found"));
        }
        self.get_project(id)
    }

    pub fn delete_project(&self, id: &str) -> Result<(), AppError> {
        let changed = self.with_conn(|conn| conn.execute("DELETE FROM projects WHERE id = ?1", [id]))?;
        if changed == 0 {
            return Err(AppError::not_found("project not found"));
        }
        Ok(())
    }
}
