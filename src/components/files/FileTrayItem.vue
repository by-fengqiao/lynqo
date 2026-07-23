<script setup lang="ts">
// 桌面端待发送文件托盘项（名称 + 大小 + 移除 + 预览）
import { X, Eye } from "lucide-vue-next";
import FileIcon from "./FileIcon.vue";
import type { TransferFile } from "@/types";
import { formatBytes } from "@/utils/format";

defineProps<{ file: TransferFile }>();
const emit = defineEmits<{ remove: []; preview: [] }>();
</script>

<template>
  <div
    class="flex items-center gap-3 px-3 py-2 rounded-md transition-colors row"
    style="transition: background var(--transition-fast)"
  >
    <FileIcon :name="file.name" :mimeType="file.mimeType" :size="20" />
    <span class="flex-1 cell-truncate" style="font-size: var(--text-sm); color: var(--color-text-primary)">{{ file.name }}</span>
    <span style="font-size: var(--text-xs); color: var(--color-text-tertiary); font-family: var(--font-mono)">{{
      formatBytes(file.size)
    }}</span>
    <button
      class="icon-btn"
      style="width: 24px; height: 24px"
      aria-label="移除"
      title="移除"
      @click="emit('remove')"
    >
      <X :size="14" :stroke-width="2.5" />
    </button>
    <button
      class="icon-btn"
      style="width: 24px; height: 24px"
      aria-label="预览"
      title="预览"
      @click="emit('preview')"
    >
      <Eye :size="14" :stroke-width="2" />
    </button>
  </div>
</template>

<style scoped>
.row:hover {
  background: var(--color-surface-secondary);
}
</style>
