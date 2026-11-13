<script setup lang="ts">
import { useLibraryStore } from "@/stores/library";
import { useSelectionStore } from "@/stores/selection";

const library = useLibraryStore();
const selection = useSelectionStore();
</script>

<template>
  <div v-if="selection.count > 0" class="selection-bar">
    <span class="selection-count">
      已选 {{ selection.count.toLocaleString() }} 张
    </span>
    <span v-if="selection.count > 4" class="selection-hint">
      对比最多 4 张
    </span>
    <span v-else-if="selection.count < 2" class="selection-hint">
      至少选择 2 张可对比
    </span>
    <span class="selection-spacer" />
    <button
      class="button primary"
      type="button"
      :disabled="!selection.canCompare"
      @click="library.openCompareSelection()"
    >
      对比
    </button>
    <button
      class="button danger"
      type="button"
      @click="library.requestDeleteSelection()"
    >
      删除选中…
    </button>
    <button class="button ghost" type="button" @click="selection.clear()">
      取消
    </button>
  </div>
</template>
