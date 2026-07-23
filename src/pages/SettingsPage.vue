<script setup lang="ts">
import { Sun, Moon, MonitorSmartphone, FolderOpen, ShieldCheck, FileText, Info } from "lucide-vue-next";
import { useSettingsStore } from "@/stores/settings";
import { useAppStore } from "@/stores/app";
import { pickDirectory } from "@/services/tauri";
import type { ThemeMode } from "@/types";

const settingsStore = useSettingsStore();
const appStore = useAppStore();

const themeOptions: { value: ThemeMode; label: string; icon: typeof Sun }[] = [
  { value: "light", label: "浅色", icon: Sun },
  { value: "dark", label: "深色", icon: Moon },
  { value: "system", label: "跟随系统", icon: MonitorSmartphone },
];

function selectTheme(mode: ThemeMode) {
  settingsStore.setThemeMode(mode);
}

function toggleApproval() {
  settingsStore.setRequireApproval(!settingsStore.requireApproval);
}

async function changeFolder() {
  const directory = await pickDirectory();
  if (directory) {
    settingsStore.setReceiveFolder(directory);
  }
}
</script>

<template>
  <div class="settings-page">
    <header class="page-header">
      <h1 class="page-title">设置</h1>
    </header>

    <div class="settings-card">
      <!-- Theme Section -->
      <section class="settings-section">
        <div class="section-header">
          <Sun :size="16" class="section-icon" />
          <h2 class="section-title">主题</h2>
        </div>
        <div class="theme-options">
          <label
            v-for="opt in themeOptions"
            :key="opt.value"
            class="theme-option"
            :class="{ 'theme-option--active': settingsStore.themeMode === opt.value }"
          >
            <input
              type="radio"
              name="theme"
              :value="opt.value"
              :checked="settingsStore.themeMode === opt.value"
              class="theme-radio"
              @change="selectTheme(opt.value)"
            />
            <component :is="opt.icon" :size="18" class="theme-option-icon" />
            <span class="theme-option-label">{{ opt.label }}</span>
          </label>
        </div>
      </section>

      <hr class="settings-divider" />

      <!-- Receive Folder Section -->
      <section class="settings-section">
        <div class="section-header">
          <FolderOpen :size="16" class="section-icon" />
          <h2 class="section-title">接收文件夹</h2>
        </div>
        <div class="folder-row">
          <code class="folder-path">{{ settingsStore.receiveFolder }}</code>
          <button class="change-btn" @click="changeFolder">更改</button>
        </div>
      </section>

      <hr class="settings-divider" />

      <!-- Security Section -->
      <section class="settings-section">
        <div class="section-header">
          <ShieldCheck :size="16" class="section-icon" />
          <h2 class="section-title">安全</h2>
        </div>
        <div class="toggle-row">
          <div class="toggle-info">
            <span class="toggle-label">接收前需要确认</span>
            <span class="toggle-desc">其他设备发送文件时，需要本机确认后才开始接收。</span>
          </div>
          <button
            class="toggle-switch"
            :class="{ 'toggle-switch--on': settingsStore.requireApproval }"
            role="switch"
            :aria-checked="settingsStore.requireApproval"
            @click="toggleApproval"
          >
            <span class="toggle-knob"></span>
          </button>
        </div>
      </section>

      <hr class="settings-divider" />

      <!-- Open-source license and legal documents -->
      <section class="settings-section">
        <div class="section-header">
          <FileText :size="16" class="section-icon" />
          <h2 class="section-title">开源许可与协议</h2>
        </div>
        <p class="legal-description">
          LYNQO 按 GPL-3.0-only 开源发布。安装包附带完整许可证；局域网传输前请了解隐私、使用规则和风险说明。
        </p>
        <nav class="legal-links" aria-label="开源许可与协议">
          <RouterLink to="/legal/terms">使用协议</RouterLink>
          <RouterLink to="/legal/privacy">隐私说明</RouterLink>
          <RouterLink to="/legal/disclaimer">免责声明</RouterLink>
        </nav>
      </section>

      <hr class="settings-divider" />

      <!-- About Section -->
      <section class="settings-section">
        <div class="section-header">
          <Info :size="16" class="section-icon" />
          <h2 class="section-title">关于</h2>
        </div>
        <div class="about-grid">
          <div class="about-item">
            <span class="about-label">应用名称</span>
            <span class="about-value">LYNQO</span>
          </div>
          <div class="about-item">
            <span class="about-label">版本</span>
            <span class="about-value">{{ appStore.appVersion }}</span>
          </div>
          <div class="about-item">
            <span class="about-label">设备名称</span>
            <span class="about-value">{{ appStore.deviceName }}</span>
          </div>
          <div class="about-item">
            <span class="about-label">网络</span>
            <span class="about-value">{{ appStore.networkName }}</span>
          </div>
          <div class="about-item">
            <span class="about-label">本机 IP</span>
            <span class="about-value">{{ appStore.localIp }}</span>
          </div>
          <div class="about-item">
            <span class="about-label">连接码</span>
            <span class="about-value about-value--mono">{{ appStore.connectionToken }}</span>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.settings-page {
  padding: 32px;
  max-width: 720px;
  margin: 0 auto;
}

.page-header {
  margin-bottom: 24px;
}

.page-title {
  font-size: var(--text-2xl);
  font-weight: var(--weight-semibold);
  color: var(--color-text-primary);
  margin: 0;
  line-height: var(--leading-tight);
}

.settings-card {
  background: var(--color-surface-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-card);
  padding: 24px;
}

.settings-section {
  padding: 4px 0;
}

.section-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 16px;
}

.section-icon {
  color: var(--color-brand-primary);
}

.section-title {
  font-size: var(--text-base);
  font-weight: var(--weight-semibold);
  color: var(--color-text-primary);
  margin: 0;
}

.settings-divider {
  border: none;
  border-top: 1px solid var(--color-border);
  margin: 20px 0;
}

/* Theme Options */
.theme-options {
  display: flex;
  gap: 12px;
}

.theme-option {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 16px 24px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition-fast);
  flex: 1;
}

.theme-option:hover {
  border-color: var(--color-border-strong);
  background: var(--color-hover);
}

.theme-option--active {
  border-color: var(--color-brand-primary);
  background: var(--color-selected);
}

.theme-radio {
  position: absolute;
  opacity: 0;
  pointer-events: none;
}

.theme-option-icon {
  color: var(--color-text-secondary);
}

.theme-option--active .theme-option-icon {
  color: var(--color-brand-primary);
}

.theme-option-label {
  font-size: var(--text-sm);
  color: var(--color-text-primary);
  font-weight: var(--weight-medium);
}

/* Folder Row */
.folder-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.folder-path {
  flex: 1;
  padding: 8px 12px;
  background: var(--color-surface-inset);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  font-family: var(--font-mono);
  color: var(--color-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.change-btn {
  padding: 8px 14px;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-md);
  background: var(--color-surface-card);
  color: var(--color-text-primary);
  font-size: var(--text-sm);
  font-weight: var(--weight-medium);
  cursor: pointer;
  transition: all var(--transition-fast);
  white-space: nowrap;
}

.change-btn:hover {
  background: var(--color-hover);
  border-color: var(--color-brand-primary);
  color: var(--color-text-brand);
}

/* Toggle */
.toggle-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.toggle-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.toggle-label {
  font-size: var(--text-base);
  color: var(--color-text-primary);
  font-weight: var(--weight-medium);
}

.toggle-desc {
  font-size: var(--text-sm);
  color: var(--color-text-secondary);
}

.toggle-switch {
  position: relative;
  width: 44px;
  height: 24px;
  border: none;
  border-radius: var(--radius-full);
  background: var(--color-border-strong);
  cursor: pointer;
  transition: background var(--transition-normal);
  flex-shrink: 0;
}

.toggle-switch--on {
  background: var(--color-brand-primary);
}

.toggle-knob {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 18px;
  height: 18px;
  border-radius: var(--radius-full);
  background: white;
  box-shadow: var(--shadow-sm);
  transition: transform var(--transition-normal);
}

.toggle-switch--on .toggle-knob {
  transform: translateX(20px);
}

.legal-description {
  margin: 0;
  color: var(--color-text-secondary);
  font-size: var(--text-sm);
  line-height: 1.7;
}

.legal-links {
  display: flex;
  flex-wrap: wrap;
  gap: 14px;
  margin-top: 12px;
}

.legal-links a {
  color: var(--color-brand-primary);
  font-size: var(--text-sm);
  text-decoration: none;
}

.legal-links a:hover {
  text-decoration: underline;
}

/* About Grid */
.about-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.about-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.about-label {
  font-size: var(--text-xs);
  color: var(--color-text-tertiary);
}

.about-value {
  font-size: var(--text-sm);
  color: var(--color-text-primary);
  font-weight: var(--weight-medium);
}

.about-value--mono {
  font-family: var(--font-mono);
}
</style>
