import { listen } from "@tauri-apps/api/event";
import { defineStore } from "pinia";
import { computed, reactive, ref } from "vue";
import {
  listImages,
  type ImageSummary,
  type IndexProgress,
} from "@/api/backend";

const PAGE_SIZE = 200;

export const useLibraryStore = defineStore("library", () => {
  const pages = reactive(new Map<number, ImageSummary[]>());
  const loadingPages = new Set<number>();
  const total = ref(0);
  const loaded = ref(false);
  const generation = ref(0);
  const revision = ref(0);
  const error = ref<string | null>(null);
  const activeIndex = ref<number | null>(null);
  const initialized = ref(false);

  const activeItem = computed(() =>
    activeIndex.value === null ? null : itemAt(activeIndex.value),
  );

  function itemAt(index: number): ImageSummary | undefined {
    const page = pages.get(Math.floor(index / PAGE_SIZE));
    return page?.[index % PAGE_SIZE];
  }

  async function loadPage(pageIndex: number) {
    if (pages.has(pageIndex) || loadingPages.has(pageIndex)) return;
    const current = generation.value;
    loadingPages.add(pageIndex);
    try {
      const page = await listImages(pageIndex * PAGE_SIZE, PAGE_SIZE);
      if (current !== generation.value) return;
      pages.set(pageIndex, page.items);
      total.value = page.total;
      loaded.value = true;
    } catch (cause) {
      if (current === generation.value) {
        error.value = String(cause);
      }
    } finally {
      loadingPages.delete(pageIndex);
    }
  }

  function ensureRange(start: number, end: number) {
    if (total.value === 0 && !pages.has(0)) {
      void loadPage(0);
      return;
    }
    const first = Math.max(0, Math.floor(start / PAGE_SIZE));
    const last = Math.floor(Math.max(start, end) / PAGE_SIZE);
    for (let page = first; page <= last; page += 1) {
      void loadPage(page);
    }
  }

  function ensurePageFor(index: number) {
    return loadPage(Math.floor(index / PAGE_SIZE));
  }

  async function reset() {
    generation.value += 1;
    pages.clear();
    total.value = 0;
    loaded.value = false;
    activeIndex.value = null;
    error.value = null;
    await loadPage(0);
  }

  async function refresh() {
    generation.value += 1;
    pages.clear();
    revision.value += 1;
    await loadPage(0);
  }

  async function openAt(index: number) {
    await ensurePageFor(index);
    if (itemAt(index)) {
      activeIndex.value = index;
    }
  }

  async function step(delta: number) {
    if (activeIndex.value === null) return;
    const target = activeIndex.value + delta;
    if (target < 0 || target >= total.value) return;
    await ensurePageFor(target);
    if (itemAt(target)) {
      activeIndex.value = target;
    }
  }

  function close() {
    activeIndex.value = null;
  }

  async function init() {
    if (initialized.value) return;
    initialized.value = true;
    await listen<IndexProgress>("index:changed", () => {
      revision.value += 1;
      void refresh();
    });
    await loadPage(0);
  }

  return {
    pages,
    total,
    loaded,
    generation,
    revision,
    error,
    activeIndex,
    activeItem,
    itemAt,
    ensureRange,
    ensurePageFor,
    reset,
    refresh,
    openAt,
    step,
    close,
    init,
  };
});
