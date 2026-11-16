<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { RouterLink, useRouter } from "vue-router";
import type { SortKey } from "@/api/backend";
import { useLibraryStore } from "@/stores/library";
import { useWorkspaceStore } from "@/stores/workspace";
import SearchChipInput from "./SearchChipInput.vue";

const store = useWorkspaceStore();
const library = useLibraryStore();
const router = useRouter();
const searchInput = ref<InstanceType<typeof SearchChipInput> | null>(null);

const workspaceLabel = computed(
  () => store.workspace?.root ?? "未打开工作目录",
);

const sortKey = computed({
  get: () => library.sort,
  set: (value: SortKey) => library.setSort(value),
});

async function openDirectory() {
  const opened = await store.chooseWorkspace();
  if (opened) {
    await router.push("/library");
  }
}

function onKeydown(event: KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f") {
    event.preventDefault();
    searchInput.value?.focus();
  }
}

onMounted(() => window.addEventListener("keydown", onKeydown));
onBeforeUnmount(() => window.removeEventListener("keydown", onKeydown));
</script>

<template>
  <header class="top-toolbar">
    <RouterLink class="brand" to="/">
      <span class="brand-name">PixTrace</span>
      <span class="brand-cn">图迹</span>
    </RouterLink>

    <div class="workspace-path" :title="workspaceLabel">
      {{ workspaceLabel }}
    </div>

    <div class="search-shell">
      <SearchChipInput
        ref="searchInput"
        :model-value="library.query"
        :disabled="!store.hasWorkspace"
        @update:model-value="library.setQuery"
        @search="library.applyQuery()"
        @clear="library.clearSearch()"
      />
    </div>

    <label v-if="library.viewMode === 'grid'" class="sort-control">
      <span>排序</span>
      <select
        v-model="sortKey"
        class="sort-select"
        :disabled="!store.hasWorkspace"
      >
        <option value="relevance">相关度</option>
        <option value="mtime">最新</option>
        <option value="path">路径</option>
        <option value="rating">评级</option>
      </select>
    </label>

    <div class="toolbar-actions">
      <button
        class="button primary"
        :disabled="store.busy"
        @click="openDirectory"
      >
        {{ store.busy ? "扫描中…" : "打开目录" }}
      </button>
      <button
        class="button ghost"
        :disabled="!store.hasWorkspace || store.busy"
        @click="store.rescan()"
      >
        重新扫描
      </button>
      <button class="button ghost" @click="store.openSettings()">设置</button>
    </div>
  </header>
</template>
