<script setup lang="ts">
import { watch, onBeforeUnmount } from "vue";
import { X, CheckCircle, AlertCircle, Info } from "lucide-vue-next";

const props = defineProps<{
  message: string;
  type: "success" | "error" | "info";
  visible: boolean;
}>();

const emit = defineEmits<{
  close: [];
}>();

const iconMap = {
  success: CheckCircle,
  error: AlertCircle,
  info: Info,
};

let timer: ReturnType<typeof setTimeout> | null = null;

watch(
  () => props.visible,
  (val) => {
    if (val) {
      timer = setTimeout(() => {
        emit("close");
      }, 3000);
    } else {
      if (timer) {
        clearTimeout(timer);
        timer = null;
      }
    }
  }
);

onBeforeUnmount(() => {
  if (timer) {
    clearTimeout(timer);
    timer = null;
  }
});
</script>

<template>
  <Teleport to="body">
    <Transition name="toast">
      <div v-if="visible" class="toast" :class="`toast--${type}`">
        <component :is="iconMap[type]" :size="16" class="toast-icon" />
        <span class="toast-message">{{ message }}</span>
        <button class="toast-close" @click="emit('close')">
          <X :size="14" />
        </button>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.toast {
  position: fixed;
  top: 16px;
  left: 50%;
  transform: translateX(-50%);
  z-index: var(--z-toast);
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  border-radius: var(--radius-md);
  background: var(--color-surface-elevated);
  border: 1px solid var(--color-border);
  box-shadow: var(--shadow-lg);
  max-width: 360px;
  animation: slide-up-toast 200ms ease forwards;
}

.toast--success .toast-icon {
  color: var(--color-state-success);
}

.toast--error .toast-icon {
  color: var(--color-state-error);
}

.toast--info .toast-icon {
  color: var(--color-state-info);
}

.toast-icon {
  flex-shrink: 0;
}

.toast-message {
  font-size: var(--text-sm);
  color: var(--color-text-primary);
  line-height: var(--leading-normal);
}

.toast-close {
  flex-shrink: 0;
  width: 20px;
  height: 20px;
  border: none;
  background: transparent;
  color: var(--color-text-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: color var(--transition-fast), background var(--transition-fast);
}

.toast-close:hover {
  color: var(--color-text-primary);
  background: var(--color-hover);
}

.toast-enter-active {
  animation: slide-up-toast 200ms ease forwards;
}

.toast-leave-active {
  animation: slide-down-toast 160ms ease forwards;
}

@keyframes slide-up-toast {
  from {
    opacity: 0;
    transform: translateX(-50%) translateY(-8px);
  }
  to {
    opacity: 1;
    transform: translateX(-50%) translateY(0);
  }
}

@keyframes slide-down-toast {
  from {
    opacity: 1;
    transform: translateX(-50%) translateY(0);
  }
  to {
    opacity: 0;
    transform: translateX(-50%) translateY(-8px);
  }
}
</style>
