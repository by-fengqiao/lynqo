// 主题管理：light / dark / system，持久化、跟随系统、自动更新
import { onMounted, onUnmounted } from "vue";
import { useSettingsStore } from "@/stores/settings";

export function useTheme() {
  const settings = useSettingsStore();

  let unwatch: (() => void) | undefined;

  function cycleTheme() {
    const order = ["light", "dark", "system"] as const;
    const idx = order.indexOf(settings.themeMode);
    settings.setThemeMode(order[(idx + 1) % order.length]);
  }

  onMounted(() => {
    settings.applyTheme();
    unwatch = settings.watchSystemTheme();
  });

  onUnmounted(() => {
    unwatch?.();
  });

  return {
    themeMode: settings.themeMode,
    resolvedTheme: settings.resolvedTheme,
    setThemeMode: settings.setThemeMode,
    cycleTheme,
  };
}
