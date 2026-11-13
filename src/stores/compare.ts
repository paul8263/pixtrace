import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type { ImageSummary } from "@/api/backend";

export const MAX_COMPARE = 4;

export const useCompareStore = defineStore("compare", () => {
  const selected = ref<ImageSummary[]>([]);
  const isOpen = ref(false);
  const warning = ref<string | null>(null);

  const count = computed(() => selected.value.length);
  const canCompare = computed(() => count.value >= 2 && count.value <= MAX_COMPARE);

  function isSelected(id: number): boolean {
    return selected.value.some((item) => item.id === id);
  }

  function toggle(image: ImageSummary) {
    const index = selected.value.findIndex((item) => item.id === image.id);
    if (index >= 0) {
      selected.value = selected.value.filter((item) => item.id !== image.id);
      warning.value = null;
      if (selected.value.length < 2) {
        isOpen.value = false;
      }
      return;
    }
    if (selected.value.length >= MAX_COMPARE) {
      warning.value = `最多选择 ${MAX_COMPARE} 张图像`;
      return;
    }
    selected.value = [...selected.value, image];
    warning.value = null;
  }

  function set(images: ImageSummary[]) {
    selected.value = images.slice(0, MAX_COMPARE);
    warning.value = null;
    if (selected.value.length < 2) {
      isOpen.value = false;
    }
  }

  function open() {
    if (canCompare.value) {
      isOpen.value = true;
    }
  }

  function close() {
    isOpen.value = false;
  }

  function clear() {
    selected.value = [];
    isOpen.value = false;
    warning.value = null;
  }

  return {
    selected,
    isOpen,
    warning,
    count,
    canCompare,
    isSelected,
    toggle,
    set,
    open,
    close,
    clear,
  };
});
