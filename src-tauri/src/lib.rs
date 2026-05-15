#![allow(dead_code)]

mod commands;
mod db;
mod errors;
mod models;
mod services;

use commands::project_commands::DbState;
use commands::terminal_commands::TerminalState;
use services::terminal_service::TerminalService;
use std::sync::Mutex;
use tauri::Emitter;
use tauri::Manager;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Initialize database
            let db_path = db::get_db_path(&app.handle());
            let conn = db::init_database(&db_path)?;
            app.manage(DbState(Mutex::new(Some(conn))));

            // Initialize terminal service
            let terminal_service = TerminalService::new();
            app.manage(TerminalState(terminal_service));

            // ── System Tray ──────────────────────────────────────────
            // Gracefully degrade on platforms without tray support (e.g. Linux without appindicator)
            if let Err(e) = (|| -> Result<(), Box<dyn std::error::Error>> {
                let show_item = MenuItemBuilder::with_id("show", "Show CliDesk")
                    .build(app)?;
                let hide_item = MenuItemBuilder::with_id("hide", "Hide to Tray")
                    .build(app)?;
                let quit_item = MenuItemBuilder::with_id("quit", "Quit")
                    .build(app)?;

                let menu = MenuBuilder::new(app)
                    .item(&show_item)
                    .item(&hide_item)
                    .separator()
                    .item(&quit_item)
                    .build()?;

                let icon = app.default_window_icon().cloned().unwrap_or_else(|| {
                    tauri::image::Image::new(&[], 0, 0)
                });

                TrayIconBuilder::new()
                    .icon(icon)
                    .menu(&menu)
                    .on_menu_event(|app, event| {
                        match event.id().as_ref() {
                            "show" => {
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                            "hide" => {
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = window.hide();
                                }
                            }
                            "quit" => {
                                let terminal_state = app.state::<TerminalState>();
                                terminal_state.0.kill_all();
                                app.exit(0);
                            }
                            _ => {}
                        }
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::DoubleClick { .. } = event {
                            if let Some(window) = tray.app_handle().get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    })
                    .build(app)?;
                Ok(())
            })() {
                log::warn!("System tray initialization failed (non-fatal): {}", e);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::project_commands::project_list,
            commands::project_commands::project_add,
            commands::project_commands::project_remove,
            commands::project_commands::project_select,
            commands::terminal_commands::terminal_spawn,
            commands::terminal_commands::terminal_write,
            commands::terminal_commands::terminal_resize,
            commands::terminal_commands::terminal_kill,
            commands::terminal_commands::terminal_kill_all,
            commands::terminal_commands::terminal_list,
            commands::terminal_commands::shell_list,
            commands::terminal_commands::is_elevated,
            commands::terminal_commands::restart_as_admin,
            commands::file_commands::fs_list_dir,
            commands::file_commands::fs_read_file,
            commands::file_commands::fs_write_file,
            commands::git_commands::git_status,
            commands::git_commands::git_diff,
            commands::settings_commands::settings_get,
            commands::settings_commands::settings_set,
            commands::tray_commands::window_hide,
            commands::tray_commands::window_show,
            commands::tray_commands::quit_app,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Always prevent the default close — ask user via frontend modal
                api.prevent_close();
                let _ = window.emit("app://close-requested", ());
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
