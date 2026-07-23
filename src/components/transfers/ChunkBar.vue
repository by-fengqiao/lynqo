<script setup lang="ts">
// 分片进度条（任务展开详情）
// chunk-done: 已完成；chunk-active: 进行中（带斜纹+脉动）；chunk-pending: 待传
import { computed } from "vue";

const props = defineProps<{
  total: number;
  done: number;
}>();

interface BlockState {
  state: "done" | "active" | "pending";
}

const blocks = computed<BlockState[]>(() => {
  const arr: BlockState[] = [];
  for (let i = 0; i < props.total; i++) {
    if (i < props.done) arr.push({ state: "done" });
    else if (i === props.done) arr.push({ state: "active" });
    else arr.push({ state: "pending" });
  }
  return arr;
});
</script>

<template>
  <div class="chunk-bar">
    <div
      v-for="(b, i) in blocks"
      :key="i"
      class="chunk-block"
      :class="`chunk-${b.state}`"
    />
  </div>
</template>

<style scoped>
.chunk-bar {
  display: flex;
  gap: 3px;
  height: 8px;
}
.chunk-block {
  flex: 1;
  border-radius: 3px;
  height: 100%;
}
.chunk-done {
  background: var(--color-primary);
}
.chunk-active {
  background: var(--color-primary);
  background-image: repeating-linear-gradient(
    135deg,
    transparent,
    transparent 3px,
    rgba(255, 255, 255, 0.25) 3px,
    rgba(255, 255, 255, 0.25) 6px
  );
  animation: chunk-pulse 1.2s ease-in-out infinite;
}
.chunk-pending {
  background: var(--color-neutral-200);
}
</style>
