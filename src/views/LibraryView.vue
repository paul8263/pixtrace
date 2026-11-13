<script setup lang="ts">
import { computed } from "vue";
import CompareOverlay from "@/components/compare/CompareOverlay.vue";
import DetailOverlay from "@/components/detail/DetailOverlay.vue";
import PromptGroupView from "@/components/groups/PromptGroupView.vue";
import DeleteImageDialog from "@/components/library/DeleteImageDialog.vue";
import ImageContextMenu from "@/components/library/ImageContextMenu.vue";
import SelectionActionBar from "@/components/library/SelectionActionBar.vue";
import VirtualImageGrid from "@/components/library/VirtualImageGrid.vue";
import EmptyState from "@/components/common/EmptyState.vue";
import { useCompareStore } from "@/stores/compare";
import { useLibraryStore } from "@/stores/library";
import { useSelectionStore } from "@/stores/selection";
import { useWorkspaceStore } from "@/stores/workspace";

const store = useWorkspaceStore();
const library = useLibraryStore();
const selection = useSelectionStore();
const compare = useCompareStore();

const total = computed(() => {
  if (library.isSearching || library.promptHash) return library.total;
  return library.total || store.workspace?.imageCount || 0;
});

const title = computed(() => {
  if (library.promptHash) return "同提示词图像";
  if (library.isSearching) return "搜索结果";
  return "全部图片";
});

function openContextImage() {
  const menu = library.contextMenu;
  if (!menu) return;
  library.closeContextMenu();
  void library.openAt(menu.index);
}

function requestDeleteFromMenu() {
  const menu = library.contextMenu;
  if (menu) library.requestDelete(menu.image);
}
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
      <PromptGroupView v-if="library.viewMode === 'groups'" />

      <template v-else>
        <div class="library-head">
          <div class="library-title-group">
            <h1 class="library-title">{{ title }}</h1>
            <span class="library-count">{{ total.toLocaleString() }} 张</span>
            <button
              v-if="library.promptHash"
              class="link-button library-clear"
              type="button"
              @click="library.showGroups()"
            >
              返回分组
            </button>
            <button
              v-if="library.isSearching"
              class="link-button library-clear"
              type="button"
              @click="library.clearSearch()"
            >
              清除搜索
            </button>
          </div>
          <span v-if="store.progress.running" class="library-index-note">
            {{ store.progress.phase }} ·
            {{ store.progress.done.toLocaleString() }} /
            {{ store.progress.total.toLocaleString() }}
          </span>
          <span
            v-else-if="store.progress.failed"
            class="library-index-note warning"
          >
            {{ store.progress.failed.toLocaleString() }} 个文件解析失败
          </span>
          <div class="selection-tools">
            <span v-if="selection.count" class="selection-inline-count">
              已选 {{ selection.count.toLocaleString() }}
            </span>
            <button
              class="link-button"
              type="button"
              :disabled="library.total === 0 || selection.allSelected"
              @click="selection.selectAll()"
            >
              全选
            </button>
            <button
              class="link-button"
              type="button"
              :disabled="selection.count === 0"
              @click="selection.selectNone()"
            >
              全不选
            </button>
            <button
              class="link-button"
              type="button"
              :disabled="library.total === 0"
              @click="selection.invert()"
            >
              反选
            </button>
          </div>
        </div>

        <div v-if="library.error" class="library-error">
          <span>{{ library.error }}</span>
          <button
            class="link-button"
            type="button"
            @click="library.clearError()"
          >
            关闭
          </button>
        </div>

        <div v-if="library.deleteNotice" class="library-notice">
          <span>{{ library.deleteNotice }}</span>
          <button
            class="link-button"
            type="button"
            @click="library.clearDeleteNotice()"
          >
            关闭
          </button>
        </div>

        <VirtualImageGrid />
      </template>
    </template>

    <DetailOverlay v-if="library.detailOpen" />
    <SelectionActionBar v-if="library.viewMode === 'grid'" />
    <CompareOverlay v-if="compare.isOpen" />
    <ImageContextMenu
      v-if="library.contextMenu"
      :image="library.contextMenu.image"
      :x="library.contextMenu.x"
      :y="library.contextMenu.y"
      @open="openContextImage"
      @delete="requestDeleteFromMenu"
      @close="library.closeContextMenu()"
    />
    <DeleteImageDialog
      v-if="library.deleteTarget"
      :label="library.deleteLabel"
      :count="library.deleteCount"
      :busy="library.deleting"
      :error="library.deleteError"
      @cancel="library.cancelDelete()"
      @confirm="library.confirmDelete"
    />
  </section>
</template>
