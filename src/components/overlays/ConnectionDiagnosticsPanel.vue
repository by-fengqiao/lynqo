<script setup lang="ts">
import { computed, type DeepReadonly } from "vue";
import { AlertTriangle, CheckCircle2, CircleHelp, LoaderCircle, RefreshCw, ShieldCheck } from "lucide-vue-next";
import type { ConnectionDiagnostics } from "@/services/tauri";
import { useLocale } from "@/i18n";

const props = defineProps<{
  diagnostics: DeepReadonly<ConnectionDiagnostics> | null;
  loading: boolean;
  error: string | null;
  firewallPending: boolean;
}>();

const emit = defineEmits<{
  retry: [];
  configureFirewall: [];
}>();

const { t } = useLocale();
type CheckStatus = "pass" | "fail" | "unknown";

const checks = computed(() => {
  const diagnostics = props.diagnostics;
  if (!diagnostics) return [];
  const statusOf = (value: boolean | null | undefined): CheckStatus =>
    value == null ? "unknown" : value ? "pass" : "fail";
  const serviceCheckStatus: CheckStatus = diagnostics.serviceStatus === "running"
    ? "pass"
    : ["starting", "stopping"].includes(diagnostics.serviceStatus)
      ? "unknown"
      : "fail";
  const firewallStatus: CheckStatus = ["allowed", "disabled"].includes(diagnostics.firewallStatus)
    ? "pass"
    : diagnostics.firewallStatus === "missing"
      ? "fail"
      : "unknown";
  const firewallDetailKey = `connect.diagnostics.firewall${diagnostics.firewallStatus.charAt(0).toUpperCase()}${diagnostics.firewallStatus.slice(1)}`;

  return [
    {
      key: "service",
      label: t("connect.diagnostics.service"),
      status: serviceCheckStatus,
      detail: t(`connect.service.${diagnostics.serviceStatus}`),
    },
    {
      key: "loopback",
      label: t("connect.diagnostics.loopback"),
      status: statusOf(diagnostics.loopbackReachable),
      detail: diagnostics.bindAddress,
    },
    {
      key: "lan",
      label: t("connect.diagnostics.lan"),
      status: statusOf(diagnostics.lanAddressReachable),
      detail: diagnostics.localUrl ?? t("connect.unavailable"),
    },
    {
      key: "mdns",
      label: t("connect.diagnostics.mdns"),
      status: diagnostics.serviceStatus === "running"
        ? statusOf(diagnostics.mdnsAdvertised)
        : "unknown" as CheckStatus,
      detail: t("connect.diagnostics.optional"),
    },
    {
      key: "firewall",
      label: t("connect.diagnostics.firewall"),
      status: firewallStatus,
      detail: t(firewallDetailKey),
    },
  ];
});

const selectedInterface = computed(() =>
  props.diagnostics?.interfaces.find((entry) => entry.selected)
);

function warningText(code: string) {
  return t(`connect.warning.${code}`);
}
</script>

<template>
  <section class="diagnostics" aria-live="polite">
    <div class="diagnostics-header">
      <span>{{ t("connect.diagnostics.title") }}</span>
      <button type="button" :disabled="loading" @click="emit('retry')">
        <RefreshCw :size="13" :class="{ spin: loading }" />
        {{ t("connect.diagnostics.retry") }}
      </button>
    </div>

    <div v-if="loading && !diagnostics" class="diagnostics-empty">
      <LoaderCircle :size="16" class="spin" />
      {{ t("connect.diagnostics.running") }}
    </div>
    <div v-else-if="error" class="diagnostics-error">
      <AlertTriangle :size="15" />
      <span>{{ error }}</span>
    </div>
    <template v-else-if="diagnostics">
      <ul class="check-list">
        <li v-for="check in checks" :key="check.key">
          <CheckCircle2 v-if="check.status === 'pass'" :size="15" class="pass" />
          <AlertTriangle v-else-if="check.status === 'fail'" :size="15" class="fail" />
          <CircleHelp v-else :size="15" class="unknown" />
          <span class="check-label">{{ check.label }}</span>
          <span class="check-detail">{{ check.detail }}</span>
        </li>
      </ul>
      <p v-if="selectedInterface" class="selected-interface">
        {{ t("connect.diagnostics.interface") }}:
        <strong>{{ selectedInterface.name }}</strong> · {{ selectedInterface.ip }}
      </p>
      <button
        v-if="diagnostics.firewallStatus === 'missing'"
        type="button"
        class="firewall-button"
        :disabled="firewallPending"
        @click="emit('configureFirewall')"
      >
        <LoaderCircle v-if="firewallPending" :size="14" class="spin" />
        <ShieldCheck v-else :size="14" />
        {{ t("connect.diagnostics.configureFirewall") }}
      </button>
      <ul v-if="diagnostics.warnings.length" class="warning-list">
        <li v-for="warning in diagnostics.warnings" :key="warning">
          {{ warningText(warning) }}
        </li>
      </ul>
    </template>
  </section>
</template>

<style scoped>
.diagnostics {
  padding-top: 14px;
  border-top: 1px solid var(--color-border);
}

.diagnostics-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 9px;
  color: var(--color-text-primary);
  font-size: var(--text-sm);
  font-weight: var(--weight-semibold);
}

.diagnostics-header button {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  border: 0;
  background: transparent;
  color: var(--color-text-brand);
  font: inherit;
  font-size: var(--text-xs);
  cursor: pointer;
}

.diagnostics-header button:disabled {
  opacity: 0.55;
  cursor: default;
}

.diagnostics-empty,
.diagnostics-error {
  display: flex;
  align-items: center;
  gap: 7px;
  color: var(--color-text-tertiary);
  font-size: var(--text-xs);
}

.diagnostics-error {
  color: var(--color-state-error);
}

.check-list,
.warning-list {
  display: flex;
  flex-direction: column;
  gap: 7px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.check-list li {
  display: grid;
  grid-template-columns: 16px minmax(90px, auto) 1fr;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.pass { color: var(--color-state-success); }
.fail { color: var(--color-state-error); }
.unknown { color: var(--color-state-warning); }

.check-label {
  color: var(--color-text-secondary);
  font-size: var(--text-xs);
}

.check-detail {
  overflow: hidden;
  color: var(--color-text-tertiary);
  font-family: var(--font-mono);
  font-size: 11px;
  text-align: right;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.selected-interface {
  margin: 9px 0 0;
  color: var(--color-text-tertiary);
  font-size: 11px;
}

.firewall-button {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  margin-top: 9px;
  padding: 6px 9px;
  border: 1px solid var(--color-brand-primary);
  border-radius: var(--radius-sm);
  background: var(--color-brand-primary-soft);
  color: var(--color-text-brand);
  font: inherit;
  font-size: var(--text-xs);
  cursor: pointer;
}

.firewall-button:disabled {
  opacity: 0.6;
  cursor: default;
}

.warning-list {
  gap: 4px;
  margin-top: 9px;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  background: var(--color-state-warning-soft);
  color: var(--color-text-secondary);
  font-size: 11px;
  line-height: 1.45;
}

.spin {
  animation: diagnostics-spin 0.9s linear infinite;
}

@keyframes diagnostics-spin {
  to { transform: rotate(360deg); }
}
</style>
