import { readonly, shallowRef } from "vue";
import {
  getConnectionDiagnostics,
  isTauri,
  type ConnectionDiagnostics,
} from "@/services/tauri";
import { useLocale } from "@/i18n";

export function useConnectionDiagnostics() {
  const { t } = useLocale();
  const diagnostics = shallowRef<ConnectionDiagnostics | null>(null);
  const loading = shallowRef(false);
  const error = shallowRef<string | null>(null);

  async function refresh(ip?: string) {
    if (!isTauri()) {
      diagnostics.value = null;
      error.value = t("connect.diagnostics.desktopOnly");
      return;
    }

    loading.value = true;
    error.value = null;
    try {
      diagnostics.value = await getConnectionDiagnostics(ip);
    } catch (reason) {
      diagnostics.value = null;
      error.value = reason instanceof Error ? reason.message : t("connect.diagnostics.failed");
    } finally {
      loading.value = false;
    }
  }

  return {
    diagnostics: readonly(diagnostics),
    loading: readonly(loading),
    error: readonly(error),
    refresh,
  };
}
