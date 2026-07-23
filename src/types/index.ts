export type Platform = "windows" | "macos" | "ios" | "android" | "web";

export type DeviceType = "desktop" | "laptop" | "phone" | "tablet";

export interface Device {
  id: string;
  name: string;
  platform: Platform;
  deviceType: DeviceType;
  ip: string;
  online: boolean;
  approved: boolean;
  /** Explicitly persisted by the desktop user. Trusted devices auto-approve on reconnect. */
  trusted: boolean;
  latencyMs?: number;
  lastSeenAt: string;
}

export interface TransferFile {
  id: string;
  name: string;
  size: number;
  mimeType?: string;
  checksum?: string;
}

export type TransferStatus =
  | "pending"
  | "waiting"
  | "requesting"
  | "awaiting_acceptance"
  | "accepted"
  | "transferring"
  | "paused"
  | "verifying"
  | "completed"
  | "rejected"
  | "expired"
  | "cancelled"
  | "failed";

export type TransferDirection = "upload_to_host" | "download_from_host" | "relay";

export type RelayStage =
  | "uploading_to_host"
  | "waiting_for_target"
  | "downloading_to_target"
  | "completed";

export interface TransferTask {
  id: string;
  direction: TransferDirection;
  sourceDeviceId: string;
  targetDeviceId: string;
  files: TransferFile[];
  totalBytes: number;
  transferredBytes: number;
  speedBytesPerSecond: number;
  remainingSeconds?: number;
  progress: number;
  status: TransferStatus;
  createdAt: string;
  completedAt?: string;
  savePath?: string;
  error?: string;
  relayStage?: RelayStage;
  acceptedAt?: string;
  expiresAt?: string;
  pausedAt?: string;
  chunkTotal?: number;
  chunkDone?: number;
  chunkSize?: number;
  latencyMs?: number;
  startedAt?: string;
  retryCount?: number;
  protocol?: string;
}

export type ThemeMode = "light" | "dark" | "system";

export type ToastKind = "success" | "error" | "info" | "warning";

export interface AppState {
  serverRunning: boolean;
  networkName: string;
  localIp: string;
  deviceName: string;
  connectionToken: string;
}
