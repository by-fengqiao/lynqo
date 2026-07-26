<script setup lang="ts">
import appIcon from "../../../src-tauri/icons/128x128.png";
import { APP_NAME } from "@/config/brand";

withDefaults(
  defineProps<{
    size?: number;
  }>(),
  {
    size: 28,
  }
);
</script>

<template>
  <img
    :src="appIcon"
    :width="size"
    :height="size"
    class="app-logo"
    :alt="APP_NAME"
  />
</template>

<style scoped>
.app-logo {
  display: inline-block;
  flex-shrink: 0;
}
</style>
