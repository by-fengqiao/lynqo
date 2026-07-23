import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { TransferTask } from "../types";
import {
  isTauri,
  getTransfers,
  cancelTransfer as tauriCancelTransfer,
  sendFilesToDevice,
  pauseTransferCmd,
  resumeTransferCmd,
} from "@/services/tauri";
import type { SendTransferResult } from "@/services/tauri";
import { wsClient } from "@/services/websocket";
import { genId } from "@/utils/format";

// A file queued for sending (from drag & drop or the file picker)
export interface PendingTransferFile {
  id: string;
  name: string;
  size: number;
  path?: string;
}

export const useTransfersStore = defineStore("transfers", () => {
  const transfers = ref<TransferTask[]>([]);

  // Files queued by the user (drag & drop / picker) waiting to be sent
  const pendingFiles = ref<PendingTransferFile[]>([]);

  // Track if WebSocket listeners have been registered to prevent duplicates
  const listenersRegistered = ref(false);
  let latestFetchRequest = 0;
  let liveUpdateSequence = 0;
  let refreshTimer: ReturnType<typeof window.setTimeout> | null = null;
  const liveUpdateVersions = new Map<string, number>();

  const activeTransfers = computed(() =>
    transfers.value.filter((t) =>
      ["transferring", "verifying", "waiting", "requesting", "awaiting_acceptance", "accepted", "paused"].includes(t.status)
    )
  );

  const completedTransfers = computed(() =>
    transfers.value.filter((t) => t.status === "completed")
  );

  function queueTransferRefresh() {
    if (refreshTimer !== null) return;
    refreshTimer = window.setTimeout(() => {
      refreshTimer = null;
      void fetchTransfers();
    }, 100);
  }

  function applyLiveUpdate(id: string, patch: Partial<TransferTask>): boolean {
    const task = transfers.value.find((transfer) => transfer.id === id);
    if (!task) {
      queueTransferRefresh();
      return false;
    }
    Object.assign(task, patch);
    liveUpdateVersions.set(id, ++liveUpdateSequence);
    return true;
  }

  function reconcileTransfers(snapshot: TransferTask[], snapshotSequence: number) {
    const currentById = new Map(transfers.value.map((transfer) => [transfer.id, transfer]));
    transfers.value = snapshot.map((transfer) => {
      const current = currentById.get(transfer.id);
      const liveVersion = liveUpdateVersions.get(transfer.id) ?? 0;
      if (!current || liveVersion <= snapshotSequence) return transfer;

      return {
        ...transfer,
        status: current.status,
        progress: current.progress,
        transferredBytes: current.transferredBytes,
        speedBytesPerSecond: current.speedBytesPerSecond,
        remainingSeconds: current.remainingSeconds,
        completedAt: current.completedAt ?? transfer.completedAt,
        savePath: current.savePath ?? transfer.savePath,
        error: current.error ?? transfer.error,
        relayStage: current.relayStage ?? transfer.relayStage,
        acceptedAt: current.acceptedAt ?? transfer.acceptedAt,
        pausedAt: current.pausedAt ?? transfer.pausedAt,
        files: current.files,
      };
    });
  }

  /**
   * Fetch host-wide transfer history only from the desktop shell.
   */
  async function fetchTransfers() {
    if (isTauri()) {
      const requestId = ++latestFetchRequest;
      const snapshotSequence = liveUpdateSequence;
      try {
        const result = await getTransfers();
        if (requestId === latestFetchRequest) {
          reconcileTransfers(result as TransferTask[], snapshotSequence);
        }
      } catch (err) {
        console.error("[transfers] Failed to fetch transfers:", err);
      }
      return;
    }

    transfers.value = [];
  }

  /**
   * Cancel a transfer via the Tauri backend.
   */
  async function cancelTransfer(id: string) {
    if (isTauri()) {
      try {
        const result = await tauriCancelTransfer(id);
        if (!result.success) throw new Error(result.error ?? "取消传输失败");
      } catch (err) {
        console.error("[transfers] Failed to cancel transfer:", err);
        return;
      }
    }
    // Update local state
    const task = transfers.value.find((t) => t.id === id);
    if (task) {
      task.status = "cancelled";
      task.speedBytesPerSecond = 0;
    }
  }

  async function pauseTransfer(id: string) {
    if (isTauri()) {
      try {
        const result = await pauseTransferCmd(id);
        if (!result.success) throw new Error(result.error ?? "暂停传输失败");
      } catch (err) {
        console.error("[transfers] Failed to pause transfer:", err);
        return;
      }
    }
    const task = transfers.value.find((t) => t.id === id);
    if (task && task.status === "transferring") {
      task.status = "paused";
      task.speedBytesPerSecond = 0;
      task.pausedAt = new Date().toISOString();
    }
  }

  async function resumeTransfer(id: string) {
    if (isTauri()) {
      try {
        const result = await resumeTransferCmd(id);
        if (!result.success) throw new Error(result.error ?? "恢复传输失败");
      } catch (err) {
        console.error("[transfers] Failed to resume transfer:", err);
        return;
      }
    }
    const task = transfers.value.find((t) => t.id === id);
    if (task && task.status === "paused") {
      task.status = "transferring";
      task.pausedAt = undefined;
    }
  }

  /**
   * Retry a failed transfer. Calls backend resume_transfer to re-attempt.
   */
  async function retryTransfer(id: string) {
    const task = transfers.value.find((t) => t.id === id);
    if (!task || task.status !== "failed") return;

    task.status = "transferring";
    task.error = undefined;
    task.progress = 0;
    task.transferredBytes = 0;
    task.speedBytesPerSecond = 0;
    task.remainingSeconds = undefined;
    task.retryCount = (task.retryCount ?? 0) + 1;

    if (isTauri()) {
      try {
        const result = await resumeTransferCmd(id);
        if (!result.success) throw new Error(result.error ?? "重试传输失败");
      } catch (err) {
        console.error("[transfers] retry failed:", err);
        task.status = "failed";
        task.error = err instanceof Error ? err.message : String(err);
      }
    }
  }

  /**
   * Send files to a device via the Tauri backend (desktop flow).
   */
  async function sendFiles(
    filePaths: string[],
    targetDeviceId: string
  ): Promise<SendTransferResult> {
    if (!isTauri()) {
      throw new Error("请使用桌面应用发送本机文件。");
    }
    try {
      const result = await sendFilesToDevice(filePaths, targetDeviceId);
      if (!result.success) {
        throw new Error(result.error ?? "未能创建发送请求。");
      }
      return result;
    } catch (err) {
      console.error("[transfers] Failed to send files:", err);
      throw err;
    }
  }

  /**
   * Queue files (from drag & drop or the file picker) as pending sends.
   */
  function addPendingFiles(
    files: Array<{ name: string; size: number; path?: string }>
  ) {
    pendingFiles.value.push(
      ...files.map((f) => ({
        id: genId("pf"),
        name: f.name,
        size: f.size,
        path: f.path,
      }))
    );
  }

  function removePendingFile(id: string) {
    pendingFiles.value = pendingFiles.value.filter((file) => file.id !== id);
  }

  function clearPendingFiles() {
    pendingFiles.value = [];
  }

  /**
   * Register WebSocket event listeners for real-time transfer updates.
   */
  function setupWebSocketListeners() {
    if (listenersRegistered.value) return;
    listenersRegistered.value = true;

    wsClient.on("transfer.created", () => {
      // The event intentionally contains only a summary. Reload the canonical
      // transfer DTO instead of inserting a partially shaped task.
      queueTransferRefresh();
    });

    // A reconnect can miss events while the desktop socket is down. Reconcile
    // the canonical list as soon as the control channel is back online.
    wsClient.on("connected", () => {
      void fetchTransfers();
    });

    wsClient.on("transfer.started", (msg) => {
      const data = msg.payload as { transferId: string };
      applyLiveUpdate(data.transferId, { status: "transferring" });
    });

    wsClient.on("transfer.progress", (msg) => {
      const data = msg.payload as {
        transferId: string;
        progress: number;
        transferredBytes: number;
        speedBytesPerSecond: number;
        remainingSeconds?: number;
      };
      applyLiveUpdate(data.transferId, {
        status: "transferring",
        progress: data.progress / 100,
        transferredBytes: data.transferredBytes,
        speedBytesPerSecond: data.speedBytesPerSecond,
        ...(data.remainingSeconds !== undefined ? { remainingSeconds: data.remainingSeconds } : {}),
      });
    });

    wsClient.on("transfer.verifying", (msg) => {
      const data = msg.payload as { transferId: string };
      applyLiveUpdate(data.transferId, {
        status: "verifying",
        speedBytesPerSecond: 0,
        remainingSeconds: 0,
      });
    });

    wsClient.on("transfer.checksum_ready", (msg) => {
      const data = msg.payload as {
        transferId: string;
        fileId: string;
        checksum: string;
      };
      const task = transfers.value.find((transfer) => transfer.id === data.transferId);
      const file = task?.files.find((entry) => entry.id === data.fileId);
      if (!task || !file) {
        queueTransferRefresh();
        return;
      }
      file.checksum = data.checksum;
      liveUpdateVersions.set(data.transferId, ++liveUpdateSequence);
    });

    wsClient.on("transfer.completed", (msg) => {
      const data = msg.payload as { transferId: string };
      applyLiveUpdate(data.transferId, {
        status: "completed",
        progress: 1,
        speedBytesPerSecond: 0,
        remainingSeconds: 0,
        completedAt: new Date().toISOString(),
      });
      queueTransferRefresh();
    });

    wsClient.on("transfer.cancelled", (msg) => {
      const data = msg.payload as { transferId: string };
      applyLiveUpdate(data.transferId, { status: "cancelled", speedBytesPerSecond: 0 });
      queueTransferRefresh();
    });

    wsClient.on("transfer.failed", (msg) => {
      const data = msg.payload as { transferId: string; error?: string };
      applyLiveUpdate(data.transferId, {
        status: "failed",
        speedBytesPerSecond: 0,
        error: data.error,
      });
      queueTransferRefresh();
    });

    // Phase 3: Bidirectional transfer events
    wsClient.on("transfer.requested", () => {
      queueTransferRefresh();
    });

    wsClient.on("transfer.accepted", (msg) => {
      const data = msg.payload as { transferId: string; acceptedAt?: string };
      applyLiveUpdate(data.transferId, {
        status: "accepted",
        acceptedAt: data.acceptedAt ?? new Date().toISOString(),
      });
      queueTransferRefresh();
    });

    wsClient.on("transfer.rejected", (msg) => {
      const data = msg.payload as { transferId: string };
      applyLiveUpdate(data.transferId, { status: "rejected", speedBytesPerSecond: 0 });
      queueTransferRefresh();
    });

    wsClient.on("transfer.paused", (msg) => {
      const data = msg.payload as { transferId: string; pausedAt?: string };
      applyLiveUpdate(data.transferId, {
        status: "paused",
        speedBytesPerSecond: 0,
        pausedAt: data.pausedAt ?? new Date().toISOString(),
      });
      queueTransferRefresh();
    });

    wsClient.on("transfer.resumed", (msg) => {
      const data = msg.payload as { transferId: string };
      applyLiveUpdate(data.transferId, { status: "transferring", pausedAt: undefined });
      queueTransferRefresh();
    });

    wsClient.on("transfer.download_started", (msg) => {
      const data = msg.payload as { transferId: string };
      applyLiveUpdate(data.transferId, { status: "transferring" });
    });

    wsClient.on("transfer.download_progress", (msg) => {
      const data = msg.payload as {
        transferId: string;
        progress: number;
        transferredBytes: number;
        speedBytesPerSecond: number;
        remainingSeconds?: number;
      };
      applyLiveUpdate(data.transferId, {
        status: "transferring",
        progress: data.progress / 100,
        transferredBytes: data.transferredBytes,
        speedBytesPerSecond: data.speedBytesPerSecond,
        ...(data.remainingSeconds !== undefined ? { remainingSeconds: data.remainingSeconds } : {}),
      });
    });

    wsClient.on("transfer.relay_stage_changed", (msg) => {
      const data = msg.payload as {
        transferId: string;
        relayStage: TransferTask["relayStage"];
      };
      applyLiveUpdate(data.transferId, { relayStage: data.relayStage });
      queueTransferRefresh();
    });

    wsClient.on("transfer.expired", (msg) => {
      const data = msg.payload as { transferId: string };
      applyLiveUpdate(data.transferId, { status: "expired", speedBytesPerSecond: 0 });
      queueTransferRefresh();
    });
  }

  return {
    // State
    transfers,
    pendingFiles,
    listenersRegistered,
    // Computed
    activeTransfers,
    completedTransfers,
    // Actions
    fetchTransfers,
    cancelTransfer,
    pauseTransfer,
    resumeTransfer,
    retryTransfer,
    sendFiles,
    addPendingFiles,
    removePendingFile,
    clearPendingFiles,
    setupWebSocketListeners,
  };
});
