use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

use crate::server::{self, ServiceStatus, SharedState};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    pub status: String,
    pub port: u16,
    pub local_ip: Option<String>,
    pub local_url: Option<String>,
    pub started_at: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInfo {
    pub ip: String,
    pub port: u16,
    pub token: String,
    pub control_token: String,
    pub local_domain: String,
    pub qr_url: String,
    pub network_name: String,
    pub receive_folder: String,
    pub device_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrCodeData {
    pub url: String,
    /// SVG string of the QR code
    pub svg: String,
}

/// Frontend settings are saved incrementally, so accepting a patch prevents a
/// UI change from accidentally resetting fields that are not shown in the UI.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsPatch {
    device_name: Option<String>,
    receive_folder: Option<String>,
    require_approval: Option<bool>,
    auto_approve_known: Option<bool>,
    port: Option<u16>,
    max_file_size: Option<i64>,
    theme_mode: Option<String>,
}

fn timestamp_to_iso(value: &str) -> String {
    value
        .parse::<i64>()
        .ok()
        .and_then(|seconds| chrono::DateTime::<chrono::Utc>::from_timestamp(seconds, 0))
        .map(|timestamp| timestamp.to_rfc3339())
        .unwrap_or_else(|| value.to_string())
}

#[tauri::command]
pub async fn start_local_service(
    state: State<'_, SharedState>,
    app: tauri::AppHandle,
) -> Result<CommandResult, String> {
    let shared = state.inner().clone();

    // Determine frontend directory with multiple fallback paths
    let serve_dir = {
        let candidates = [
            app.path().resource_dir().ok().map(|p| p.join("dist")),
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|pp| pp.join("../dist"))),
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .map(|p| p.join("dist")),
        ];
        candidates
            .into_iter()
            .flatten()
            .find(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "../dist".to_string())
    };

    // Register mDNS service. The guard is kept in AppState so that
    // stop_local_service can drop it later, which unregisters the service.
    {
        let mut s = shared.lock().await;
        // Drop any previous registration before re-registering.
        s.mdns_guard = None;

        let ip = s.local_ip.clone();
        let port = s.port;
        let device_name = s.device_name.clone();
        if let Some(ip) = ip {
            match crate::discovery::MdnsGuard::register(&ip, port, &device_name) {
                Ok(guard) => {
                    s.mdns_guard = Some(guard);
                    tracing::info!("mDNS service registered");
                }
                Err(e) => {
                    tracing::warn!("mDNS registration failed: {}", e);
                }
            }
        }
    }

    // Start server in a background task
    let server_state = shared.clone();
    tokio::spawn(async move {
        if let Err(e) = server::start_server(server_state, serve_dir).await {
            tracing::error!("Server failed: {}", e);
        }
    });

    // Wait briefly for server to start
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let s = shared.lock().await;
    if s.status == ServiceStatus::Running {
        Ok(CommandResult {
            success: true,
            error: None,
        })
    } else {
        Ok(CommandResult {
            success: false,
            error: s
                .error
                .clone()
                .or(Some("Failed to start server".to_string())),
        })
    }
}

#[tauri::command]
pub async fn stop_local_service(state: State<'_, SharedState>) -> Result<CommandResult, String> {
    let shared = state.inner().clone();

    // Drop the mDNS guard to unregister the service from the LAN.
    {
        let mut s = shared.lock().await;
        if let Some(guard) = s.mdns_guard.take() {
            drop(guard);
        }
    }

    match server::stop_server(shared).await {
        Ok(()) => Ok(CommandResult {
            success: true,
            error: None,
        }),
        Err(e) => Ok(CommandResult {
            success: false,
            error: Some(e),
        }),
    }
}

#[tauri::command]
pub async fn get_local_service_status(
    state: State<'_, SharedState>,
) -> Result<ServiceInfo, String> {
    let s = state.lock().await;
    Ok(ServiceInfo {
        status: s.status.to_string(),
        port: s.port,
        local_ip: s.local_ip.clone(),
        local_url: s.local_url(),
        started_at: s.started_at.clone(),
        error: s.error.clone(),
    })
}

#[tauri::command]
pub async fn refresh_local_ip(state: State<'_, SharedState>) -> Result<String, String> {
    let mut s = state.lock().await;
    s.local_ip = server::get_local_ip();
    s.local_ip
        .clone()
        .ok_or_else(|| "No local IP available".to_string())
}

#[tauri::command]
pub async fn regenerate_connection_token(state: State<'_, SharedState>) -> Result<String, String> {
    // Update the pairing token first, then release the lock before mDNS IO.
    let (new_token, ip, port, device_name, was_running) = {
        let mut s = state.lock().await;
        s.connection_token = uuid::Uuid::new_v4().to_string();
        (
            s.connection_token.clone(),
            s.local_ip.clone(),
            s.port,
            s.device_name.clone(),
            s.status == ServiceStatus::Running,
        )
    };

    // Re-register mDNS outside the lock to avoid blocking other API calls
    if was_running {
        if let Some(ip) = ip {
            // Drop old registration first
            {
                let mut s = state.lock().await;
                s.mdns_guard = None;
            }
            // Re-register the discoverable endpoint. Pairing credentials are
            // intentionally not advertised via mDNS.
            match crate::discovery::MdnsGuard::register(&ip, port, &device_name) {
                Ok(guard) => {
                    let mut s = state.lock().await;
                    s.mdns_guard = Some(guard);
                    tracing::info!("mDNS service re-registered with new token");
                }
                Err(e) => {
                    tracing::warn!("mDNS re-registration failed: {}", e);
                }
            }
        }
    }

    Ok(new_token)
}

#[tauri::command]
pub async fn get_connection_info(state: State<'_, SharedState>) -> Result<ConnectionInfo, String> {
    let s = state.lock().await;
    let ip = s
        .local_ip
        .clone()
        .ok_or_else(|| "No local IP available".to_string())?;

    let local_domain = format!("{}.local", s.device_name.to_lowercase().replace(' ', "-"));

    Ok(ConnectionInfo {
        ip: ip.clone(),
        port: s.port,
        token: s.connection_token.clone(),
        control_token: s.desktop_control_token.clone(),
        local_domain,
        qr_url: s.qr_url().unwrap_or_default(),
        network_name: s.network_name.clone(),
        receive_folder: s.receive_folder.clone(),
        device_name: s.device_name.clone(),
    })
}

#[tauri::command]
pub async fn get_connection_qr_code(state: State<'_, SharedState>) -> Result<QrCodeData, String> {
    let s = state.lock().await;
    let url = s
        .qr_url()
        .ok_or_else(|| "No local IP available for QR code".to_string())?;

    // Generate QR code as SVG manually
    use qrcode::QrCode;
    let code = QrCode::new(url.as_bytes()).map_err(|e| format!("QR generation failed: {}", e))?;

    let colors = code.to_colors();
    let width = code.width();
    let scale = 4;
    let size = width * scale;

    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {size} {size}" width="{size}" height="{size}">"#,
    );
    svg.push_str(&format!(
        r#"<rect width="{size}" height="{size}" fill="white"/>"#
    ));

    for y in 0..width {
        for x in 0..width {
            if colors[y * width + x] == qrcode::Color::Dark {
                let px = x * scale;
                let py = y * scale;
                svg.push_str(&format!(
                    r#"<rect x="{px}" y="{py}" width="{scale}" height="{scale}" fill="black"/>"#
                ));
            }
        }
    }
    svg.push_str("</svg>");

    Ok(QrCodeData { url, svg })
}

#[tauri::command]
pub async fn get_devices(state: State<'_, SharedState>) -> Result<String, String> {
    let s = state.lock().await;
    let devices = s.db.list_visible_devices().map_err(|e| e.to_string())?;
    let connected = &s.connected_devices;
    let payload: Vec<serde_json::Value> = devices
        .into_iter()
        .filter(|device| device.id != "desktop")
        .map(|device| {
            let online = connected.contains_key(&device.id);
            serde_json::json!({
                "id": device.id,
                "name": device.name,
                "platform": device.platform,
                "deviceType": device.device_type,
                "ip": device.ip,
                "approved": device.approved,
                "trusted": device.trusted,
                "online": online,
                "lastSeenAt": timestamp_to_iso(&device.last_seen),
            })
        })
        .collect();
    serde_json::to_string(&payload).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn approve_device(
    state: State<'_, SharedState>,
    device_id: String,
    trusted: bool,
) -> Result<CommandResult, String> {
    let s = state.lock().await;
    match s.db.set_device_access(&device_id, true, trusted) {
        Ok(()) => {
            let _ = s.event_tx.send(crate::server::WsEvent::DeviceApproved {
                device_id: device_id.clone(),
            });
            Ok(CommandResult {
                success: true,
                error: None,
            })
        }
        Err(e) => Ok(CommandResult {
            success: false,
            error: Some(e.to_string()),
        }),
    }
}

#[tauri::command]
pub async fn reject_device(
    state: State<'_, SharedState>,
    device_id: String,
) -> Result<CommandResult, String> {
    let s = state.lock().await;
    match s.db.set_device_access(&device_id, false, false) {
        Ok(()) => {
            let _ = s.event_tx.send(crate::server::WsEvent::DeviceRejected {
                device_id: device_id.clone(),
            });
            Ok(CommandResult {
                success: true,
                error: None,
            })
        }
        Err(e) => Ok(CommandResult {
            success: false,
            error: Some(e.to_string()),
        }),
    }
}

#[tauri::command]
pub async fn forget_device(
    state: State<'_, SharedState>,
    device_id: String,
) -> Result<CommandResult, String> {
    let s = state.lock().await;
    match s.db.hide_device(&device_id) {
        Ok(()) => Ok(CommandResult { success: true, error: None }),
        Err(e) => Ok(CommandResult { success: false, error: Some(e.to_string()) }),
    }
}

#[tauri::command]
pub async fn get_transfers(state: State<'_, SharedState>) -> Result<String, String> {
    let s = state.lock().await;
    let transfers = s.db.list_transfers().map_err(|e| e.to_string())?;

    let mut transfer_list = Vec::new();
    for t in &transfers {
        let files = s.db.get_transfer_files(&t.id).map_err(|e| e.to_string())?;
        let file_infos: Vec<serde_json::Value> = files
            .iter()
            .map(|f| {
                serde_json::json!({
                    "id": f.id.clone(),
                    "name": f.name.clone(),
                    "size": f.size,
                    "mimeType": f.mime_type.clone(),
                    "checksum": f.sha256.clone(),
                })
            })
            .collect();

        let (direction, source_device_id, target_device_id) = match t.direction.as_str() {
            // The desktop receives a mobile upload in this flow.
            "receive" => ("upload_to_host", t.device_id.clone(), "local".to_string()),
            // Desktop-originated files use a synthetic "desktop" record in
            // storage; never expose that implementation detail to the UI.
            "download_from_host" => (
                "download_from_host",
                "local".to_string(),
                t.target_device_id.clone().unwrap_or_default(),
            ),
            "relay" => (
                "relay",
                t.device_id.clone(),
                t.target_device_id.clone().unwrap_or_default(),
            ),
            other => (
                other,
                t.device_id.clone(),
                t.target_device_id.clone().unwrap_or_default(),
            ),
        };
        let progress = if t.total_bytes > 0 {
            (t.transferred_bytes as f64 / t.total_bytes as f64).clamp(0.0, 1.0)
        } else if t.status == "completed" {
            1.0
        } else {
            0.0
        };
        let telemetry = s.transfer_telemetry.get(&t.id).cloned().unwrap_or_default();

        transfer_list.push(serde_json::json!({
            "id": t.id.clone(),
            "deviceId": source_device_id.clone(),
            "sourceDeviceId": source_device_id,
            "targetDeviceId": target_device_id,
            "direction": direction,
            "status": t.status.clone(),
            "totalBytes": t.total_bytes,
            "transferredBytes": t.transferred_bytes,
            "fileCount": t.file_count,
            "files": file_infos,
            "speedBytesPerSecond": telemetry.speed_bytes_per_second,
            "remainingSeconds": telemetry.remaining_seconds,
            "progress": progress,
            "savePath": t.save_path.clone(),
            "createdAt": timestamp_to_iso(&t.created_at),
            "completedAt": t.completed_at.as_deref().map(timestamp_to_iso),
            "relayStage": t.relay_stage.clone(),
            "acceptedAt": t.accepted_at.as_deref().map(timestamp_to_iso),
            "expiresAt": t.expires_at.as_deref().map(timestamp_to_iso),
            "pausedAt": t.paused_at.as_deref().map(timestamp_to_iso),
        }));
    }

    serde_json::to_string(&transfer_list).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cancel_transfer(
    state: State<'_, SharedState>,
    transfer_id: String,
) -> Result<CommandResult, String> {
    // Gather info needed for cleanup before updating status.
    let (receive_folder, files, direction) = {
        let s = state.lock().await;
        let transfer = s.db.get_transfer(&transfer_id).map_err(|e| e.to_string())?;
        match transfer {
            Some(t) => {
                let files =
                    s.db.get_transfer_files(&transfer_id)
                        .map_err(|e| e.to_string())?;
                (s.receive_folder.clone(), files, t.direction.clone())
            }
            None => return Err("Transfer not found".to_string()),
        }
    };

    // Clean up temp files on disk.
    if direction == "relay" {
        let relay_dir = std::path::PathBuf::from(&receive_folder)
            .join(".relay")
            .join(&transfer_id);
        if relay_dir.exists() {
            let _ = tokio::fs::remove_dir_all(&relay_dir).await;
        }
    } else {
        let receive_path = std::path::PathBuf::from(&receive_folder);
        for file_record in &files {
            let temp_path =
                crate::transfer::temp_file_path(&receive_path, &transfer_id, &file_record.name);
            if temp_path.exists() {
                if let Err(e) = tokio::fs::remove_file(&temp_path).await {
                    tracing::error!("Failed to remove temp file {}: {}", temp_path.display(), e);
                }
            }
        }
        let temp_dir = crate::transfer::temp_transfer_dir(&receive_path, &transfer_id);
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    let s = state.lock().await;
    match s.db.update_transfer_status(&transfer_id, "cancelled") {
        Ok(()) => {
            let _ = s.event_tx.send(crate::server::WsEvent::TransferCancelled {
                transfer_id: transfer_id.clone(),
            });
            Ok(CommandResult {
                success: true,
                error: None,
            })
        }
        Err(e) => Ok(CommandResult {
            success: false,
            error: Some(e.to_string()),
        }),
    }
}

#[tauri::command]
pub async fn get_settings(state: State<'_, SharedState>) -> Result<String, String> {
    let s = state.lock().await;
    let settings = s.db.get_settings().map_err(|e| e.to_string())?;
    serde_json::to_string(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, SharedState>,
    settings_json: String,
) -> Result<CommandResult, String> {
    let patch: SettingsPatch =
        serde_json::from_str(&settings_json).map_err(|e| format!("Invalid settings: {}", e))?;

    let mut s = state.lock().await;
    let mut settings = s.db.get_settings().map_err(|e| e.to_string())?;
    if let Some(value) = patch.device_name {
        settings.device_name = value;
    }
    if let Some(value) = patch.receive_folder {
        settings.receive_folder = value;
    }
    if let Some(value) = patch.require_approval {
        settings.require_approval = value;
    }
    if let Some(value) = patch.auto_approve_known {
        settings.auto_approve_known = value;
    }
    if let Some(value) = patch.port {
        settings.port = value;
    }
    if let Some(value) = patch.max_file_size {
        if value < 0 {
            return Err("maxFileSize must not be negative".to_string());
        }
        settings.max_file_size = value;
    }
    if let Some(value) = patch.theme_mode {
        if !matches!(value.as_str(), "light" | "dark" | "system") {
            return Err("themeMode must be light, dark, or system".to_string());
        }
        settings.theme_mode = value;
    }
    std::fs::create_dir_all(&settings.receive_folder)
        .map_err(|e| format!("Unable to create receive folder: {}", e))?;
    match s.db.save_settings(&settings) {
        Ok(()) => {
            s.receive_folder = settings.receive_folder.clone();
            s.device_name = settings.device_name.clone();
            // Port change applies on next service start; sync to AppState so it is
            // picked up when the server is (re)started. A live server keeps its
            // current bound port until restarted.
            s.port = settings.port;
            Ok(CommandResult {
                success: true,
                error: None,
            })
        }
        Err(e) => Ok(CommandResult {
            success: false,
            error: Some(e.to_string()),
        }),
    }
}

#[tauri::command]
pub async fn open_receive_folder(state: State<'_, SharedState>) -> Result<CommandResult, String> {
    let folder = {
        let s = state.lock().await;
        s.receive_folder.clone()
    };

    // Use tauri-plugin-shell to open the folder
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&folder)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&folder)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&folder)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    Ok(CommandResult {
        success: true,
        error: None,
    })
}
