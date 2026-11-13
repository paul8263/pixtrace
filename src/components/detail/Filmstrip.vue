<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { thumbUrl, type ImageSummary } from "@/api/backend";
import { useLibraryStore } from "@/stores/library";
import { useWorkspaceStore } from "@/stores/workspace";

const RADIUS = 14;

const library = useLibraryStore();
const workspace = useWorkspaceStore();
const strip = ref<HTMLDivElement | null>(null);

const base = computed(() => workspace.appInfo?.protocolBase ?? "");

interface Entry {
  index: number;
  image: ImageSummary;
}

const entries = computed<Entry[]>(() => {
  const active = library.activeIndex;
  if (active === null) return [];
  const start = Math.max(0, active - RADIUS);
  const end = Math.min(library.total, active + RADIUS + 1);
  const result: Entry[] = [];
  for (let index = start; index < end; index += 1) {
    const image = library.itemAt(index);
    if (image) result.push({ index, image });
  }
  return result;
});

watch(
  () => library.activeIndex,
  async (active) => {
    if (active === null) return;
    library.ensureRange(active - RADIUS, active + RADIUS);
    await nextTick();
    strip.value
      ?.querySelector(".filmstrip-item.active")
      ?.scrollIntoView({ block: "nearest", inline: "center", behavior: "smooth" });
  },
  { immediate: true },
);

function thumb(image: ImageSummary): string {
  return base.value
    ? thumbUrl(base.value, image, 160, workspace.cacheEpoch)
    : "";
}
</script>

<template>
  <div ref="strip" class="filmstrip">
    <button
      v-for="entry in entries"
      :key="entry.image.id"
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
  </div>
</template>
