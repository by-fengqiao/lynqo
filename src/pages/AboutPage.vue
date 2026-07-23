<script setup lang="ts">
import { useRouter } from "vue-router";
import { ArrowLeft, BookOpen, Shield } from "lucide-vue-next";
import AppLogo from "../components/common/AppLogo.vue";
import { useAppStore } from "@/stores/app";

const router = useRouter();
const appStore = useAppStore();

function goBack() {
  router.back();
}

const thirdPartyLibs = [
  { name: "Vue", license: "MIT", url: "https://vuejs.org" },
  { name: "Tauri", license: "MIT / Apache-2.0", url: "https://tauri.app" },
  { name: "Lucide Icons", license: "ISC", url: "https://lucide.dev" },
  { name: "Axum", license: "MIT", url: "https://github.com/tokio-rs/axum" },
];
</script>

<template>
  <div class="about-page">
    <button class="back-btn" @click="goBack">
      <ArrowLeft :size="16" />
      <span>返回</span>
    </button>

    <div class="about-card">
      <!-- Logo & Name -->
      <div class="about-header">
        <AppLogo :size="64" />
        <h1 class="app-name">
          LYN<span class="app-name-q">Q</span>O
        </h1>
        <span class="app-version">v{{ appStore.appVersion }}</span>
      </div>

      <!-- Description -->
      <p class="app-desc">连接附近，自由传输</p>
      <p class="app-tagline">CONNECT NEARBY. TRANSFER FREELY.</p>

      <div class="about-links">
        <RouterLink to="/help" class="about-link">
          <BookOpen :size="14" />
          <span>使用帮助</span>
        </RouterLink>
        <RouterLink to="/legal/privacy" class="about-link">
          <Shield :size="14" />
          <span>隐私说明</span>
        </RouterLink>
        <RouterLink to="/legal/terms" class="about-link">使用协议</RouterLink>
        <RouterLink to="/legal/disclaimer" class="about-link">免责声明</RouterLink>
      </div>

      <!-- Third-party credits -->
      <div class="about-credits">
        <h3 class="credits-title">第三方组件</h3>
        <div class="credits-list">
          <a
            v-for="lib in thirdPartyLibs"
            :key="lib.name"
            :href="lib.url"
            target="_blank"
            rel="noopener"
            class="credit-item"
          >
            <span class="credit-name">{{ lib.name }}</span>
            <span class="credit-license">{{ lib.license }}</span>
          </a>
        </div>
      </div>

      <!-- Privacy note -->
      <p class="privacy-note">
        文件仅在局域网设备之间传输，不会上传到任何服务器。
      </p>
    </div>
  </div>
</template>

<style scoped>
.about-page {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding-top: 16px;
}

.back-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  align-self: flex-start;
  padding: 6px 12px;
  font-size: var(--text-sm);
  color: var(--color-text-secondary);
  background: transparent;
  border: none;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background var(--transition-fast), color var(--transition-fast);
}

.back-btn:hover {
  background: var(--color-hover);
  color: var(--color-text-primary);
}

.about-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  max-width: 420px;
  width: 100%;
  margin-top: 32px;
  padding: 40px 32px;
  background: var(--color-surface-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
}

.about-header {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}

.app-name {
  font-size: 28px;
  font-weight: var(--weight-bold);
  color: var(--color-text-primary);
  letter-spacing: 0.04em;
  margin: 0;
}

.app-name-q {
  color: var(--color-brand-primary, #246BFF);
}

.app-version {
  font-size: var(--text-xs);
  color: var(--color-text-tertiary);
  background: var(--color-surface-inset);
  padding: 2px 10px;
  border-radius: var(--radius-full);
}

.app-desc {
  margin-top: 16px;
  font-size: var(--text-base);
  color: var(--color-text-secondary);
}

.app-tagline {
  margin-top: 4px;
  font-size: var(--text-xs);
  color: var(--color-text-tertiary);
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.about-links {
  display: flex;
  gap: 16px;
  margin-top: 24px;
}

.about-link {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: var(--text-sm);
  color: var(--color-text-secondary);
  text-decoration: none;
  transition: color var(--transition-fast);
}

.about-link:hover {
  color: var(--color-brand-primary);
}

.about-credits {
  width: 100%;
  margin-top: 28px;
  padding-top: 20px;
  border-top: 1px solid var(--color-border);
}

.credits-title {
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
  color: var(--color-text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.06em;
  margin: 0 0 12px;
}

.credits-list {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.credit-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  background: var(--color-surface-inset);
  text-decoration: none;
  transition: background var(--transition-fast);
}

.credit-item:hover {
  background: var(--color-hover);
}

.credit-name {
  font-size: var(--text-sm);
  color: var(--color-text-primary);
}

.credit-license {
  font-size: var(--text-xs);
  color: var(--color-text-tertiary);
}

.privacy-note {
  margin-top: 24px;
  font-size: var(--text-xs);
  color: var(--color-text-tertiary);
  text-align: center;
  line-height: 1.6;
}
</style>
