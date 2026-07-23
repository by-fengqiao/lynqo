mod commands;
pub mod discovery;
pub mod error;
pub mod server;
pub mod storage;
pub mod transfer;

use server::SharedState;
use std::sync::Arc;
use storage::Database;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Keep the non-blocking guard alive for the entire application lifetime;
    // otherwise buffered log writes can be dropped when the function returns.
    let log_dir = get_log_dir();
    let file_appender = tracing_appender::rolling::daily(&log_dir, "lynqo.log");
    let (non_blocking, _file_log_guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(non_blocking)
        .init();

    // Open database in app data directory
    let db_path = get_db_path();
    let db = Database::open(&db_path).expect("Failed to open database");
    let db = Arc::new(db);

    // Create shared application state
    let state: SharedState = Arc::new(Mutex::new(server::AppState::new(db)));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Focus existing window when second instance launched
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .manage(state.clone())
        .setup(move |app| {
            // Build tray menu
            let show_item = MenuItem::with_id(app, "show", "打开 LYNQO", true, None::<&str>)?;
            let start_item =
                MenuItem::with_id(app, "start_service", "开始局域网服务", true, None::<&str>)?;
            let stop_item =
                MenuItem::with_id(app, "stop_service", "停止局域网服务", true, None::<&str>)?;
            let separator1 = PredefinedMenuItem::separator(app)?;
            let open_folder_item =
                MenuItem::with_id(app, "open_folder", "打开接收文件夹", true, None::<&str>)?;
            let settings_item = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
            let separator2 = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &show_item,
                    &separator1,
                    &start_item,
                    &stop_item,
                    &separator1,
                    &open_folder_item,
                    &settings_item,
                    &separator2,
                    &quit_item,
                ],
            )?;

            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().cloned().ok_or_else(|| {
                    tauri::Error::AssetNotFound("default application icon".into())
                })?)
                .menu(&menu)
                .tooltip("LYNQO — 连接附近，自由传输")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "start_service" => {
                        let _ = app.emit("tray-start-service", ());
                    }
                    "stop_service" => {
                        let _ = app.emit("tray-stop-service", ());
                    }
                    "open_folder" => {
                        let _ = app.emit("tray-open-folder", ());
                    }
                    "settings" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        let _ = app.emit("navigate", "/settings");
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::DoubleClick { .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Handle close-to-tray: intercept window close request
            let window = app.get_webview_window("main").unwrap();
            let window_handle = window.clone();
            let app_handle = app.handle().clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    match commands::lifecycle::read_close_behavior().as_str() {
                        "quit" => app_handle.exit(0),
                        "ask" => {
                            api.prevent_close();
                            let _ = window_handle.show();
                            let _ = window_handle.set_focus();
                            let _ = app_handle.emit("close-requested", ());
                        }
                        _ => {
                            api.prevent_close();
                            let _ = window_handle.hide();
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::server_cmd::start_local_service,
            commands::server_cmd::stop_local_service,
            commands::server_cmd::get_local_service_status,
            commands::server_cmd::refresh_local_ip,
            commands::server_cmd::regenerate_connection_token,
            commands::server_cmd::get_connection_info,
            commands::server_cmd::get_connection_qr_code,
            commands::server_cmd::get_devices,
            commands::server_cmd::approve_device,
            commands::server_cmd::reject_device,
            commands::server_cmd::forget_device,
            commands::server_cmd::get_transfers,
            commands::server_cmd::cancel_transfer,
            commands::server_cmd::get_settings,
            commands::server_cmd::update_settings,
            commands::server_cmd::open_receive_folder,
            commands::transfer_cmd::send_files_to_device,
            commands::transfer_cmd::get_file_metadata,
            commands::transfer_cmd::get_pending_transfers,
            commands::transfer_cmd::pause_transfer,
            commands::transfer_cmd::resume_transfer,
            commands::lifecycle::get_autostart_enabled,
            commands::lifecycle::set_autostart,
            commands::lifecycle::get_close_behavior,
            commands::lifecycle::set_close_behavior,
            commands::lifecycle::quit_application,
            commands::lifecycle::get_app_version,
            commands::lifecycle::export_diagnostics,
            commands::lifecycle::open_log_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Get the database file path in the app data directory
fn get_db_path() -> std::path::PathBuf {
    let base = if cfg!(target_os = "windows") {
        std::env::var("APPDATA")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("USERPROFILE")
                    .map(|p| std::path::PathBuf::from(p).join("AppData").join("Roaming"))
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
            })
    } else {
        std::env::var("HOME")
            .map(|p| std::path::PathBuf::from(p).join(".config"))
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
    };

    let app_dir = base.join("LYNQO");
    std::fs::create_dir_all(&app_dir).ok();
    app_dir.join("lynqo.db")
}

/// Get the log directory path
pub fn get_log_dir() -> std::path::PathBuf {
    let base = if cfg!(target_os = "windows") {
        std::env::var("APPDATA")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("USERPROFILE")
                    .map(|p| std::path::PathBuf::from(p).join("AppData").join("Roaming"))
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
            })
    } else {
        std::env::var("HOME")
            .map(|p| std::path::PathBuf::from(p).join(".config"))
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
    };

    let log_dir = base.join("LYNQO").join("logs");
    std::fs::create_dir_all(&log_dir).ok();
    log_dir
}
