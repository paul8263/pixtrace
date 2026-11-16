<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { buildWorkflowGraph } from "@/utils/workflowGraph";
import JsonTreeNode from "./JsonTreeNode.vue";
import WorkflowGraph from "./WorkflowGraph.vue";

const props = defineProps<{
  text: string | null;
  loading: boolean;
  emptyText: string;
}>();

type ViewMode = "graph" | "tree" | "raw";

const mode = ref<ViewMode>("graph");

const parsed = computed<unknown>(() => {
  if (!props.text) return null;
  try {
    return JSON.parse(props.text);
  } catch {
    return null;
  }
});

const graph = computed(() => buildWorkflowGraph(parsed.value));

watch(
  () => props.text,
  () => {
    mode.value = graph.value ? "graph" : "tree";
  },
  { immediate: true },
);
</script>

<template>
  <div class="workflow-viewer">
    <p v-if="loading" class="muted">加载中…</p>
    <p v-else-if="!text" class="muted">{{ emptyText }}</p>
    <template v-else>
      <div class="json-toolbar workflow-toolbar">
        <div class="workflow-mode-tabs">
          <button
            v-if="graph"
            type="button"
            :class="{ active: mode === 'graph' }"
            @click="mode = 'graph'"
          >
            节点图
          </button>
          <button
            type="button"
            :class="{ active: mode === 'tree' }"
            @click="mode = 'tree'"
          >
            JSON 树
          </button>
          <button
            type="button"
            :class="{ active: mode === 'raw' }"
            @click="mode = 'raw'"
          >
            原始文本
          </button>
        </div>
        <span class="muted">
          <template v-if="graph">
            {{ graph.source === "api" ? "API 工作流" : "界面工作流" }} ·
            {{ graph.nodes.length }} 节点 · {{ graph.edges.length }} 连线
          </template>
        </span>
      </div>

      <WorkflowGraph v-if="mode === 'graph' && graph" :graph="graph" />
      <div v-else-if="mode === 'tree' && parsed" class="json-tree">
        <JsonTreeNode :value="parsed" :depth="0" />
      </div>
      <pre v-else class="json-raw">{{ text }}</pre>
    </template>
  </div>
</template>
