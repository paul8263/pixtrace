<script setup lang="ts">
import { computed, ref } from "vue";
import JsonTreeNode from "./JsonTreeNode.vue";

const props = defineProps<{
  text: string | null;
  loading: boolean;
  emptyText: string;
}>();

const showRaw = ref(false);

const parsed = computed<unknown>(() => {
  if (!props.text) return null;
  try {
    return JSON.parse(props.text);
  } catch {
    return null;
  }
});
</script>

<template>
  <div class="workflow-viewer">
    <p v-if="loading" class="muted">加载中…</p>
    <p v-else-if="!text" class="muted">{{ emptyText }}</p>
    <template v-else>
      <div class="json-toolbar">
        <span class="muted">{{ showRaw ? "原始文本" : "JSON 树" }}</span>
        <button class="link-button" @click="showRaw = !showRaw">
          {{ showRaw ? "切换为树形" : "切换为原始文本" }}
        </button>
      </div>
      <pre v-if="showRaw" class="json-raw">{{ text }}</pre>
      <div v-else-if="parsed" class="json-tree">
        <JsonTreeNode :value="parsed" :depth="0" />
      </div>
      <pre v-else class="json-raw">{{ text }}</pre>
    </template>
  </div>
</template>
