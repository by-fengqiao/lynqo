<script setup lang="ts">
import { computed, onMounted, onUnmounted, shallowRef } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  FolderOpen,
  FileText,
  FileImage,
  FileVideo,
  FileArchive,
  Pause,
  X,
  ChevronDown,
  ChevronRight,
  ArrowDownToLine,
  ArrowUpFromLine,
  RefreshCw,
} from "lucide-vue-next";
import { useTransfersStore } from "@/stores/transfers";
import { useDevicesStore } from "@/stores/devices";
import { useSettingsStore } from "@/stores/settings";
import { useAppStore } from "@/stores/app";
import { isTauri, openReceiveFolder } from "@/services/tauri";
import type { TransferTask } from "@/types";
import TransferCenterFilterBar, { type TransferCenterFilter } from "@/components/transfers/TransferCenterFilterBar.vue";

const transfersStore = useTransfersStore();
const devicesStore = useDevicesStore();
const settingsStore = useSettingsStore();
const appStore = useAppStore();
const route = useRoute();
const router = useRouter();

const expandedId = shallowRef<string | null>(null);
const now = shallowRef(Date.now());
let elapsedTimer: ReturnType<typeof window.setInterval> | null = null;

const attentionStatuses = new Set(["paused", "failed", "awaiting_acceptance"]);

function parseFilter(value: unknown): TransferCenterFilter {
  if (value === "active" || value === "completed" || value === "attention") {
    return value;
  }
  return "all";
}

const selectedFilter = computed(() => parseFilter(route.query.filter));

const filteredTransfers = computed(() => {
  switch (selectedFilter.value) {
    case "active":
      return transfersStore.activeTransfers;
    case "completed":
      return transfersStore.completedTransfers;
    case "attention":
      return transfersStore.transfers.filter((task) => attentionStatuses.has(task.status));
    default:
      return transfersStore.transfers;
  }
});

const filterCounts = computed<Record<TransferCenterFilter, number>>(() => ({
  all: transfersStore.transfers.length,
  active: transfersStore.activeTransfers.length,
  completed: transfersStore.completedTransfers.length,
  attention: transfersStore.transfers.filter((task) => attentionStatuses.has(task.status)).length,
}));

const emptyStateText = computed(() => {
  const labels: Record<TransferCenterFilter, string> = {
    all: "暂无传输记录",
    active: "暂无进行中的传输",
    completed: "暂无已完成的传输",
    attention: "暂无需要处理的传输",
  };
  return labels[selectedFilter.value];
});

const activeCount = computed(
  () => transfersStore.activeTransfers.length
);

const totalSpeed = computed(() => {
  const total = transfersStore.transfers.reduce(
    (sum, t) => sum + t.speedBytesPerSecond,
    0
  );
  return formatSpeed(total);
});

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024)
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatSpeed(bytesPerSec: number): string {
  if (bytesPerSec <= 0) return "—";
  if (bytesPerSec < 1024 * 1024)
    return `${(bytesPerSec / 1024).toFixed(0)} KB/s`;
  return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`;
}

function formatRemaining(seconds?: number): string {
  if (seconds == null || seconds <= 0) return "—";
  if (seconds < 60) return `${seconds}s`;
  const min = Math.floor(seconds / 60);
  const sec = seconds % 60;
  return `${min}m ${sec}s`;
}

function parseTimestamp(value: string | undefined): number | null {
  if (!value) return null;
  const parsed = Date.parse(value);
  if (!Number.isNaN(parsed)) return parsed;
  const seconds = Number(value);
  return Number.isFinite(seconds) ? seconds * 1_000 : null;
}

function formatElapsed(task: TransferTask): string {
  const start = parseTimestamp(task.createdAt);
  const end = task.completedAt ? parseTimestamp(task.completedAt) : now.value;
  if (!start || !end || end < start) return "—";
  const elapsed = Math.floor((end - start) / 1_000);
  const minutes = Math.floor(elapsed / 60);
  const seconds = elapsed % 60;
  return minutes > 0 ? `${minutes}m ${seconds}s` : `${seconds}s`;
}

function getDeviceName(id: string): string {
  if (id === "local") return "本机";
  const device = devicesStore.devices.find((d) => d.id === id);
  return device?.name ?? id;
}

function getFileIcon(name: string) {
  if (/\.(png|jpg|jpeg|gif|svg|webp)$/i.test(name)) return FileImage;
  if (/\.(mp4|mov|avi|mkv)$/i.test(name)) return FileVideo;
  if (/\.(zip|rar|7z|tar|gz)$/i.test(name)) return FileArchive;
  return FileText;
}

function getDirectionLabel(direction: string): string {
  switch (direction) {
    case "upload_to_host":
      return "↑ 上传";
    case "download_from_host":
      return "↓ 下载";
    case "relay":
      return "⟳ 中转";
    default:
      return direction;
  }
}

function getRelayPath(task: TransferTask): string | null {
  if (task.direction !== "relay") return null;
  const source = getDeviceName(task.sourceDeviceId);
  const target = getDeviceName(task.targetDeviceId);
  return `${source} → 主机 → ${target}`;
}

function getStatusLabel(status: string): string {
  const map: Record<string, string> = {
    transferring: "传输中",
    verifying: "正在校验",
    paused: "已暂停",
    completed: "已完成",
    waiting: "等待中",
    requesting: "请求中",
    awaiting_acceptance: "等待接收",
    accepted: "已接受",
    rejected: "已拒绝",
    expired: "已过期",
    cancelled: "已取消",
    failed: "失败",
  };
  return map[status] ?? status;
}

function getStatusClass(status: string): string {
  if (status === "transferring") return "badge--success";
  if (status === "verifying") return "badge--warning";
  if (status === "paused") return "badge--neutral";
  if (status === "completed") return "badge--success";
  if (status === "awaiting_acceptance") return "badge--warning";
  if (status === "accepted") return "badge--success";
  if (status === "rejected" || status === "cancelled" || status === "failed") return "badge--error";
  if (status === "expired") return "badge--neutral";
  return "badge--neutral";
}

function getChecksum(task: TransferTask): string | null {
  return task.files[0]?.checksum ?? null;
}

function getChecksumLabel(task: TransferTask): string {
  if (getChecksum(task)) return "可查看";
  return task.status === "completed" ? "未生成" : "计算中...";
}

function getShortChecksum(checksum: string): string {
  const normalized = checksum.replace(/\s/g, "").toUpperCase();
  if (normalized.length <= 16) return normalized;
  return `${normalized.slice(0, 8)} · ${normalized.slice(-8)}`;
}

function toggleExpand(id: string) {
  expandedId.value = expandedId.value === id ? null : id;
}

function selectFilter(filter: TransferCenterFilter) {
  if (filter === selectedFilter.value) return;
  expandedId.value = null;
  void router.replace({
    name: "transfers",
    query: filter === "all" ? {} : { filter },
  });
}

function handlePause(task: TransferTask) {
  if (task.status === "paused") {
    transfersStore.resumeTransfer(task.id);
  } else {
    transfersStore.pauseTransfer(task.id);
  }
}

function handleCancel(id: string) {
  transfersStore.cancelTransfer(id);
}

async function handleOpenReceiveFolder() {
  if (!isTauri()) {
    appStore.pushToast("info", "仅桌面端支持", "请在桌面应用中打开接收文件夹");
    return;
  }
  try {
    await openReceiveFolder();
  } catch (err) {
    console.error("[transfers] Failed to open receive folder:", err);
    appStore.pushToast("error", "打开失败", "无法打开接收文件夹");
  }
}

function handleRetry(id: string) {
  transfersStore.retryTransfer(id);
}

onMounted(() => {
  elapsedTimer = window.setInterval(() => {
    now.value = Date.now();
  }, 1_000);
  void transfersStore.fetchTransfers();
  // Only register listeners once
  if (!transfersStore.listenersRegistered) {
    transfersStore.setupWebSocketListeners();
  }
});

onUnmounted(() => {
  if (elapsedTimer !== null) window.clearInterval(elapsedTimer);
});
</script>

<template>
  <div class="transfers-page">
    <!-- Title Section -->
    <header class="page-header">
      <div class="header-left">
        <h1 class="page-title">传输中心</h1>
        <p class="page-subtitle">
          {{ activeCount }} 个活跃传输 · 总速度 {{ totalSpeed }}
        </p>
      </div>
      <button class="outline-btn" @click="handleOpenReceiveFolder">
        <FolderOpen :size="15" />
        打开接收文件夹
      </button>
    </header>

    <TransferCenterFilterBar
      :model-value="selectedFilter"
      :counts="filterCounts"
      @update:model-value="selectFilter"
    />

    <!-- Transfer Table Card -->
    <div class="table-card">
      <div class="table-header">
        <span class="col-icon"></span>
        <span>文件</span>
        <span>来源</span>
        <span>目标</span>
        <span>大小</span>
        <span>进度</span>
        <span>速度</span>
        <span>剩余时间</span>
        <span>状态</span>
        <span>操作</span>
      </div>

      <div class="table-body">
        <template v-if="filteredTransfers.length === 0">
          <div class="table-empty">{{ emptyStateText }}</div>
        </template>
        <template v-else>
          <template v-for="task in filteredTransfers" :key="task.id">
            <div
              class="transfer-row"
              :class="{ 'transfer-row--expanded': expandedId === task.id }"
            >
            <button class="expand-btn" @click="toggleExpand(task.id)">
              <ChevronDown v-if="expandedId === task.id" :size="14" />
              <ChevronRight v-else :size="14" />
            </button>
            <span class="col-file">
              <component
                :is="getFileIcon(task.files[0]?.name ?? '')"
                :size="16"
                class="file-icon"
              />
              <span class="file-name-wrap">
                <span class="file-name">{{ task.files[0]?.name }}</span>
                <span v-if="getRelayPath(task)" class="relay-path">{{ getRelayPath(task) }}</span>
              </span>
            </span>
            <span class="col-source">
              <ArrowUpFromLine v-if="task.direction === 'upload_to_host'" :size="12" class="dir-icon" />
              <ArrowDownToLine v-else-if="task.direction === 'download_from_host'" :size="12" class="dir-icon" />
              <RefreshCw v-else-if="task.direction === 'relay'" :size="12" class="dir-icon" />
              {{ getDeviceName(task.sourceDeviceId) }}
            </span>
            <span class="col-target">{{ getDeviceName(task.targetDeviceId) }}</span>
            <span class="col-size">{{ formatSize(task.totalBytes) }}</span>
            <span class="col-progress">
              <div class="progress-bar">
                <div
                  class="progress-fill"
                  :style="{ width: `${Math.round(task.progress * 100)}%` }"
                ></div>
              </div>
              <span class="progress-text">{{ Math.round(task.progress * 100) }}%</span>
            </span>
            <span class="col-speed">
              <span>{{ formatSpeed(task.speedBytesPerSecond) }}</span>
              <span class="elapsed-time">已用 {{ formatElapsed(task) }}</span>
            </span>
            <span class="col-remaining">{{ formatRemaining(task.remainingSeconds) }}</span>
            <span class="col-status">
              <span class="dir-badge">{{ getDirectionLabel(task.direction) }}</span>
              <span class="badge" :class="getStatusClass(task.status)">
                {{ getStatusLabel(task.status) }}
              </span>
            </span>
            <span class="col-actions">
              <button
                v-if="task.status === 'failed'"
                class="resume-btn"
                @click="handleRetry(task.id)"
              >
                重试
              </button>
              <button
                v-else-if="task.status === 'paused'"
                class="resume-btn"
                @click="handlePause(task)"
              >
                继续
              </button>
              <button
                v-else-if="task.status === 'transferring'"
                class="action-btn"
                title="暂停"
                @click="handlePause(task)"
              >
                <Pause :size="14" />
              </button>
              <button
                v-if="task.status !== 'completed' && task.status !== 'cancelled' && task.status !== 'rejected' && task.status !== 'expired' && task.status !== 'failed'"
                class="action-btn action-btn--danger"
                title="取消"
                @click="handleCancel(task.id)"
              >
                <X :size="14" />
              </button>
            </span>
            </div>

            <!-- Expanded Detail -->
            <div v-if="expandedId === task.id" class="transfer-detail">
              <div class="detail-grid">
              <div class="detail-item">
                <span class="detail-label">分块进度</span>
                <div class="chunk-viz">
                  <span
                    v-for="i in 20"
                    :key="i"
                    class="chunk"
                    :class="{ 'chunk--done': i / 20 <= task.progress }"
                  ></span>
                </div>
              </div>
              <div class="detail-item">
                <span class="detail-label">已用时间</span>
                <span class="detail-value">{{ formatElapsed(task) }}</span>
              </div>
              <div class="detail-item">
                <span class="detail-label">开始时间</span>
                <span class="detail-value">{{ new Date(task.createdAt).toLocaleTimeString() }}</span>
              </div>
              <div class="detail-item">
                <span class="detail-label">保存路径</span>
                <span class="detail-value">{{ task.savePath ?? settingsStore.receiveFolder }}</span>
              </div>
              <div class="detail-item">
                <span class="detail-label">快速校验码</span>
                <details v-if="getChecksum(task)" class="checksum-details">
                  <summary class="detail-value detail-value--mono" title="点击查看完整 SHA-256">
                    {{ getShortChecksum(getChecksum(task)!) }}
                  </summary>
                  <code class="checksum-full">{{ getChecksum(task) }}</code>
                </details>
                <span v-else class="detail-value detail-value--mono">{{ getChecksumLabel(task) }}</span>
              </div>
              <div class="detail-item">
                <span class="detail-label">重试次数</span>
                <span class="detail-value">{{ task.retryCount ?? 0 }}</span>
              </div>
              </div>
            </div>
          </template>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.transfers-page {
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

.outline-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  border: 1px solid var(--color-brand-primary);
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--color-text-brand);
  font-size: var(--text-sm);
  font-weight: var(--weight-medium);
  cursor: pointer;
  transition: all var(--transition-fast);
  white-space: nowrap;
}

.outline-btn:hover {
  background: var(--color-selected);
}

/* Table Card */
.table-card {
  background: var(--color-surface-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-card);
  overflow: hidden;
}

.table-empty {
  padding: 48px 20px;
  color: var(--color-text-tertiary);
  font-size: var(--text-sm);
  text-align: center;
}

.table-header {
  display: grid;
  grid-template-columns: 28px 1.2fr 100px 100px 72px 100px 80px 68px 72px 56px;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  background: var(--color-surface-inset);
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
  color: var(--color-text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.02em;
  position: sticky;
  top: 0;
  z-index: var(--z-sticky);
}

.transfer-row {
  display: grid;
  grid-template-columns: 28px 1.2fr 100px 100px 72px 100px 80px 68px 72px 56px;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--color-border);
  font-size: var(--text-sm);
  transition: background var(--transition-fast);
}

.transfer-row:hover {
  background: var(--color-hover);
}

.transfer-row:last-child {
  border-bottom: none;
}

.expand-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border: none;
  background: transparent;
  border-radius: var(--radius-sm);
  color: var(--color-text-tertiary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.expand-btn:hover {
  background: var(--color-hover);
  color: var(--color-text-primary);
}

.col-file {
  display: flex;
  align-items: center;
  gap: 8px;
  overflow: hidden;
}

.file-icon {
  color: var(--color-text-tertiary);
  flex-shrink: 0;
}

.file-name {
  color: var(--color-text-primary);
  font-weight: var(--weight-medium);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-name-wrap {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  overflow: hidden;
}

.relay-path {
  font-size: var(--text-xs);
  color: var(--color-text-tertiary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.col-source,
.col-target {
  display: flex;
  align-items: center;
  gap: 4px;
  color: var(--color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dir-icon {
  color: var(--color-text-tertiary);
  flex-shrink: 0;
}

.col-size {
  color: var(--color-text-secondary);
  font-family: var(--font-mono);
  font-size: var(--text-xs);
}

.col-progress {
  display: flex;
  align-items: center;
  gap: 6px;
}

.progress-bar {
  flex: 1;
  height: 4px;
  background: var(--color-surface-inset);
  border-radius: var(--radius-full);
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: var(--color-brand-primary);
  border-radius: var(--radius-full);
  transition: width 0.4s ease;
}

.progress-text {
  font-size: var(--text-xs);
  color: var(--color-text-secondary);
  font-family: var(--font-mono);
  min-width: 30px;
}

.col-speed {
  display: flex;
  flex-direction: column;
  gap: 2px;
  color: var(--color-text-secondary);
  font-family: var(--font-mono);
  font-size: var(--text-xs);
}

.elapsed-time {
  color: var(--color-text-tertiary);
  white-space: nowrap;
}

.col-remaining {
  color: var(--color-text-tertiary);
  font-size: var(--text-xs);
  font-family: var(--font-mono);
}

.col-status {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 3px;
}

.dir-badge {
  font-size: 10px;
  color: var(--color-text-tertiary);
  white-space: nowrap;
  line-height: 1;
}

.col-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  opacity: 0;
  transition: opacity var(--transition-fast);
}

.transfer-row:hover .col-actions {
  opacity: 1;
}

.action-btn {
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

.action-btn:hover {
  background: var(--color-hover);
  color: var(--color-text-primary);
}

.action-btn--danger:hover {
  background: var(--color-state-error-soft);
  color: var(--color-state-error);
}

.resume-btn {
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  border: 1px solid var(--color-brand-primary);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-brand);
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
  cursor: pointer;
  white-space: nowrap;
  transition: all var(--transition-fast);
}

.resume-btn:hover {
  background: var(--color-selected);
}

/* Badge */
.badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  border-radius: var(--radius-full);
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
  white-space: nowrap;
}

.badge--success {
  background: var(--color-state-success-soft);
  color: var(--color-state-success);
}

.badge--warning {
  background: var(--color-state-warning-soft);
  color: var(--color-state-warning);
}

.badge--error {
  background: var(--color-state-error-soft);
  color: var(--color-state-error);
}

.badge--neutral {
  background: var(--color-surface-inset);
  color: var(--color-text-secondary);
}

/* Expanded Detail */
.transfer-detail {
  padding: 16px 16px 16px 54px;
  background: var(--color-surface-inset);
  border-bottom: 1px solid var(--color-border);
}

.detail-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
}

.detail-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.detail-label {
  font-size: var(--text-xs);
  color: var(--color-text-tertiary);
  font-weight: var(--weight-medium);
}

.detail-value {
  font-size: var(--text-sm);
  color: var(--color-text-primary);
}

.detail-value--mono {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
}

.checksum-details {
  min-width: 0;
}

.checksum-details summary {
  list-style: none;
  cursor: pointer;
}

.checksum-details summary::-webkit-details-marker {
  display: none;
}

.checksum-details summary::after {
  content: "展开";
  margin-left: 8px;
  color: var(--color-brand-primary);
  font-family: var(--font-sans);
  font-size: var(--text-xs);
}

.checksum-details[open] summary::after {
  content: "收起";
}

.checksum-full {
  display: block;
  max-width: 280px;
  margin-top: 6px;
  color: var(--color-text-secondary);
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  line-height: 1.5;
  overflow-wrap: anywhere;
}

.chunk-viz {
  display: flex;
  gap: 2px;
}

.chunk {
  width: 12px;
  height: 8px;
  border-radius: 2px;
  background: var(--color-border);
}

.chunk--done {
  background: var(--color-brand-primary);
}
</style>
