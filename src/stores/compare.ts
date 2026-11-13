import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type { ImageSummary } from "@/api/backend";
import { MAX_COMPARE } from "./selection";

export const useCompareStore = defineStore("compare", () => {
  const selected = ref<ImageSummary[]>([]);
  const isOpen = ref(false);

  const count = computed(() => selected.value.length);
  const canCompare = computed(
    () => count.value >= 2 && count.value <= MAX_COMPARE,
  );

  function set(images: ImageSummary[]) {
    selected.value = images.slice(0, MAX_COMPARE);
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
  }

  return {
    selected,
    isOpen,
    count,
    canCompare,
    set,
    open,
    close,
    clear,
  };
});
