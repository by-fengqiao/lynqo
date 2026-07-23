// 拖拽入站 composable：桌面端文件拖入反馈
import { ref } from "vue";
import type { TransferFile } from "@/types";
import { genId } from "@/utils/format";

export function useDragDrop() {
  const dragging = ref(false);
  let dragCounter = 0;

  function onDragEnter(e: DragEvent) {
    if (!e.dataTransfer?.types.includes("Files")) return;
    dragCounter++;
    dragging.value = true;
  }
  function onDragOver(e: DragEvent) {
    if (e.dataTransfer?.types.includes("Files")) {
      e.preventDefault(); // 允许 drop
      if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
    }
  }
  function onDragLeave() {
    dragCounter = Math.max(0, dragCounter - 1);
    if (dragCounter === 0) dragging.value = false;
  }
  function onDrop(e: DragEvent): TransferFile[] {
    e.preventDefault();
    dragCounter = 0;
    dragging.value = false;
    const files: TransferFile[] = [];
    if (e.dataTransfer) {
      const items = e.dataTransfer.items;
      if (items) {
        for (let i = 0; i < items.length; i++) {
          const it = items[i];
          if (it.kind === "file") {
            const f = it.getAsFile();
            if (f) files.push(toTransferFile(f));
          }
        }
      } else if (e.dataTransfer.files) {
        for (let i = 0; i < e.dataTransfer.files.length; i++) {
          files.push(toTransferFile(e.dataTransfer.files[i]));
        }
      }
    }
    return files;
  }

  function toTransferFile(f: File): TransferFile {
    return {
      id: genId("f"),
      name: f.name,
      size: f.size,
      mimeType: f.type || undefined,
    };
  }

  return {
    dragging,
    onDragEnter,
    onDragOver,
    onDragLeave,
    onDrop,
  };
}
