<script setup lang="ts">
import { computed } from "vue";
import { Sun, Moon, Monitor } from "lucide-vue-next";
import { useSettingsStore } from "../../stores/settings";
import type { ThemeMode } from "../../types";
import { useLocale } from "@/i18n";

const settingsStore = useSettingsStore();
const { t } = useLocale();

const currentMode = computed(() => settingsStore.themeMode);

const iconComponent = computed(() => {
  switch (currentMode.value) {
    case "light":
      return Sun;
    case "dark":
      return Moon;
    default:
      return Monitor;
  }
});

function cycleTheme() {
  const order: ThemeMode[] = ["light", "dark", "system"];
  const currentIndex = order.indexOf(currentMode.value);
  const nextIndex = (currentIndex + 1) % order.length;
  settingsStore.setThemeMode(order[nextIndex]);
}
</script>

<template>
  <button
    class="theme-toggle"
    :title="t('theme.current', { mode: t(`theme.${currentMode}`) })"
    @click="cycleTheme"
  >
    <Transition name="icon-fade" mode="out-in">
      <component :is="iconComponent" :key="currentMode" :size="16" />
    </Transition>
  </button>
</template>

<style scoped>
.theme-toggle {
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

.theme-toggle:hover {
  background: var(--color-hover);
  color: var(--color-text-secondary);
}

.theme-toggle:active {
  background: var(--color-active);
}

.icon-fade-enter-active,
.icon-fade-leave-active {
  transition: opacity 120ms ease, transform 120ms ease;
}

.icon-fade-enter-from {
  opacity: 0;
  transform: scale(0.8);
}

.icon-fade-leave-to {
  opacity: 0;
  transform: scale(0.8);
}
</style>
