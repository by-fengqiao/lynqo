<script setup lang="ts">
import { Languages } from "lucide-vue-next";
import { useLocale, type Locale } from "@/i18n";

interface Props {
  modelValue: Locale;
}

const props = defineProps<Props>();
const emit = defineEmits<{
  updateModelValue: [locale: Locale];
}>();
const { t } = useLocale();

const options: { value: Locale; labelKey: string }[] = [
  { value: "zh-CN", labelKey: "language.zh" },
  { value: "en-US", labelKey: "language.en" },
];
</script>

<template>
  <section class="language-section">
    <div class="section-header">
      <Languages :size="16" class="section-icon" />
      <div>
        <h2 class="section-title">{{ t("language.title") }}</h2>
        <p class="section-description">{{ t("language.description") }}</p>
      </div>
    </div>
    <div class="language-options" role="radiogroup" :aria-label="t('language.title')">
      <label
        v-for="option in options"
        :key="option.value"
        class="language-option"
        :class="{ 'language-option--active': props.modelValue === option.value }"
      >
        <input
          class="language-radio"
          type="radio"
          name="language"
          :value="option.value"
          :checked="props.modelValue === option.value"
          @change="emit('updateModelValue', option.value)"
        />
        <span>{{ t(option.labelKey) }}</span>
      </label>
    </div>
  </section>
</template>

<style scoped>
.language-section { padding: 4px 0; }
.section-header { display: flex; align-items: flex-start; gap: 8px; margin-bottom: 16px; }
.section-icon { flex-shrink: 0; margin-top: 2px; color: var(--color-brand-primary); }
.section-title { margin: 0; color: var(--color-text-primary); font-size: var(--text-base); font-weight: var(--weight-semibold); }
.section-description { margin: 3px 0 0; color: var(--color-text-secondary); font-size: var(--text-sm); line-height: 1.5; }
.language-options { display: flex; flex-wrap: wrap; gap: 10px; }
.language-option { display: inline-flex; align-items: center; min-height: 38px; padding: 0 14px; border: 1px solid var(--color-border); border-radius: var(--radius-md); color: var(--color-text-primary); cursor: pointer; font-size: var(--text-sm); transition: border-color var(--transition-fast), background var(--transition-fast); }
.language-option:hover { border-color: var(--color-border-strong); background: var(--color-hover); }
.language-option--active { border-color: var(--color-brand-primary); background: var(--color-selected); color: var(--color-text-brand); }
.language-radio { position: absolute; opacity: 0; pointer-events: none; }
</style>
