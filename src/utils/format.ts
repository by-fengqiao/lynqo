// 设备图标与平台显示文案
import type { Device, Platform, DeviceType } from "@/types";
import { getCurrentLocale } from "@/i18n";

export function platformLabel(platform: Platform): string {
  switch (platform) {
    case "ios":
      return "iOS";
    case "android":
      return "Android";
    case "macos":
      return "macOS";
    case "windows":
      return "Windows";
    case "web":
      return "Web";
    default:
      return "—";
  }
}

export function deviceTypeLabel(t: DeviceType): string {
  const isEnglish = getCurrentLocale() === "en-US";
  switch (t) {
    case "desktop":
      return isEnglish ? "Desktop" : "台式机";
    case "laptop":
      return isEnglish ? "Laptop" : "笔记本";
    case "phone":
      return isEnglish ? "Phone" : "手机";
    case "tablet":
      return isEnglish ? "Tablet" : "平板";
    default:
      return "—";
  }
}

// 字节格式化（1024 进制，与设计稿 1.28 GB 一致）
export function formatBytes(bytes: number, digits = 1): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "—";
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(
    units.length - 1,
    Math.floor(Math.log(bytes) / Math.log(1024)),
  );
  const value = bytes / Math.pow(1024, i);
  const d = i <= 1 ? 0 : digits;
  return `${value.toFixed(d)} ${units[i]}`;
}

// 速度格式化
export function formatSpeed(bytesPerSec: number): string {
  if (!Number.isFinite(bytesPerSec) || bytesPerSec <= 0) return "—";
  return `${formatBytes(bytesPerSec)}/s`;
}

// 剩余时间格式化
export function formatRemaining(seconds?: number): string {
  const isEnglish = getCurrentLocale() === "en-US";
  if (seconds == null || !Number.isFinite(seconds)) return "—";
  if (seconds <= 0) return isEnglish ? "0 sec" : "0 秒";
  if (seconds < 60) return isEnglish ? `${Math.ceil(seconds)} sec` : `${Math.ceil(seconds)} 秒`;
  const m = Math.floor(seconds / 60);
  const s = Math.ceil(seconds % 60);
  if (isEnglish) return `${m}:${String(s).padStart(2, "0")}`;
  return `${m}:${String(s).padStart(2, "0")}`;
}

// 相对时间
export function formatRelativeTime(iso: string, now: Date = new Date()): string {
  const isEnglish = getCurrentLocale() === "en-US";
  const t = new Date(iso).getTime();
  if (Number.isNaN(t)) return "—";
  const diff = now.getTime() - t;
  const sec = Math.floor(diff / 1000);
  if (sec < 60) return isEnglish ? "Just now" : "刚刚";
  const min = Math.floor(sec / 60);
  if (min < 60) return isEnglish ? `${min} min ago` : `${min} 分钟前`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return isEnglish ? `${hr} hr ago` : `${hr} 小时前`;
  const day = Math.floor(hr / 24);
  return isEnglish ? `${day} day${day === 1 ? "" : "s"} ago` : `${day} 天前`;
}

// HH:MM:SS 或 m:ss
export function formatClock(iso?: string): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "—";
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

// 设备显示名优先级：name > hostName
export function deviceDisplayName(d: Pick<Device, "name">): string {
  return d.name || (getCurrentLocale() === "en-US" ? "Unknown device" : "未知设备");
}

// Generates a transient client-side ID before the server assigns its own ID.
// This is identity generation, not simulated business data.
let transientIdSequence = 0;
export function genId(prefix: string): string {
  const uuid = globalThis.crypto?.randomUUID?.();
  if (uuid) return `${prefix}_${uuid}`;

  const bytes = new Uint8Array(12);
  globalThis.crypto?.getRandomValues?.(bytes);
  transientIdSequence += 1;
  const entropy = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
  return `${prefix}_${Date.now().toString(36)}_${transientIdSequence}_${entropy}`;
}

// 简易延时
export function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// 文件名清洗（防止路径穿越，第二阶段后端会再校验一次）
export function sanitizeFileName(name: string): string {
  // 去掉路径分隔符，去掉控制字符
  const base = name.replace(/[/\\]+/g, "_").replace(/[\x00-\x1f]/g, "");
  return base.trim() || "unnamed";
}
