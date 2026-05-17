use rusqlite::Connection;
use crate::models::{AppError, Project};
use uuid::Uuid;
use chrono::Utc;
use std::path::Path;

pub struct ProjectService;

impl ProjectService {
    pub fn list_projects(conn: &Connection) -> Result<Vec<Project>, AppError> {
        let mut stmt = conn
            .prepare("SELECT id, name, path, created_at, updated_at, last_opened_at FROM projects ORDER BY last_opened_at DESC, updated_at DESC")
            .map_err(|e| AppError::new("DB_QUERY_ERROR", &format!("Failed to list projects: {}", e)))?;

        let projects = stmt
            .query_map([], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    last_opened_at: row.get(5)?,
                })
            })
            .map_err(|e| AppError::new("DB_QUERY_ERROR", &format!("Failed to map projects: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(projects)
    }

    pub fn add_project(conn: &Connection, path_str: &str) -> Result<Project, AppError> {
        let path = Path::new(path_str);

        if !path.exists() {
            return Err(AppError::new("PATH_NOT_FOUND", "Path does not exist"));
        }
        if !path.is_dir() {
            return Err(AppError::new("NOT_DIRECTORY", "Path is not a directory"));
        }

        let canonical_path = std::fs::canonicalize(path)
            .map_err(|e| AppError::new("PATH_ERROR", &format!("Cannot canonicalize path: {}", e)))?;
        let canonical_str = canonical_path.to_string_lossy().to_string();

        // Check duplicate
        let existing: Result<String, _> = conn.query_row(
            "SELECT id FROM projects WHERE path = ?1",
            [&canonical_str],
            |row| row.get(0),
        );
        if let Ok(_existing_id) = existing {
            // Return existing project instead of creating duplicate
            let mut stmt = conn
                .prepare("SELECT id, name, path, created_at, updated_at, last_opened_at FROM projects WHERE path = ?1")
                .map_err(|e| AppError::new("DB_QUERY_ERROR", &format!("Failed to query project: {}", e)))?;

            let project = stmt
                .query_row([&canonical_str], |row| {
                    Ok(Project {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        path: row.get(2)?,
                        created_at: row.get(3)?,
                        updated_at: row.get(4)?,
                        last_opened_at: row.get(5)?,
                    })
                })
                .map_err(|e| AppError::new("DB_QUERY_ERROR", &format!("Failed to get project: {}", e)))?;

            return Ok(project);
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unnamed".to_string());

        conn.execute(
            "INSERT INTO projects (id, name, path, created_at, updated_at, last_opened_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, name, canonical_str, now, now, now],
        )
        .map_err(|e| AppError::new("DB_INSERT_ERROR", &format!("Failed to insert project: {}", e)))?;

        Ok(Project {
            id,
            name,
            path: canonical_str,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_opened_at: Some(now),
        })
    }


    pub fn get_project_by_id(conn: &Connection, project_id: &str) -> Result<Project, AppError> {
        let mut stmt = conn
            .prepare("SELECT id, name, path, created_at, updated_at, last_opened_at FROM projects WHERE id = ?1")
            .map_err(|e| AppError::new("DB_QUERY_ERROR", &format!("Failed to query project: {}", e)))?;

        stmt.query_row(rusqlite::params![project_id], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                last_opened_at: row.get(5)?,
            })
        })
        .map_err(|e| AppError::new("PROJECT_NOT_FOUND", &format!("Project not found: {}", e)))
    }

    pub fn remove_project(conn: &Connection, project_id: &str) -> Result<(), AppError> {
        conn.execute("DELETE FROM projects WHERE id = ?1", rusqlite::params![project_id])
            .map_err(|e| AppError::new("DB_DELETE_ERROR", &format!("Failed to delete project: {}", e)))?;
        Ok(())
    }

    pub fn select_project(conn: &Connection, project_id: &str) -> Result<Project, AppError> {
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE projects SET last_opened_at = ?1 WHERE id = ?2",
            rusqlite::params![now, project_id],
        )
        .map_err(|e| AppError::new("DB_UPDATE_ERROR", &format!("Failed to update project: {}", e)))?;

        Self::get_project_by_id(conn, project_id)
    }
}
