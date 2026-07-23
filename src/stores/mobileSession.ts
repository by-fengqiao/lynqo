import { defineStore } from "pinia";
import { shallowRef } from "vue";
import {
  acceptTransfer,
  getCurrentDevice,
  getDownloadUrl,
  getPendingTransfersApi,
  registerDevice,
  rejectTransfer,
  validateToken,
} from "@/services/api";
import { wsClient } from "@/services/websocket";

export interface IncomingTransfer {
  id: string;
  sourceDeviceName: string;
  files: { id: string; name: string; size: number }[];
  totalBytes: number;
  expiresAt?: string;
}

interface StoredMobileSession {
  sessionToken: string;
  deviceId: string;
  approved: boolean;
}

interface RegistrationResponse {
  sessionToken?: string;
  deviceId?: string;
  approved?: boolean;
}

const MOBILE_CLIENT_ID_KEY = "lynqo-mobile-client-id";
let transientClientIdSequence = 0;

interface UserAgentDataLike {
  getHighEntropyValues?: (hints: string[]) => Promise<{ model?: string }>;
}

/**
 * The single source of truth for a browser-based phone connection.
 *
 * Mobile routes share one identity, one authorization state, and one incoming
 * transfer queue. Keeping this state in a Pinia store prevents route changes
 * from accidentally creating new device records or losing receive prompts.
 */
export const useMobileSessionStore = defineStore("mobileSession", () => {
  const sessionToken = shallowRef<string | null>(null);
  const deviceId = shallowRef<string | null>(null);
  const isApproved = shallowRef(false);
  const isReady = shallowRef(false);
  const connectionError = shallowRef<string | null>(null);
  const receiveError = shallowRef<string | null>(null);
  const pendingReceiveTransfer = shallowRef<IncomingTransfer | null>(null);
  const showReceiveDialog = shallowRef(false);

  let pairingToken = "";
  let storageKey = "";
  let socketToken = "";
  let approvalPollTimer: ReturnType<typeof window.setInterval> | null = null;
  let initialization: Promise<void> | null = null;
  let listenersBound = false;

  function getAndroidModelFromUserAgent(userAgent: string): string | null {
    const match = userAgent.match(
      /Android\s+[^;]+;\s*(?:[a-z]{2}-[A-Z]{2};\s*)?([^;()]+?)(?:\s+Build\/|;|\))/i
    );
    const model = match?.[1]?.trim();
    return model && model.length > 1 && model.toLowerCase() !== "wv" ? model : null;
  }

  async function getBrowserDeviceName(clientId: string): Promise<string> {
    const ua = navigator.userAgent;
    if (/Android/.test(ua)) {
      const userAgentData = (navigator as Navigator & { userAgentData?: UserAgentDataLike }).userAgentData;
      try {
        const model = (await userAgentData?.getHighEntropyValues?.(["model"]))?.model?.trim();
        if (model) return `Android · ${model}`;
      } catch {
        // Chromium may reject high-entropy hints; the regular UA remains a
        // useful fallback on Android browsers and WebViews.
      }
      const model = getAndroidModelFromUserAgent(ua);
      return model ? `Android · ${model}` : `Android · ${clientId.slice(-4).toUpperCase()}`;
    }

    // Safari deliberately hides precise iPhone/iPad generations from web
    // pages. The stable suffix still distinguishes two Apple devices without
    // pretending the browser can identify an iPhone model it cannot see.
    const suffix = clientId.slice(-4).toUpperCase();
    if (/iPhone/.test(ua)) return `iPhone · ${suffix}`;
    if (/iPad/.test(ua)) return `iPad · ${suffix}`;
    return `移动浏览器 · ${suffix}`;
  }

  function detectPlatform(): string {
    const ua = navigator.userAgent;
    if (/iPhone|iPad/.test(ua)) return "ios";
    if (/Android/.test(ua)) return "android";
    return "web";
  }

  function createClientId(): string {
    const uuid = window.crypto?.randomUUID?.();
    if (uuid) return uuid;

    const bytes = new Uint8Array(16);
    window.crypto?.getRandomValues?.(bytes);
    transientClientIdSequence += 1;
    const entropy = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
    return `mobile-${Date.now().toString(36)}-${transientClientIdSequence}-${entropy}`;
  }

  function getStableClientId(): string {
    try {
      const existing = window.localStorage.getItem(MOBILE_CLIENT_ID_KEY);
      if (existing && existing.length >= 16) return existing;

      const generated = createClientId();
      window.localStorage.setItem(MOBILE_CLIENT_ID_KEY, generated);
      return generated;
    } catch {
      // Private browser mode can disable storage. The active tab still keeps
      // one session and the server will continue to require authorization.
      return createClientId();
    }
  }

  function readStoredSession(key: string): StoredMobileSession | null {
    try {
      const raw = window.sessionStorage.getItem(key);
      if (!raw) return null;
      const value = JSON.parse(raw) as Partial<StoredMobileSession>;
      if (typeof value.sessionToken !== "string" || typeof value.deviceId !== "string") {
        return null;
      }
      return {
        sessionToken: value.sessionToken,
        deviceId: value.deviceId,
        approved: value.approved === true,
      };
    } catch {
      return null;
    }
  }

  function persistSession() {
    if (!storageKey || !sessionToken.value || !deviceId.value) return;
    try {
      window.sessionStorage.setItem(
        storageKey,
        JSON.stringify({
          sessionToken: sessionToken.value,
          deviceId: deviceId.value,
          approved: isApproved.value,
        } satisfies StoredMobileSession)
      );
    } catch {
      // The active in-memory session remains usable when storage is disabled.
    }
  }

  function stopApprovalPolling() {
    if (approvalPollTimer !== null) {
      window.clearInterval(approvalPollTimer);
      approvalPollTimer = null;
    }
  }

  async function refreshPendingReceiveTransfers() {
    const token = sessionToken.value;
    if (!token || !isApproved.value) return;

    try {
      const response = await getPendingTransfersApi(token) as {
        transfers?: Array<{
          id: string;
          sourceDeviceName?: string;
          totalBytes?: number;
          files?: { id: string; name: string; size: number }[];
          expiresAt?: string;
        }>;
      };
      const pending = response.transfers?.[0];
      if (!pending || pendingReceiveTransfer.value?.id === pending.id) return;

      pendingReceiveTransfer.value = {
        id: pending.id,
        sourceDeviceName: pending.sourceDeviceName || "未知设备",
        files: pending.files ?? [],
        totalBytes: pending.totalBytes ?? 0,
        expiresAt: pending.expiresAt,
      };
      showReceiveDialog.value = true;
    } catch (error) {
      console.warn("[mobile-session] Failed to load pending transfers:", error);
    }
  }

  function markDeviceApproved() {
    if (!sessionToken.value) return;
    isApproved.value = true;
    persistSession();
    void refreshPendingReceiveTransfers();
  }

  async function syncApprovalState(token = sessionToken.value): Promise<void> {
    if (!token || !deviceId.value) return;
    try {
      const state = await getCurrentDevice(token);
      if (state.deviceId !== deviceId.value) return;

      isApproved.value = state.approved;
      persistSession();
      if (state.approved) {
        markDeviceApproved();
      }
    } catch (error) {
      // Do not discard a cached session for a transient network failure.
      console.warn("[mobile-session] Failed to refresh authorization state:", error);
    }
  }

  function startApprovalPolling() {
    stopApprovalPolling();
    void syncApprovalState();
    approvalPollTimer = window.setInterval(() => {
      void syncApprovalState();
    }, 2500);
  }

  function handleTransferRequested(msg: { payload?: Record<string, unknown> }) {
    if (!isApproved.value) return;
    const data = msg.payload as {
      transferId?: string;
      id?: string;
      sourceDeviceName?: string;
      files?: { id: string; name: string; size: number }[];
      totalBytes?: number;
      expiresAt?: string;
    } | undefined;
    const transferId = data?.transferId ?? data?.id;
    if (!data || !transferId || pendingReceiveTransfer.value?.id === transferId) return;

    pendingReceiveTransfer.value = {
      id: transferId,
      sourceDeviceName: data.sourceDeviceName || "未知设备",
      files: data.files ?? [],
      totalBytes: data.totalBytes ?? 0,
      expiresAt: data.expiresAt,
    };
    showReceiveDialog.value = true;
  }

  function handleDeviceApproved(msg: { payload?: Record<string, unknown> }) {
    const approvedId = msg.payload?.deviceId;
    if (approvedId === deviceId.value) markDeviceApproved();
  }

  function handleDeviceRejected(msg: { payload?: Record<string, unknown> }) {
    const rejectedId = msg.payload?.deviceId;
    if (rejectedId !== deviceId.value) return;

    isApproved.value = false;
    persistSession();
  }

  function bindSocketListeners() {
    if (listenersBound) return;
    listenersBound = true;
    wsClient.on("transfer.requested", handleTransferRequested);
    wsClient.on("device.approved", handleDeviceApproved);
    wsClient.on("device.rejected", handleDeviceRejected);
  }

  function connectSocket(token: string) {
    bindSocketListeners();
    if (socketToken === token) return;
    socketToken = token;
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    wsClient.connect(`${protocol}//${window.location.host}/ws?token=${encodeURIComponent(token)}`);
  }

  function setSession(token: string, id: string, approved: boolean) {
    sessionToken.value = token;
    deviceId.value = id;
    isApproved.value = approved;
    connectionError.value = null;
    isReady.value = true;
    persistSession();
    connectSocket(token);

    startApprovalPolling();
    if (approved) {
      void refreshPendingReceiveTransfers();
      void syncApprovalState(token);
    }
  }

  async function registerCurrentBrowser(token: string): Promise<RegistrationResponse> {
    const clientId = getStableClientId();
    return registerDevice({
      name: await getBrowserDeviceName(clientId),
      platform: detectPlatform(),
      deviceType: "phone",
      userAgent: navigator.userAgent,
      clientId,
      token,
    }) as Promise<RegistrationResponse>;
  }

  function reset() {
    stopApprovalPolling();
    wsClient.disconnect();
    socketToken = "";
    sessionToken.value = null;
    deviceId.value = null;
    isApproved.value = false;
    isReady.value = false;
    pendingReceiveTransfer.value = null;
    showReceiveDialog.value = false;
  }

  async function initialize(token: string | null | undefined) {
    const nextPairingToken = token?.trim() ?? "";
    if (!nextPairingToken) {
      reset();
      connectionError.value = "连接链接无效，缺少授权参数。请重新扫描电脑上的二维码。";
      return;
    }
    if (nextPairingToken === pairingToken && isReady.value) {
      void syncApprovalState();
      return;
    }
    if (initialization) return initialization;

    initialization = (async () => {
      reset();
      pairingToken = nextPairingToken;
      storageKey = `lynqo-mobile-session:${pairingToken}`;

      const cached = readStoredSession(storageKey);
      if (cached) {
        setSession(cached.sessionToken, cached.deviceId, cached.approved);
        // Refresh the persisted name after an app upgrade. Stable client IDs
        // ensure this updates the existing phone instead of adding a duplicate.
        void registerCurrentBrowser(pairingToken)
          .then((registration) => {
            if (registration.sessionToken && registration.deviceId === cached.deviceId) {
              setSession(
                registration.sessionToken,
                registration.deviceId,
                registration.approved === true
              );
            }
          })
          .catch((error) => console.warn("[mobile-session] Failed to refresh device identity:", error));
        return;
      }

      try {
        await validateToken(pairingToken);
        const registration = await registerCurrentBrowser(pairingToken);

        if (!registration.sessionToken || !registration.deviceId) {
          throw new Error("服务器未返回有效的设备会话。");
        }
        setSession(registration.sessionToken, registration.deviceId, registration.approved === true);
      } catch (error) {
        const message = error instanceof Error ? error.message : "连接验证失败。";
        connectionError.value = `${message} 请重新扫描电脑上的二维码。`;
        console.error("[mobile-session] Token validation failed:", error);
      }
    })();

    try {
      await initialization;
    } finally {
      initialization = null;
    }
  }

  async function acceptIncomingTransfer(transferId: string) {
    const token = sessionToken.value;
    const transfer = pendingReceiveTransfer.value;
    if (!token || !deviceId.value || !transfer || transfer.id !== transferId) return;

    receiveError.value = null;
    try {
      const accepted = await acceptTransfer(transferId, token) as {
        downloadTokens?: Array<{ fileId: string; downloadToken: string }>;
      };
      const tokenByFile = new Map(
        (accepted.downloadTokens ?? []).map((item) => [item.fileId, item.downloadToken])
      );

      for (const file of transfer.files) {
        const downloadToken = tokenByFile.get(file.id);
        if (!downloadToken) {
          throw new Error(`服务器未为 ${file.name} 创建下载凭证。`);
        }
        const link = document.createElement("a");
        link.href = getDownloadUrl(transferId, file.id, downloadToken, deviceId.value);
        link.download = file.name;
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
      }

      pendingReceiveTransfer.value = null;
      showReceiveDialog.value = false;
      void refreshPendingReceiveTransfers();
    } catch (error) {
      receiveError.value = error instanceof Error ? error.message : "接收传输失败。";
      console.error("[mobile-session] Failed to accept transfer:", error);
    }
  }

  async function rejectIncomingTransfer(transferId: string) {
    const token = sessionToken.value;
    if (!token) return;

    receiveError.value = null;
    try {
      await rejectTransfer(transferId, token);
      pendingReceiveTransfer.value = null;
      showReceiveDialog.value = false;
      void refreshPendingReceiveTransfers();
    } catch (error) {
      receiveError.value = error instanceof Error ? error.message : "拒绝传输失败。";
      console.error("[mobile-session] Failed to reject transfer:", error);
    }
  }

  return {
    sessionToken,
    deviceId,
    isApproved,
    isReady,
    connectionError,
    receiveError,
    pendingReceiveTransfer,
    showReceiveDialog,
    initialize,
    syncApprovalState,
    refreshPendingReceiveTransfers,
    acceptIncomingTransfer,
    rejectIncomingTransfer,
  };
});
