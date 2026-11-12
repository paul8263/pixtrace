<script setup lang="ts">
import { computed } from "vue";
import { RouterLink, useRouter } from "vue-router";
import { useWorkspaceStore } from "@/stores/workspace";

const store = useWorkspaceStore();
const router = useRouter();

const workspaceLabel = computed(
  () => store.workspace?.root ?? "未打开工作目录",
);

async function openDirectory() {
  const opened = await store.chooseWorkspace();
  if (opened) {
    await router.push("/library");
  }
}
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
      <input
        class="search-input"
        type="search"
        placeholder="搜索提示词（M2 开放）"
        disabled
      />
    </div>

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
