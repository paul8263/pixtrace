<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useZoomPan } from "@/composables/useZoomPan";

const props = defineProps<{
  src: string;
  alt?: string;
}>();

const container = ref<HTMLDivElement | null>(null);
const image = ref<HTMLImageElement | null>(null);
const loading = ref(true);
const failed = ref(false);
const zoom = useZoomPan(container);

let observer: ResizeObserver | null = null;

function syncContainerSize() {
  const element = container.value;
  if (!element) return;
  const rect = element.getBoundingClientRect();
  zoom.setContainerSize(rect.width, rect.height);
}

function onImageLoad() {
  const element = image.value;
  if (!element) return;
  loading.value = false;
  zoom.setNaturalSize(element.naturalWidth, element.naturalHeight);
}

function onImageError() {
  loading.value = false;
  failed.value = true;
}

function onKeydown(event: KeyboardEvent) {
  if (event.target instanceof HTMLInputElement) return;
  if (event.key === "+" || event.key === "=") {
    zoom.zoomBy(1.2);
    event.preventDefault();
  } else if (event.key === "-") {
    zoom.zoomBy(1 / 1.2);
    event.preventDefault();
  } else if (event.key === "0") {
    zoom.fit();
    event.preventDefault();
  } else if (event.key === "1") {
    zoom.oneToOne();
    event.preventDefault();
  }
}

watch(
  () => props.src,
  () => {
    zoom.reset();
    loading.value = true;
    failed.value = false;
  },
);

onMounted(() => {
  syncContainerSize();
  const element = container.value;
  if (element) {
    observer = new ResizeObserver(() => syncContainerSize());
    observer.observe(element);
  }
  window.addEventListener("keydown", onKeydown);
});

onBeforeUnmount(() => {
  observer?.disconnect();
  observer = null;
  window.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div
    ref="container"
    class="zoom-viewer"
    :class="{ dragging: zoom.dragging.value, pixelated: zoom.pixelated.value }"
    @wheel="zoom.onWheel"
    @pointerdown="zoom.onPointerDown"
    @pointermove="zoom.onPointerMove"
    @pointerup="zoom.onPointerUp"
    @pointercancel="zoom.onPointerUp"
    @dblclick="zoom.toggleOneToOne"
  >
    <div class="zoom-canvas" :style="zoom.transformStyle.value">
      <img
        ref="image"
        :src="src"
        :alt="alt ?? ''"
        draggable="false"
        @load="onImageLoad"
        @error="onImageError"
      />
    </div>

    <div v-if="loading && !failed" class="viewer-hint">加载中…</div>
    <div v-else-if="failed" class="viewer-hint error">图像加载失败</div>

    <div class="viewer-toolbar" @pointerdown.stop @dblclick.stop>
      <button class="viewer-button" title="缩小 (-)" @click="zoom.zoomBy(1 / 1.2)">
        −
      </button>
      <span class="viewer-percent">{{ zoom.percent.value }}%</span>
      <button class="viewer-button" title="放大 (+)" @click="zoom.zoomBy(1.2)">
        +
      </button>
      <button class="viewer-button" title="适应窗口 (0)" @click="zoom.fit()">
        适应
      </button>
      <button class="viewer-button" title="原始尺寸 (1)" @click="zoom.oneToOne()">
        1:1
      </button>
    </div>
  </div>
</template>
