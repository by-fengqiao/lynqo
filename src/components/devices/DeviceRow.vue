<script setup lang="ts">
// 桌面端设备行（首页"附近设备"列表）
// 选中态：背景品牌色浅底 + 左侧 2.5px 边框 + 名称品牌色 + check 图标
import { computed } from "vue";
import {
  Smartphone,
  Laptop,
  Monitor,
  Tablet,
  CheckCircle2,
  Check,
} from "lucide-vue-next";
import type { Device } from "@/types";
import { platformLabel } from "@/utils/format";

const props = defineProps<{
  device: Device;
  selected?: boolean;
}>();

const emit = defineEmits<{ select: [id: string] }>();

const platformIcon = computed(() => {
  switch (props.device.deviceType) {
    case "phone":
      return Smartphone;
    case "tablet":
      return Tablet;
    case "laptop":
      return Laptop;
    case "desktop":
      return Monitor;
  }
});

function onClick() {
  if (props.device.online) emit("select", props.device.id);
}
</script>

<template>
  <div
    class="device-row grid items-center px-3 py-2.5 cursor-pointer transition-colors"
    :class="selected ? 'row-selected' : 'row-idle'"
    style="grid-template-columns: 28px 1fr 72px 72px 56px"
    @click="onClick"
  >
    <component
      :is="platformIcon"
      :size="18"
      :stroke-width="2"
      style="color: var(--color-text-tertiary)"
    />
    <div class="flex items-center gap-2 min-w-0">
      <span
        class="cell-truncate"
        :style="{
          fontSize: 'var(--text-sm)',
          fontWeight: selected ? 'var(--font-weight-medium)' : 'normal',
          color: selected ? 'var(--color-primary)' : 'var(--color-text-primary)',
        }"
        >{{ device.name }}</span
      >
      <CheckCircle2
        v-if="selected"
        :size="14"
        :stroke-width="2.5"
        style="color: var(--color-primary)"
      />
    </div>
    <span style="font-size: var(--text-xs); color: var(--color-text-tertiary)">
      {{ platformLabel(device.platform) }}
    </span>
    <div v-if="selected" class="flex items-center gap-1" style="font-size: var(--text-xs); color: var(--color-primary); font-weight: var(--font-weight-medium)">
      <Check :size="12" :stroke-width="3" />已选中
    </div>
    <div
      v-else-if="device.online"
      class="flex items-center gap-1"
      style="font-size: var(--text-xs); color: var(--state-success)"
    >
      <span class="status-dot" style="background: var(--state-success)" />在线
    </div>
    <div v-else class="flex items-center gap-1" style="font-size: var(--text-xs); color: var(--color-text-tertiary)">
      <span class="status-dot" style="background: var(--color-text-tertiary)" />离线
    </div>
    <span style="font-size: var(--text-xs); color: var(--color-text-tertiary); text-align: right; font-family: var(--font-mono)">
      {{ device.online && device.latencyMs != null ? `${device.latencyMs}ms` : "—" }}
    </span>
  </div>
</template>

<style scoped>
.row-idle {
  border-bottom: 1px solid var(--color-border-default);
  transition: background var(--transition-fast);
}
.row-idle:hover {
  background: var(--color-surface-secondary);
}
.row-selected {
  border-radius: var(--radius-sm);
  background: var(--color-primary-light-bg);
  border-left: 2.5px solid var(--color-primary);
  margin-left: -2px;
  transition: background var(--transition-fast);
}
</style>
