<script setup lang="ts">
// Toast 宿主：右下角浮层提示
import { CheckCircle2, XCircle, Info, AlertTriangle } from "lucide-vue-next";
import { useAppStore } from "@/stores/app";
import type { ToastKind } from "@/types";

const app = useAppStore();

const iconMap = {
  success: CheckCircle2,
  error: XCircle,
  info: Info,
  warning: AlertTriangle,
};
const colorMap: Record<ToastKind, string> = {
  success: "var(--state-success)",
  error: "var(--state-error)",
  info: "var(--state-info)",
  warning: "var(--state-warning)",
};
</script>

<template>
  <div class="toast-host" aria-live="polite">
    <TransitionGroup name="toast">
      <div
        v-for="t in app.toasts"
        :key="t.id"
        class="toast-item"
        role="status"
        @click="app.dismissToast(t.id)"
      >
        <component
          :is="iconMap[t.kind]"
          :size="16"
          :stroke-width="2.5"
          :style="{ color: colorMap[t.kind] }"
        />
        <div class="min-w-0 flex-1">
          <div style="font-size: var(--text-sm); font-weight: var(--font-weight-medium); color: var(--color-text-primary)">
            {{ t.title }}
          </div>
          <div v-if="t.description" style="font-size: var(--text-xs); color: var(--color-text-tertiary); margin-top: 2px">
            {{ t.description }}
          </div>
        </div>
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.toast-host {
  position: fixed;
  bottom: 24px;
  right: 24px;
  z-index: 100;
  display: flex;
  flex-direction: column;
  gap: 8px;
  pointer-events: none;
}
.toast-item {
  pointer-events: auto;
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 12px 14px;
  min-width: 240px;
  max-width: 360px;
  background: var(--color-surface-content);
  border: 1px solid var(--color-border-default);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-float);
  cursor: pointer;
}
</style>
