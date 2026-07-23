<script setup lang="ts">
// 桌面端文件拖入区 + 拖入覆盖反馈
import { ref } from "vue";
import { UploadCloud, FolderOpen } from "lucide-vue-next";
import { useAppStore } from "@/stores/app";
import { useTransfersStore } from "@/stores/transfers";
import { useDragDrop } from "@/composables/useDragDrop";
import { pickFiles } from "@/services/tauri";

const props = defineProps<{
  targetName: string;
}>();

const app = useAppStore();
const transfers = useTransfersStore();
const { dragging, onDragEnter, onDragOver, onDragLeave, onDrop } = useDragDrop();
const detecting = ref(false);

function handleFiles(files: Array<{ name: string; size: number; path?: string }>) {
  if (!files.length) return;
  detecting.value = true;
  setTimeout(() => {
    transfers.addPendingFiles(files);
    detecting.value = false;
    app.pushToast("success", `已添加 ${files.length} 个文件`, "进入待发送队列");
  }, 350);
}

function onDropHandler(e: DragEvent) {
  const files = onDrop(e);
  handleFiles(files);
}

async function onSelectClick() {
  const picked = await pickFiles();
  const files = picked.map((p) => ({
    name: p.name,
    size: p.size,
    path: p.path,
  }));
  handleFiles(files);
}
</script>

<template>
  <div
    class="mx-5 my-4 border-2 border-dashed rounded-lg flex flex-col items-center justify-center py-6 cursor-pointer transition-colors zone"
    :class="{ 'zone-dragging': dragging }"
    data-droppable
    @dragenter.prevent="onDragEnter"
    @dragover="onDragOver"
    @dragleave="onDragLeave"
    @drop="onDropHandler"
    @click="onSelectClick"
  >
    <UploadCloud :size="32" :stroke-width="2" style="color: var(--color-text-tertiary)" class="mb-2" />
    <span style="font-size: var(--text-sm); color: var(--color-text-primary); font-weight: var(--font-weight-medium)">
      拖入文件发送到 {{ props.targetName }}
    </span>
    <span class="mt-1" style="font-size: var(--text-xs); color: var(--color-text-tertiary)">
      支持照片、视频、文档、压缩包和文件夹。
    </span>
    <a
      href="#"
      class="mt-1.5 transition-colors flex items-center gap-1"
      style="font-size: var(--text-xs); color: var(--color-text-link)"
      @click.stop.prevent="onSelectClick"
    >
      <FolderOpen :size="12" :stroke-width="2.5" />
      或点击选择文件
    </a>

    <!-- 拖入覆盖反馈（设计稿 文件拖入反馈.html） -->
    <Transition name="fade">
      <div v-if="dragging" class="drag-overlay">
        <UploadCloud class="drag-overlay-icon" :size="56" :stroke-width="1.75" />
        <span style="font-size: var(--text-xl); font-weight: var(--font-weight-semibold); color: var(--color-primary)">
          释放即可添加文件
        </span>
        <span class="mt-1.5" style="font-size: var(--text-base); color: var(--color-text-secondary)">
          {{ detecting ? "正在检测文件..." : "正在检测文件..." }}
        </span>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.zone {
  border-color: var(--color-border-strong);
  background: var(--color-surface-secondary);
  position: relative;
  transition: border-color var(--transition-normal),
    background var(--transition-normal);
}
.zone:hover {
  border-color: var(--color-primary);
  background: var(--color-primary-light-bg);
}
.zone-dragging {
  border-color: var(--color-primary);
  background: var(--color-primary-light-bg);
}
.drag-overlay {
  position: absolute;
  inset: 0;
  z-index: 10;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: rgba(62, 175, 124, 0.06);
  border: 2px solid rgba(62, 175, 124, 0.5);
  border-radius: var(--radius-lg);
  pointer-events: none;
  animation: dragFadeIn 200ms ease-out forwards;
}
.drag-overlay-icon {
  color: var(--color-primary);
  margin-bottom: 12px;
}
</style>
