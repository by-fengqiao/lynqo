<script setup lang="ts">
import { CheckCircle2, MonitorSmartphone, Router } from "lucide-vue-next";
import type { ConnectionAddress } from "@/services/tauri";
import { useLocale } from "@/i18n";

defineProps<{
  addresses: readonly ConnectionAddress[];
  selectedIp: string;
  loading: boolean;
}>();

const emit = defineEmits<{
  select: [ip: string];
}>();

const { t } = useLocale();
</script>

<template>
  <section v-if="addresses.length" class="address-picker">
    <div class="address-heading">
      <span>{{ t("connect.addressPicker.title") }}</span>
      <small>{{ t("connect.addressPicker.description") }}</small>
    </div>
    <div class="address-list">
      <button
        v-for="address in addresses"
        :key="`${address.interfaceName}-${address.ip}`"
        type="button"
        class="address-option"
        :class="{ selected: selectedIp === address.ip }"
        :disabled="loading"
        :aria-pressed="selectedIp === address.ip"
        @click="emit('select', address.ip)"
      >
        <MonitorSmartphone v-if="address.kind === 'hotspot'" :size="17" />
        <Router v-else :size="17" />
        <span class="address-copy">
          <strong>{{ t(`connect.addressKind.${address.kind}`) }}</strong>
          <small>{{ address.interfaceName }} · {{ address.ip }}</small>
        </span>
        <CheckCircle2 v-if="selectedIp === address.ip" :size="16" class="selected-icon" />
      </button>
    </div>
  </section>
</template>

<style scoped>
.address-picker {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 14px;
}

.address-heading {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 10px;
  color: var(--color-text-secondary);
  font-size: var(--text-xs);
}

.address-heading small {
  color: var(--color-text-tertiary);
  font-size: 10px;
  text-align: right;
}

.address-list {
  display: grid;
  gap: 7px;
}

.address-option {
  display: grid;
  grid-template-columns: 20px 1fr 18px;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 9px 10px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-surface-card);
  color: var(--color-text-secondary);
  text-align: left;
  cursor: pointer;
}

.address-option:hover:not(:disabled) {
  border-color: var(--color-brand-primary);
  background: var(--color-hover);
}

.address-option.selected {
  border-color: var(--color-brand-primary);
  background: var(--color-brand-primary-soft);
  color: var(--color-text-brand);
}

.address-option:disabled {
  opacity: 0.6;
  cursor: default;
}

.address-copy {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 2px;
}

.address-copy strong {
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
}

.address-copy small {
  overflow: hidden;
  color: var(--color-text-tertiary);
  font-family: var(--font-mono);
  font-size: 10px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.selected-icon {
  color: var(--color-brand-primary);
}
</style>
