<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  getImageDetail,
  type ImageDetail,
  type ImageSummary,
} from "@/api/backend";
import {
  createViewport,
  resetViewport,
} from "@/composables/useSyncedViewport";
import ImageContextMenu from "@/components/library/ImageContextMenu.vue";
import { useCompareStore } from "@/stores/compare";
import { useLibraryStore } from "@/stores/library";
import { useWorkspaceStore } from "@/stores/workspace";
import ComparePane from "./ComparePane.vue";
import DiffTable from "./DiffTable.vue";

const compare = useCompareStore();
const library = useLibraryStore();
const workspace = useWorkspaceStore();

const details = ref<(ImageDetail | null)[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const sync = ref(true);
const resetToken = ref(0);
const contextMenu = ref<{
  image: ImageSummary;
  x: number;
  y: number;
} | null>(null);
const viewport = createViewport();

const base = computed(() => workspace.appInfo?.protocolBase ?? "");
const selected = computed(() => compare.selected);

const completeDetails = computed<ImageDetail[]>(() => {
  if (details.value.length !== selected.value.length) return [];
  const result: ImageDetail[] = [];
  for (const item of details.value) {
    if (!item) return [];
    result.push(item);
  }
  return result;
});

watch(
  () => compare.selected.map((item) => item.id).join(","),
  async () => {
    loading.value = true;
    error.value = null;
    details.value = await Promise.all(
      compare.selected.map((item) =>
        getImageDetail(item.id).catch(() => null),
      ),
    );
    loading.value = false;
  },
  { immediate: true },
);

function resetViews() {
  resetViewport(viewport);
  resetToken.value += 1;
}

function openContextMenu(image: ImageSummary, event: MouseEvent) {
  contextMenu.value = {
    image,
    x: event.clientX,
    y: event.clientY,
  };
}

function openContextImage() {
  const menu = contextMenu.value;
  if (!menu) return;
  contextMenu.value = null;
  compare.close();
  library.openImage(menu.image);
}

function requestDeleteFromMenu() {
  const menu = contextMenu.value;
  if (!menu) return;
  contextMenu.value = null;
  library.requestDelete(menu.image, { fromCompare: true });
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    if (contextMenu.value || library.deleteTarget) return;
    event.preventDefault();
    compare.close();
  }
}

onMounted(() => window.addEventListener("keydown", onKeydown));
onBeforeUnmount(() => window.removeEventListener("keydown", onKeydown));
</script>

<template>
  <Teleport to="body">
    <div class="compare-overlay">
      <header class="compare-topbar">
        <span class="compare-title">对比 {{ compare.count }} 张</span>
        <label class="compare-sync">
          <input v-model="sync" type="checkbox" />
          <span>同步缩放</span>
        </label>
        <button class="button ghost" type="button" @click="resetViews">
          重置视图
        </button>
        <span class="compare-hint">滚轮缩放 · 拖拽平移 · 双击复位</span>
        <span class="compare-spacer" />
        <button class="button ghost" type="button" @click="compare.close()">
          关闭
        </button>
      </header>

      <div v-if="error" class="library-error">
        <span>{{ error }}</span>
      </div>

      <div class="compare-panes">
        <ComparePane
          v-for="(image, index) in selected"
          :key="image.id"
          :image="image"
          :detail="details[index] ?? null"
          :base="base"
          :viewport="viewport"
          :sync="sync"
          :reset-token="resetToken"
          @contextmenu="openContextMenu(image, $event)"
        />
      </div>

      <DiffTable v-if="completeDetails.length" :items="completeDetails" />

      <div v-if="loading" class="compare-loading">正在加载参数…</div>

      <ImageContextMenu
        v-if="contextMenu"
        :image="contextMenu.image"
        :x="contextMenu.x"
        :y="contextMenu.y"
        @open="openContextImage"
        @delete="requestDeleteFromMenu"
        @close="contextMenu = null"
      />
    </div>
  </Teleport>
</template>
