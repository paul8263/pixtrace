<script setup lang="ts">
import {
  computed,
  onBeforeUnmount,
  onMounted,
  reactive,
  ref,
  watch,
} from "vue";
import { imageUrl, type ImageDetail, type ImageSummary } from "@/api/backend";
import {
  panViewport,
  resetViewport,
  zoomViewport,
  type SyncedViewport,
} from "@/composables/useSyncedViewport";

const props = defineProps<{
  image: ImageSummary;
  detail: ImageDetail | null;
  base: string;
  viewport: SyncedViewport;
  sync: boolean;
  resetToken: number;
}>();

const pane = ref<HTMLDivElement | null>(null);
const failed = ref(false);
const local = reactive<SyncedViewport>({ cx: 0.5, cy: 0.5, scale: 1 });
const natural = reactive({ width: 0, height: 0 });
const container = reactive({ width: 0, height: 0 });

const active = computed(() => (props.sync ? props.viewport : local));
const src = computed(() => imageUrl(props.base, props.image));

const fit = computed(() => {
  if (natural.width <= 0 || natural.height <= 0) return 1;
  if (container.width <= 0 || container.height <= 0) return 1;
  return Math.min(
    1,
    container.width / natural.width,
    container.height / natural.height,
  );
});

const baseWidth = computed(() => natural.width * fit.value);
const baseHeight = computed(() => natural.height * fit.value);

const transform = computed(() => {
  const tx = (0.5 - active.value.cx) * baseWidth.value * active.value.scale;
  const ty = (0.5 - active.value.cy) * baseHeight.value * active.value.scale;
  return `translate(${tx}px, ${ty}px) scale(${active.value.scale})`;
});

const zoomLabel = computed(() => `${Math.round(active.value.scale * 100)}%`);

let observer: ResizeObserver | null = null;
let dragging = false;
let lastX = 0;
let lastY = 0;

function onLoad(event: Event) {
  const image = event.target as HTMLImageElement;
  natural.width = image.naturalWidth;
  natural.height = image.naturalHeight;
  failed.value = false;
}

function onWheel(event: WheelEvent) {
  const element = pane.value;
  if (!element) return;
  const rect = element.getBoundingClientRect();
  const px = (event.clientX - rect.left) / rect.width;
  const py = (event.clientY - rect.top) / rect.height;
  const factor = Math.exp(-event.deltaY * 0.0015);
  zoomViewport(
    active.value,
    factor,
    px,
    py,
    rect,
    baseWidth.value,
    baseHeight.value,
  );
}

function onPointerDown(event: PointerEvent) {
  if (event.button !== 0 || active.value.scale <= 1) return;
  dragging = true;
  lastX = event.clientX;
  lastY = event.clientY;
  try {
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  } catch {
    // Synthetic pointer events may not be capturable.
  }
}

function onPointerMove(event: PointerEvent) {
  if (!dragging) return;
  const dx = event.clientX - lastX;
  const dy = event.clientY - lastY;
  lastX = event.clientX;
  lastY = event.clientY;
  panViewport(active.value, dx, dy, baseWidth.value, baseHeight.value);
}

function onPointerUp(event: PointerEvent) {
  if (!dragging) return;
  dragging = false;
  const element = event.currentTarget as HTMLElement;
  if (element.hasPointerCapture(event.pointerId)) {
    element.releasePointerCapture(event.pointerId);
  }
}

function onDoubleClick() {
  resetViewport(active.value);
}

onMounted(() => {
  const element = pane.value;
  if (!element) return;
  observer = new ResizeObserver((entries) => {
    const rect = entries[0]?.contentRect;
    if (!rect) return;
    container.width = rect.width;
    container.height = rect.height;
  });
  observer.observe(element);
});

onBeforeUnmount(() => {
  observer?.disconnect();
  observer = null;
});

watch(
  () => props.resetToken,
  () => resetViewport(active.value),
);
</script>

<template>
  <section class="compare-pane">
    <header class="compare-pane-head">
      <span class="compare-pane-name" :title="image.relPath">
        {{ image.relPath }}
      </span>
      <span v-if="detail" class="compare-pane-meta">
        Seed {{ detail.seed ?? "—" }} · {{ detail.steps ?? "—" }} 步
      </span>
      <span class="compare-pane-zoom">{{ zoomLabel }}</span>
    </header>
    <div
      ref="pane"
      class="compare-pane-canvas"
      @wheel.prevent="onWheel"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
      @pointercancel="onPointerUp"
      @dblclick="onDoubleClick"
    >
      <img
        v-if="!failed"
        class="compare-pane-image"
        :src="src"
        :alt="image.relPath"
        :style="{ transform }"
        draggable="false"
        @load="onLoad"
        @error="failed = true"
      />
      <span v-else class="compare-pane-fallback">无法预览</span>
    </div>
  </section>
</template>
