import { defineStore } from "pinia";
import { ref, computed, shallowRef } from "vue";
import type { ThemeMode } from "../types";
import {
  isTauri,
  getSettings,
  updateSettings,
  openReceiveFolder,
  getAutostartEnabled,
  setAutostart as setAutostartNative,
  getCloseBehavior,
  setCloseBehavior as setCloseBehaviorNative,
  type CloseBehavior,
} from "@/services/tauri";

// The Tauri backend is the source of truth after fetchSettings() resolves.
let persistedThemeMode: ThemeMode = "system";

export const useSettingsStore = defineStore("settings", () => {
  const themeMode = ref<ThemeMode>(persistedThemeMode);
  // Do not expose another user's local path before the backend returns settings.
  const receiveFolder = ref<string>("");
  const requireApproval = ref<boolean>(true);
  // Zero means no client-side limit until the persisted backend setting is loaded.
  const maxFileSize = ref<number>(0);
  const autostartEnabled = ref(false);
  const closeBehavior = ref<CloseBehavior>("minimize");

  let mediaQuery: MediaQueryList | null = null;
  let mediaListener: ((e: MediaQueryListEvent) => void) | null = null;

  // Tracks the OS preference so "system" mode stays reactive
  const systemPrefersDark = shallowRef(
    window.matchMedia("(prefers-color-scheme: dark)").matches
  );

  // The theme actually applied to the UI ("system" resolved via matchMedia)
  const resolvedTheme = computed<"light" | "dark">(() => {
    if (themeMode.value === "system") {
      return systemPrefersDark.value ? "dark" : "light";
    }
    return themeMode.value;
  });

  function getResolvedTheme(): "light" | "dark" {
    return resolvedTheme.value;
  }

  function applyTheme() {
    const resolved = getResolvedTheme();
    document.documentElement.dataset.theme = resolved;

    // Listen for system preference changes when in system mode
    if (themeMode.value === "system") {
      if (!mediaQuery) {
        mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
      }
      // Remove old listener
      if (mediaListener) {
        mediaQuery.removeEventListener("change", mediaListener);
      }
      mediaListener = (e: MediaQueryListEvent) => {
        document.documentElement.dataset.theme = e.matches ? "dark" : "light";
      };
      mediaQuery.addEventListener("change", mediaListener);
    } else {
      // Clean up system listener when not in system mode
      if (mediaQuery && mediaListener) {
        mediaQuery.removeEventListener("change", mediaListener);
        mediaListener = null;
      }
    }
  }

  function setThemeMode(mode: ThemeMode) {
    themeMode.value = mode;
    persistedThemeMode = mode;
    applyTheme();
    void saveSettings();
  }

  /**
   * Follow OS theme changes while in "system" mode.
   * Returns a cleanup function that removes the listener.
   */
  function watchSystemTheme(): () => void {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = (e: MediaQueryListEvent) => {
      systemPrefersDark.value = e.matches;
      if (themeMode.value === "system") {
        applyTheme();
      }
    };
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }

  function setReceiveFolder(path: string) {
    receiveFolder.value = path;
    void saveSettings();
  }

  function setRequireApproval(value: boolean) {
    requireApproval.value = value;
    void saveSettings();
  }

  function setMaxFileSize(bytes: number) {
    maxFileSize.value = bytes;
    void saveSettings();
  }

  async function setAutostart(value: boolean) {
    if (!isTauri()) {
      autostartEnabled.value = value;
      return;
    }
    const result = await setAutostartNative(value);
    if (result.success) autostartEnabled.value = value;
  }

  async function setCloseBehavior(value: CloseBehavior) {
    if (!isTauri()) {
      closeBehavior.value = value;
      return;
    }
    const result = await setCloseBehaviorNative(value);
    if (result.success) closeBehavior.value = value;
  }

  /**
   * Fetch settings from the Tauri backend.
   * In browser mode, keeps neutral in-memory values because it has no local
   * Tauri settings database to query.
   */
  async function fetchSettings() {
    if (isTauri()) {
      try {
        const settings = await getSettings();
        if (settings.receiveFolder) {
          receiveFolder.value = settings.receiveFolder as string;
        }
        if (settings.requireApproval !== undefined) {
          requireApproval.value = settings.requireApproval as boolean;
        }
        if (settings.maxFileSize !== undefined) {
          maxFileSize.value = settings.maxFileSize as number;
        }
        if (settings.themeMode) {
          themeMode.value = settings.themeMode as ThemeMode;
          persistedThemeMode = settings.themeMode as ThemeMode;
          applyTheme();
        }
        try {
          autostartEnabled.value = await getAutostartEnabled();
          closeBehavior.value = await getCloseBehavior();
        } catch (err) {
          console.error("[settings] Failed to fetch lifecycle settings:", err);
        }
      } catch (err) {
        console.error("[settings] Failed to fetch settings:", err);
      }
    }
  }

  /**
   * Save current settings to the Tauri backend.
   */
  async function saveSettings() {
    if (isTauri()) {
      try {
        const result = await updateSettings({
          receiveFolder: receiveFolder.value,
          requireApproval: requireApproval.value,
          maxFileSize: maxFileSize.value,
          themeMode: themeMode.value,
        });
        if (!result.success) throw new Error(result.error ?? "保存设置失败");
      } catch (err) {
        console.error("[settings] Failed to save settings:", err);
      }
    }
    // In browser mode, settings are kept in memory only
  }

  /**
   * Open the receive folder in the system file manager via Tauri.
   */
  async function openFolder() {
    if (isTauri()) {
      try {
        const result = await openReceiveFolder();
        if (!result.success) throw new Error(result.error ?? "打开接收文件夹失败");
      } catch (err) {
        console.error("[settings] Failed to open receive folder:", err);
      }
    }
  }

  return {
    // State
    themeMode,
    receiveFolder,
    requireApproval,
    maxFileSize,
    autostartEnabled,
    closeBehavior,
    // Theme actions (unchanged)
    resolvedTheme,
    getResolvedTheme,
    applyTheme,
    setThemeMode,
    watchSystemTheme,
    // Setting actions
    setReceiveFolder,
    setRequireApproval,
    setMaxFileSize,
    setAutostart,
    setCloseBehavior,
    // Tauri-backed actions
    fetchSettings,
    saveSettings,
    openFolder,
  };
});
