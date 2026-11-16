<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
} from "vue";
import { thumbUrl, type ImageSummary } from "@/api/backend";
import { useLibraryStore } from "@/stores/library";
import { useWorkspaceStore } from "@/stores/workspace";

const PRELOAD_RADIUS = 14;
const OVERSCAN = 4;
const ITEM_GAP = 6;
const EDGE_PADDING = 12;

const props = defineProps<{
  height: number;
}>();

const library = useLibraryStore();
const workspace = useWorkspaceStore();
const strip = ref<HTMLDivElement | null>(null);
const scrollLeft = ref(0);
const viewportWidth = ref(0);
const ready = ref(false);
let resizeObserver: ResizeObserver | null = null;

const base = computed(() => workspace.appInfo?.protocolBase ?? "");
const thumbSize = computed(() => Math.max(40, Math.round(props.height - 20)));
const pitch = computed(() => thumbSize.value + ITEM_GAP);
const thumbRequestSize = computed(() => {
  if (thumbSize.value <= 96) return 160;
  if (thumbSize.value <= 176) return 320;
  if (thumbSize.value <= 256) return 480;
  return 640;
});

const renderStart = computed(() => {
  const first = Math.floor((scrollLeft.value - EDGE_PADDING) / pitch.value);
  return Math.max(0, first - OVERSCAN);
});

const renderEnd = computed(() => {
  const last = Math.ceil(
    (scrollLeft.value + viewportWidth.value - EDGE_PADDING) / pitch.value,
  );
  return Math.min(library.total, last + OVERSCAN);
});

interface Entry {
  index: number;
  image?: ImageSummary;
}

const entries = computed<Entry[]>(() => {
  const result: Entry[] = [];
  for (let index = renderStart.value; index < renderEnd.value; index += 1) {
    result.push({ index, image: library.itemAt(index) });
  }
  return result;
});

function targetScrollLeft(index: number) {
  return Math.max(
    0,
    EDGE_PADDING +
      index * pitch.value +
      thumbSize.value / 2 -
      viewportWidth.value / 2,
  );
}

function indexVisible(index: number) {
  const left = EDGE_PADDING + index * pitch.value;
  const right = left + thumbSize.value;
  return (
    left >= scrollLeft.value - 1 &&
    right <= scrollLeft.value + viewportWidth.value + 1
  );
}

function scrollToIndex(index: number, behavior: ScrollBehavior = "auto") {
  strip.value?.scrollTo({ left: targetScrollLeft(index), behavior });
}

function measure() {
  const el = strip.value;
  if (!el) return;
  viewportWidth.value = el.clientWidth;
  scrollLeft.value = el.scrollLeft;
}

function onScroll() {
  const el = strip.value;
  if (!el) return;
  scrollLeft.value = el.scrollLeft;
}

watch(
  () => library.activeIndex,
  async (active) => {
    if (active === null) return;
    library.ensureRange(active - PRELOAD_RADIUS, active + PRELOAD_RADIUS);
    await nextTick();
    measure();
    if (!indexVisible(active)) scrollToIndex(active, "smooth");
  },
  { immediate: true },
);

watch(
  [renderStart, renderEnd],
  ([start, end]) => {
    if (end > start) library.ensureRange(start, end - 1);
  },
  { immediate: true },
);

watch(
  () => props.height,
  async () => {
    await nextTick();
    measure();
    if (library.activeIndex !== null) scrollToIndex(library.activeIndex);
  },
);

function thumb(image: ImageSummary): string {
  return base.value
    ? thumbUrl(base.value, image, thumbRequestSize.value, workspace.cacheEpoch)
    : "";
}

onMounted(async () => {
  await nextTick();
  const el = strip.value;
  if (!el) return;
  viewportWidth.value = el.clientWidth;
  if (library.activeIndex !== null) {
    scrollLeft.value = targetScrollLeft(library.activeIndex);
  }
  ready.value = true;
  await nextTick();
  el.scrollLeft = scrollLeft.value;
  measure();
  resizeObserver = new ResizeObserver(measure);
  resizeObserver.observe(el);
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  resizeObserver = null;
});
</script>

<template>
  <div
    ref="strip"
    class="filmstrip"
    :style="{ '--filmstrip-thumb-size': `${thumbSize}px` }"
    @scroll.passive="onScroll"
  >
    <template v-if="ready">
      <div
        v-if="renderStart > 0"
        class="filmstrip-spacer"
        :style="{ width: `${renderStart * pitch}px` }"
      />
      <template v-for="entry in entries" :key="entry.index">
        <button
          v-if="entry.image"
          class="filmstrip-item"
          :class="{ active: entry.index === library.activeIndex }"
          :title="entry.image.relPath"
          type="button"
          @click="library.openAt(entry.index)"
        >
          <img
            :src="thumb(entry.image)"
            :alt="entry.image.relPath"
            loading="lazy"
            decoding="async"
          />
        </button>
        <div v-else class="filmstrip-item placeholder" aria-hidden="true" />
      </template>
      <div
        class="filmstrip-spacer"
        :style="{
          width: `${(library.total - renderEnd) * pitch + EDGE_PADDING}px`,
        }"
      />
    </template>
  </div>
</template>
