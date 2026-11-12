<script setup lang="ts">
import { computed } from "vue";
import { imageUrl, type ImageSummary } from "@/api/backend";
import EmptyState from "@/components/common/EmptyState.vue";
import { useWorkspaceStore } from "@/stores/workspace";

const store = useWorkspaceStore();

const scannedAt = computed(() => {
  const value = store.workspace?.scannedAt ?? 0;
  if (!value) return "—";
  return new Date(value * 1000).toLocaleString("zh-CN", { hour12: false });
});

function previewUrl(image: ImageSummary): string {
  if (!store.appInfo) return "";
  return imageUrl(store.appInfo.protocolBase, image);
}
</script>

<template>
  <section class="library-view">
    <EmptyState
      v-if="!store.workspace"
      title="打开一个工作目录"
      description="PixTrace 会扫描其中的 ComfyUI 图像，并在目录内建立本地索引。"
    >
      <button
        class="button primary"
        :disabled="store.busy"
        @click="store.chooseWorkspace()"
      >
        选择目录
      </button>
    </EmptyState>

    <template v-else>
      <div class="summary-grid">
        <article class="summary-card">
          <span>图像数量</span>
          <strong>{{ store.workspace?.imageCount.toLocaleString() }}</strong>
          <small :title="store.workspace?.root">{{ store.workspace?.root }}</small>
        </article>
        <article class="summary-card">
          <span>索引版本</span>
          <strong>v{{ store.workspace?.schemaVersion }}</strong>
          <small :title="store.workspace?.dbPath">{{ store.workspace?.dbPath }}</small>
        </article>
        <article class="summary-card">
          <span>最近扫描</span>
          <strong>{{ scannedAt }}</strong>
          <small>M0 仅扫描文件，不解析元数据</small>
        </article>
      </div>

      <div class="section-heading">
        <h2>目录预览</h2>
        <span>前 {{ store.images.length }} 张</span>
      </div>

      <div v-if="store.images.length" class="preview-grid">
        <figure
          v-for="image in store.images"
          :key="image.id"
          class="preview-card"
        >
          <div class="preview-frame">
            <img :src="previewUrl(image)" :alt="image.relPath" loading="lazy" />
          </div>
          <figcaption :title="image.relPath">{{ image.relPath }}</figcaption>
        </figure>
      </div>
      <p v-else class="muted">
        目录中没有受支持的图像文件（PNG / JPEG / WebP）。
      </p>
    </template>
  </section>
</template>
