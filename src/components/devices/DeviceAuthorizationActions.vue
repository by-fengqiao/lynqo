<script setup lang="ts">
import { Check, ShieldCheck, ShieldOff, X } from "lucide-vue-next";

interface Props {
  approved: boolean;
  trusted: boolean;
  pending?: boolean;
}

withDefaults(defineProps<Props>(), {
  pending: false,
});

const emit = defineEmits<{
  allowOnce: [];
  trust: [];
  reject: [];
}>();
</script>

<template>
  <div class="device-actions">
    <template v-if="approved">
      <span class="authorized-label">
        <ShieldCheck v-if="trusted" :size="14" />
        <Check v-else :size="14" />
        {{ trusted ? "受信任" : "本次已允许" }}
      </span>
      <button class="revoke-button" type="button" :disabled="pending" @click="emit('reject')">
        <ShieldOff :size="14" /> {{ trusted ? "撤销信任" : "结束本次授权" }}
      </button>
    </template>
    <template v-else>
      <button class="reject-button" type="button" :disabled="pending" @click="emit('reject')">
        <X :size="14" /> 拒绝
      </button>
      <button class="approve-button" type="button" :disabled="pending" @click="emit('allowOnce')">
        <Check :size="14" /> {{ pending ? "处理中…" : "仅本次允许" }}
      </button>
      <button class="trust-button" type="button" :disabled="pending" @click="emit('trust')">
        <ShieldCheck :size="14" /> 信任并允许
      </button>
    </template>
  </div>
</template>

<style scoped>
.device-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin-top: 16px;
}

.authorized-label,
.approve-button,
.trust-button,
.reject-button,
.revoke-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  min-height: 32px;
  padding: 0 12px;
  border-radius: var(--radius-sm);
  font-size: var(--text-xs);
  font-weight: var(--weight-medium);
}

.authorized-label {
  color: var(--color-state-success);
  background: var(--color-state-success-soft);
}

.approve-button {
  color: var(--color-text-inverse);
  background: var(--color-brand-primary);
  border: 1px solid var(--color-brand-primary);
}

.trust-button {
  color: var(--color-text-brand);
  background: var(--color-brand-primary-soft);
  border: 1px solid color-mix(in srgb, var(--color-brand-primary) 28%, transparent);
}

.reject-button,
.revoke-button {
  color: var(--color-text-secondary);
  background: var(--color-surface-card);
  border: 1px solid var(--color-border);
}

.approve-button:disabled,
.trust-button:disabled,
.reject-button:disabled,
.revoke-button:disabled {
  cursor: wait;
  opacity: 0.6;
}
</style>
