pub mod download;
pub mod handlers;
pub mod ws;

use axum::extract::DefaultBodyLimit;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tower_http::services::{ServeDir, ServeFile};

use crate::storage::Database;

/// Default server port
pub const DEFAULT_PORT: u16 = 53317;

/// File metadata included in transfer request events
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub id: String,
    pub name: String,
    pub size: i64,
}

/// Events broadcast to all WebSocket clients
#[derive(Debug, Clone, serde::Serialize)]
#[serde(
    tag = "type",
    content = "payload",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum WsEvent {
    #[serde(rename = "device.connected")]
    DeviceConnected {
        device_id: String,
        name: String,
        platform: String,
        device_type: String,
        ip: String,
        approved: bool,
    },
    #[serde(rename = "device.disconnected")]
    DeviceDisconnected { device_id: String },
    #[serde(rename = "device.approval_required")]
    DeviceApprovalRequired {
        device_id: String,
        name: String,
        ip: String,
        platform: String,
        device_type: String,
        user_agent: String,
    },
    #[serde(rename = "device.approved")]
    DeviceApproved { device_id: String },
    #[serde(rename = "device.rejected")]
    DeviceRejected { device_id: String },
    #[serde(rename = "transfer.created")]
    TransferCreated {
        transfer_id: String,
        file_name: String,
        total_bytes: i64,
    },
    #[serde(rename = "transfer.started")]
    TransferStarted { transfer_id: String },
    #[serde(rename = "transfer.progress")]
    TransferProgress {
        transfer_id: String,
        file_name: String,
        transferred_bytes: i64,
        total_bytes: i64,
        progress: f64,
        speed_bytes_per_second: i64,
        remaining_seconds: Option<i64>,
    },
    #[serde(rename = "transfer.checksum_ready")]
    TransferChecksumReady {
        transfer_id: String,
        file_id: String,
        checksum: String,
    },
    #[serde(rename = "transfer.verifying")]
    TransferVerifying { transfer_id: String },
    #[serde(rename = "transfer.completed")]
    TransferCompleted {
        transfer_id: String,
        save_path: String,
    },
    #[serde(rename = "transfer.cancelled")]
    TransferCancelled { transfer_id: String },
    #[serde(rename = "transfer.failed")]
    TransferFailed { transfer_id: String, error: String },
    // --- Bidirectional transfer events ---
    #[serde(rename = "transfer.requested")]
    TransferRequested {
        transfer_id: String,
        source_device_name: String,
        files: Vec<FileInfo>,
        total_bytes: i64,
        /// Used only by the WebSocket fan-out layer. The target never needs
        /// to receive its own identifier in the payload.
        #[serde(skip_serializing)]
        target_device_id: String,
    },
    #[serde(rename = "transfer.accepted")]
    TransferAccepted { transfer_id: String },
    #[serde(rename = "transfer.rejected")]
    TransferRejected { transfer_id: String },
    #[serde(rename = "transfer.expired")]
    TransferExpired { transfer_id: String },
    #[serde(rename = "transfer.paused")]
    TransferPaused { transfer_id: String },
    #[serde(rename = "transfer.resumed")]
    TransferResumed { transfer_id: String },
    #[serde(rename = "transfer.download_started")]
    TransferDownloadStarted { transfer_id: String },
    #[serde(rename = "transfer.download_progress")]
    TransferDownloadProgress {
        transfer_id: String,
        file_id: String,
        transferred_bytes: i64,
        total_bytes: i64,
        progress: f64,
        speed_bytes_per_second: i64,
        remaining_seconds: Option<i64>,
    },
    #[serde(rename = "transfer.relay_stage_changed")]
    TransferRelayStageChanged { transfer_id: String, stage: String },
}

/// Latest measured transfer telemetry. This is intentionally ephemeral: the
/// database stores durable progress, while this cache lets a freshly opened
/// desktop or mobile page render live speed and ETA immediately.
#[derive(Debug, Clone, Default)]
pub struct TransferTelemetry {
    pub speed_bytes_per_second: i64,
    pub remaining_seconds: Option<i64>,
}

/// Service lifecycle status
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceStatus::Stopped => write!(f, "stopped"),
            ServiceStatus::Starting => write!(f, "starting"),
            ServiceStatus::Running => write!(f, "running"),
            ServiceStatus::Stopping => write!(f, "stopping"),
            ServiceStatus::Failed => write!(f, "failed"),
        }
    }
}

/// Shared application state for the HTTP server
pub struct AppState {
    pub status: ServiceStatus,
    pub port: u16,
    pub local_ip: Option<String>,
    /// Pairing capability embedded in the QR code. It can only validate a
    /// pairing link and create a normal device session.
    pub connection_token: String,
    /// Desktop-only control credential. It is returned through Tauri IPC and
    /// never appears in a browser URL or the QR code.
    pub desktop_control_token: String,
    pub device_name: String,
    pub network_name: String,
    pub started_at: Option<String>,
    pub error: Option<String>,
    pub db: Arc<Database>,
    pub event_tx: broadcast::Sender<WsEvent>,
    pub shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    pub receive_folder: String,
    /// Devices with an active authenticated WebSocket session. This is the
    /// source of truth for the desktop's online indicator.
    /// Number of active authenticated WebSocket sessions per device. A device
    /// stays online until its last browser tab/socket disconnects.
    pub connected_devices: HashMap<String, usize>,
    /// Live speed/remaining values keyed by transfer id.
    pub transfer_telemetry: HashMap<String, TransferTelemetry>,
    /// Keeps the mDNS service registered while present. Taking and dropping
    /// this guard unregisters the service from the LAN (see stop_local_service).
    pub mdns_guard: Option<crate::discovery::MdnsGuard>,
}

impl AppState {
    pub fn new(db: Arc<Database>) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        let connection_token = uuid::Uuid::new_v4().to_string();
        let desktop_control_token = uuid::Uuid::new_v4().to_string();

        let device_name = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "LYNQO Device".to_string());

        // Restore the persisted receive folder so a user-configured folder
        // survives app restarts; fall back to the default on any failure.
        let receive_folder = match db.get_settings() {
            Ok(settings) if !settings.receive_folder.is_empty() => settings.receive_folder,
            Ok(_) => crate::transfer::default_receive_folder()
                .to_string_lossy()
                .to_string(),
            Err(e) => {
                tracing::warn!("Failed to read receive_folder setting: {}", e);
                crate::transfer::default_receive_folder()
                    .to_string_lossy()
                    .to_string()
            }
        };

        Self {
            status: ServiceStatus::Stopped,
            port: DEFAULT_PORT,
            local_ip: get_local_ip(),
            connection_token,
            desktop_control_token,
            device_name,
            network_name: "Local Network".to_string(),
            started_at: None,
            error: None,
            db,
            event_tx,
            shutdown_tx: None,
            receive_folder,
            connected_devices: HashMap::new(),
            transfer_telemetry: HashMap::new(),
            mdns_guard: None,
        }
    }

    /// Returns the local HTTP URL, e.g. http://192.168.1.100:53317
    pub fn local_url(&self) -> Option<String> {
        self.local_ip
            .as_ref()
            .map(|ip| format!("http://{}:{}", ip, self.port))
    }

    /// Returns the QR connection URL with embedded token
    pub fn qr_url(&self) -> Option<String> {
        self.local_ip.as_ref().map(|ip| {
            format!(
                "http://{}:{}/mobile?token={}",
                ip, self.port, self.connection_token
            )
        })
    }
}

pub type SharedState = Arc<Mutex<AppState>>;

/// Get the best local IPv4 address (exclude 127.x, 169.254.x, prefer 192.168.x)
pub fn get_local_ip() -> Option<String> {
    // Try local-ip-address crate first
    if let Ok(std::net::IpAddr::V4(v4)) = local_ip_address::local_ip() {
        let octets = v4.octets();
        if octets[0] != 127 && !(octets[0] == 169 && octets[1] == 254) {
            return Some(v4.to_string());
        }
    }

    // Fallback: enumerate interfaces and pick the best one
    if let Ok(addrs) = local_ip_address::list_afinet_netifas() {
        let mut fallback: Option<String> = None;

        for (_name, ip) in addrs {
            if let std::net::IpAddr::V4(v4) = ip {
                let octets = v4.octets();
                // Skip loopback and link-local
                if octets[0] == 127 || (octets[0] == 169 && octets[1] == 254) {
                    continue;
                }
                // Prefer 192.168.x.x
                if octets[0] == 192 && octets[1] == 168 {
                    return Some(v4.to_string());
                }
                // Keep 10.x.x.x or 172.16-31.x.x as fallback
                if fallback.is_none() {
                    fallback = Some(v4.to_string());
                }
            }
        }

        return fallback;
    }

    None
}

/// Start the Axum server on 0.0.0.0:53317
pub async fn start_server(state: SharedState, frontend_dir: String) -> Result<(), String> {
    {
        let mut s = state.lock().await;
        if s.status == ServiceStatus::Running {
            return Err("Server is already running".to_string());
        }
        s.status = ServiceStatus::Starting;
        s.error = None;
    }

    let port = {
        let s = state.lock().await;
        s.port
    };

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));

    // The desktop UI uses history-mode Vue routes (for example `/mobile`).
    // Serve the entry document for an unknown path so a phone opening the QR
    // URL can boot the SPA and let the client router render that route.
    let index_html = std::path::Path::new(&frontend_dir).join("index.html");
    let frontend = ServeDir::new(&frontend_dir).not_found_service(ServeFile::new(index_html));

    // Build the router
    let app = axum::Router::new()
        .route("/api/status", axum::routing::get(handlers::get_status))
        .route("/api/connect", axum::routing::get(handlers::connect))
        .route(
            "/api/devices/register",
            axum::routing::post(handlers::register_device),
        )
        .route("/api/devices", axum::routing::get(handlers::list_devices))
        .route(
            "/api/devices/me",
            axum::routing::get(handlers::get_current_device),
        )
        .route(
            "/api/devices/me/transfers/pending",
            axum::routing::get(handlers::get_pending_transfers),
        )
        .route(
            "/api/transfers",
            axum::routing::get(handlers::list_transfers).post(handlers::create_transfer),
        )
        .route(
            "/api/transfers/relay",
            axum::routing::post(handlers::create_relay),
        )
        .route(
            "/api/transfers/{id}/accept",
            axum::routing::post(handlers::accept_transfer),
        )
        .route(
            "/api/transfers/{id}/reject",
            axum::routing::post(handlers::reject_transfer),
        )
        .route(
            "/api/transfers/{id}/pause",
            axum::routing::post(handlers::pause_transfer),
        )
        .route(
            "/api/transfers/{id}/resume",
            axum::routing::post(handlers::resume_transfer),
        )
        .route(
            "/api/transfers/{id}/resume-info",
            axum::routing::get(handlers::get_resume_info),
        )
        .route(
            "/api/transfers/{id}/files/{file_id}/download",
            axum::routing::get(handlers::download_file),
        )
        .route(
            "/api/transfers/{id}/chunks/{index}",
            axum::routing::post(handlers::upload_chunk),
        )
        .route(
            "/api/transfers/{id}/chunks",
            axum::routing::get(handlers::get_chunks),
        )
        .route(
            "/api/transfers/{id}/complete",
            axum::routing::post(handlers::complete_transfer),
        )
        .route(
            "/api/transfers/{id}/relay-complete",
            axum::routing::post(handlers::complete_relay_download),
        )
        .route(
            "/api/transfers/{id}/cancel",
            axum::routing::post(handlers::cancel_transfer),
        )
        .route("/ws", axum::routing::get(ws::ws_handler))
        .fallback_service(frontend)
        // File chunks are 4 MiB. Axum otherwise rejects bodies above 2 MiB
        // before `upload_chunk` can process them.
        .layer(DefaultBodyLimit::max(
            crate::transfer::CHUNK_SIZE as usize + 1024,
        ))
        .with_state(state.clone());

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    {
        let mut s = state.lock().await;
        s.shutdown_tx = Some(shutdown_tx);
    }

    // Bind the listener
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        let err_msg = format!("Failed to bind to {}: {}", addr, e);
        tracing::error!("{}", err_msg);
        err_msg
    })?;

    // Update state to running
    {
        let mut s = state.lock().await;
        s.status = ServiceStatus::Running;
        s.started_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs().to_string())
                .unwrap_or_default(),
        );
        s.local_ip = get_local_ip();
    }

    tracing::info!("LYNQO server started on {}", addr);

    // Run the server
    let server_result = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = shutdown_rx.await;
    })
    .await;

    // Update state after server stops
    {
        let mut s = state.lock().await;
        s.status = ServiceStatus::Stopped;
        s.shutdown_tx = None;
        if let Err(e) = server_result {
            s.error = Some(e.to_string());
            s.status = ServiceStatus::Failed;
            tracing::error!("Server error: {}", e);
        } else {
            tracing::info!("Server stopped gracefully");
        }
    }

    Ok(())
}

/// Stop the server gracefully
pub async fn stop_server(state: SharedState) -> Result<(), String> {
    let shutdown_tx = {
        let mut s = state.lock().await;
        if s.status != ServiceStatus::Running {
            return Err("Server is not running".to_string());
        }
        s.status = ServiceStatus::Stopping;
        s.shutdown_tx.take()
    };

    match shutdown_tx {
        Some(tx) => {
            let _ = tx.send(());
            tracing::info!("Server shutdown signal sent");
            Ok(())
        }
        None => Err("No shutdown channel available".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn websocket_event_payload_uses_camel_case_fields() {
        let event = WsEvent::TransferProgress {
            transfer_id: "transfer-1".to_string(),
            file_name: "photo.jpg".to_string(),
            transferred_bytes: 42,
            total_bytes: 100,
            progress: 42.0,
            speed_bytes_per_second: 7,
            remaining_seconds: Some(8),
        };

        let payload = serde_json::to_value(event).unwrap();
        assert_eq!(payload["type"], "transfer.progress");
        assert_eq!(payload["payload"]["transferId"], "transfer-1");
        assert_eq!(payload["payload"]["speedBytesPerSecond"], 7);
        assert!(payload["payload"].get("transfer_id").is_none());
    }
}
