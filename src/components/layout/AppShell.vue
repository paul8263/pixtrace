<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { RouterView } from "vue-router";
import TopToolbar from "./TopToolbar.vue";
import FilterSidebar from "./FilterSidebar.vue";
import StatusBar from "./StatusBar.vue";
import SettingsOverlay from "./SettingsOverlay.vue";
import { useLibraryStore } from "@/stores/library";
import { useWorkspaceStore } from "@/stores/workspace";

const SIDEBAR_MIN = 160;
const SIDEBAR_MAX = 480;
const SIDEBAR_KEY = "pixtrace.sidebarWidth";

const store = useWorkspaceStore();
const library = useLibraryStore();
const dismissedWarning = ref<string | null>(null);
const sidebarWidth = ref(loadSidebarWidth());
const sidebarResizing = ref(false);
let resizeStartX = 0;
let resizeStartWidth = 0;

const warning = computed(
  () => store.workspace?.warning ?? store.appInfo?.warning ?? null,
);

function clampSidebarWidth(value: number) {
  const max = Math.min(SIDEBAR_MAX, window.innerWidth * 0.45);
  return Math.round(Math.max(SIDEBAR_MIN, Math.min(value, max)));
}

function defaultSidebarWidth() {
  return window.innerWidth <= 1100 ? 180 : 220;
}

function loadSidebarWidth() {
  try {
    const stored = Number(localStorage.getItem(SIDEBAR_KEY));
    if (Number.isFinite(stored) && stored > 0) {
      return clampSidebarWidth(stored);
    }
  } catch {
    // Ignore storage failures and fall back to the default width.
  }
  return defaultSidebarWidth();
}

function persistSidebarWidth() {
  try {
    localStorage.setItem(SIDEBAR_KEY, String(sidebarWidth.value));
  } catch {
    // Width persistence is optional.
  }
}

function startSidebarResize(event: PointerEvent) {
  if (event.button !== 0) return;
  resizeStartX = event.clientX;
  resizeStartWidth = sidebarWidth.value;
  sidebarResizing.value = true;
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  document.body.classList.add("sidebar-resizing");
  event.preventDefault();
}

function onSidebarResize(event: PointerEvent) {
  if (!sidebarResizing.value) return;
  sidebarWidth.value = clampSidebarWidth(
    resizeStartWidth + event.clientX - resizeStartX,
  );
}

function endSidebarResize(event: PointerEvent) {
  if (!sidebarResizing.value) return;
  sidebarResizing.value = false;
  const handle = event.currentTarget as HTMLElement;
  if (handle.hasPointerCapture(event.pointerId)) {
    handle.releasePointerCapture(event.pointerId);
  }
  document.body.classList.remove("sidebar-resizing");
  persistSidebarWidth();
}

function resetSidebarWidth() {
  sidebarWidth.value = defaultSidebarWidth();
  sidebarResizing.value = false;
  document.body.classList.remove("sidebar-resizing");
  try {
    localStorage.removeItem(SIDEBAR_KEY);
  } catch {
    // Ignore storage failures.
  }
}

function onSidebarResizeKeydown(event: KeyboardEvent) {
  if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
    event.preventDefault();
    sidebarWidth.value = clampSidebarWidth(
      sidebarWidth.value + (event.key === "ArrowRight" ? 16 : -16),
    );
    persistSidebarWidth();
  } else if (event.key === "Home") {
    event.preventDefault();
    resetSidebarWidth();
  }
}

function onWindowResize() {
  sidebarWidth.value = clampSidebarWidth(sidebarWidth.value);
}

watch(warning, () => {
  dismissedWarning.value = null;
});

onMounted(() => {
  void store.init();
  void library.init();
  window.addEventListener("resize", onWindowResize);
});

onBeforeUnmount(() => {
  window.removeEventListener("resize", onWindowResize);
  document.body.classList.remove("sidebar-resizing");
});
</script>

<template>
  <div
    class="app-shell"
    :style="{ '--sidebar-width': `${sidebarWidth}px` }"
  >
    <TopToolbar />
    <FilterSidebar />
    <div
      class="sidebar-resizer"
      :class="{ resizing: sidebarResizing }"
      role="separator"
      aria-orientation="vertical"
      aria-label="调整左侧栏宽度"
      title="拖动调整宽度，双击恢复默认"
      tabindex="0"
      @pointerdown="startSidebarResize"
      @pointermove="onSidebarResize"
      @pointerup="endSidebarResize"
      @pointercancel="endSidebarResize"
      @dblclick="resetSidebarWidth"
      @keydown="onSidebarResizeKeydown"
    />
    <main class="app-main">
      <div v-if="store.error" class="error-banner">
        <span>{{ store.error }}</span>
        <button class="link-button" @click="store.clearError()">关闭</button>
      </div>
      <div
        v-if="warning && warning !== dismissedWarning"
        class="warning-banner"
      >
        <span>{{ warning }}</span>
        <button class="link-button" @click="dismissedWarning = warning">
          关闭
        </button>
      </div>
      <RouterView />
    </main>
    <StatusBar />
    <SettingsOverlay v-if="store.settingsOpen" />
  </div>
</template>
