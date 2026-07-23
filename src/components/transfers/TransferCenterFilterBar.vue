<script setup lang="ts">
export type TransferCenterFilter = "all" | "active" | "completed" | "attention";

interface Props {
  modelValue: TransferCenterFilter;
  counts: Record<TransferCenterFilter, number>;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  "update:modelValue": [value: TransferCenterFilter];
}>();

const filters: { value: TransferCenterFilter; label: string }[] = [
  { value: "all", label: "全部" },
  { value: "active", label: "进行中" },
  { value: "completed", label: "已完成" },
  { value: "attention", label: "需处理" },
];
</script>

<template>
  <nav class="filter-bar" aria-label="传输状态筛选">
    <button
      v-for="filter in filters"
      :key="filter.value"
      class="filter-button"
      :class="{ 'filter-button--active': props.modelValue === filter.value }"
      :aria-pressed="props.modelValue === filter.value"
      type="button"
      @click="emit('update:modelValue', filter.value)"
    >
      <span>{{ filter.label }}</span>
      <span class="filter-count">{{ props.counts[filter.value] }}</span>
    </button>
  </nav>
</template>

<style scoped>
.filter-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 16px;
}

.filter-button {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  min-height: 34px;
  padding: 0 12px;
  color: var(--color-text-secondary);
  background: var(--color-surface-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-full);
  cursor: pointer;
  font-size: var(--text-sm);
  font-weight: var(--weight-medium);
  transition: background var(--transition-fast), border-color var(--transition-fast), color var(--transition-fast);
}

.filter-button:hover {
  color: var(--color-text-primary);
  background: var(--color-hover);
}

.filter-button--active {
  color: var(--color-text-brand);
  background: var(--color-brand-primary-soft);
  border-color: color-mix(in srgb, var(--color-brand-primary) 36%, transparent);
}

.filter-count {
  min-width: 18px;
  padding: 1px 5px;
  color: inherit;
  background: color-mix(in srgb, currentColor 10%, transparent);
  border-radius: var(--radius-full);
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  text-align: center;
}
</style>
