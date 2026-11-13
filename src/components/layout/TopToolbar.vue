<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { RouterLink, useRouter } from "vue-router";
import type { SortKey } from "@/api/backend";
import { useLibraryStore } from "@/stores/library";
import { useWorkspaceStore } from "@/stores/workspace";

const store = useWorkspaceStore();
const library = useLibraryStore();
const router = useRouter();
const searchInput = ref<HTMLInputElement | null>(null);

const workspaceLabel = computed(
  () => store.workspace?.root ?? "未打开工作目录",
);

const searchText = computed({
  get: () => library.query,
  set: (value: string) => library.setQuery(value),
});

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

function clearSearch() {
  library.clearSearch();
  searchInput.value?.focus();
}

function onKeydown(event: KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f") {
    event.preventDefault();
    searchInput.value?.focus();
    searchInput.value?.select();
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
      <div class="search-box">
        <input
          ref="searchInput"
          v-model="searchText"
          class="search-input"
          type="text"
          :disabled="!store.hasWorkspace"
          placeholder="搜索提示词，空格分词，-排除"
          @keydown.enter.prevent="library.applyQuery()"
          @keydown.esc.prevent="clearSearch"
        />
        <button
          v-if="library.query"
          class="search-clear"
          type="button"
          title="清除搜索"
          @click="clearSearch"
        >
          ×
        </button>
      </div>
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
      <button class="button ghost" disabled title="M4 开放">设置</button>
    </div>
  </header>
</template>
