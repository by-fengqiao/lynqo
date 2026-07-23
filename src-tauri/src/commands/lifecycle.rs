use serde::Serialize;
use tauri_plugin_autostart::ManagerExt;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppVersionInfo {
    pub version: String,
    pub commit: String,
    pub build_date: String,
    pub channel: String,
}

/// Get whether autostart is currently enabled.
#[tauri::command]
pub async fn get_autostart_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    let autostart = app.autolaunch();
    autostart
        .is_enabled()
        .map_err(|e| format!("Failed to check autostart status: {}", e))
}

/// Enable or disable autostart on login.
#[tauri::command]
pub async fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<CommandResult, String> {
    let autostart = app.autolaunch();
    let result = if enabled {
        autostart.enable()
    } else {
        autostart.disable()
    };

    match result {
        Ok(()) => Ok(CommandResult {
            success: true,
            error: None,
        }),
        Err(e) => Ok(CommandResult {
            success: false,
            error: Some(format!("Failed to set autostart: {}", e)),
        }),
    }
}

/// Get the current close behavior setting.
/// Returns one of: "minimize", "quit", "ask"
#[tauri::command]
pub async fn get_close_behavior(app: tauri::AppHandle) -> Result<String, String> {
    let _ = app;
    Ok(read_close_behavior())
}

pub fn read_close_behavior() -> String {
    let config_path = get_config_path();
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(behavior) = json.get("close_behavior").and_then(|v| v.as_str()) {
                if ["minimize", "quit", "ask"].contains(&behavior) {
                    return behavior.to_string();
                }
            }
        }
    }
    "minimize".to_string()
}

/// Set the close behavior: "minimize" (hide to tray), "quit" (exit app), or "ask" (prompt user).
#[tauri::command]
pub async fn set_close_behavior(behavior: String) -> Result<CommandResult, String> {
    if !["minimize", "quit", "ask"].contains(&behavior.as_str()) {
        return Ok(CommandResult {
            success: false,
            error: Some(format!(
                "Invalid close behavior: '{}'. Must be one of: minimize, quit, ask",
                behavior
            )),
        });
    }

    let config_path = get_config_path();
    let mut config: serde_json::Value = if let Ok(content) = std::fs::read_to_string(&config_path) {
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    config["close_behavior"] = serde_json::Value::String(behavior);

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    match std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&config).unwrap_or_default(),
    ) {
        Ok(()) => Ok(CommandResult {
            success: true,
            error: None,
        }),
        Err(e) => Ok(CommandResult {
            success: false,
            error: Some(format!("Failed to save close behavior: {}", e)),
        }),
    }
}

#[tauri::command]
pub async fn quit_application(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

/// Get application version information.
#[tauri::command]
pub async fn get_app_version() -> Result<AppVersionInfo, String> {
    // Use compile-time date so the build_date reflects when the binary was
    // actually built, not when it is running (ISSUE 12).
    let build_date = option_env!("VERGEN_BUILD_DATE")
        .unwrap_or(env!("CARGO_PKG_VERSION")) // fallback: just the version
        .to_string();

    Ok(AppVersionInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        commit: option_env!("VERGEN_GIT_SHA")
            .unwrap_or("unknown")
            .to_string(),
        build_date,
        channel: if cfg!(debug_assertions) {
            "development"
        } else {
            "stable"
        }
        .to_string(),
    })
}

/// Export a diagnostic report as text.
#[tauri::command]
pub async fn export_diagnostics(app: tauri::AppHandle) -> Result<String, String> {
    let version = env!("CARGO_PKG_VERSION");
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let log_dir = crate::get_log_dir();
    let log_files: Vec<String> = std::fs::read_dir(&log_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();

    let db_path = get_db_path_for_diag();
    let db_exists = db_path.exists();
    let db_size = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);

    let autostart = app.autolaunch();
    let autostart_enabled = autostart.is_enabled().unwrap_or(false);

    let report = format!(
        r#"LYNQO Diagnostics Report
========================
Generated: {}

Application
-----------
Version: {}
Channel: {}
OS: {} ({})

Autostart
---------
Enabled: {}

Database
--------
Path: {}
Exists: {}
Size: {} bytes

Log Directory
-------------
Path: {}
Files: {}
"#,
        now,
        version,
        if cfg!(debug_assertions) {
            "development"
        } else {
            "stable"
        },
        os,
        arch,
        autostart_enabled,
        db_path.display(),
        db_exists,
        db_size,
        log_dir.display(),
        if log_files.is_empty() {
            "(none)".to_string()
        } else {
            log_files.join(", ")
        },
    );

    Ok(report)
}

/// Open the log directory in the system file manager.
#[tauri::command]
pub async fn open_log_dir() -> Result<CommandResult, String> {
    let log_dir = crate::get_log_dir();
    std::fs::create_dir_all(&log_dir).ok();

    let path_str = log_dir.to_string_lossy().to_string();

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path_str)
            .spawn()
            .map_err(|e| format!("Failed to open log directory: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| format!("Failed to open log directory: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| format!("Failed to open log directory: {}", e))?;
    }

    Ok(CommandResult {
        success: true,
        error: None,
    })
}

/// Get the config file path for storing preferences.
fn get_config_path() -> std::path::PathBuf {
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

    base.join("LYNQO").join("config.json")
}

/// Get the database path (for diagnostics).
fn get_db_path_for_diag() -> std::path::PathBuf {
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

    base.join("LYNQO").join("lynqo.db")
}
