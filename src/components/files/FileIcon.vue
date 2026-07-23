<script setup lang="ts">
// 文件图标：根据扩展名返回对应的 Lucide 图标 + 色调
import { computed } from "vue";
import {
  Film,
  Image as ImageIcon,
  Archive,
  FileText,
  Music,
  File as FileIcon,
} from "lucide-vue-next";
import type { Component } from "vue";

const props = defineProps<{
  name: string;
  mimeType?: string;
  size?: number;
}>();

interface IconMeta {
  icon: Component;
  color: string;
  bg: string;
}

function ext(name: string): string {
  const i = name.lastIndexOf(".");
  return i < 0 ? "" : name.slice(i + 1).toLowerCase();
}

const meta = computed<IconMeta>(() => {
  const e = ext(props.name);
  const mt = props.mimeType ?? "";
  if (["mp4", "mov", "avi", "mkv", "webm"].includes(e) || mt.startsWith("video/")) {
    return {
      icon: Film,
      color: "var(--state-info)",
      bg: "rgba(91,141,239,0.10)",
    };
  }
  if (["png", "jpg", "jpeg", "gif", "webp", "heic", "svg"].includes(e) || mt.startsWith("image/")) {
    return {
      icon: ImageIcon,
      color: "var(--state-success)",
      bg: "rgba(62,175,124,0.10)",
    };
  }
  if (["zip", "rar", "7z", "tar", "gz"].includes(e)) {
    return {
      icon: Archive,
      color: "var(--state-warning)",
      bg: "rgba(212,160,23,0.10)",
    };
  }
  if (["mp3", "wav", "flac", "aac", "m4a"].includes(e) || mt.startsWith("audio/")) {
    return {
      icon: Music,
      color: "var(--state-info)",
      bg: "rgba(91,141,239,0.10)",
    };
  }
  if (["pdf", "doc", "docx", "txt", "md", "ppt", "pptx", "xls", "xlsx"].includes(e)) {
    return {
      icon: FileText,
      color: "var(--state-warning)",
      bg: "rgba(212,160,23,0.10)",
    };
  }
  return {
    icon: FileIcon,
    color: "var(--color-text-tertiary)",
    bg: "var(--color-surface-secondary)",
  };
});

const iconSize = props.size ?? 20;
</script>

<template>
  <component
    :is="meta.icon"
    :size="iconSize"
    :stroke-width="2"
    :style="{ color: meta.color }"
  />
</template>
