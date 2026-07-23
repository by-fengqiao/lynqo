<script setup lang="ts">
import { computed, ref, onMounted, watch } from "vue";
import { storeToRefs } from "pinia";
import { RouterLink, RouterView, useRoute } from "vue-router";
import { Sun, Moon, Menu, Send, ArrowLeftRight } from "lucide-vue-next";
import { useSettingsStore } from "../stores/settings";
import { useMobileSessionStore } from "@/stores/mobileSession";
import AppLogo from "../components/common/AppLogo.vue";
import ReceiveRequestDialog from "@/components/overlays/ReceiveRequestDialog.vue";
import { fetchHostStatus } from "../services/api";
import { useLocale } from "@/i18n";

const settingsStore = useSettingsStore();
const mobileSession = useMobileSessionStore();
const route = useRoute();
const { t } = useLocale();
const {
  connectionError,
  isReady,
  receiveError,
  pendingReceiveTransfer,
  showReceiveDialog,
} = storeToRefs(mobileSession);

const showMenu = ref(false);
const navItems = computed(() => [
  { name: "mobile-send", label: t("mobile.send"), icon: Send },
  { name: "mobile-transfers", label: t("mobile.transfers"), icon: ArrowLeftRight },
] as const);
const hostName = ref("");

const connectionLabel = computed(() => {
  if (connectionError.value) return t("mobile.connectionFailed");
  return isReady.value
    ? t("mobile.connectedTo", { name: hostName.value || t("mobile.host") })
    : t("mobile.connecting");
});

onMounted(async () => {
  settingsStore.applyTheme();
  try {
    const status = await fetchHostStatus();
    if (status.name) {
      hostName.value = status.name;
    }
  } catch {
    // Keep the default label if the host status cannot be fetched.
  }
});

watch(
  () => route.query.token,
  (token) => {
    void mobileSession.initialize(typeof token === "string" ? token : null);
  },
  { immediate: true }
);

function toggleTheme() {
  const current = settingsStore.themeMode;
  if (current === "light") {
    settingsStore.setThemeMode("dark");
  } else {
    settingsStore.setThemeMode("light");
  }
}

function toggleMenu() {
  showMenu.value = !showMenu.value;
}

function closeMenu() {
  showMenu.value = false;
}
</script>

<template>
  <div class="mobile-layout">
    <!-- Top Bar -->
    <header class="mobile-topbar">
      <div class="topbar-left">
        <AppLogo :size="24" />
        <span class="logo-text">
          <span>LYN</span><span class="logo-q">Q</span><span>O</span>
        </span>
      </div>

      <div class="topbar-center">
        <span class="status-dot" :class="{ 'status-dot--offline': !!connectionError }"></span>
        <span class="status-text">{{ connectionLabel }}</span>
      </div>

      <div class="topbar-right">
        <button class="icon-btn" :title="t('mobile.toggleTheme')" @click="toggleTheme">
          <Sun v-if="settingsStore.getResolvedTheme() === 'dark'" :size="16" />
          <Moon v-else :size="16" />
        </button>
        <button class="icon-btn" :title="t('mobile.menu')" @click="toggleMenu">
          <Menu :size="16" />
        </button>
      </div>
    </header>

    <Teleport to="body">
      <div v-if="showMenu" class="mobile-menu-layer" @click.self="closeMenu">
        <button class="menu-backdrop" :aria-label="t('mobile.closeMenu')" @click="closeMenu" />
        <nav class="mobile-menu" :aria-label="t('mobile.navigation')">
          <RouterLink
            v-for="item in navItems"
            :key="item.name"
            class="menu-link"
            :to="{ name: item.name, query: route.query }"
            @click="closeMenu"
          >
            <component :is="item.icon" :size="18" />
            <span>{{ item.label }}</span>
          </RouterLink>
        </nav>
      </div>
    </Teleport>

    <!-- Main Content -->
    <main class="mobile-content">
      <p v-if="connectionError" class="session-error">{{ connectionError }}</p>
      <p v-if="receiveError" class="session-error">{{ receiveError }}</p>
      <RouterView />
    </main>

    <ReceiveRequestDialog
      :visible="showReceiveDialog"
      :transfer="pendingReceiveTransfer"
      @accept="mobileSession.acceptIncomingTransfer"
      @reject="mobileSession.rejectIncomingTransfer"
    />
  </div>
</template>

<style scoped>
.mobile-layout {
  min-height: 100vh;
  background: var(--color-surface-page);
}

/* ─── Top Bar ─── */
.mobile-topbar {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  background: color-mix(in srgb, var(--color-surface-card) 85%, transparent);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border-bottom: 1px solid var(--color-border);
  z-index: var(--z-sticky);
}

.topbar-left {
  display: flex;
  align-items: center;
  gap: 6px;
}

.logo-q {
  color: var(--color-primary, #246BFF);
}

.logo-text {
  font-size: var(--text-base);
  font-weight: var(--weight-semibold);
  color: var(--color-text-primary);
  letter-spacing: 0.02em;
}

.topbar-center {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  display: flex;
  align-items: center;
  gap: 5px;
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: var(--radius-full);
  background: var(--color-state-success);
}

.status-dot--offline {
  background: var(--color-state-error);
}

.status-text {
  font-size: var(--text-xs);
  color: var(--color-text-secondary);
  white-space: nowrap;
}

.topbar-right {
  display: flex;
  align-items: center;
  gap: 4px;
}

.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: none;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--color-text-tertiary);
  cursor: pointer;
  transition: background var(--transition-fast), color var(--transition-fast);
}

.icon-btn:hover {
  background: var(--color-hover);
  color: var(--color-text-secondary);
}

.icon-btn:active {
  background: var(--color-active);
}

/* ─── Main Content ─── */
.mobile-menu-layer {
  position: fixed;
  inset: 0;
  z-index: var(--z-modal);
}

.menu-backdrop {
  position: absolute;
  inset: 0;
  width: 100%;
  border: 0;
  background: rgba(0, 0, 0, 0.25);
}

.mobile-menu {
  position: absolute;
  top: calc(48px + env(safe-area-inset-top));
  right: 8px;
  display: flex;
  flex-direction: column;
  min-width: 156px;
  padding: 6px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface-card);
  box-shadow: var(--shadow-lg);
}

.menu-link {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 42px;
  padding: 0 12px;
  border-radius: var(--radius-sm);
  color: var(--color-text-primary);
  font-size: var(--text-sm);
  text-decoration: none;
}

.menu-link.router-link-exact-active {
  color: var(--color-brand-primary);
  background: var(--color-brand-primary-soft);
}

.mobile-content {
  max-width: 375px;
  margin: 0 auto;
  padding-top: 48px;
  min-height: 100vh;
}

.session-error {
  max-width: 375px;
  margin: 12px auto 0;
  padding: 10px 12px;
  border-radius: var(--radius-md);
  background: var(--color-state-error-soft);
  color: var(--color-state-error);
  font-size: var(--text-sm);
  line-height: 1.5;
}
</style>
