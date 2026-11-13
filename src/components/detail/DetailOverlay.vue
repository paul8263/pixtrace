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

const detail = ref<ImageDetail | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);

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

onMounted(() => window.addEventListener("keydown", onKeydown));
onBeforeUnmount(() => window.removeEventListener("keydown", onKeydown));
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

      <div class="detail-body">
        <div class="detail-viewer">
          <ZoomPanViewer
            v-if="imageSrc"
            :src="imageSrc"
            :alt="item?.relPath ?? ''"
          />
        </div>
        <MetadataPanel :detail="detail" :loading="loading" :error="error" />
      </div>

      <Filmstrip v-if="library.activeIndex !== null" />
    </div>
  </Teleport>
</template>
