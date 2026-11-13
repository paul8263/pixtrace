<script setup lang="ts">
import { computed } from "vue";
import DetailOverlay from "@/components/detail/DetailOverlay.vue";
import VirtualImageGrid from "@/components/library/VirtualImageGrid.vue";
import EmptyState from "@/components/common/EmptyState.vue";
import { useLibraryStore } from "@/stores/library";
import { useWorkspaceStore } from "@/stores/workspace";

const store = useWorkspaceStore();
const library = useLibraryStore();

const total = computed(() => library.total || store.workspace?.imageCount || 0);
</script>

<template>
  <section class="library-view">
    <EmptyState
      v-if="!store.workspace"
      title="打开一个工作目录"
      description="PixTrace 会扫描其中的 ComfyUI 图像，解析提示词与生成参数，并在目录内建立本地索引。"
    >
      <button
        class="button primary"
        :disabled="store.busy"
        @click="store.chooseWorkspace()"
      >
        选择目录
      </button>
    </EmptyState>

    <template v-else>
      <div class="library-head">
        <div class="library-title-group">
          <h1 class="library-title">全部图片</h1>
          <span class="library-count">{{ total.toLocaleString() }} 张</span>
        </div>
        <span v-if="store.progress.running" class="library-index-note">
          {{ store.progress.phase }} · {{ store.progress.done.toLocaleString() }} /
          {{ store.progress.total.toLocaleString() }}
        </span>
        <span v-else-if="store.progress.failed" class="library-index-note warning">
          {{ store.progress.failed.toLocaleString() }} 个文件解析失败
        </span>
      </div>
      <VirtualImageGrid />
    </template>

    <DetailOverlay v-if="library.activeIndex !== null" />
  </section>
</template>
