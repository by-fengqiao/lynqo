<script setup lang="ts">
// 传输状态徽章：彩色圆点 + 文案，颜色使用设计令牌
import { computed } from "vue";
import type { TransferStatus } from "@/types";

const props = defineProps<{
  status: TransferStatus;
}>();

// 全量映射，新增状态时 TS 会强制补齐
const statusStyles: Record<
  TransferStatus,
  { label: string; color: string; bg: string }
> = {
  pending: { label: "Pending", color: "var(--color-state-warning)", bg: "var(--color-state-warning-soft)" },
  waiting: { label: "等待中", color: "var(--color-state-info)", bg: "var(--color-state-info-soft)" },
  requesting: { label: "请求中", color: "var(--color-state-info)", bg: "var(--color-state-info-soft)" },
  awaiting_acceptance: { label: "待确认", color: "var(--color-state-warning)", bg: "var(--color-state-warning-soft)" },
  accepted: { label: "已接受", color: "var(--color-state-info)", bg: "var(--color-state-info-soft)" },
  transferring: { label: "传输中", color: "var(--color-state-info)", bg: "var(--color-state-info-soft)" },
  paused: { label: "已暂停", color: "var(--color-state-warning)", bg: "var(--color-state-warning-soft)" },
  verifying: { label: "校验中", color: "var(--color-state-info)", bg: "var(--color-state-info-soft)" },
  completed: { label: "已完成", color: "var(--color-state-success)", bg: "var(--color-state-success-soft)" },
  rejected: { label: "已拒绝", color: "var(--color-state-error)", bg: "var(--color-state-error-soft)" },
  expired: { label: "已过期", color: "var(--color-text-tertiary)", bg: "var(--color-surface-secondary)" },
  cancelled: { label: "已取消", color: "var(--color-text-tertiary)", bg: "var(--color-surface-secondary)" },
  failed: { label: "失败", color: "var(--color-state-error)", bg: "var(--color-state-error-soft)" },
};

const config = computed(() => statusStyles[props.status]);
</script>

<template>
  <span
    class="status-badge"
    :style="{ color: config.color, backgroundColor: config.bg }"
  >
    <span class="status-dot" :style="{ backgroundColor: config.color }" />
    {{ config.label }}
  </span>
</template>

<style scoped>
.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  flex-shrink: 0;
  padding: 1px 8px;
  border-radius: 999px;
  font-size: var(--text-xs);
  font-weight: var(--font-weight-medium);
  line-height: var(--leading-normal);
  white-space: nowrap;
}
.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}
</style>
