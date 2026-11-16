<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";
import type { ImageDetail } from "@/api/backend";

const props = defineProps<{
  items: ImageDetail[];
}>();

const MIN_HEIGHT = 96;
const sectionRef = ref<HTMLElement | null>(null);
const height = ref<number | null>(null);
const resizing = ref(false);
let startY = 0;
let startHeight = 0;

function maxHeight() {
  const available =
    sectionRef.value?.parentElement?.clientHeight ?? window.innerHeight;
  return Math.max(MIN_HEIGHT, available * 0.7);
}

function applyHeight(next: number) {
  height.value = Math.min(Math.max(next, MIN_HEIGHT), maxHeight());
}

function startResize(event: PointerEvent) {
  if (event.button !== 0 || !sectionRef.value) return;
  startY = event.clientY;
  startHeight = sectionRef.value.getBoundingClientRect().height;
  height.value = startHeight;
  resizing.value = true;
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  document.body.classList.add("diff-resizing");
  event.preventDefault();
}

function onResize(event: PointerEvent) {
  if (!resizing.value) return;
  applyHeight(startHeight + (startY - event.clientY));
}

function endResize(event: PointerEvent) {
  if (!resizing.value) return;
  resizing.value = false;
  const handle = event.currentTarget as HTMLElement;
  if (handle.hasPointerCapture(event.pointerId)) {
    handle.releasePointerCapture(event.pointerId);
  }
  document.body.classList.remove("diff-resizing");
}

function resetHeight() {
  height.value = null;
  resizing.value = false;
  document.body.classList.remove("diff-resizing");
}

function onResizeKeydown(event: KeyboardEvent) {
  if (event.key === "ArrowUp" || event.key === "ArrowDown") {
    event.preventDefault();
    const current =
      sectionRef.value?.getBoundingClientRect().height ?? MIN_HEIGHT;
    applyHeight(current + (event.key === "ArrowUp" ? 24 : -24));
  } else if (event.key === "Home") {
    event.preventDefault();
    resetHeight();
  }
}

onBeforeUnmount(() => {
  document.body.classList.remove("diff-resizing");
});

interface Field {
  key: string;
  label: string;
  value: (item: ImageDetail) => string;
}

const fields: Field[] = [
  {
    key: "positive",
    label: "正面提示词",
    value: (item) => item.positivePrompt ?? "",
  },
  {
    key: "negative",
    label: "负面提示词",
    value: (item) => item.negativePrompt ?? "",
  },
  { key: "model", label: "模型", value: (item) => item.modelName ?? "" },
  { key: "sampler", label: "采样器", value: (item) => item.sampler ?? "" },
  {
    key: "steps",
    label: "步数",
    value: (item) => (item.steps === null ? "" : String(item.steps)),
  },
  {
    key: "cfg",
    label: "CFG",
    value: (item) => (item.cfg === null ? "" : String(item.cfg)),
  },
  {
    key: "seed",
    label: "种子",
    value: (item) => (item.seed === null ? "" : String(item.seed)),
  },
  { key: "loras", label: "LoRA", value: (item) => item.loras ?? "" },
];

const rows = computed(() =>
  fields.map((field) => {
    const values = props.items.map(field.value);
    return {
      ...field,
      values,
      different: new Set(values).size > 1,
    };
  }),
);

const differentCount = computed(
  () => rows.value.filter((row) => row.different).length,
);
</script>

<template>
  <section
    ref="sectionRef"
    class="diff-table"
    :class="{ resized: height !== null, resizing }"
    :style="height === null ? undefined : { height: `${height}px` }"
  >
    <div
      class="diff-resizer"
      role="separator"
      aria-orientation="horizontal"
      aria-label="调整参数差异栏高度"
      title="拖动调整高度，双击恢复默认"
      tabindex="0"
      @pointerdown="startResize"
      @pointermove="onResize"
      @pointerup="endResize"
      @pointercancel="endResize"
      @dblclick="resetHeight"
      @keydown="onResizeKeydown"
    />
    <header class="diff-head">
      <span class="diff-title">参数差异</span>
      <span class="diff-summary">{{ differentCount }} 项不同</span>
    </header>
    <div class="diff-scroll">
      <table>
        <thead>
          <tr>
            <th class="diff-field">参数</th>
            <th
              v-for="item in items"
              :key="item.id"
              :title="item.relPath"
            >
              {{ item.relPath }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="row in rows"
            :key="row.key"
            :class="{ different: row.different }"
          >
            <th class="diff-field">{{ row.label }}</th>
            <td
              v-for="(value, index) in row.values"
              :key="index"
              :title="value"
            >
              <span class="diff-value">{{ value || "—" }}</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
