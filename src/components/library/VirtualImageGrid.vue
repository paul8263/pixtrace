<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { ImageSummary } from "@/api/backend";
import { useLibraryStore } from "@/stores/library";
import { useSelectionStore } from "@/stores/selection";
import { useWorkspaceStore } from "@/stores/workspace";
import ImageCard from "./ImageCard.vue";

const CARD_SIZE = 176;
const GAP = 10;
const PADDING = 16;
const ROW_HEIGHT = CARD_SIZE + GAP;

const library = useLibraryStore();
const selection = useSelectionStore();
const workspace = useWorkspaceStore();

const scroller = ref<HTMLDivElement | null>(null);
const viewportWidth = ref(0);
const viewportHeight = ref(0);
const scrollTop = ref(0);

const base = computed(() => workspace.appInfo?.protocolBase ?? "");

const columns = computed(() => {
  const available = viewportWidth.value - PADDING * 2 + GAP;
  return Math.max(1, Math.floor(available / (CARD_SIZE + GAP)));
});

const rowCount = computed(() =>
  columns.value > 0 ? Math.ceil(library.total / columns.value) : 0,
);

const contentHeight = computed(() => {
  if (rowCount.value === 0) return 0;
  return PADDING * 2 + rowCount.value * ROW_HEIGHT - GAP;
});

const startRow = computed(() =>
  Math.max(0, Math.floor((scrollTop.value - PADDING) / ROW_HEIGHT) - 1),
);

const endRow = computed(() =>
  Math.min(
    rowCount.value,
    Math.ceil((scrollTop.value + viewportHeight.value - PADDING) / ROW_HEIGHT) + 1,
  ),
);

const startIndex = computed(() => startRow.value * columns.value);
const endIndex = computed(() =>
  Math.min(library.total, endRow.value * columns.value) - 1,
);

interface Cell {
  index: number;
  image: ImageSummary;
  left: number;
  top: number;
}

const visibleCells = computed<Cell[]>(() => {
  const cells: Cell[] = [];
  const cols = columns.value;
  if (cols <= 0) return cells;
  const first = startRow.value * cols;
  const last = Math.min(library.total, endRow.value * cols);
  for (let index = first; index < last; index += 1) {
    const image = library.itemAt(index);
    if (!image) continue;
    cells.push({
      index,
      image,
      left: PADDING + (index % cols) * (CARD_SIZE + GAP),
      top: PADDING + Math.floor(index / cols) * ROW_HEIGHT,
    });
  }
  return cells;
});

let observer: ResizeObserver | null = null;
let frame = 0;

function onScroll() {
  if (frame) return;
  frame = requestAnimationFrame(() => {
    frame = 0;
    scrollTop.value = scroller.value?.scrollTop ?? 0;
  });
}

onMounted(() => {
  const element = scroller.value;
  if (!element) return;
  observer = new ResizeObserver((entries) => {
    const rect = entries[0]?.contentRect;
    if (!rect) return;
    viewportWidth.value = rect.width;
    viewportHeight.value = rect.height;
    scrollTop.value = element.scrollTop;
  });
  observer.observe(element);
  element.addEventListener("scroll", onScroll, { passive: true });
});

onBeforeUnmount(() => {
  observer?.disconnect();
  observer = null;
  if (frame) cancelAnimationFrame(frame);
  scroller.value?.removeEventListener("scroll", onScroll);
});

watch(
  [startIndex, endIndex, columns, () => library.generation, () => library.total],
  () => {
    if (endIndex.value >= startIndex.value) {
      library.ensureRange(startIndex.value, endIndex.value);
    }
  },
  { immediate: true },
);

function open(index: number) {
  void library.openAt(index);
}

function openContextMenu(
  image: ImageSummary,
  index: number,
  event: MouseEvent,
) {
  library.openContextMenu(image, index, event.clientX, event.clientY);
}
</script>

<template>
  <div ref="scroller" class="virtual-grid-scroller">
    <div class="virtual-grid-content" :style="{ height: `${contentHeight}px` }">
      <ImageCard
        v-for="cell in visibleCells"
        :key="cell.image.id"
        class="virtual-grid-cell"
        :style="{
          transform: `translate(${cell.left}px, ${cell.top}px)`,
          width: `${CARD_SIZE}px`,
          height: `${CARD_SIZE}px`,
        }"
        :image="cell.image"
        :base="base"
        :active="library.activeIndex === cell.index"
        :selected="selection.isSelected(cell.image.id)"
        :cache-epoch="workspace.cacheEpoch"
        @open="open(cell.index)"
        @toggle-select="selection.toggle(cell.image.id)"
        @rate="library.setRating(cell.image.id, $event)"
        @contextmenu="openContextMenu(cell.image, cell.index, $event)"
      />
    </div>
    <div v-if="library.loaded && library.total === 0" class="grid-empty">
      <div class="empty-mark" />
      <template v-if="library.isSearching || library.promptHash">
        <h2>没有匹配的图像</h2>
        <p>尝试更换关键词，或清除筛选条件。</p>
        <button
          class="button empty-action"
          type="button"
          @click="library.clearSearch()"
        >
          清除搜索
        </button>
      </template>
      <template v-else>
        <h2>目录中没有图像</h2>
        <p>受支持的格式为 PNG / JPEG / WebP。</p>
      </template>
    </div>
    <div v-else-if="!library.loaded" class="grid-empty">
      <p class="muted">
        {{
          library.isSearching || library.promptHash
            ? "正在搜索…"
            : "正在读取索引…"
        }}
      </p>
    </div>
  </div>
</template>
