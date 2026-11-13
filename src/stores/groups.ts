import { listen } from "@tauri-apps/api/event";
import { defineStore } from "pinia";
import { computed, ref } from "vue";
import {
  listPromptGroups,
  type GroupSort,
  type PromptGroup,
} from "@/api/backend";

const PAGE_SIZE = 100;

export const useGroupsStore = defineStore("groups", () => {
  const items = ref<PromptGroup[]>([]);
  const total = ref(0);
  const loading = ref(false);
  const loaded = ref(false);
  const error = ref<string | null>(null);
  const sort = ref<GroupSort>("count");
  const initialized = ref(false);

  let requestId = 0;

  const hasMore = computed(() => items.value.length < total.value);

  async function load(reset = false) {
    if (loading.value && !reset) return;
    const current = ++requestId;
    loading.value = true;
    if (reset) {
      items.value = [];
      total.value = 0;
      loaded.value = false;
    }
    try {
      const page = await listPromptGroups({
        sort: sort.value,
        offset: reset ? 0 : items.value.length,
        limit: PAGE_SIZE,
      });
      if (current !== requestId) return;
      items.value = reset ? page.items : [...items.value, ...page.items];
      total.value = page.total;
      loaded.value = true;
      error.value = null;
    } catch (cause) {
      if (current === requestId) {
        error.value = String(cause);
        loaded.value = true;
      }
    } finally {
      if (current === requestId) {
        loading.value = false;
      }
    }
  }

  async function refresh() {
    if (!loaded.value) return;
    await load(true);
  }

  function setSort(value: GroupSort) {
    if (sort.value === value) return;
    sort.value = value;
    void load(true);
  }

  function reset() {
    requestId += 1;
    items.value = [];
    total.value = 0;
    loading.value = false;
    loaded.value = false;
    error.value = null;
  }

  async function init() {
    if (initialized.value) return;
    initialized.value = true;
    await listen("index:changed", () => {
      void refresh();
    });
  }

  return {
    items,
    total,
    loading,
    loaded,
    error,
    sort,
    hasMore,
    load,
    refresh,
    setSort,
    reset,
    init,
  };
});
