<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { getImageDetail, imageUrl, type ImageDetail } from "@/api/backend";
import RatingStars from "@/components/common/RatingStars.vue";
import { useLibraryStore } from "@/stores/library";
import { useWorkspaceStore } from "@/stores/workspace";
import Filmstrip from "./Filmstrip.vue";
import MetadataPanel from "./MetadataPanel.vue";
import ZoomPanViewer from "./ZoomPanViewer.vue";

const library = useLibraryStore();
const workspace = useWorkspaceStore();

const PANEL_MIN = 260;
const PANEL_MAX = 620;
const PANEL_KEY = "pixtrace.detailPanelWidth";

const detail = ref<ImageDetail | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);
const panelWidth = ref(loadPanelWidth());
const panelResizing = ref(false);
let panelStartX = 0;
let panelStartWidth = 0;

function clampPanelWidth(value: number) {
  const max = Math.min(PANEL_MAX, window.innerWidth * 0.6);
  return Math.round(Math.max(PANEL_MIN, Math.min(value, max)));
}

function loadPanelWidth() {
  try {
    const stored = Number(localStorage.getItem(PANEL_KEY));
    if (Number.isFinite(stored) && stored > 0) {
      return clampPanelWidth(stored);
    }
  } catch {
    // Ignore storage failures and fall back to the default width.
  }
  return 360;
}

function persistPanelWidth() {
  try {
    localStorage.setItem(PANEL_KEY, String(panelWidth.value));
  } catch {
    // Width persistence is optional.
  }
}

function startPanelResize(event: PointerEvent) {
  if (event.button !== 0) return;
  panelStartX = event.clientX;
  panelStartWidth = panelWidth.value;
  panelResizing.value = true;
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  document.body.classList.add("detail-panel-resizing");
  event.preventDefault();
}

function onPanelResize(event: PointerEvent) {
  if (!panelResizing.value) return;
  panelWidth.value = clampPanelWidth(
    panelStartWidth - (event.clientX - panelStartX),
  );
}

function endPanelResize(event: PointerEvent) {
  if (!panelResizing.value) return;
  panelResizing.value = false;
  const handle = event.currentTarget as HTMLElement;
  if (handle.hasPointerCapture(event.pointerId)) {
    handle.releasePointerCapture(event.pointerId);
  }
  document.body.classList.remove("detail-panel-resizing");
  persistPanelWidth();
}

function resetPanelWidth() {
  panelWidth.value = 360;
  panelResizing.value = false;
  document.body.classList.remove("detail-panel-resizing");
  try {
    localStorage.removeItem(PANEL_KEY);
  } catch {
    // Ignore storage failures.
  }
}

function onPanelResizeKeydown(event: KeyboardEvent) {
  if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
    event.preventDefault();
    event.stopPropagation();
    panelWidth.value = clampPanelWidth(
      panelWidth.value + (event.key === "ArrowLeft" ? 16 : -16),
    );
    persistPanelWidth();
  } else if (event.key === "Home") {
    event.preventDefault();
    event.stopPropagation();
    resetPanelWidth();
  }
}

function onWindowResize() {
  panelWidth.value = clampPanelWidth(panelWidth.value);
}

const item = computed(() => library.activeItem);

const imageSrc = computed(() => {
  if (!item.value || !workspace.appInfo) return "";
  return imageUrl(workspace.appInfo.protocolBase, item.value);
});

const position = computed(() => {
  if (library.activeIndex === null) return "单张图片";
  return `${(library.activeIndex + 1).toLocaleString()} / ${library.total.toLocaleString()}`;
});

const stars = computed(() => item.value?.stars ?? null);

function setStars(value: number | null) {
  if (item.value) {
    void library.setRating(item.value.id, value);
  }
}

function isTyping(event: KeyboardEvent): boolean {
  const target = event.target as HTMLElement | null;
  return (
    target?.tagName === "INPUT" ||
    target?.tagName === "TEXTAREA" ||
    target?.tagName === "SELECT"
  );
}

watch(
  [() => item.value?.id, () => library.revision],
  async ([id]) => {
    if (id === undefined || id === null) {
      detail.value = null;
      return;
    }
    loading.value = true;
    error.value = null;
    try {
      detail.value = await getImageDetail(id);
    } catch (cause) {
      detail.value = null;
      error.value = String(cause);
    } finally {
      loading.value = false;
    }
  },
  { immediate: true },
);

function onKeydown(event: KeyboardEvent) {
  if (isTyping(event)) return;
  if (event.key === "Escape") {
    event.preventDefault();
    library.close();
  } else if (event.key === "ArrowLeft") {
    event.preventDefault();
    void library.step(-1);
  } else if (event.key === "ArrowRight") {
    event.preventDefault();
    void library.step(1);
  } else if (event.key >= "1" && event.key <= "5") {
    event.preventDefault();
    setStars(Number(event.key));
  }
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown);
  window.addEventListener("resize", onWindowResize);
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown);
  window.removeEventListener("resize", onWindowResize);
  document.body.classList.remove("detail-panel-resizing");
});
</script>

<template>
  <Teleport to="body">
    <div
      class="detail-overlay"
      :class="{ single: library.activeIndex === null }"
    >
      <header class="detail-topbar">
        <button class="button ghost" @click="library.close()">← 返回</button>
        <span class="detail-filename" :title="item?.relPath">
          {{ item?.relPath ?? "载入中…" }}
        </span>
        <div class="detail-actions">
          <button
            class="button ghost"
            :disabled="
              library.activeIndex === null || library.activeIndex <= 0
            "
            @click="library.step(-1)"
          >
            上一张
          </button>
          <span class="detail-position">{{ position }}</span>
          <button
            class="button ghost"
            :disabled="
              library.activeIndex === null ||
              library.activeIndex >= library.total - 1
            "
            @click="library.step(1)"
          >
            下一张
          </button>
          <RatingStars
            :model-value="stars"
            class="detail-stars"
            @update:model-value="setStars"
          />
        </div>
      </header>

      <div
        class="detail-body"
        :style="{ '--detail-panel-width': `${panelWidth}px` }"
      >
        <div class="detail-viewer">
          <ZoomPanViewer
            v-if="imageSrc"
            :src="imageSrc"
            :alt="item?.relPath ?? ''"
          />
        </div>
        <MetadataPanel :detail="detail" :loading="loading" :error="error" />
        <div
          class="detail-panel-resizer"
          :class="{ resizing: panelResizing }"
          role="separator"
          aria-orientation="vertical"
          aria-label="调整详情面板宽度"
          title="拖动调整宽度，双击恢复默认"
          tabindex="0"
          @pointerdown="startPanelResize"
          @pointermove="onPanelResize"
          @pointerup="endPanelResize"
          @pointercancel="endPanelResize"
          @dblclick="resetPanelWidth"
          @keydown="onPanelResizeKeydown"
        />
      </div>

      <Filmstrip v-if="library.activeIndex !== null" />
    </div>
  </Teleport>
</template>
