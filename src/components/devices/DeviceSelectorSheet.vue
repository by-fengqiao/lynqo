<script setup lang="ts">
// 手机端设备选择底部面板（sheet）
// - 点击遮罩关闭
// - 下滑关闭（touch）
// - 键盘 ESC 关闭
// - 移动端安全区域
import { ref, watch, onUnmounted } from "vue";
import { X } from "lucide-vue-next";
import { useAppStore } from "@/stores/app";
import { useDevicesStore } from "@/stores/devices";
import DeviceListItem from "./DeviceListItem.vue";

const app = useAppStore();
const devices = useDevicesStore();

// 下滑关闭手势
const sheetEl = ref<HTMLElement | null>(null);
const dragY = ref(0);
let startY: number | null = null;
let dragging = false;

function onTouchStart(e: TouchEvent) {
  if (e.touches.length !== 1) return;
  // 仅在列表滚动到顶部时启用下滑关闭
  const list = sheetEl.value?.querySelector(".panel-device-list") as HTMLElement | null;
  if (list && list.scrollTop > 0) return;
  startY = e.touches[0].clientY;
  dragging = true;
}
function onTouchMove(e: TouchEvent) {
  if (!dragging || startY == null) return;
  const dy = e.touches[0].clientY - startY;
  dragY.value = Math.max(0, dy);
}
function onTouchEnd() {
  if (dragY.value > 100) {
    app.closeDeviceSheet();
  }
  dragging = false;
  startY = null;
  dragY.value = 0;
}

// ESC 关闭
function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape" && app.deviceSheetOpen) {
    app.closeDeviceSheet();
  }
}
watch(
  () => app.deviceSheetOpen,
  (open) => {
    if (open) {
      window.addEventListener("keydown", onKeydown);
      document.body.style.overflow = "hidden";
    } else {
      window.removeEventListener("keydown", onKeydown);
      document.body.style.overflow = "";
    }
  },
);
onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
  document.body.style.overflow = "";
});

function onSelect(id: string) {
  devices.selectDevice(id);
  app.closeDeviceSheet();
}
</script>

<template>
  <Transition name="backdrop">
    <div
      v-if="app.deviceSheetOpen"
      class="panel-backdrop"
      @click="app.closeDeviceSheet()"
    />
  </Transition>
  <Transition name="sheet">
    <section
      v-if="app.deviceSheetOpen"
      ref="sheetEl"
      class="panel-sheet"
      :style="{ transform: dragY ? `translateY(${dragY}px)` : undefined }"
      @touchstart.passive="onTouchStart"
      @touchmove.passive="onTouchMove"
      @touchend="onTouchEnd"
    >
      <div class="panel-drag-handle" />
      <header class="panel-header">
        <span style="font-size: 16px; font-weight: 600; color: var(--color-text-primary)">选择设备</span>
        <button class="panel-close-btn" aria-label="关闭" @click="app.closeDeviceSheet()">
          <X :size="20" :stroke-width="2" />
        </button>
      </header>
      <div class="panel-device-list">
        <DeviceListItem
          v-for="d in devices.onlineDevices"
          :key="d.id"
          :device="d"
          :selected="d.id === devices.selectedDeviceId"
          @select="onSelect"
        />
        <p
          v-if="!devices.onlineDevices.length"
          class="text-center py-8"
          style="font-size: var(--text-sm); color: var(--color-text-tertiary)"
        >
          附近没有在线设备
        </p>
      </div>
    </section>
  </Transition>
</template>

<style scoped>
.panel-backdrop {
  position: fixed;
  inset: 0;
  z-index: 60;
  background: rgba(0, 0, 0, 0.3);
  max-width: var(--content-max-width-mobile);
  margin: 0 auto;
}
.panel-sheet {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  z-index: 61;
  min-height: 45vh;
  max-height: 80vh;
  background: var(--color-surface-content);
  border-radius: 16px 16px 0 0;
  box-shadow: 0 -4px 24px rgba(0, 0, 0, 0.12), 0 -1px 6px rgba(0, 0, 0, 0.06);
  max-width: var(--content-max-width-mobile);
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  /* iOS 安全区域 */
  padding-bottom: env(safe-area-inset-bottom, 0px);
  touch-action: pan-y;
  transition: transform 60ms linear;
}
.panel-drag-handle {
  width: 36px;
  height: 4px;
  border-radius: 2px;
  background: var(--color-neutral-300);
  margin: 10px auto 0;
  flex-shrink: 0;
}
.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 20px 12px;
  flex-shrink: 0;
}
.panel-device-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 16px 16px;
}
.panel-close-btn {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text-tertiary);
  background: transparent;
  border: none;
  cursor: pointer;
  transition: background var(--transition-fast);
}
.panel-close-btn:hover {
  background: var(--color-surface-secondary);
}
</style>
