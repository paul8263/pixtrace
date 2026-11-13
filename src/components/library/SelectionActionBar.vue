<script setup lang="ts">
import { useCompareStore } from "@/stores/compare";

const compare = useCompareStore();
</script>

<template>
  <div v-if="compare.count > 0" class="selection-bar">
    <span class="selection-count">已选 {{ compare.count }} 张</span>
    <span v-if="compare.warning" class="selection-warning">
      {{ compare.warning }}
    </span>
    <span v-else-if="compare.count < 2" class="selection-hint">
      至少选择 2 张
    </span>
    <span class="selection-spacer" />
    <button
      class="button primary"
      type="button"
      :disabled="!compare.canCompare"
      @click="compare.open()"
    >
      对比
    </button>
    <button class="button ghost" type="button" @click="compare.clear()">
      取消
    </button>
  </div>
</template>
