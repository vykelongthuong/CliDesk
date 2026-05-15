use rusqlite::Connection;
use crate::models::AppError;
use chrono::Utc;

pub struct SettingsService;

impl SettingsService {
    pub fn new() -> Self {
        Self
    }

    pub fn get_all(conn: &Connection) -> Result<std::collections::HashMap<String, String>, AppError> {
        let mut stmt = conn
            .prepare("SELECT key, value FROM settings")
            .map_err(|e| AppError::new("DB_QUERY_ERROR", &format!("Failed to query settings: {}", e)))?;

        let settings = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| AppError::new("DB_QUERY_ERROR", &format!("Failed to map settings: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(settings)
    }

    pub fn get(conn: &Connection, key: &str) -> Result<Option<String>, AppError> {
        let result: Result<String, _> = conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            rusqlite::params![key],
            |row| row.get(0),
        );

        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::new("DB_QUERY_ERROR", &format!("Failed to get setting: {}", e))),
        }
    }

    pub fn set(conn: &Connection, key: &str, value: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();

        let valid_keys = vec![
            "security.mode",
            "ui.theme",
            "ui.language",
            "terminal.defaultShell.windows",
            "terminal.defaultShell.linux",
            "editor.fontSize",
            "terminal.fontSize",
            "tray.close_to_tray",
            "tray.minimize_to_tray",
        ];

        if !valid_keys.contains(&key) {
            return Err(AppError::new("INVALID_SETTING_KEY", &format!("Unknown setting key: {}", key)));
        }

        // Validate enum values
        match key {
            "security.mode" => {
                if !["relaxed", "standard", "strict"].contains(&value) {
                    return Err(AppError::new("INVALID_SETTING_VALUE", "Security mode must be: relaxed, standard, or strict"));
                }
            }
            "ui.theme" => {
                if !["light", "dark"].contains(&value) {
                    return Err(AppError::new("INVALID_SETTING_VALUE", "Theme must be: light or dark"));
                }
            }
            "ui.language" => {
                if !["vi", "en"].contains(&value) {
                    return Err(AppError::new("INVALID_SETTING_VALUE", "Language must be: vi or en"));
                }
            }
            "editor.fontSize" | "terminal.fontSize" => {
                if let Ok(size) = value.parse::<i32>() {
                    if size < 8 || size > 72 {
                        return Err(AppError::new("INVALID_SETTING_VALUE", "Font size must be between 8 and 72"));
                    }
                } else {
                    return Err(AppError::new("INVALID_SETTING_VALUE", "Font size must be a number"));
                }
            }
            "tray.close_to_tray" | "tray.minimize_to_tray" => {
                if value != "true" && value != "false" {
                    return Err(AppError::new("INVALID_SETTING_VALUE", "Tray setting must be 'true' or 'false'"));
                }
            }
            _ => {}
        }

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![key, value, now],
        )
        .map_err(|e| AppError::new("DB_UPDATE_ERROR", &format!("Failed to save setting: {}", e)))?;

        Ok(())
    }
}
