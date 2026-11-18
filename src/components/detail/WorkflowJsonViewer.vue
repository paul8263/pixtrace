<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import type { WorkflowGraph as WorkflowGraphData } from "@/utils/workflowGraph";
import type { WorkflowParseResponse } from "@/workers/workflowParse.worker";
import JsonTreeNode from "./JsonTreeNode.vue";
import WorkflowGraph from "./WorkflowGraph.vue";

const props = defineProps<{
  text: string | null;
  loading: boolean;
  emptyText: string;
}>();

type ViewMode = "graph" | "tree" | "raw";

const RAW_DISPLAY_LIMIT = 200_000;

const mode = ref<ViewMode>("graph");
const graph = ref<WorkflowGraphData | null>(null);
const tree = ref<unknown>(null);
const treeTruncated = ref(false);
const parseError = ref<string | null>(null);
const parsing = ref(false);
let worker: Worker | null = null;
let requestId = 0;

function stopWorker() {
  worker?.terminate();
  worker = null;
}

function startParse(text: string) {
  stopWorker();
  requestId += 1;
  graph.value = null;
  tree.value = null;
  treeTruncated.value = false;
  parseError.value = null;
  if (!text) {
    parsing.value = false;
    mode.value = "tree";
    return;
  }

  parsing.value = true;
  const id = requestId;
  const nextWorker = new Worker(
    new URL("../../workers/workflowParse.worker.ts", import.meta.url),
    { type: "module" },
  );
  worker = nextWorker;

  nextWorker.onmessage = (event: MessageEvent<WorkflowParseResponse>) => {
    if (worker !== nextWorker || event.data.id !== id) return;
    const result = event.data;
    graph.value = result.graph;
    tree.value = result.tree;
    treeTruncated.value = result.treeTruncated;
    parseError.value = result.error;
    mode.value = result.graph ? "graph" : result.error ? "raw" : "tree";
    parsing.value = false;
    stopWorker();
  };

  nextWorker.onerror = (event) => {
    if (worker !== nextWorker) return;
    parseError.value = event.message || "JSON 解析失败";
    mode.value = "raw";
    parsing.value = false;
    stopWorker();
  };

  nextWorker.postMessage({ id, text });
}

watch(() => props.text, (text) => startParse(text ?? ""), { immediate: true });

onBeforeUnmount(() => {
  requestId += 1;
  stopWorker();
});

const rawPreview = computed(() => {
  const text = props.text ?? "";
  return text.length > RAW_DISPLAY_LIMIT
    ? text.slice(0, RAW_DISPLAY_LIMIT)
    : text;
});

const rawTruncated = computed(
  () => (props.text?.length ?? 0) > RAW_DISPLAY_LIMIT,
);
</script>

<template>
  <div class="workflow-viewer">
    <p v-if="loading || parsing" class="muted">
      {{ loading ? "加载中…" : "正在解析 JSON…" }}
    </p>
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
            :disabled="tree === null"
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
          <template v-else-if="parseError">JSON 解析失败</template>
        </span>
      </div>

      <p v-if="parseError" class="muted json-truncated">
        JSON 解析失败，可切换到“原始文本”查看。
      </p>
      <WorkflowGraph v-if="mode === 'graph' && graph" :graph="graph" />
      <div v-else-if="mode === 'tree' && tree !== null" class="json-tree">
        <p v-if="treeTruncated" class="muted json-truncated">
          JSON 过大，仅显示部分结构。
        </p>
        <JsonTreeNode :value="tree" :depth="0" />
      </div>
      <template v-else-if="mode === 'raw'">
        <p v-if="rawTruncated" class="muted json-truncated">
          原始文本过长，仅显示前 200,000 个字符。
        </p>
        <pre class="json-raw">{{ rawPreview }}</pre>
      </template>
      <p v-else class="muted">正在准备视图…</p>
    </template>
  </div>
</template>
