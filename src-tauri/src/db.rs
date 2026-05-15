use rusqlite::{Connection, Result};
use std::path::PathBuf;

pub fn get_db_path(app_handle: &tauri::AppHandle) -> PathBuf {
    let app_dir = tauri::Manager::path(app_handle)
        .app_data_dir()
        .expect("Failed to get app data dir");
    std::fs::create_dir_all(&app_dir).expect("Failed to create app data dir");
    app_dir.join("clidesk.db")
}

pub fn init_database(db_path: &PathBuf) -> Result<Connection, crate::models::AppError> {
    let conn = Connection::open(db_path).map_err(|e| {
        crate::models::AppError::new("DB_OPEN_ERROR", &format!("Failed to open database: {}", e))
    })?;

    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|e| crate::models::AppError::new("DB_PRAGMA_ERROR", &format!("Failed to set pragmas: {}", e)))?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            last_opened_at TEXT
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES ('security.mode', 'standard', datetime('now'));
        INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES ('ui.theme', 'dark', datetime('now'));
        INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES ('terminal.fontSize', '14', datetime('now'));
        INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES ('editor.fontSize', '14', datetime('now'));
        ",
    )
    .map_err(|e| crate::models::AppError::new("DB_INIT_ERROR", &format!("Failed to init database: {}", e)))?;

    Ok(conn)
}
