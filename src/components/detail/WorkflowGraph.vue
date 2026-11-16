<script setup lang="ts">
import {
  computed,
  nextTick,
  onMounted,
  reactive,
  ref,
  watch,
} from "vue";
import {
  GRAPH_PADDING,
  NODE_HEADER_HEIGHT,
  PORT_ROW_HEIGHT,
  PORT_TOP,
  type WorkflowEdge,
  type WorkflowGraph,
  type WorkflowNode,
} from "@/utils/workflowGraph";

const props = defineProps<{
  graph: WorkflowGraph;
}>();

const viewportRef = ref<HTMLElement | null>(null);
const view = reactive({ x: 0, y: 0, scale: 1 });
const panning = ref(false);
let panStartX = 0;
let panStartY = 0;
let panViewX = 0;
let panViewY = 0;

const nodeById = computed(
  () => new Map(props.graph.nodes.map((node) => [node.id, node])),
);

const edges = computed(() =>
  props.graph.edges.map((edge) => ({
    ...edge,
    path: edgePath(edge),
    color: edgeColor(edge.type),
  })),
);

const zoomLabel = computed(() => `${Math.round(view.scale * 100)}%`);

function portY(node: WorkflowNode, slot: number) {
  return (
    node.y +
    NODE_HEADER_HEIGHT +
    PORT_TOP +
    slot * PORT_ROW_HEIGHT +
    PORT_ROW_HEIGHT / 2
  );
}

function edgePath(edge: WorkflowEdge) {
  const from = nodeById.value.get(edge.from);
  const to = nodeById.value.get(edge.to);
  if (!from || !to) return "";
  const fromX = from.x + from.width;
  const fromY = portY(from, edge.fromSlot);
  const toX = to.x;
  const toY = portY(to, edge.toSlot);
  const bend = Math.max(40, Math.abs(toX - fromX) * 0.45);
  return `M ${fromX} ${fromY} C ${fromX + bend} ${fromY}, ${toX - bend} ${toY}, ${toX} ${toY}`;
}

function edgeColor(type: string) {
  const lower = type.toLowerCase();
  if (lower.includes("clip")) return "#81c784";
  if (lower.includes("model")) return "#b39ddb";
  if (lower.includes("vae")) return "#e57373";
  if (
    lower.includes("condition") ||
    lower.includes("positive") ||
    lower.includes("negative")
  ) {
    return "#ffb74d";
  }
  if (lower.includes("latent")) return "#f06292";
  if (lower.includes("image")) return "#4dd0e1";
  return "var(--accent)";
}

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}

function fitView() {
  const viewport = viewportRef.value;
  if (!viewport) return;
  const rect = viewport.getBoundingClientRect();
  if (rect.width <= 0 || rect.height <= 0) return;
  const scale = clamp(
    Math.min(
      rect.width / props.graph.width,
      rect.height / props.graph.height,
      1.4,
    ),
    0.12,
    1.4,
  );
  view.scale = scale;
  view.x = (rect.width - props.graph.width * scale) / 2;
  view.y = (rect.height - props.graph.height * scale) / 2;
}

function resetView() {
  view.scale = 1;
  view.x = GRAPH_PADDING;
  view.y = GRAPH_PADDING;
}

function initialView() {
  const viewport = viewportRef.value;
  if (!viewport) return;
  const rect = viewport.getBoundingClientRect();
  if (rect.width <= 0 || rect.height <= 0) return;
  const scale = Math.min(
    rect.width / props.graph.width,
    rect.height / props.graph.height,
  );
  if (scale >= 0.9) {
    fitView();
  } else {
    resetView();
  }
}

function onWheel(event: WheelEvent) {
  const viewport = viewportRef.value;
  if (!viewport) return;
  const rect = viewport.getBoundingClientRect();
  const pointerX = event.clientX - rect.left;
  const pointerY = event.clientY - rect.top;
  const factor = Math.exp(-event.deltaY * 0.0015);
  const next = clamp(view.scale * factor, 0.12, 3);
  const ratio = next / view.scale;
  view.x = pointerX - (pointerX - view.x) * ratio;
  view.y = pointerY - (pointerY - view.y) * ratio;
  view.scale = next;
}

function startPan(event: PointerEvent) {
  if (event.button !== 0) return;
  panning.value = true;
  panStartX = event.clientX;
  panStartY = event.clientY;
  panViewX = view.x;
  panViewY = view.y;
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  event.preventDefault();
}

function onPan(event: PointerEvent) {
  if (!panning.value) return;
  view.x = panViewX + event.clientX - panStartX;
  view.y = panViewY + event.clientY - panStartY;
}

function endPan(event: PointerEvent) {
  if (!panning.value) return;
  panning.value = false;
  const viewport = event.currentTarget as HTMLElement;
  if (viewport.hasPointerCapture(event.pointerId)) {
    viewport.releasePointerCapture(event.pointerId);
  }
}

watch(
  () => props.graph,
  async () => {
    await nextTick();
    initialView();
  },
  { immediate: true },
);

onMounted(() => {
  void nextTick(initialView);
});
</script>

<template>
  <div
    ref="viewportRef"
    class="workflow-graph"
    :class="{ panning }"
    @wheel.prevent="onWheel"
    @pointerdown="startPan"
    @pointermove="onPan"
    @pointerup="endPan"
    @pointercancel="endPan"
    @dblclick="fitView"
  >
    <div
      class="workflow-canvas"
      :style="{
        width: `${graph.width}px`,
        height: `${graph.height}px`,
        transform: `translate(${view.x}px, ${view.y}px) scale(${view.scale})`,
      }"
    >
      <svg
        class="workflow-edges"
        :width="graph.width"
        :height="graph.height"
        :viewBox="`0 0 ${graph.width} ${graph.height}`"
      >
        <path
          v-for="edge in edges"
          :key="edge.id"
          :d="edge.path"
          :stroke="edge.color"
          fill="none"
          stroke-width="1.5"
          stroke-linecap="round"
          vector-effect="non-scaling-stroke"
        />
      </svg>

      <div
        v-for="node in graph.nodes"
        :key="node.id"
        class="workflow-node"
        :class="node.category"
        :style="{
          left: `${node.x}px`,
          top: `${node.y}px`,
          width: `${node.width}px`,
          height: `${node.height}px`,
        }"
      >
        <div
          class="workflow-node-header"
          :title="node.type !== node.title ? `${node.title} (${node.type})` : node.type"
        >
          <span class="workflow-node-title">{{ node.title }}</span>
          <span class="workflow-node-id">#{{ node.id }}</span>
        </div>
        <div class="workflow-node-body">
          <div
            v-for="(port, index) in node.inputs"
            :key="`in-${index}`"
            class="workflow-port-row input"
            :style="{
              top: `${NODE_HEADER_HEIGHT + PORT_TOP + index * PORT_ROW_HEIGHT}px`,
            }"
          >
            <span class="workflow-port-dot" />
            <span class="workflow-port-name" :title="port.name">
              {{ port.name }}
            </span>
          </div>
          <div
            v-for="(port, index) in node.outputs"
            :key="`out-${index}`"
            class="workflow-port-row output"
            :style="{
              top: `${NODE_HEADER_HEIGHT + PORT_TOP + index * PORT_ROW_HEIGHT}px`,
            }"
          >
            <span class="workflow-port-name" :title="port.name">
              {{ port.name }}
            </span>
            <span class="workflow-port-dot" />
          </div>
          <div
            v-if="node.widgets.length"
            class="workflow-widgets"
            :style="{ top: `${node.widgetsTop}px` }"
          >
            <div
              v-for="(widget, index) in node.widgets"
              :key="`widget-${index}`"
              class="workflow-widget-row"
            >
              <span class="workflow-widget-name" :title="widget.name">
                {{ widget.name }}
              </span>
              <span class="workflow-widget-value" :title="widget.value">
                {{ widget.value }}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div
      class="workflow-graph-controls"
      @pointerdown.stop
      @dblclick.stop
      @wheel.prevent.stop
    >
      <span>{{ zoomLabel }}</span>
      <button class="link-button" type="button" @click="resetView">
        1:1
      </button>
      <button class="link-button" type="button" @click="fitView">
        适应窗口
      </button>
    </div>
  </div>
</template>
