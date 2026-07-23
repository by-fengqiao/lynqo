use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRecord {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub device_type: String,
    pub user_agent: String,
    pub client_id: String,
    pub session_token: String,
    pub approved: bool,
    pub trusted: bool,
    pub ip: String,
    pub created_at: String,
    pub last_seen: String,
}

fn device_from_row(row: &Row<'_>) -> rusqlite::Result<DeviceRecord> {
    Ok(DeviceRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        platform: row.get(2)?,
        device_type: row.get(3)?,
        user_agent: row.get(4)?,
        client_id: row.get(5)?,
        session_token: row.get(6)?,
        approved: row.get::<_, i32>(7)? != 0,
        trusted: row.get::<_, i32>(8)? != 0,
        ip: row.get(9)?,
        created_at: row.get(10)?,
        last_seen: row.get(11)?,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferRecord {
    pub id: String,
    pub device_id: String,
    pub direction: String, // "receive", "send", "download_from_host", "relay"
    pub status: String, // "pending", "accepted", "transferring", "paused", "completed", "cancelled", "failed"
    pub total_bytes: i64,
    pub transferred_bytes: i64,
    pub file_count: i32,
    pub save_path: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub target_device_id: Option<String>,
    pub relay_stage: Option<String>,
    pub accepted_at: Option<String>,
    pub expires_at: Option<String>,
    pub paused_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferFileRecord {
    pub id: String,
    pub transfer_id: String,
    pub name: String,
    pub size: i64,
    pub mime_type: String,
    pub chunk_size: i64,
    pub total_chunks: i32,
    pub completed_chunks: i32,
    pub sha256: Option<String>,
    pub save_path: Option<String>,
    pub status: String, // "pending", "transferring", "completed", "failed"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkRecord {
    pub id: String,
    pub file_id: String,
    pub chunk_index: i32,
    pub offset: i64,
    pub size: i64,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadSession {
    pub id: String,
    pub transfer_id: String,
    pub file_id: String,
    pub device_id: String,
    pub token: String,
    pub expires_at: String,
    pub created_at: String,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayFile {
    pub id: String,
    pub transfer_id: String,
    pub file_id: String,
    pub temp_path: String,
    pub cleanup_at: String,
    pub cleaned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferEvent {
    pub id: String,
    pub transfer_id: String,
    pub event_type: String,
    pub event_id: String,
    pub timestamp: String,
    pub payload_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub device_name: String,
    pub receive_folder: String,
    pub require_approval: bool,
    pub auto_approve_known: bool,
    pub port: u16,
    #[serde(default = "default_max_file_size")]
    pub max_file_size: i64,
    #[serde(default = "default_theme_mode")]
    pub theme_mode: String,
}

fn default_max_file_size() -> i64 {
    10 * 1024 * 1024 * 1024 // 10 GB
}

fn default_theme_mode() -> String {
    "system".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            device_name: hostname(),
            receive_folder: default_receive_folder_string(),
            require_approval: true,
            auto_approve_known: true,
            port: 53317,
            max_file_size: default_max_file_size(),
            theme_mode: default_theme_mode(),
        }
    }
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "LYNQO Device".to_string())
}

fn default_receive_folder_string() -> String {
    dirs_download()
        .map(|p| p.join("LYNQO").to_string_lossy().to_string())
        .unwrap_or_else(|| "./received".to_string())
}

fn dirs_download() -> Option<std::path::PathBuf> {
    // Simple cross-platform download folder detection
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE")
            .ok()
            .map(|p| std::path::PathBuf::from(p).join("Downloads"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME")
            .ok()
            .map(|p| std::path::PathBuf::from(p).join("Downloads"))
    }
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(db_path: &Path) -> AppResult<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::Database(format!("Failed to create db directory: {}", e)))?;
        }

        let conn = Connection::open(db_path)
            .map_err(|e| AppError::Database(format!("Failed to open database: {}", e)))?;

        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;",
        )
        .map_err(|e| AppError::Database(format!("Failed to set pragmas: {}", e)))?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS devices (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                platform TEXT NOT NULL DEFAULT '',
                device_type TEXT NOT NULL DEFAULT '',
                user_agent TEXT NOT NULL DEFAULT '',
                client_id TEXT NOT NULL DEFAULT '',
                session_token TEXT NOT NULL UNIQUE,
                approved INTEGER NOT NULL DEFAULT 0,
                trusted INTEGER NOT NULL DEFAULT 0,
                ip TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                last_seen TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS transfers (
                id TEXT PRIMARY KEY,
                device_id TEXT NOT NULL,
                direction TEXT NOT NULL DEFAULT 'receive',
                status TEXT NOT NULL DEFAULT 'pending',
                total_bytes INTEGER NOT NULL DEFAULT 0,
                transferred_bytes INTEGER NOT NULL DEFAULT 0,
                file_count INTEGER NOT NULL DEFAULT 0,
                save_path TEXT,
                created_at TEXT NOT NULL,
                completed_at TEXT,
                target_device_id TEXT,
                relay_stage TEXT,
                accepted_at TEXT,
                expires_at TEXT,
                paused_at TEXT,
                FOREIGN KEY (device_id) REFERENCES devices(id)
            );

            CREATE TABLE IF NOT EXISTS transfer_files (
                id TEXT PRIMARY KEY,
                transfer_id TEXT NOT NULL,
                name TEXT NOT NULL,
                size INTEGER NOT NULL,
                mime_type TEXT NOT NULL DEFAULT '',
                chunk_size INTEGER NOT NULL,
                total_chunks INTEGER NOT NULL,
                completed_chunks INTEGER NOT NULL DEFAULT 0,
                sha256 TEXT,
                save_path TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                FOREIGN KEY (transfer_id) REFERENCES transfers(id)
            );

            CREATE TABLE IF NOT EXISTS chunks (
                id TEXT PRIMARY KEY,
                file_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                offset INTEGER NOT NULL,
                size INTEGER NOT NULL,
                completed INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (file_id) REFERENCES transfer_files(id)
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS download_sessions (
                id TEXT PRIMARY KEY,
                transfer_id TEXT NOT NULL,
                file_id TEXT NOT NULL,
                device_id TEXT NOT NULL,
                token TEXT NOT NULL UNIQUE,
                expires_at TEXT NOT NULL,
                created_at TEXT NOT NULL,
                completed INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (transfer_id) REFERENCES transfers(id),
                FOREIGN KEY (file_id) REFERENCES transfer_files(id),
                FOREIGN KEY (device_id) REFERENCES devices(id)
            );

            CREATE TABLE IF NOT EXISTS relay_files (
                id TEXT PRIMARY KEY,
                transfer_id TEXT NOT NULL,
                file_id TEXT NOT NULL,
                temp_path TEXT NOT NULL DEFAULT '',
                cleanup_at TEXT NOT NULL DEFAULT '',
                cleaned INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (transfer_id) REFERENCES transfers(id),
                FOREIGN KEY (file_id) REFERENCES transfer_files(id)
            );

            CREATE TABLE IF NOT EXISTS transfer_events (
                id TEXT PRIMARY KEY,
                transfer_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                event_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                payload_json TEXT NOT NULL DEFAULT '{}',
                FOREIGN KEY (transfer_id) REFERENCES transfers(id)
            );

            CREATE INDEX IF NOT EXISTS idx_chunks_file ON chunks(file_id, chunk_index);
            CREATE INDEX IF NOT EXISTS idx_transfer_files_transfer ON transfer_files(transfer_id);
            CREATE INDEX IF NOT EXISTS idx_transfers_device ON transfers(device_id);
            CREATE INDEX IF NOT EXISTS idx_transfers_target ON transfers(target_device_id);
            CREATE INDEX IF NOT EXISTS idx_download_sessions_token ON download_sessions(token);
            CREATE INDEX IF NOT EXISTS idx_relay_files_transfer ON relay_files(transfer_id);
            CREATE INDEX IF NOT EXISTS idx_transfer_events_transfer ON transfer_events(transfer_id);",
        )
        .map_err(|e| AppError::Database(format!("Migration failed: {}", e)))?;

        // Idempotent ALTER TABLE: add new columns to transfers if they don't exist yet.
        // SQLite raises "duplicate column name" if the column already exists, so we
        // intentionally ignore errors here.
        let alter_statements = [
            "ALTER TABLE devices ADD COLUMN client_id TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE devices ADD COLUMN trusted INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE transfers ADD COLUMN target_device_id TEXT",
            "ALTER TABLE transfers ADD COLUMN relay_stage TEXT",
            "ALTER TABLE transfers ADD COLUMN accepted_at TEXT",
            "ALTER TABLE transfers ADD COLUMN expires_at TEXT",
            "ALTER TABLE transfers ADD COLUMN paused_at TEXT",
            "ALTER TABLE download_sessions ADD COLUMN completed INTEGER NOT NULL DEFAULT 0",
        ];
        for stmt in &alter_statements {
            let _ = conn.execute(stmt, []);
        }

        conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_devices_client_id
             ON devices(client_id) WHERE client_id <> ''",
            [],
        )
        .map_err(|e| AppError::Database(format!("Create device identity index failed: {}", e)))?;

        // Ensure a synthetic "desktop" device exists so that desktop-initiated
        // transfers (which have no remote device) satisfy the FK constraint.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();
        let _ = conn.execute(
            "INSERT OR IGNORE INTO devices (id, name, platform, device_type, user_agent, client_id, session_token, approved, trusted, ip, created_at, last_seen)
             VALUES ('desktop', 'This PC', 'desktop', 'desktop', '', '', 'desktop-internal-token', 1, 1, '127.0.0.1', ?1, ?2)",
            params![now, now],
        );

        Ok(())
    }

    // --- Device operations ---

    pub fn insert_device(&self, device: &DeviceRecord) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        conn.execute(
            "INSERT INTO devices (id, name, platform, device_type, user_agent, client_id, session_token, approved, trusted, ip, created_at, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                device.id,
                device.name,
                device.platform,
                device.device_type,
                device.user_agent,
                device.client_id,
                device.session_token,
                device.approved as i32,
                device.trusted as i32,
                device.ip,
                device.created_at,
                device.last_seen,
            ],
        )
        .map_err(|e| AppError::Database(format!("Insert device failed: {}", e)))?;

        Ok(())
    }

    pub fn get_device_by_session_token(&self, token: &str) -> AppResult<Option<DeviceRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let result = conn
            .query_row(
                "SELECT id, name, platform, device_type, user_agent, client_id, session_token, approved, trusted, ip, created_at, last_seen
                 FROM devices WHERE session_token = ?1",
                params![token],
                device_from_row,
            )
            .optional()
            .map_err(|e| AppError::Database(format!("Query device failed: {}", e)))?;

        Ok(result)
    }

    pub fn get_device_by_id(&self, id: &str) -> AppResult<Option<DeviceRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let result = conn
            .query_row(
                "SELECT id, name, platform, device_type, user_agent, client_id, session_token, approved, trusted, ip, created_at, last_seen
                 FROM devices WHERE id = ?1",
                params![id],
                device_from_row,
            )
            .optional()
            .map_err(|e| AppError::Database(format!("Query device failed: {}", e)))?;

        Ok(result)
    }

    pub fn list_devices(&self) -> AppResult<Vec<DeviceRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, platform, device_type, user_agent, client_id, session_token, approved, trusted, ip, created_at, last_seen
                 FROM devices ORDER BY last_seen DESC",
            )
            .map_err(|e| AppError::Database(format!("Prepare statement failed: {}", e)))?;

        let devices = stmt
            .query_map([], device_from_row)
            .map_err(|e| AppError::Database(format!("Query devices failed: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(format!("Collect devices failed: {}", e)))?;

        Ok(devices)
    }

    pub fn get_device_by_client_id(&self, client_id: &str) -> AppResult<Option<DeviceRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        conn.query_row(
            "SELECT id, name, platform, device_type, user_agent, client_id, session_token, approved, trusted, ip, created_at, last_seen
             FROM devices WHERE client_id = ?1 AND id <> 'desktop'",
            params![client_id],
            device_from_row,
        )
        .optional()
        .map_err(|e| AppError::Database(format!("Query device identity failed: {}", e)))
    }

    /// Associate the first matching pre-identity record with a stable browser
    /// identity. This keeps old 1.0.4 duplicate records from prompting again.
    pub fn claim_legacy_device(
        &self,
        client_id: &str,
        name: &str,
        platform: &str,
        device_type: &str,
        user_agent: &str,
        ip: &str,
    ) -> AppResult<Option<DeviceRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let device = conn
            .query_row(
                "SELECT id, name, platform, device_type, user_agent, client_id, session_token, approved, trusted, ip, created_at, last_seen
                 FROM devices
                 WHERE id <> 'desktop' AND client_id = '' AND name = ?1 AND platform = ?2
                   AND device_type = ?3 AND user_agent = ?4 AND ip = ?5
                 ORDER BY approved DESC, last_seen DESC LIMIT 1",
                params![name, platform, device_type, user_agent, ip],
                device_from_row,
            )
            .optional()
            .map_err(|e| AppError::Database(format!("Query legacy device failed: {}", e)))?;

        if let Some(mut device) = device {
            conn.execute(
                "UPDATE devices SET client_id = ?1 WHERE id = ?2 AND client_id = ''",
                params![client_id, device.id],
            )
            .map_err(|e| AppError::Database(format!("Claim legacy device failed: {}", e)))?;
            device.client_id = client_id.to_string();
            return Ok(Some(device));
        }

        Ok(None)
    }

    pub fn update_device_registration(
        &self,
        id: &str,
        name: &str,
        platform: &str,
        device_type: &str,
        user_agent: &str,
        ip: &str,
    ) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        conn.execute(
            "UPDATE devices
             SET name = ?1, platform = ?2, device_type = ?3, user_agent = ?4, ip = ?5, last_seen = ?6
             WHERE id = ?7",
            params![name, platform, device_type, user_agent, ip, chrono_now(), id],
        )
        .map_err(|e| AppError::Database(format!("Update device registration failed: {}", e)))?;

        Ok(())
    }

    /// Return a display-safe list with stale records from the same browser
    /// collapsed. The underlying records are preserved for transfer history.
    pub fn list_visible_devices(&self) -> AppResult<Vec<DeviceRecord>> {
        let mut visible: HashMap<String, DeviceRecord> = HashMap::new();
        for device in self.list_devices()? {
            // Keep a newly claimed client record and its old pre-identity
            // siblings in the same display group until the stale rows age
            // out. The database identity remains client_id; this is only a
            // conservative UI cleanup key for records with the same browser
            // fingerprint and LAN address.
            let key = format!(
                "browser|{}|{}|{}|{}|{}",
                device.ip, device.name, device.platform, device.device_type, device.user_agent
            );

            match visible.get(&key) {
                Some(existing)
                    if existing.approved && !device.approved
                        || (existing.approved == device.approved
                            && existing.last_seen >= device.last_seen) => {}
                _ => {
                    visible.insert(key, device);
                }
            }
        }

        let mut devices: Vec<_> = visible.into_values().collect();
        devices.sort_by(|left, right| right.last_seen.cmp(&left.last_seen));
        Ok(devices)
    }

    pub fn set_device_access(&self, id: &str, approved: bool, trusted: bool) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let updated = conn
            .execute(
                "UPDATE devices SET approved = ?1, trusted = ?2 WHERE id = ?3",
                params![approved as i32, trusted as i32, id],
            )
            .map_err(|e| AppError::Database(format!("Update device failed: {}", e)))?;

        if updated == 0 {
            return Err(AppError::DeviceNotFound);
        }

        Ok(())
    }

    pub fn update_device_last_seen(&self, id: &str, ip: &str) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let now = chrono_now();
        conn.execute(
            "UPDATE devices SET last_seen = ?1, ip = ?2 WHERE id = ?3",
            params![now, ip, id],
        )
        .map_err(|e| AppError::Database(format!("Update device failed: {}", e)))?;

        Ok(())
    }

    // --- Transfer operations ---

    pub fn insert_transfer(&self, transfer: &TransferRecord) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        conn.execute(
            "INSERT INTO transfers (id, device_id, direction, status, total_bytes, transferred_bytes, file_count, save_path, created_at, completed_at, target_device_id, relay_stage, accepted_at, expires_at, paused_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                transfer.id,
                transfer.device_id,
                transfer.direction,
                transfer.status,
                transfer.total_bytes,
                transfer.transferred_bytes,
                transfer.file_count,
                transfer.save_path,
                transfer.created_at,
                transfer.completed_at,
                transfer.target_device_id,
                transfer.relay_stage,
                transfer.accepted_at,
                transfer.expires_at,
                transfer.paused_at,
            ],
        )
        .map_err(|e| AppError::Database(format!("Insert transfer failed: {}", e)))?;

        Ok(())
    }

    pub fn get_transfer(&self, id: &str) -> AppResult<Option<TransferRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let result = conn
            .query_row(
                "SELECT id, device_id, direction, status, total_bytes, transferred_bytes, file_count, save_path, created_at, completed_at, target_device_id, relay_stage, accepted_at, expires_at, paused_at
                 FROM transfers WHERE id = ?1",
                params![id],
                |row| {
                    Ok(TransferRecord {
                        id: row.get(0)?,
                        device_id: row.get(1)?,
                        direction: row.get(2)?,
                        status: row.get(3)?,
                        total_bytes: row.get(4)?,
                        transferred_bytes: row.get(5)?,
                        file_count: row.get(6)?,
                        save_path: row.get(7)?,
                        created_at: row.get(8)?,
                        completed_at: row.get(9)?,
                        target_device_id: row.get(10)?,
                        relay_stage: row.get(11)?,
                        accepted_at: row.get(12)?,
                        expires_at: row.get(13)?,
                        paused_at: row.get(14)?,
                    })
                },
            )
            .optional()
            .map_err(|e| AppError::Database(format!("Query transfer failed: {}", e)))?;

        Ok(result)
    }

    pub fn list_transfers(&self) -> AppResult<Vec<TransferRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, device_id, direction, status, total_bytes, transferred_bytes, file_count, save_path, created_at, completed_at, target_device_id, relay_stage, accepted_at, expires_at, paused_at
                 FROM transfers ORDER BY created_at DESC LIMIT 100",
            )
            .map_err(|e| AppError::Database(format!("Prepare statement failed: {}", e)))?;

        let transfers = stmt
            .query_map([], |row| {
                Ok(TransferRecord {
                    id: row.get(0)?,
                    device_id: row.get(1)?,
                    direction: row.get(2)?,
                    status: row.get(3)?,
                    total_bytes: row.get(4)?,
                    transferred_bytes: row.get(5)?,
                    file_count: row.get(6)?,
                    save_path: row.get(7)?,
                    created_at: row.get(8)?,
                    completed_at: row.get(9)?,
                    target_device_id: row.get(10)?,
                    relay_stage: row.get(11)?,
                    accepted_at: row.get(12)?,
                    expires_at: row.get(13)?,
                    paused_at: row.get(14)?,
                })
            })
            .map_err(|e| AppError::Database(format!("Query transfers failed: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(format!("Collect transfers failed: {}", e)))?;

        Ok(transfers)
    }

    pub fn update_transfer_status(&self, id: &str, status: &str) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        conn.execute(
            "UPDATE transfers SET status = ?1 WHERE id = ?2",
            params![status, id],
        )
        .map_err(|e| AppError::Database(format!("Update transfer failed: {}", e)))?;

        Ok(())
    }

    pub fn update_transfer_progress(&self, id: &str, transferred_bytes: i64) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        conn.execute(
            "UPDATE transfers SET transferred_bytes = ?1 WHERE id = ?2",
            params![transferred_bytes, id],
        )
        .map_err(|e| AppError::Database(format!("Update transfer progress failed: {}", e)))?;

        Ok(())
    }

    pub fn complete_transfer(&self, id: &str, save_path: &str) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let now = chrono_now();
        conn.execute(
            "UPDATE transfers SET status = 'completed', transferred_bytes = total_bytes, save_path = ?1, completed_at = ?2 WHERE id = ?3",
            params![save_path, now, id],
        )
        .map_err(|e| AppError::Database(format!("Complete transfer failed: {}", e)))?;

        Ok(())
    }

    // --- Transfer file operations ---

    pub fn insert_transfer_file(&self, file: &TransferFileRecord) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        conn.execute(
            "INSERT INTO transfer_files (id, transfer_id, name, size, mime_type, chunk_size, total_chunks, completed_chunks, sha256, save_path, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                file.id,
                file.transfer_id,
                file.name,
                file.size,
                file.mime_type,
                file.chunk_size,
                file.total_chunks,
                file.completed_chunks,
                file.sha256,
                file.save_path,
                file.status,
            ],
        )
        .map_err(|e| AppError::Database(format!("Insert transfer file failed: {}", e)))?;

        Ok(())
    }

    pub fn get_transfer_files(&self, transfer_id: &str) -> AppResult<Vec<TransferFileRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, transfer_id, name, size, mime_type, chunk_size, total_chunks, completed_chunks, sha256, save_path, status
                 FROM transfer_files WHERE transfer_id = ?1 ORDER BY rowid",
            )
            .map_err(|e| AppError::Database(format!("Prepare statement failed: {}", e)))?;

        let files = stmt
            .query_map(params![transfer_id], |row| {
                Ok(TransferFileRecord {
                    id: row.get(0)?,
                    transfer_id: row.get(1)?,
                    name: row.get(2)?,
                    size: row.get(3)?,
                    mime_type: row.get(4)?,
                    chunk_size: row.get(5)?,
                    total_chunks: row.get(6)?,
                    completed_chunks: row.get(7)?,
                    sha256: row.get(8)?,
                    save_path: row.get(9)?,
                    status: row.get(10)?,
                })
            })
            .map_err(|e| AppError::Database(format!("Query files failed: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(format!("Collect files failed: {}", e)))?;

        Ok(files)
    }

    pub fn get_transfer_file(&self, file_id: &str) -> AppResult<Option<TransferFileRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let result = conn
            .query_row(
                "SELECT id, transfer_id, name, size, mime_type, chunk_size, total_chunks, completed_chunks, sha256, save_path, status
                 FROM transfer_files WHERE id = ?1",
                params![file_id],
                |row| {
                    Ok(TransferFileRecord {
                        id: row.get(0)?,
                        transfer_id: row.get(1)?,
                        name: row.get(2)?,
                        size: row.get(3)?,
                        mime_type: row.get(4)?,
                        chunk_size: row.get(5)?,
                        total_chunks: row.get(6)?,
                        completed_chunks: row.get(7)?,
                        sha256: row.get(8)?,
                        save_path: row.get(9)?,
                        status: row.get(10)?,
                    })
                },
            )
            .optional()
            .map_err(|e| AppError::Database(format!("Query file failed: {}", e)))?;

        Ok(result)
    }

    pub fn increment_file_completed_chunks(&self, file_id: &str) -> AppResult<i32> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        conn.execute(
            "UPDATE transfer_files SET completed_chunks = completed_chunks + 1, status = 'transferring' WHERE id = ?1",
            params![file_id],
        )
        .map_err(|e| AppError::Database(format!("Update file chunks failed: {}", e)))?;

        let count: i32 = conn
            .query_row(
                "SELECT completed_chunks FROM transfer_files WHERE id = ?1",
                params![file_id],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(format!("Query chunk count failed: {}", e)))?;

        Ok(count)
    }

    pub fn complete_transfer_file(
        &self,
        file_id: &str,
        sha256: &str,
        save_path: &str,
    ) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        conn.execute(
            "UPDATE transfer_files SET status = 'completed', sha256 = ?1, save_path = ?2 WHERE id = ?3",
            params![sha256, save_path, file_id],
        )
        .map_err(|e| AppError::Database(format!("Complete file failed: {}", e)))?;

        Ok(())
    }

    /// Store a digest that was computed for a source file without changing its
    /// transfer status. Desktop-originated files remain downloadable while
    /// their SHA-256 is calculated in the background.
    pub fn update_transfer_file_checksum(&self, file_id: &str, sha256: &str) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        conn.execute(
            "UPDATE transfer_files SET sha256 = ?1 WHERE id = ?2",
            params![sha256, file_id],
        )
        .map_err(|e| AppError::Database(format!("Update file checksum failed: {}", e)))?;

        Ok(())
    }

    // --- Chunk operations ---

    pub fn insert_chunks(&self, chunks: &[ChunkRecord]) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| AppError::Database(format!("Begin transaction failed: {}", e)))?;

        for chunk in chunks {
            tx.execute(
                "INSERT INTO chunks (id, file_id, chunk_index, offset, size, completed)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    chunk.id,
                    chunk.file_id,
                    chunk.chunk_index,
                    chunk.offset,
                    chunk.size,
                    chunk.completed as i32,
                ],
            )
            .map_err(|e| AppError::Database(format!("Insert chunk failed: {}", e)))?;
        }

        tx.commit()
            .map_err(|e| AppError::Database(format!("Commit transaction failed: {}", e)))?;

        Ok(())
    }

    /// Mark a chunk as completed. Returns `true` if the chunk was newly
    /// completed by this call, or `false` if it was already marked completed
    /// (i.e. the operation is idempotent and callers should not increment
    /// progress counters again).
    pub fn mark_chunk_completed(&self, file_id: &str, chunk_index: i32) -> AppResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let already_completed: bool = conn
            .query_row(
                "SELECT completed FROM chunks WHERE file_id = ?1 AND chunk_index = ?2",
                params![file_id, chunk_index],
                |row| row.get::<_, i32>(0),
            )
            .optional()
            .map_err(|e| AppError::Database(format!("Query chunk failed: {}", e)))?
            .map(|c| c != 0)
            // Chunk record missing (should not happen): treat as not completed
            // so the UPDATE below still runs.
            .unwrap_or(false);

        if already_completed {
            return Ok(false);
        }

        conn.execute(
            "UPDATE chunks SET completed = 1 WHERE file_id = ?1 AND chunk_index = ?2",
            params![file_id, chunk_index],
        )
        .map_err(|e| AppError::Database(format!("Mark chunk failed: {}", e)))?;

        Ok(true)
    }

    pub fn get_completed_chunk_indices(&self, file_id: &str) -> AppResult<Vec<i32>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let mut stmt = conn
            .prepare("SELECT chunk_index FROM chunks WHERE file_id = ?1 AND completed = 1 ORDER BY chunk_index")
            .map_err(|e| AppError::Database(format!("Prepare statement failed: {}", e)))?;

        let indices = stmt
            .query_map(params![file_id], |row| row.get(0))
            .map_err(|e| AppError::Database(format!("Query chunks failed: {}", e)))?
            .collect::<Result<Vec<i32>, _>>()
            .map_err(|e| AppError::Database(format!("Collect chunks failed: {}", e)))?;

        Ok(indices)
    }

    pub fn get_all_completed_chunk_indices(&self, transfer_id: &str) -> AppResult<Vec<i32>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT c.chunk_index FROM chunks c
                 JOIN transfer_files tf ON c.file_id = tf.id
                 WHERE tf.transfer_id = ?1 AND c.completed = 1
                 ORDER BY c.chunk_index",
            )
            .map_err(|e| AppError::Database(format!("Prepare statement failed: {}", e)))?;

        let indices = stmt
            .query_map(params![transfer_id], |row| row.get(0))
            .map_err(|e| AppError::Database(format!("Query chunks failed: {}", e)))?
            .collect::<Result<Vec<i32>, _>>()
            .map_err(|e| AppError::Database(format!("Collect chunks failed: {}", e)))?;

        Ok(indices)
    }

    // --- Settings operations ---

    pub fn get_settings(&self) -> AppResult<Settings> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let mut settings = Settings::default();

        let mut stmt = conn
            .prepare("SELECT key, value FROM settings")
            .map_err(|e| AppError::Database(format!("Prepare statement failed: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| AppError::Database(format!("Query settings failed: {}", e)))?;

        for row in rows {
            let (key, value) =
                row.map_err(|e| AppError::Database(format!("Read setting failed: {}", e)))?;
            match key.as_str() {
                "device_name" => settings.device_name = value,
                "receive_folder" => settings.receive_folder = value,
                "require_approval" => settings.require_approval = value == "true",
                "auto_approve_known" => settings.auto_approve_known = value == "true",
                "port" => {
                    if let Ok(p) = value.parse() {
                        settings.port = p;
                    }
                }
                "max_file_size" => {
                    if let Ok(v) = value.parse() {
                        settings.max_file_size = v;
                    }
                }
                "theme_mode" => settings.theme_mode = value,
                _ => {}
            }
        }

        Ok(settings)
    }

    pub fn save_settings(&self, settings: &Settings) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let pairs = [
            ("device_name", settings.device_name.clone()),
            ("receive_folder", settings.receive_folder.clone()),
            ("require_approval", settings.require_approval.to_string()),
            (
                "auto_approve_known",
                settings.auto_approve_known.to_string(),
            ),
            ("port", settings.port.to_string()),
            ("max_file_size", settings.max_file_size.to_string()),
            ("theme_mode", settings.theme_mode.clone()),
        ];

        for (key, value) in &pairs {
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                params![key, value],
            )
            .map_err(|e| AppError::Database(format!("Save setting failed: {}", e)))?;
        }

        Ok(())
    }

    // --- Download session operations ---

    pub fn create_download_session(
        &self,
        transfer_id: &str,
        file_id: &str,
        device_id: &str,
        token: &str,
        expires_at: &str,
    ) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        // Opportunistically purge expired sessions whenever a new one is
        // created so the table does not grow without bound (ISSUE 10).
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = conn.execute(
            "DELETE FROM download_sessions WHERE CAST(expires_at AS INTEGER) < ?1",
            params![now_secs],
        );

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono_now();
        conn.execute(
            "INSERT INTO download_sessions (id, transfer_id, file_id, device_id, token, expires_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, transfer_id, file_id, device_id, token, expires_at, now],
        )
        .map_err(|e| AppError::Database(format!("Insert download session failed: {}", e)))?;

        Ok(())
    }

    /// Delete all download sessions whose expiry timestamp is in the past.
    /// Returns the number of rows removed. `expires_at` is stored as a unix
    /// timestamp string, so it is cast to an integer for the comparison.
    pub fn purge_expired_download_sessions(&self) -> AppResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let removed = conn
            .execute(
                "DELETE FROM download_sessions WHERE CAST(expires_at AS INTEGER) < ?1",
                params![now_secs],
            )
            .map_err(|e| AppError::Database(format!("Purge download sessions failed: {}", e)))?;

        Ok(removed)
    }

    pub fn validate_download_token(
        &self,
        token: &str,
        device_id: &str,
    ) -> AppResult<Option<DownloadSession>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let result = conn
            .query_row(
                "SELECT id, transfer_id, file_id, device_id, token, expires_at, created_at, completed
                 FROM download_sessions WHERE token = ?1 AND device_id = ?2",
                params![token, device_id],
                |row| {
                    Ok(DownloadSession {
                        id: row.get(0)?,
                        transfer_id: row.get(1)?,
                        file_id: row.get(2)?,
                        device_id: row.get(3)?,
                        token: row.get(4)?,
                        expires_at: row.get(5)?,
                        created_at: row.get(6)?,
                        completed: row.get::<_, i32>(7)? != 0,
                    })
                },
            )
            .optional()
            .map_err(|e| AppError::Database(format!("Query download session failed: {}", e)))?;

        // Check expiry using numeric comparison of unix timestamps
        if let Some(ref session) = result {
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let expires_secs: u64 = session.expires_at.parse().unwrap_or(0);
            if expires_secs < now_secs {
                return Ok(None);
            }
        }

        Ok(result)
    }

    /// Mark one accepted file download as fully streamed. Returns true only
    /// when every file session for the transfer has completed.
    pub fn complete_download_session(
        &self,
        session_id: &str,
        transfer_id: &str,
    ) -> AppResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        conn.execute(
            "UPDATE download_sessions SET completed = 1 WHERE id = ?1 AND transfer_id = ?2",
            params![session_id, transfer_id],
        )
        .map_err(|e| AppError::Database(format!("Complete download session failed: {}", e)))?;

        let incomplete: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM download_sessions WHERE transfer_id = ?1 AND completed = 0",
                params![transfer_id],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(format!("Count download sessions failed: {}", e)))?;

        Ok(incomplete == 0)
    }

    // --- Relay file operations ---

    pub fn insert_relay_file(&self, relay_file: &RelayFile) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        conn.execute(
            "INSERT INTO relay_files (id, transfer_id, file_id, temp_path, cleanup_at, cleaned)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                relay_file.id,
                relay_file.transfer_id,
                relay_file.file_id,
                relay_file.temp_path,
                relay_file.cleanup_at,
                relay_file.cleaned as i32,
            ],
        )
        .map_err(|e| AppError::Database(format!("Insert relay file failed: {}", e)))?;

        Ok(())
    }

    pub fn get_expired_relay_files(&self) -> AppResult<Vec<RelayFile>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let now = chrono_now();
        let mut stmt = conn
            .prepare(
                "SELECT id, transfer_id, file_id, temp_path, cleanup_at, cleaned
                 FROM relay_files WHERE cleaned = 0 AND cleanup_at != '' AND cleanup_at <= ?1",
            )
            .map_err(|e| AppError::Database(format!("Prepare statement failed: {}", e)))?;

        let files = stmt
            .query_map(params![now], |row| {
                Ok(RelayFile {
                    id: row.get(0)?,
                    transfer_id: row.get(1)?,
                    file_id: row.get(2)?,
                    temp_path: row.get(3)?,
                    cleanup_at: row.get(4)?,
                    cleaned: row.get::<_, i32>(5)? != 0,
                })
            })
            .map_err(|e| AppError::Database(format!("Query relay files failed: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(format!("Collect relay files failed: {}", e)))?;

        Ok(files)
    }

    pub fn mark_relay_cleaned(&self, id: &str) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        conn.execute(
            "UPDATE relay_files SET cleaned = 1 WHERE id = ?1",
            params![id],
        )
        .map_err(|e| AppError::Database(format!("Mark relay cleaned failed: {}", e)))?;

        Ok(())
    }

    /// Look up the relay temp-file record for a single transfer file. Used by
    /// the upload path to resolve where relay chunks must be written.
    pub fn get_relay_file_by_file_id(&self, file_id: &str) -> AppResult<Option<RelayFile>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let result = conn
            .query_row(
                "SELECT id, transfer_id, file_id, temp_path, cleanup_at, cleaned
                 FROM relay_files WHERE file_id = ?1",
                params![file_id],
                |row| {
                    Ok(RelayFile {
                        id: row.get(0)?,
                        transfer_id: row.get(1)?,
                        file_id: row.get(2)?,
                        temp_path: row.get(3)?,
                        cleanup_at: row.get(4)?,
                        cleaned: row.get::<_, i32>(5)? != 0,
                    })
                },
            )
            .optional()
            .map_err(|e| AppError::Database(format!("Query relay file failed: {}", e)))?;

        Ok(result)
    }

    /// Return all relay temp-file records belonging to a transfer. Used to
    /// clean up relay temp files once a relay completes or is cancelled.
    pub fn get_relay_files_by_transfer(&self, transfer_id: &str) -> AppResult<Vec<RelayFile>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, transfer_id, file_id, temp_path, cleanup_at, cleaned
                 FROM relay_files WHERE transfer_id = ?1 ORDER BY rowid",
            )
            .map_err(|e| AppError::Database(format!("Prepare statement failed: {}", e)))?;

        let files = stmt
            .query_map(params![transfer_id], |row| {
                Ok(RelayFile {
                    id: row.get(0)?,
                    transfer_id: row.get(1)?,
                    file_id: row.get(2)?,
                    temp_path: row.get(3)?,
                    cleanup_at: row.get(4)?,
                    cleaned: row.get::<_, i32>(5)? != 0,
                })
            })
            .map_err(|e| AppError::Database(format!("Query relay files failed: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(format!("Collect relay files failed: {}", e)))?;

        Ok(files)
    }

    // --- Transfer event operations ---

    pub fn insert_transfer_event(&self, event: &TransferEvent) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        conn.execute(
            "INSERT INTO transfer_events (id, transfer_id, event_type, event_id, timestamp, payload_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                event.id,
                event.transfer_id,
                event.event_type,
                event.event_id,
                event.timestamp,
                event.payload_json,
            ],
        )
        .map_err(|e| AppError::Database(format!("Insert transfer event failed: {}", e)))?;

        Ok(())
    }

    // --- Extended transfer operations ---

    pub fn get_pending_transfers_for_device(
        &self,
        device_id: &str,
    ) -> AppResult<Vec<TransferRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, device_id, direction, status, total_bytes, transferred_bytes, file_count, save_path, created_at, completed_at, target_device_id, relay_stage, accepted_at, expires_at, paused_at
                 FROM transfers WHERE target_device_id = ?1 AND status = 'pending'
                 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(format!("Prepare statement failed: {}", e)))?;

        let transfers = stmt
            .query_map(params![device_id], |row| {
                Ok(TransferRecord {
                    id: row.get(0)?,
                    device_id: row.get(1)?,
                    direction: row.get(2)?,
                    status: row.get(3)?,
                    total_bytes: row.get(4)?,
                    transferred_bytes: row.get(5)?,
                    file_count: row.get(6)?,
                    save_path: row.get(7)?,
                    created_at: row.get(8)?,
                    completed_at: row.get(9)?,
                    target_device_id: row.get(10)?,
                    relay_stage: row.get(11)?,
                    accepted_at: row.get(12)?,
                    expires_at: row.get(13)?,
                    paused_at: row.get(14)?,
                })
            })
            .map_err(|e| AppError::Database(format!("Query pending transfers failed: {}", e)))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(format!("Collect pending transfers failed: {}", e)))?;

        Ok(transfers)
    }

    pub fn update_transfer_direction(
        &self,
        id: &str,
        direction: &str,
        target_device_id: &str,
    ) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        conn.execute(
            "UPDATE transfers SET direction = ?1, target_device_id = ?2 WHERE id = ?3",
            params![direction, target_device_id, id],
        )
        .map_err(|e| AppError::Database(format!("Update transfer direction failed: {}", e)))?;

        Ok(())
    }

    pub fn update_transfer_relay_stage(&self, id: &str, stage: &str) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        conn.execute(
            "UPDATE transfers SET relay_stage = ?1 WHERE id = ?2",
            params![stage, id],
        )
        .map_err(|e| AppError::Database(format!("Update relay stage failed: {}", e)))?;

        Ok(())
    }

    pub fn set_transfer_accepted(&self, id: &str) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let now = chrono_now();
        conn.execute(
            "UPDATE transfers SET status = 'accepted', accepted_at = ?1 WHERE id = ?2",
            params![now, id],
        )
        .map_err(|e| AppError::Database(format!("Accept transfer failed: {}", e)))?;

        Ok(())
    }

    pub fn pause_transfer(&self, id: &str) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let now = chrono_now();
        conn.execute(
            "UPDATE transfers SET status = 'paused', paused_at = ?1 WHERE id = ?2",
            params![now, id],
        )
        .map_err(|e| AppError::Database(format!("Pause transfer failed: {}", e)))?;

        Ok(())
    }

    /// Resume a paused transfer. The status is only advanced to
    /// `'transferring'` when the transfer is currently `'paused'` (guarded in
    /// the SQL `WHERE` clause) so that resuming a transfer in any other state
    /// is a no-op and cannot cause illegal state-machine jumps.
    ///
    /// Returns `true` if a row was actually updated (i.e. the transfer was
    /// paused), or `false` when the transfer was not in the `'paused'` state.
    pub fn resume_transfer(&self, id: &str) -> AppResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::Database(format!("Lock poisoned: {}", e)))?;

        let updated = conn.execute(
            "UPDATE transfers SET status = 'transferring', paused_at = NULL WHERE id = ?1 AND status = 'paused'",
            params![id],
        )
        .map_err(|e| AppError::Database(format!("Resume transfer failed: {}", e)))?;

        Ok(updated > 0)
    }
}

fn chrono_now() -> String {
    // Simple ISO-8601 timestamp without chrono dependency
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Format as a simple timestamp
    format!("{}", secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "lynqo-storage-{}-{}.sqlite",
            label,
            uuid::Uuid::new_v4()
        ))
    }

    fn device(id: &str, approved: bool, trusted: bool, last_seen: &str) -> DeviceRecord {
        DeviceRecord {
            id: id.to_string(),
            name: "Android Device".to_string(),
            platform: "android".to_string(),
            device_type: "phone".to_string(),
            user_agent: "test-browser".to_string(),
            client_id: String::new(),
            session_token: format!("session-{}", id),
            approved,
            trusted,
            ip: "192.168.1.5".to_string(),
            created_at: last_seen.to_string(),
            last_seen: last_seen.to_string(),
        }
    }

    #[test]
    fn migrates_existing_device_database_before_creating_identity_index() {
        let path = test_db_path("migration");
        let legacy = Connection::open(&path).unwrap();
        legacy
            .execute_batch(
                "CREATE TABLE devices (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    platform TEXT NOT NULL DEFAULT '',
                    device_type TEXT NOT NULL DEFAULT '',
                    user_agent TEXT NOT NULL DEFAULT '',
                    session_token TEXT NOT NULL UNIQUE,
                    approved INTEGER NOT NULL DEFAULT 0,
                    ip TEXT NOT NULL DEFAULT '',
                    created_at TEXT NOT NULL,
                    last_seen TEXT NOT NULL
                );",
            )
            .unwrap();
        legacy
            .execute(
                "INSERT INTO devices (id, name, platform, device_type, user_agent, session_token, approved, ip, created_at, last_seen)
                 VALUES ('legacy-phone', 'Legacy Phone', 'android', 'phone', 'test', 'legacy-token', 1, '192.168.1.5', '1', '1')",
                [],
            )
            .unwrap();
        drop(legacy);

        let db = Database::open(&path).unwrap();
        assert!(db.get_device_by_id("desktop").unwrap().is_some());
        let migrated = db.get_device_by_id("legacy-phone").unwrap().unwrap();
        assert!(migrated.approved);
        assert!(!migrated.trusted);
    }

    #[test]
    fn legacy_duplicates_collapse_and_keep_the_existing_approval() {
        let db = Database::open(&test_db_path("identity")).unwrap();
        db.insert_device(&device("older-approved", true, true, "10"))
            .unwrap();
        db.insert_device(&device("newer-pending", false, false, "20"))
            .unwrap();

        let visible: Vec<_> = db
            .list_visible_devices()
            .unwrap()
            .into_iter()
            .filter(|entry| entry.id != "desktop")
            .collect();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, "older-approved");

        let claimed = db
            .claim_legacy_device(
                "stable-browser-id",
                "Android Device",
                "android",
                "phone",
                "test-browser",
                "192.168.1.5",
            )
            .unwrap()
            .unwrap();
        assert_eq!(claimed.id, "older-approved");
        assert!(claimed.approved);
        assert_eq!(
            db.get_device_by_client_id("stable-browser-id")
                .unwrap()
                .unwrap()
                .session_token,
            "session-older-approved"
        );
    }

    #[test]
    fn trusted_access_persists_and_can_be_revoked() {
        let db = Database::open(&test_db_path("trusted-access")).unwrap();
        db.insert_device(&device("phone", false, false, "1"))
            .unwrap();

        db.set_device_access("phone", true, true).unwrap();
        let trusted = db.get_device_by_id("phone").unwrap().unwrap();
        assert!(trusted.approved);
        assert!(trusted.trusted);

        db.set_device_access("phone", false, false).unwrap();
        let revoked = db.get_device_by_id("phone").unwrap().unwrap();
        assert!(!revoked.approved);
        assert!(!revoked.trusted);
    }
}
