import { defineStore } from "pinia";
import { computed, ref } from "vue";

export const MAX_COMPARE = 4;

export const useSelectionStore = defineStore("selection", () => {
  const total = ref(0);
  const selectAllActive = ref(false);
  const selected = ref<Set<number>>(new Set());
  const excluded = ref<Set<number>>(new Set());

  const count = computed(() =>
    selectAllActive.value
      ? Math.max(0, total.value - excluded.value.size)
      : selected.value.size,
  );
  const canCompare = computed(
    () => count.value >= 2 && count.value <= MAX_COMPARE,
  );
  const allSelected = computed(
    () => total.value > 0 && count.value >= total.value,
  );

  function setTotal(value: number) {
    total.value = Math.max(0, Math.floor(value));
    if (total.value === 0) {
      selectAllActive.value = false;
      selected.value = new Set();
      excluded.value = new Set();
    }
  }

  function isSelected(id: number): boolean {
    return selectAllActive.value
      ? !excluded.value.has(id)
      : selected.value.has(id);
  }

  function toggle(id: number) {
    if (selectAllActive.value) {
      const next = new Set(excluded.value);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      excluded.value = next;
      return;
    }
    const next = new Set(selected.value);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    selected.value = next;
  }

  function remove(id: number) {
    if (selectAllActive.value) {
      if (excluded.value.has(id)) {
        const next = new Set(excluded.value);
        next.delete(id);
        excluded.value = next;
      }
      return;
    }
    const next = new Set(selected.value);
    next.delete(id);
    selected.value = next;
  }

  function selectAll() {
    if (total.value === 0) return;
    selectAllActive.value = true;
    selected.value = new Set();
    excluded.value = new Set();
  }

  function selectNone() {
    selectAllActive.value = false;
    selected.value = new Set();
    excluded.value = new Set();
  }

  function invert() {
    if (total.value === 0) return;
    if (selectAllActive.value) {
      const next = new Set(excluded.value);
      selectAllActive.value = false;
      selected.value = next;
      excluded.value = new Set();
      return;
    }
    const next = new Set(selected.value);
    selectAllActive.value = true;
    selected.value = new Set();
    excluded.value = next;
  }

  function set(ids: number[]) {
    selectAllActive.value = false;
    selected.value = new Set(ids);
    excluded.value = new Set();
  }

  function clear() {
    selectNone();
  }

  return {
    total,
    selectAllActive,
    selectedIds: selected,
    excludedIds: excluded,
    count,
    canCompare,
    allSelected,
    setTotal,
    isSelected,
    toggle,
    remove,
    selectAll,
    selectNone,
    invert,
    set,
    clear,
  };
});
