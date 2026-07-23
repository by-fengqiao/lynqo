import { defineStore } from "pinia";
import { ref } from "vue";
import type { AppState, ToastKind } from "../types";
import {
  isTauri,
  startLocalService,
  stopLocalService,
  getLocalServiceStatus,
  getConnectionInfo,
  getConnectionQrCode,
  getAppVersion,
  regenerateConnectionToken,
} from "@/services/tauri";
import type { ServiceInfo, ConnectionInfo, QrCodeData } from "@/services/tauri";
import { wsClient } from "@/services/websocket";
import { fetchHostStatus } from "@/services/api";

export const useAppStore = defineStore("app", () => {
  const serverRunning = ref<AppState["serverRunning"]>(false);
  const trayReady = ref(isTauri());
  const networkName = ref<AppState["networkName"]>("本地网络");
  const localIp = ref<AppState["localIp"]>("");
  const deviceName = ref<AppState["deviceName"]>("LYNQO");
  const connectionToken = ref<AppState["connectionToken"]>("");
  const appVersion = ref("—");

  // New Tauri-backed state
  const serviceStatus = ref<ServiceInfo | null>(null);
  const connectionInfo = ref<ConnectionInfo | null>(null);
  const qrCode = ref<QrCodeData | null>(null);

  // True when the WebSocket gave up reconnecting; the UI shows a reconnect prompt.
  const connectionLost = ref(false);

  // Toast notifications (rendered by ToastHost)
  interface Toast {
    id: number;
    kind: ToastKind;
    title: string;
    description?: string;
  }
  const toasts = ref<Toast[]>([]);
  let toastId = 0;

  // Mobile device-selector sheet visibility
  const deviceSheetOpen = ref(false);

  /**
   * Initialize the app store. In Tauri mode, fetches real service status
   * and connection info from the backend. A hosted browser only receives the
   * public service status; device management remains in the desktop app.
   */
  async function initialize() {
    if (isTauri()) {
      try {
        const status = await getLocalServiceStatus();
        serviceStatus.value = status;
        serverRunning.value = status.status === "running";
      } catch (err) {
        console.error("[app] Failed to get service status:", err);
      }

      try {
        const info = await getConnectionInfo();
        connectionInfo.value = info;
        localIp.value = info.ip;
        connectionToken.value = info.token;
        if (info.deviceName) {
          deviceName.value = info.deviceName;
        }
      } catch (err) {
        console.error("[app] Failed to get connection info:", err);
      }
      try {
        appVersion.value = (await getAppVersion()).version;
      } catch (err) {
        console.error("[app] Failed to get application version:", err);
      }
      return;
    }

    try {
      const status = await fetchHostStatus();
      serverRunning.value = status.status === "running";
      localIp.value = status.localIp ?? window.location.hostname;
      networkName.value = status.networkName ?? networkName.value;
      deviceName.value = status.name ?? deviceName.value;
      appVersion.value = status.version ?? appVersion.value;
    } catch (err) {
      console.error("[app] Failed to load hosted service status:", err);
      return;
    }

  }

  /**
   * Start the local file-sharing service via Tauri backend.
   */
  async function startServer() {
    if (isTauri()) {
      try {
        const result = await startLocalService();
        if (!result.success) throw new Error(result.error ?? "服务启动失败");
        const status = await getLocalServiceStatus();
        serviceStatus.value = status;
        serverRunning.value = status.status === "running";

        // Refresh connection info after starting
        const info = await getConnectionInfo();
        connectionInfo.value = info;
        localIp.value = info.ip;
        connectionToken.value = info.token;
      } catch (err) {
        console.error("[app] Failed to start server:", err);
        pushToast("error", "启动失败", err instanceof Error ? err.message : "无法启动服务");
      }
    }
  }

  /**
   * Stop the local file-sharing service via Tauri backend.
   */
  async function stopServer() {
    if (isTauri()) {
      try {
        const result = await stopLocalService();
        if (!result.success) throw new Error(result.error ?? "服务停止失败");
        const status = await getLocalServiceStatus();
        serviceStatus.value = status;
        serverRunning.value = status.status === "running";
      } catch (err) {
        console.error("[app] Failed to stop server:", err);
        pushToast("error", "停止失败", err instanceof Error ? err.message : "无法停止服务");
      }
    }
  }

  /**
   * Toggle server on/off (used by UI toggle).
   */
  async function toggleServer() {
    if (serverRunning.value) {
      await stopServer();
    } else {
      await startServer();
    }
  }

  /**
   * Refresh the QR code data from the backend.
   */
  async function refreshQrCode() {
    if (isTauri()) {
      try {
        const data = await getConnectionQrCode();
        qrCode.value = data;
      } catch (err) {
        console.error("[app] Failed to refresh QR code:", err);
      }
    }
  }

  /**
   * Connect the WebSocket client to the local service.
   * Uses connection info (IP, port, token) to build the WS URL.
   */
  function connectWebSocket() {
    if (isTauri() && connectionInfo.value) {
      const { ip, port, controlToken } = connectionInfo.value;
      const wsUrl = `ws://${ip}:${port}/ws?token=${controlToken}`;
      wsClient.connect(wsUrl);
    }
  }

  /**
   * Surface WebSocket connection loss to the UI. When reconnect attempts are
   * exhausted we flag connectionLost and notify the user; a successful
   * reconnect clears the flag. Idempotent — safe to call once at startup.
   */
  let connectionMonitorSetup = false;
  function setupConnectionMonitor() {
    if (connectionMonitorSetup) return;
    connectionMonitorSetup = true;

    wsClient.on("reconnect_failed", () => {
      connectionLost.value = true;
      pushToast("error", "连接已断开", "无法连接到服务，实时进度已暂停。请点击重连。");
    });
    wsClient.on("connected", () => {
      if (connectionLost.value) {
        connectionLost.value = false;
        pushToast("success", "已重新连接", "实时进度已恢复。");
      }
    });
  }

  /** Manually retry the WebSocket connection (used by the reconnect prompt). */
  function manualReconnect() {
    wsClient.reconnect();
  }

  function setDeviceName(name: string) {
    deviceName.value = name;
  }

  /**
   * Regenerate the connection token. In Tauri mode the backend generates
   * the new token (so connecting devices use the real one) and the QR code
   * is refreshed to match. Browser mode falls back to a local random token.
   */
  async function regenerateToken() {
    if (isTauri()) {
      try {
        const newToken = await regenerateConnectionToken();
        connectionToken.value = newToken;
        if (connectionInfo.value) {
          connectionInfo.value = { ...connectionInfo.value, token: newToken };
        }
        await refreshQrCode();
        return;
      } catch (err) {
        console.error("[app] Failed to regenerate token via backend:", err);
      }
    }
    pushToast("info", "请使用桌面应用", "连接令牌只能在桌面应用中重新生成。");
  }

  /**
   * Show a toast notification. Auto-dismisses after 3 seconds.
   */
  function pushToast(kind: ToastKind, title: string, description?: string) {
    const id = ++toastId;
    toasts.value.push({ id, kind, title, description });
    setTimeout(() => dismissToast(id), 3000);
  }

  /**
   * Remove a toast by id (click-to-dismiss or auto timeout).
   */
  function dismissToast(id: number) {
    toasts.value = toasts.value.filter((t) => t.id !== id);
  }

  function openDeviceSheet() {
    deviceSheetOpen.value = true;
  }

  function closeDeviceSheet() {
    deviceSheetOpen.value = false;
  }

  return {
    // State
    serverRunning,
    trayReady,
    networkName,
    localIp,
    deviceName,
    connectionToken,
    appVersion,
    serviceStatus,
    connectionInfo,
    qrCode,
    toasts,
    deviceSheetOpen,
    connectionLost,
    // Actions
    initialize,
    startServer,
    stopServer,
    toggleServer,
    refreshQrCode,
    connectWebSocket,
    setupConnectionMonitor,
    manualReconnect,
    setDeviceName,
    regenerateToken,
    pushToast,
    dismissToast,
    openDeviceSheet,
    closeDeviceSheet,
  };
});
