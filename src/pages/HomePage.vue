<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, inject } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  Radar,
  UploadCloud,
  Check,
  X,
  Eye,
  FileText,
  FileImage,
  FileArchive,
  Smartphone,
  Laptop,
  Monitor,
  Tablet,
  Loader,
} from "lucide-vue-next";
import { useDevicesStore } from "@/stores/devices";
import { useAppStore } from "@/stores/app";
import { useSettingsStore } from "@/stores/settings";
import { useTransfersStore } from "@/stores/transfers";
import { getFileMetadata, isTauri, pickFiles } from "@/services/tauri";
import { formatRelativeTime } from "@/utils/format";
import { openConnectPanelKey } from "@/composables/useConnectPanel";
import type { Device } from "@/types";
import type { PendingTransferFile } from "@/stores/transfers";

const devicesStore = useDevicesStore();
const appStore = useAppStore();
const settingsStore = useSettingsStore();
const transfersStore = useTransfersStore();

// Opens the connect panel hosted by DesktopLayout (查看连接地址)
const openConnectPanel = inject(openConnectPanelKey, () => {});

const selectedDevice = computed(() => devicesStore.selectedDevice);

let unlistenNativeDrop: UnlistenFn | null = null;

onMounted(async () => {
  void devicesStore.fetchDevices();
  if (!isTauri()) return;

  unlistenNativeDrop = await getCurrentWindow().onDragDropEvent((event) => {
    if (event.payload.type === "enter" || event.payload.type === "over") {
      isDragging.value = true;
    } else if (event.payload.type === "leave") {
      isDragging.value = false;
    } else {
      isDragging.value = false;
      void addPendingFilesFromPaths(event.payload.paths);
    }
  });
});

onUnmounted(() => {
  unlistenNativeDrop?.();
});

// Pending files - in Tauri mode these hold real file paths
const pendingFiles = computed(() => transfersStore.pendingFiles);

// Send state
const sendStatus = ref<"idle" | "waiting" | "sending" | "requested" | "error">("idle");
const sendError = ref<string | null>(null);

// Drag state
const isDragging = ref(false);

// Display row shape for the 最近传输 list
interface RecentTransferRow {
  id: string;
  fileName: string;
  route: string;
  time: string;
  size: string;
  status: string;
}

function peerName(id: string): string {
  if (id === "local") return "本机";
  return devicesStore.devices.find((d) => d.id === id)?.name ?? id;
}

function statusLabel(status: string): string {
  const map: Record<string, string> = {
    waiting: "等待中",
    requesting: "请求中",
    awaiting_acceptance: "等待接收",
    accepted: "已接受",
    transferring: "传输中",
    paused: "已暂停",
    verifying: "校验中",
    completed: "已完成",
    rejected: "已拒绝",
    expired: "已过期",
    cancelled: "已取消",
    failed: "失败",
  };
  return map[status] ?? status;
}

function statusBadgeClass(status: string): string {
  if (["completed", "transferring", "accepted"].includes(status))
    return "badge--success";
  if (["failed", "rejected", "cancelled"].includes(status))
    return "badge--error";
  if (["paused", "verifying", "awaiting_acceptance"].includes(status))
    return "badge--warning";
  return "badge--neutral";
}

// Recent transfers always come from the real transfer store.
const recentTransfers = computed<RecentTransferRow[]>(() => {
  const latest = transfersStore.transfers
    .slice()
    .sort(
      (a, b) =>
        new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()
    )
    .slice(0, 5);

  if (latest.length > 0) {
    return latest.map((t) => ({
      id: t.id,
      fileName: t.files[0]?.name ?? "未命名文件",
      route: `${peerName(t.sourceDeviceId)} → ${peerName(t.targetDeviceId)}`,
      time: formatRelativeTime(t.completedAt ?? t.createdAt),
      size: formatSize(t.totalBytes),
      status: t.status,
    }));
  }
  return [];
});

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024)
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function getDeviceIcon(device: Device) {
  switch (device.deviceType) {
    case "phone":
      return Smartphone;
    case "laptop":
      return Laptop;
    case "tablet":
      return Tablet;
    default:
      return Monitor;
  }
}

function getFileIcon(name: string) {
  if (/\.(png|jpg|jpeg|gif|svg|webp|fig)$/i.test(name)) return FileImage;
  if (/\.(zip|rar|7z|tar|gz)$/i.test(name)) return FileArchive;
  return FileText;
}

function selectDevice(device: Device) {
  if (!device.approved) {
    appStore.pushToast(
      "warning",
      "设备尚未授权",
      `请先在“设备”页面批准 ${device.name}，再发送文件。`
    );
    return;
  }
  if (!device.online) {
    appStore.pushToast(
      "warning",
      "设备离线",
      `${device.name} 当前不在线，无法作为发送目标`
    );
    return;
  }
  devicesStore.selectDevice(device.id);
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message.trim()) return error.message;
  if (typeof error === "string" && error.trim()) return error;
  if (typeof error === "object" && error !== null) {
    const value = error as { message?: unknown; error?: unknown };
    if (typeof value.message === "string" && value.message.trim()) return value.message;
    if (typeof value.error === "string" && value.error.trim()) return value.error;
  }
  return "未能发出文件请求，请确认手机保持连接后重试。";
}

function handlePreview(file: PendingTransferFile) {
  appStore.pushToast("info", "预览功能开发中", file.name);
}

function removePendingFile(id: string) {
  transfersStore.removePendingFile(id);
}

function onDragOver(e: DragEvent) {
  e.preventDefault();
  isDragging.value = true;
}

function onDragLeave(e: DragEvent) {
  e.preventDefault();
  isDragging.value = false;
}

function onDrop(e: DragEvent) {
  e.preventDefault();
  isDragging.value = false;
  // Native windows provide absolute paths through onDragDropEvent above.
  if (isTauri()) return;
  if (e.dataTransfer?.files) {
    addPendingFiles(Array.from(e.dataTransfer.files));
  }
}

// Hidden file input ref
const fileInputRef = ref<HTMLInputElement | null>(null);

async function openFilePicker() {
  if (isTauri()) {
    addPendingFiles(await pickFiles());
    return;
  }
  fileInputRef.value?.click();
}

function onFileInputChange(e: Event) {
  const input = e.target as HTMLInputElement;
  if (input.files && input.files.length > 0) {
    addPendingFiles(Array.from(input.files));
  }
  // Reset input so the same file can be selected again
  input.value = "";
}

/**
 * Reject files larger than the configured maxFileSize (when the limit is > 0),
 * warning the user about each rejected file. Returns PendingFile entries for
 * the accepted files only.
 */
type PendingFileInput = { name: string; size: number; path?: string };

function addPendingFiles(files: PendingFileInput[]) {
  transfersStore.addPendingFiles(filterOversized(files));
}

async function addPendingFilesFromPaths(paths: string[]) {
  try {
    addPendingFiles(await getFileMetadata(paths));
  } catch (error) {
    console.error("[home] Failed to read dropped files:", error);
    appStore.pushToast("error", "无法读取文件", "请确认文件仍然存在且可访问。");
  }
}

function filterOversized(files: PendingFileInput[]): PendingFileInput[] {
  const limit = settingsStore.maxFileSize;
  const accepted: PendingFileInput[] = [];
  const rejected: string[] = [];
  files.forEach((file) => {
    if (limit > 0 && file.size > limit) {
      rejected.push(file.name);
      return;
    }
    accepted.push(file);
  });
  if (rejected.length > 0) {
    appStore.pushToast(
      "warning",
      "文件过大",
      `已跳过 ${rejected.length} 个超过大小限制的文件：${rejected.join("、")}`
    );
  }
  return accepted;
}

async function handleSend() {
  const targetDevice = selectedDevice.value;
  if (pendingFiles.value.length === 0 || !targetDevice) return;

  sendStatus.value = "waiting";
  sendError.value = null;

  try {
    if (!targetDevice.approved) {
      throw new Error(`请先在“设备”页面批准 ${targetDevice.name}。`);
    }
    if (!targetDevice.online) {
      throw new Error(`${targetDevice.name} 已离线，请保持手机页面打开后重试。`);
    }

    // In Tauri mode, use real file paths
    if (isTauri()) {
      const filePaths = pendingFiles.value
        .map((f) => f.path)
        .filter((p): p is string => p != null);

      if (filePaths.length === 0) {
        throw new Error("没有可发送的文件路径");
      }

      sendStatus.value = "sending";
      await transfersStore.sendFiles(filePaths, targetDevice.id);
      sendStatus.value = "requested";

      // The phone downloads only after its user accepts this invitation.
      transfersStore.clearPendingFiles();
      void transfersStore.fetchTransfers();
      appStore.pushToast(
        "success",
        "已发出接收请求",
        `请在 ${targetDevice.name} 上确认下载文件。`
      );

      // The request is now visible in the transfer list; reset the composer.
      setTimeout(() => {
        sendStatus.value = "idle";
      }, 3000);
    } else {
      throw new Error("浏览器控制页不能读取本机文件路径，请使用桌面应用发送文件。");
    }
  } catch (err: unknown) {
    sendStatus.value = "idle";
    sendError.value = getErrorMessage(err);
    appStore.pushToast("error", "未能发出文件", sendError.value);
  }
}

const sendBtnLabel = computed(() => {
  switch (sendStatus.value) {
    case "waiting":
      return "等待接收...";
    case "sending":
      return "正在发送...";
    case "requested":
      return "已发出请求";
    default:
      return `发送 ${pendingFiles.value.length} 个文件`;
  }
});
</script>

<template>
  <div class="home-page">
    <!-- Title Section -->
    <header class="page-header">
      <div class="header-left">
        <h1 class="page-title">发送文件</h1>
        <p class="page-subtitle">选择附近设备，或将文件拖入页面开始传输。</p>
      </div>
      <a class="header-link" href="#" @click.prevent="openConnectPanel">查看连接地址</a>
    </header>

    <!-- Content Card -->
    <div class="content-card">
      <!-- Nearby Devices -->
      <section class="devices-section">
        <div class="section-header">
          <Radar :size="16" class="section-icon" />
          <span class="section-title">附近设备</span>
        </div>

        <div class="device-table">
          <div class="device-row device-row--header">
            <span></span>
            <span>设备名称</span>
            <span>平台</span>
            <span>状态</span>
            <span>延迟</span>
          </div>
          <div
            v-for="device in devicesStore.devices"
            :key="device.id"
            class="device-row"
            :class="{
              'device-row--selected': devicesStore.selectedDeviceId === device.id,
              'device-row--offline': !device.online,
              'device-row--unapproved': !device.approved,
            }"
            @click="selectDevice(device)"
          >
            <span class="device-check">
              <Check
                v-if="devicesStore.selectedDeviceId === device.id"
                :size="14"
              />
            </span>
            <span class="device-name">
              <component :is="getDeviceIcon(device)" :size="15" class="device-type-icon" />
              {{ device.name }}
            </span>
            <span class="device-platform">{{ device.platform }}</span>
            <span class="device-status">
              <span
                class="status-dot"
                :class="device.online && device.approved ? 'status-dot--online' : 'status-dot--offline'"
              ></span>
              {{ !device.approved ? "未授权" : device.online ? "在线" : "离线" }}
            </span>
            <span class="device-latency">
              {{ device.latencyMs != null ? `${device.latencyMs}ms` : "—" }}
            </span>
          </div>
        </div>
      </section>

      <!-- File Drop Zone -->
      <section
        class="drop-zone"
        :class="{ 'drop-zone--active': isDragging }"
        @dragover="onDragOver"
        @dragleave="onDragLeave"
        @drop="onDrop"
      >
        <UploadCloud :size="32" class="drop-zone-icon" />
        <p class="drop-zone-text">
          拖入文件发送到
          <strong>{{ selectedDevice?.name ?? "未选择设备" }}</strong>
        </p>
        <p class="drop-zone-hint">支持照片、视频、文档、压缩包和文件夹。</p>
        <button class="drop-zone-link" @click="openFilePicker">或点击选择文件</button>
      </section>

      <!-- File Tray -->
      <section v-if="pendingFiles.length > 0" class="file-tray">
        <div class="file-tray-header">
          <span class="file-tray-title">待发送文件</span>
          <span class="file-tray-count">{{ pendingFiles.length }}</span>
        </div>
        <div class="file-list">
          <div v-for="file in pendingFiles" :key="file.id" class="file-item">
            <component :is="getFileIcon(file.name)" :size="16" class="file-icon" />
            <span class="file-name">{{ file.name }}</span>
            <span class="file-size">{{ formatSize(file.size) }}</span>
            <button class="file-action" title="预览" @click="handlePreview(file)">
              <Eye :size="14" />
            </button>
            <button class="file-action file-action--remove" title="移除" @click="removePendingFile(file.id)">
              <X :size="14" />
            </button>
          </div>
        </div>
        <p v-if="sendError" class="send-error">{{ sendError }}</p>
        <button
          class="send-btn"
          :class="{
            'send-btn--sending': sendStatus === 'waiting' || sendStatus === 'sending',
            'send-btn--done': sendStatus === 'requested',
          }"
          :disabled="sendStatus !== 'idle' || !selectedDevice || !selectedDevice.approved"
          @click="handleSend"
        >
          <Loader v-if="sendStatus === 'waiting' || sendStatus === 'sending'" :size="14" class="spin" />
          {{ sendBtnLabel }}
        </button>
      </section>

      <!-- Divider -->
      <hr class="divider" />

      <!-- Recent Transfers -->
      <section class="recent-section">
        <div class="section-header">
          <span class="section-title">最近传输</span>
        </div>
        <div class="recent-table">
          <div class="recent-row recent-row--header">
            <span>文件名</span>
            <span>来源/目标</span>
            <span>时间</span>
            <span>大小</span>
            <span>状态</span>
          </div>
          <div v-for="item in recentTransfers" :key="item.id" class="recent-row">
            <span class="recent-filename">{{ item.fileName }}</span>
            <span class="recent-target">{{ item.route }}</span>
            <span class="recent-time">{{ item.time }}</span>
            <span class="recent-size">{{ item.size }}</span>
            <span class="recent-status">
              <span class="badge" :class="statusBadgeClass(item.status)">
                {{ statusLabel(item.status) }}
              </span>
            </span>
          </div>
          <div v-if="recentTransfers.length === 0" class="recent-empty">
            暂无传输记录
          </div>
        </div>
      </section>
    </div>

    <!-- Hidden file input for file selection -->
    <input
      ref="fileInputRef"
      type="file"
      multiple
      style="display: none"
      @change="onFileInputChange"
    />
  </div>
</template>

<style scoped>
.home-page {
  padding: 32px;
  max-width: var(--content-max-width);
  margin: 0 auto;
}

.page-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: 24px;
}

.page-title {
  font-size: var(--text-2xl);
  font-weight: var(--weight-semibold);
  color: var(--color-text-primary);
  margin: 0 0 4px;
  line-height: var(--leading-tight);
}

.page-subtitle {
  font-size: var(--text-base);
  color: var(--color-text-secondary);
  margin: 0;
}

.header-link {
  font-size: var(--text-sm);
  color: var(--color-text-brand);
  text-decoration: none;
  white-space: nowrap;
  margin-top: 4px;
}

.header-link:hover {
  text-decoration: underline;
}

.content-card {
  background: var(--color-surface-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-card);
  padding: 24px;
}

/* Devices Section */
.section-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}

.section-icon {
  color: var(--color-brand-primary);
}

.section-title {
  font-size: var(--text-base);
  font-weight: var(--weight-semibold);
  color: var(--color-text-primary);
}

.device-table {
  margin-bottom: 24px;
}

.device-row {
  display: grid;
  grid-template-columns: 28px 1fr 72px 72px 56px;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background var(--transition-fast);
  border-left: 3px solid transparent;
}

.device-row:hover {
  background: var(--color-hover);
}

.device-row--header {
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
  color: var(--color-text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.02em;
  cursor: default;
  border-left: none;
}

.device-row--header:hover {
  background: transparent;
}

.device-row--selected {
  background: var(--color-selected);
  border-left-color: var(--color-brand-primary);
}

.device-row--offline {
  opacity: 0.55;
  cursor: not-allowed;
}

.device-row--unapproved {
  opacity: 0.7;
  cursor: not-allowed;
}

.device-check {
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-brand-primary);
}

.device-name {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: var(--text-base);
  color: var(--color-text-primary);
  font-weight: var(--weight-medium);
}

.device-type-icon {
  color: var(--color-text-tertiary);
  flex-shrink: 0;
}

.device-platform {
  font-size: var(--text-sm);
  color: var(--color-text-secondary);
  text-transform: capitalize;
}

.device-status {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: var(--text-sm);
  color: var(--color-text-secondary);
}

.status-dot {
  width: 7px;
  height: 7px;
  border-radius: var(--radius-full);
}

.status-dot--online {
  background: var(--color-state-success);
}

.status-dot--offline {
  background: var(--color-text-tertiary);
}

.device-latency {
  font-size: var(--text-sm);
  color: var(--color-text-secondary);
  font-family: var(--font-mono);
}

/* Drop Zone */
.drop-zone {
  border: 2px dashed var(--color-border-strong);
  border-radius: var(--radius-lg);
  padding: 40px 24px;
  text-align: center;
  transition: all var(--transition-normal);
  margin-bottom: 24px;
}

.drop-zone--active {
  border-color: var(--color-brand-primary);
  background: var(--color-selected);
}

.drop-zone-icon {
  color: var(--color-text-tertiary);
  margin-bottom: 12px;
}

.drop-zone--active .drop-zone-icon {
  color: var(--color-brand-primary);
}

.drop-zone-text {
  font-size: var(--text-base);
  color: var(--color-text-primary);
  margin: 0 0 4px;
}

.drop-zone-hint {
  font-size: var(--text-sm);
  color: var(--color-text-tertiary);
  margin: 0 0 12px;
}

.drop-zone-link {
  font-size: var(--text-sm);
  color: var(--color-text-brand);
  text-decoration: none;
  background: none;
  border: none;
  cursor: pointer;
  padding: 0;
}

.drop-zone-link:hover {
  text-decoration: underline;
}

/* File Tray */
.file-tray {
  margin-bottom: 24px;
}

.file-tray-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}

.file-tray-title {
  font-size: var(--text-base);
  font-weight: var(--weight-semibold);
  color: var(--color-text-primary);
}

.file-tray-count {
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
  color: var(--color-text-inverse);
  background: var(--color-brand-primary);
  border-radius: var(--radius-full);
  padding: 1px 7px;
  line-height: 1.5;
}

.file-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 16px;
}

.file-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-radius: var(--radius-md);
  background: var(--color-surface-inset);
}

.file-icon {
  color: var(--color-text-tertiary);
  flex-shrink: 0;
}

.file-name {
  flex: 1;
  font-size: var(--text-sm);
  color: var(--color-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-size {
  font-size: var(--text-xs);
  color: var(--color-text-tertiary);
  font-family: var(--font-mono);
  white-space: nowrap;
}

.file-action {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  border-radius: var(--radius-sm);
  color: var(--color-text-tertiary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.file-action:hover {
  background: var(--color-hover);
  color: var(--color-text-primary);
}

.file-action--remove:hover {
  color: var(--color-state-error);
  background: var(--color-state-error-soft);
}

.send-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  width: 100%;
  padding: 10px 16px;
  border: none;
  border-radius: var(--radius-md);
  background: var(--color-brand-primary);
  color: var(--color-text-inverse);
  font-size: var(--text-base);
  font-weight: var(--weight-medium);
  cursor: pointer;
  transition: background var(--transition-fast);
}

.send-btn:hover {
  background: var(--color-brand-primary-hover);
}

.send-btn:active {
  background: var(--color-brand-primary-active);
}

.send-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.send-btn--sending {
  background: var(--color-state-warning);
}

.send-btn--done {
  background: var(--color-state-success);
}

.send-btn .spin {
  animation: spin 1s linear infinite;
}

.send-error {
  margin: 0 0 10px;
  font-size: var(--text-sm);
  color: var(--color-state-error);
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* Divider */
.divider {
  border: none;
  border-top: 1px solid var(--color-border);
  margin: 24px 0;
}

/* Recent Transfers */
.recent-table {
  font-size: var(--text-sm);
}

.recent-row {
  display: grid;
  grid-template-columns: 1.5fr 1.2fr 72px 72px 72px;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-radius: var(--radius-md);
}

.recent-row:hover {
  background: var(--color-hover);
}

.recent-row--header {
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
  color: var(--color-text-tertiary);
}

.recent-row--header:hover {
  background: transparent;
}

.recent-filename {
  color: var(--color-text-primary);
  font-weight: var(--weight-medium);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.recent-target {
  color: var(--color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.recent-time,
.recent-size {
  color: var(--color-text-tertiary);
  font-family: var(--font-mono);
  font-size: var(--text-xs);
}

.badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  border-radius: var(--radius-full);
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
}

.badge--success {
  background: var(--color-state-success-soft);
  color: var(--color-state-success);
}

.badge--error {
  background: var(--color-state-error-soft);
  color: var(--color-state-error);
}

.badge--warning {
  background: var(--color-state-warning-soft);
  color: var(--color-state-warning);
}

.badge--neutral {
  background: var(--color-surface-inset);
  color: var(--color-text-secondary);
}

.recent-empty {
  padding: 20px 12px;
  text-align: center;
  font-size: var(--text-sm);
  color: var(--color-text-tertiary);
}
</style>
