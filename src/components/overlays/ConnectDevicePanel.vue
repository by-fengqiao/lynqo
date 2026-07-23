<script setup lang="ts">
import { ref, computed } from "vue";
import { X, Copy, Check, Wifi } from "lucide-vue-next";
import { useAppStore } from "@/stores/app";
import { useSettingsStore } from "@/stores/settings";

defineProps<{
  visible: boolean;
}>();

const emit = defineEmits<{
  close: [];
}>();

const appStore = useAppStore();
const settingsStore = useSettingsStore();

// Bound to the persisted setting so toggling here actually changes backend
// approval behavior and survives the panel being closed/reopened.
const requireConfirm = computed(() => settingsStore.requireApproval);
function toggleRequireConfirm() {
  settingsStore.setRequireApproval(!settingsStore.requireApproval);
}
const copiedField = ref<string | null>(null);

const displayLocalDomain = computed(
  () => appStore.connectionInfo?.localDomain ?? "office-mac.local"
);
const displayLocalUrl = computed(() => {
  const info = appStore.connectionInfo;
  if (info) return `${info.ip}:${info.port}`;
  return `${appStore.localIp}:53317`;
});
const displayReceiveFolder = computed(
  () => appStore.connectionInfo?.receiveFolder ?? "~/Downloads/LYNQO"
);

async function copyToClipboard(text: string, field: string) {
  try {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      await navigator.clipboard.writeText(text);
    } else {
      const textarea = document.createElement("textarea");
      textarea.value = text;
      textarea.style.position = "fixed";
      textarea.style.opacity = "0";
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand("copy");
      document.body.removeChild(textarea);
    }
    copiedField.value = field;
    setTimeout(() => {
      copiedField.value = null;
    }, 2000);
  } catch {
    // silently fail
  }
}
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="panel-wrapper">
      <div class="backdrop" @click="emit('close')" />
      <div class="panel">
        <!-- Header -->
        <div class="panel-header">
          <span class="panel-title">连接设备</span>
          <button class="close-btn" @click="emit('close')">
            <X :size="16" />
          </button>
        </div>

        <!-- QR Code -->
        <div class="qr-section">
          <div class="qr-code">
            <div v-if="appStore.qrCode?.svg" v-html="appStore.qrCode.svg" class="qr-svg" />
            <div v-else class="qr-pattern" />
          </div>
        </div>

        <!-- Connection Info -->
        <div class="info-section">
          <div class="info-row">
            <span class="info-label">.local 地址</span>
            <div class="info-value-group">
              <span class="info-value">{{ displayLocalDomain }}</span>
              <button
                class="copy-btn"
                @click="copyToClipboard(displayLocalDomain, 'local')"
              >
                <Check v-if="copiedField === 'local'" :size="13" class="copied" />
                <Copy v-else :size="13" />
              </button>
            </div>
          </div>
          <div class="info-row">
            <span class="info-label">局域网 IP</span>
            <div class="info-value-group">
              <span class="info-value">{{ displayLocalUrl }}</span>
              <button
                class="copy-btn"
                @click="copyToClipboard(displayLocalUrl, 'ip')"
              >
                <Check v-if="copiedField === 'ip'" :size="13" class="copied" />
                <Copy v-else :size="13" />
              </button>
            </div>
          </div>
        </div>

        <!-- Divider -->
        <div class="divider" />

        <!-- Settings -->
        <div class="settings-section">
          <div class="setting-row">
            <span class="setting-label">需要确认新设备</span>
            <button
              class="toggle"
              :class="{ active: requireConfirm }"
              @click="toggleRequireConfirm"
            >
              <span class="toggle-knob" />
            </button>
          </div>
          <div class="setting-row">
            <span class="setting-label">接收文件夹</span>
            <span class="setting-value">{{ displayReceiveFolder }}</span>
          </div>
          <div class="setting-row">
            <span class="setting-label">防火墙状态</span>
            <span class="setting-value setting-value--success">已允许</span>
          </div>
        </div>

        <!-- Network Badge -->
        <div class="network-badge">
          <Wifi :size="12" />
          <span>{{ appStore.networkName }}</span>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.panel-wrapper {
  position: fixed;
  inset: 0;
  z-index: var(--z-overlay);
}

.backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.05);
  animation: fade-in 180ms ease forwards;
}

.panel {
  position: fixed;
  top: calc(var(--topbar-height) + 6px);
  right: 80px;
  width: 320px;
  background: var(--color-surface-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
  padding: 20px;
  animation: panel-in 200ms ease forwards;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.panel-title {
  font-size: var(--text-base);
  font-weight: var(--weight-semibold);
  color: var(--color-text-primary);
}

.close-btn {
  width: 26px;
  height: 26px;
  border: none;
  background: transparent;
  color: var(--color-text-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: background var(--transition-fast), color var(--transition-fast);
}

.close-btn:hover {
  background: var(--color-hover);
  color: var(--color-text-primary);
}

/* QR Code */
.qr-section {
  display: flex;
  justify-content: center;
  margin-bottom: 16px;
}

.qr-code {
  width: 160px;
  height: 160px;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border);
  background: var(--color-surface-card);
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}

.qr-pattern {
  width: 130px;
  height: 130px;
  background-image:
    repeating-linear-gradient(
      0deg,
      var(--color-text-primary) 0px,
      var(--color-text-primary) 4px,
      transparent 4px,
      transparent 8px
    ),
    repeating-linear-gradient(
      90deg,
      var(--color-text-primary) 0px,
      var(--color-text-primary) 4px,
      transparent 4px,
      transparent 8px
    );
  background-size: 8px 8px;
  opacity: 0.7;
  border-radius: 2px;
}

.qr-svg {
  width: 140px;
  height: 140px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.qr-svg :deep(svg) {
  width: 100%;
  height: 100%;
}

/* Info Section */
.info-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 16px;
}

.info-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.info-label {
  font-size: var(--text-xs);
  color: var(--color-text-tertiary);
}

.info-value-group {
  display: flex;
  align-items: center;
  gap: 6px;
}

.info-value {
  font-size: var(--text-sm);
  font-family: var(--font-mono);
  color: var(--color-text-primary);
}

.copy-btn {
  width: 22px;
  height: 22px;
  border: none;
  background: transparent;
  color: var(--color-text-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: color var(--transition-fast), background var(--transition-fast);
}

.copy-btn:hover {
  background: var(--color-hover);
  color: var(--color-text-primary);
}

.copy-btn .copied {
  color: var(--color-state-success);
}

/* Divider */
.divider {
  height: 1px;
  background: var(--color-border);
  margin: 0 0 16px;
}

/* Settings */
.settings-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-bottom: 16px;
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.setting-label {
  font-size: var(--text-sm);
  color: var(--color-text-secondary);
}

.setting-value {
  font-size: var(--text-sm);
  color: var(--color-text-primary);
  font-family: var(--font-mono);
}

.setting-value--success {
  color: var(--color-state-success);
  font-family: var(--font-sans);
}

/* Toggle Switch */
.toggle {
  width: 36px;
  height: 20px;
  border-radius: var(--radius-full);
  border: none;
  background: var(--color-border-strong);
  position: relative;
  cursor: pointer;
  transition: background var(--transition-normal);
  padding: 0;
}

.toggle.active {
  background: var(--color-state-success);
}

.toggle-knob {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  border-radius: var(--radius-full);
  background: #fff;
  box-shadow: var(--shadow-sm);
  transition: transform var(--transition-normal);
}

.toggle.active .toggle-knob {
  transform: translateX(16px);
}

/* Network Badge */
.network-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 10px;
  border-radius: var(--radius-full);
  background: var(--color-brand-primary-soft);
  color: var(--color-text-brand);
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
}

@keyframes fade-in {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

@keyframes panel-in {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
