import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

// Types matching Rust responses
export interface ServiceInfo {
  status: string;
  port: number;
  localIp?: string;
  localUrl?: string;
  startedAt?: string;
  error?: string;
}

export interface ConnectionInfo {
  ip: string;
  port: number;
  token: string;
  controlToken: string;
  localDomain: string;
  qrUrl: string;
  networkName: string;
  receiveFolder: string;
  deviceName: string;
}

export interface QrCodeData {
  url: string;
  svg: string;
}

export interface AppVersionInfo {
  version: string;
  commit: string;
  buildDate: string;
  channel: string;
}

export interface CommandResult {
  success: boolean;
  error?: string;
}

/** A desktop-to-device transfer invitation created by the native backend. */
export interface SendTransferResult {
  success: boolean;
  error?: string;
  transferId: string;
  status: string;
  totalBytes: number;
  fileCount: number;
}

// Check if running in Tauri (not plain browser)
export function isTauri(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

// All commands wrapped with error handling
export async function startLocalService(): Promise<CommandResult> {
  return invoke("start_local_service");
}

export async function stopLocalService(): Promise<CommandResult> {
  return invoke("stop_local_service");
}

export async function getLocalServiceStatus(): Promise<ServiceInfo> {
  return invoke("get_local_service_status");
}

export async function refreshLocalIp(): Promise<string> {
  return invoke("refresh_local_ip");
}

export async function regenerateConnectionToken(): Promise<string> {
  return invoke("regenerate_connection_token");
}

export async function getConnectionInfo(): Promise<ConnectionInfo> {
  return invoke("get_connection_info");
}

export async function getConnectionQrCode(): Promise<QrCodeData> {
  return invoke("get_connection_qr_code");
}

export async function getAppVersion(): Promise<AppVersionInfo> {
  return invoke("get_app_version");
}

export async function getDevices(): Promise<unknown[]> {
  const json = await invoke<string>("get_devices");
  return JSON.parse(json);
}

export async function approveDevice(deviceId: string, trusted: boolean): Promise<CommandResult> {
  return invoke("approve_device", { deviceId, trusted });
}

export async function rejectDevice(deviceId: string): Promise<CommandResult> {
  return invoke("reject_device", { deviceId });
}

export async function forgetDevice(deviceId: string): Promise<CommandResult> {
  return invoke("forget_device", { deviceId });
}

export async function getTransfers(): Promise<unknown[]> {
  const json = await invoke<string>("get_transfers");
  return JSON.parse(json);
}

export async function cancelTransfer(transferId: string): Promise<CommandResult> {
  return invoke("cancel_transfer", { transferId });
}

export async function sendFilesToDevice(
  filePaths: string[],
  targetDeviceId: string
): Promise<SendTransferResult> {
  return invoke("send_files_to_device", { filePaths, targetDeviceId });
}

export async function getPendingTransfers(): Promise<unknown[]> {
  const json = await invoke<string>("get_pending_transfers");
  return JSON.parse(json);
}

export async function pauseTransferCmd(transferId: string): Promise<CommandResult> {
  return invoke("pause_transfer", { transferId });
}

export async function resumeTransferCmd(transferId: string): Promise<CommandResult> {
  return invoke("resume_transfer", { transferId });
}

export async function getSettings(): Promise<Record<string, unknown>> {
  const json = await invoke<string>("get_settings");
  return JSON.parse(json);
}

export async function updateSettings(settings: Record<string, unknown>): Promise<CommandResult> {
  return invoke("update_settings", { settingsJson: JSON.stringify(settings) });
}

export async function openReceiveFolder(): Promise<CommandResult> {
  return invoke("open_receive_folder");
}

export type CloseBehavior = "minimize" | "quit" | "ask";

export async function getAutostartEnabled(): Promise<boolean> {
  return invoke("get_autostart_enabled");
}

export async function setAutostart(enabled: boolean): Promise<CommandResult> {
  return invoke("set_autostart", { enabled });
}

export async function getCloseBehavior(): Promise<CloseBehavior> {
  return invoke("get_close_behavior");
}

export async function setCloseBehavior(behavior: CloseBehavior): Promise<CommandResult> {
  return invoke("set_close_behavior", { behavior });
}

export async function quitApplication(): Promise<void> {
  return invoke("quit_application");
}

// Metadata for a user-picked file (path + display info)
export interface PickedFile {
  path: string;
  name: string;
  size: number;
  mimeType?: string;
}

export async function getFileMetadata(filePaths: string[]): Promise<PickedFile[]> {
  if (!isTauri() || filePaths.length === 0) return [];
  return invoke<PickedFile[]>("get_file_metadata", { filePaths });
}

/**
 * Open the platform file dialog, then resolve the selected paths through the
 * native backend so desktop sends always retain an absolute file path.
 */
export async function pickFiles(): Promise<PickedFile[]> {
  if (!isTauri()) return [];
  const result = await open({
    multiple: true,
    directory: false,
    title: "选择要发送的文件",
  });
  const filePaths = Array.isArray(result) ? result : result ? [result] : [];
  if (filePaths.length === 0) return [];
  return getFileMetadata(filePaths);
}

/** Open the platform folder dialog for the receive-folder setting. */
export async function pickDirectory(): Promise<string | null> {
  if (!isTauri()) return null;
  const result = await open({
    multiple: false,
    directory: true,
    title: "选择接收文件夹",
  });
  return typeof result === "string" ? result : null;
}
