<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { ImageSummary } from "@/api/backend";
import { useLibraryStore } from "@/stores/library";
import { useSelectionStore } from "@/stores/selection";
import { useWorkspaceStore } from "@/stores/workspace";
import ImageCard from "./ImageCard.vue";

const DEFAULT_CARD_SIZE = 176;
const MIN_CARD_SIZE = 96;
const MAX_CARD_SIZE = 320;
const CARD_SIZE_STEP = 4;
const CARD_SIZE_KEY = "pixtrace.gridCardSize";
const GAP = 10;
const PADDING = 16;

const library = useLibraryStore();
const selection = useSelectionStore();
const workspace = useWorkspaceStore();

const scroller = ref<HTMLDivElement | null>(null);
const viewportWidth = ref(0);
const viewportHeight = ref(0);
const scrollTop = ref(0);
const cardSize = ref(loadCardSize());

const base = computed(() => workspace.appInfo?.protocolBase ?? "");
const rowHeight = computed(() => cardSize.value + GAP);
const zoomPercent = computed(() =>
  Math.round((cardSize.value / DEFAULT_CARD_SIZE) * 100),
);
const thumbSize = computed(() => {
  if (cardSize.value <= 200) return 320;
  if (cardSize.value <= 260) return 480;
  return 640;
});

const columns = computed(() => {
  const available = viewportWidth.value - PADDING * 2 + GAP;
  return Math.max(1, Math.floor(available / (cardSize.value + GAP)));
});

function clampCardSize(value: number) {
  return Math.round(
    Math.min(MAX_CARD_SIZE, Math.max(MIN_CARD_SIZE, value)),
  );
}

function loadCardSize() {
  try {
    const stored = Number(localStorage.getItem(CARD_SIZE_KEY));
    if (Number.isFinite(stored) && stored > 0) {
      return clampCardSize(stored);
    }
  } catch {
    // Ignore storage failures and fall back to the default size.
  }
  return DEFAULT_CARD_SIZE;
}

function persistCardSize(value: number) {
  try {
    localStorage.setItem(CARD_SIZE_KEY, String(value));
  } catch {
    // Size persistence is optional.
  }
}

function setCardSize(value: number) {
  const next = clampCardSize(value);
  if (next === cardSize.value) return;
  const element = scroller.value;
  const oldScrollTop = element?.scrollTop ?? 0;
  const oldRowHeight = rowHeight.value;
  const oldRow = Math.max(
    0,
    Math.ceil((oldScrollTop - PADDING) / oldRowHeight),
  );
  const oldAnchorTop = PADDING + oldRow * oldRowHeight;
  const anchorIndex = oldRow * columns.value;
  cardSize.value = next;
  persistCardSize(next);
  if (!element) return;
  void nextTick(() => {
    const cols = columns.value;
    if (cols <= 0) return;
    const row = Math.floor(anchorIndex / cols);
    const anchorTop = PADDING + row * rowHeight.value;
    element.scrollTop = Math.max(
      0,
      anchorTop - (oldAnchorTop - oldScrollTop),
    );
  });
}

function onSizeSliderInput(event: Event) {
  setCardSize(Number((event.target as HTMLInputElement).value));
}

function onWheel(event: WheelEvent) {
  if (!event.ctrlKey) return;
  event.preventDefault();
  setCardSize(cardSize.value * Math.exp(-event.deltaY * 0.0015));
}

const rowCount = computed(() =>
  columns.value > 0 ? Math.ceil(library.total / columns.value) : 0,
);

const contentHeight = computed(() => {
  if (rowCount.value === 0) return 0;
  return PADDING * 2 + rowCount.value * rowHeight.value - GAP;
});

const startRow = computed(() =>
  Math.max(0, Math.floor((scrollTop.value - PADDING) / rowHeight.value) - 1),
);

const endRow = computed(() =>
  Math.min(
    rowCount.value,
    Math.ceil(
      (scrollTop.value + viewportHeight.value - PADDING) / rowHeight.value,
    ) + 1,
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
      left: PADDING + (index % cols) * (cardSize.value + GAP),
      top: PADDING + Math.floor(index / cols) * rowHeight.value,
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
  <div class="virtual-grid-shell">
    <div
      ref="scroller"
      class="virtual-grid-scroller"
      @wheel="onWheel"
    >
      <div class="virtual-grid-content" :style="{ height: `${contentHeight}px` }">
        <ImageCard
          v-for="cell in visibleCells"
          :key="cell.image.id"
          class="virtual-grid-cell"
          :style="{
            transform: `translate(${cell.left}px, ${cell.top}px)`,
            width: `${cardSize}px`,
            height: `${cardSize}px`,
          }"
          :image="cell.image"
          :base="base"
          :active="library.activeIndex === cell.index"
          :selected="selection.isSelected(cell.image.id)"
          :cache-epoch="workspace.cacheEpoch"
          :thumb-size="thumbSize"
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

    <div
      class="grid-zoom-control"
      title="拖动调整缩略图显示比例；按住 Ctrl 滚动鼠标也可调整，双击恢复默认"
      @dblclick="setCardSize(DEFAULT_CARD_SIZE)"
    >
      <span class="grid-zoom-label">显示比例</span>
      <input
        class="grid-zoom-slider"
        type="range"
        :min="MIN_CARD_SIZE"
        :max="MAX_CARD_SIZE"
        :step="CARD_SIZE_STEP"
        :value="cardSize"
        aria-label="缩略图显示比例"
        @input="onSizeSliderInput"
      />
      <span class="grid-zoom-value">{{ zoomPercent }}%</span>
    </div>
  </div>
</template>
