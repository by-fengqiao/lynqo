use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use futures_util::StreamExt;
use std::collections::HashMap;

use crate::server::{SharedState, WsEvent};
use crate::storage::DeviceRecord;

fn add_device_connection(connections: &mut HashMap<String, usize>, device_id: &str) -> bool {
    let count = connections.entry(device_id.to_string()).or_insert(0);
    let became_online = *count == 0;
    *count += 1;
    became_online
}

fn remove_device_connection(connections: &mut HashMap<String, usize>, device_id: &str) -> bool {
    let Some(count) = connections.get_mut(device_id) else {
        return false;
    };
    if *count > 1 {
        *count -= 1;
        false
    } else {
        connections.remove(device_id);
        true
    }
}

/// Announce a device which has opened a WebSocket session. A device that is
/// not yet approved needs an authorization prompt, rather than a generic
/// online event that the desktop has no reason to surface to the user.
fn device_session_event(device: &DeviceRecord) -> WsEvent {
    if device.approved {
        WsEvent::DeviceConnected {
            device_id: device.id.clone(),
            name: device.name.clone(),
            platform: device.platform.clone(),
            device_type: device.device_type.clone(),
            ip: device.ip.clone(),
            approved: true,
        }
    } else {
        WsEvent::DeviceApprovalRequired {
            device_id: device.id.clone(),
            name: device.name.clone(),
            ip: device.ip.clone(),
            platform: device.platform.clone(),
            device_type: device.device_type.clone(),
            user_agent: device.user_agent.clone(),
        }
    }
}

/// GET /ws?token=xxx - upgrade to WebSocket
///
/// Validates the token (either the desktop-only control token, or a session
/// token for a mobile/remote device), then subscribes to the
/// broadcast channel and forwards WsEvents as JSON to the client.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let token = params.get("token").cloned().unwrap_or_default();

    // Validate token and remember the device identity for targeted events and
    // the desktop online indicator. The desktop control token has no device
    // id, while mobile sessions do.
    let device = {
        let s = state.lock().await;
        if token == s.desktop_control_token {
            None
        } else {
            match s.db.get_device_by_session_token(&token) {
                Ok(Some(device)) => Some(device),
                _ => {
                    tracing::error!("WebSocket connection rejected: invalid token");
                    return (axum::http::StatusCode::UNAUTHORIZED, "Invalid token").into_response();
                }
            }
        }
    };

    let device_id = device.as_ref().map(|device| device.id.clone());

    // Subscribe to the broadcast channel before upgrading
    let rx = {
        let mut s = state.lock().await;
        if let Some(device) = &device {
            if add_device_connection(&mut s.connected_devices, &device.id) {
                let _ = s.event_tx.send(device_session_event(device));
            }
        }
        s.event_tx.subscribe()
    };

    tracing::info!("WebSocket connection accepted");

    ws.on_upgrade(move |socket| handle_socket(socket, rx, state, device_id))
}

/// Handle an individual WebSocket connection.
/// Forwards broadcast events to the client and handles incoming messages (ping/pong).
async fn handle_socket(
    socket: WebSocket,
    mut rx: tokio::sync::broadcast::Receiver<crate::server::WsEvent>,
    state: SharedState,
    device_id: Option<String>,
) {
    let (mut sender, mut receiver) = socket.split();

    // Task: forward broadcast events to this WebSocket client
    let recipient_id = device_id.clone();
    let mut send_task = tokio::spawn(async move {
        use futures_util::SinkExt;

        loop {
            match rx.recv().await {
                Ok(event) => {
                    if !should_deliver_event(&event, recipient_id.as_deref()) {
                        continue;
                    }
                    let json = match serde_json::to_string(&event) {
                        Ok(j) => j,
                        Err(e) => {
                            tracing::error!("Failed to serialize WsEvent: {}", e);
                            continue;
                        }
                    };

                    if sender.send(Message::Text(json.into())).await.is_err() {
                        // Client disconnected
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::error!("WebSocket client lagged, skipped {} events", n);
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    // Broadcast channel closed (server shutting down)
                    break;
                }
            }
        }
    });

    // Task: handle incoming messages from the client (ping/pong, close)
    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Ping(data)) => {
                    // axum handles pong automatically in most cases,
                    // but we acknowledge the ping here for completeness
                    tracing::debug!("Received ping ({} bytes)", data.len());
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("WebSocket client sent close frame");
                    break;
                }
                Ok(_) => {
                    // Ignore text/binary messages from clients
                }
                Err(e) => {
                    tracing::error!("WebSocket receive error: {}", e);
                    break;
                }
            }
        }
    });

    // Wait for either task to complete (client disconnect or server shutdown)
    tokio::select! {
        _ = &mut send_task => {
            tracing::info!("WebSocket send task ended");
            recv_task.abort();
            let _ = recv_task.await;
        }
        _ = &mut recv_task => {
            tracing::info!("WebSocket receive task ended");
            send_task.abort();
            let _ = send_task.await;
        }
    }

    if let Some(device_id) = device_id {
        let mut s = state.lock().await;
        if remove_device_connection(&mut s.connected_devices, &device_id) {
            // "Allow once" is deliberately scoped to an active browser
            // session. Only a device the user explicitly trusted keeps its
            // authorization after its final socket closes.
            match s.db.get_device_by_id(&device_id) {
                Ok(Some(device)) if !device.trusted => {
                    if let Err(error) = s.db.set_device_access(&device_id, false, false) {
                        tracing::warn!("Failed to revoke one-time device access: {}", error);
                    }
                }
                Ok(_) => {}
                Err(error) => tracing::warn!("Failed to read device at disconnect: {}", error),
            }
            let _ = s.event_tx.send(WsEvent::DeviceDisconnected { device_id });
        }
    }

    tracing::info!("WebSocket connection closed");
}

/// Transfer invitations must only be delivered to their selected target. Other
/// lifecycle events remain visible to the desktop control channel.
fn should_deliver_event(event: &WsEvent, device_id: Option<&str>) -> bool {
    // The desktop control channel can observe lifecycle events. A mobile
    // browser only needs an invitation explicitly addressed to its device;
    // suppressing every other broadcast avoids leaking another device's
    // transfer names, sizes, or progress.
    match (event, device_id) {
        (_, None) => true,
        (
            WsEvent::TransferRequested {
                target_device_id, ..
            },
            Some(id),
        ) => id == target_device_id,
        (WsEvent::DeviceApproved { device_id, .. }, Some(id)) => id == device_id,
        (WsEvent::DeviceRejected { device_id, .. }, Some(id)) => id == device_id,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{add_device_connection, device_session_event, remove_device_connection};
    use crate::storage::DeviceRecord;
    use std::collections::HashMap;

    fn device(approved: bool, trusted: bool) -> DeviceRecord {
        DeviceRecord {
            id: "device-1".to_string(),
            name: "Android Device".to_string(),
            platform: "android".to_string(),
            device_type: "phone".to_string(),
            user_agent: "test-agent".to_string(),
            client_id: "client-1".to_string(),
            session_token: "session-1".to_string(),
            approved,
            trusted,
            ip: "192.168.1.5".to_string(),
            created_at: "0".to_string(),
            last_seen: "0".to_string(),
        }
    }

    #[test]
    fn device_stays_online_until_its_last_socket_closes() {
        let mut connections = HashMap::new();
        assert!(add_device_connection(&mut connections, "device-1"));
        assert!(!add_device_connection(&mut connections, "device-1"));
        assert!(!remove_device_connection(&mut connections, "device-1"));
        assert!(connections.contains_key("device-1"));
        assert!(remove_device_connection(&mut connections, "device-1"));
        assert!(!connections.contains_key("device-1"));
    }

    #[test]
    fn unapproved_reconnecting_device_reopens_the_authorization_request() {
        let payload = serde_json::to_value(device_session_event(&device(false, false))).unwrap();

        assert_eq!(payload["type"], "device.approval_required");
        assert_eq!(payload["payload"]["deviceId"], "device-1");
        assert_eq!(payload["payload"]["name"], "Android Device");
    }
}
