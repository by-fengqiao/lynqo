<script setup lang="ts">
// 手机端设备选择面板（底部 sheet）中的设备项
import { computed } from "vue";
import {
  Smartphone,
  Laptop,
  Monitor,
  Tablet,
  Check,
} from "lucide-vue-next";
import type { Device } from "@/types";
import { platformLabel } from "@/utils/format";

const props = defineProps<{
  device: Device;
  selected?: boolean;
}>();
const emit = defineEmits<{ select: [id: string] }>();

const icon = computed(() => {
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

const iconBg = computed(() => {
  if (props.selected) return "var(--color-primary-light-bg)";
  return "var(--color-surface-secondary)";
});
const iconColor = computed(() => {
  if (props.selected) return "var(--state-success)";
  return "var(--color-text-tertiary)";
});

function onClick() {
  if (props.device.online) emit("select", props.device.id);
}
</script>

<template>
  <div
    class="panel-device-item flex items-center gap-3"
    :class="{ selected }"
    role="button"
    tabindex="0"
    @click="onClick"
    @keydown.enter.prevent="onClick"
    @keydown.space.prevent="onClick"
  >
    <div
      class="panel-device-icon"
      :style="{ background: iconBg }"
    >
      <component :is="icon" :size="20" :stroke-width="2" :style="{ color: iconColor }" />
    </div>
    <div class="panel-device-info">
      <div class="panel-device-name">{{ device.name }}</div>
      <div class="panel-device-meta">
        {{ platformLabel(device.platform) }} ·
        {{ device.online ? "在线" : "离线" }}
        <template v-if="device.online && device.latencyMs != null"
          >· {{ device.latencyMs }}ms</template
        >
      </div>
    </div>
    <div class="panel-check">
      <Check :size="20" :stroke-width="2.5" />
    </div>
  </div>
</template>

<style scoped>
.panel-device-item {
  padding: 0 14px;
  min-height: 60px;
  border-radius: 10px;
  cursor: pointer;
  transition: background var(--transition-fast);
  border-left: 3px solid transparent;
  margin-bottom: 4px;
}
.panel-device-item:hover {
  background: var(--color-surface-secondary);
}
.panel-device-item:active {
  background: var(--color-surface-active);
}
.panel-device-item.selected {
  background: var(--color-primary-light-bg);
  border-left-color: var(--state-success);
}
.panel-device-icon {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.panel-device-info {
  flex: 1;
  min-width: 0;
}
.panel-device-name {
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text-primary);
  line-height: 1.3;
}
.panel-device-meta {
  font-size: 12px;
  color: var(--color-text-tertiary);
  margin-top: 2px;
}
.panel-check {
  flex-shrink: 0;
  color: var(--state-success);
  opacity: 0;
  transition: opacity var(--transition-fast);
}
.panel-device-item.selected .panel-check {
  opacity: 1;
}
</style>
