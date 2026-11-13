<script setup lang="ts">
import { computed, ref } from "vue";
import { thumbUrl, type ImageSummary } from "@/api/backend";

const props = defineProps<{
  image: ImageSummary;
  base: string;
  active?: boolean;
}>();

const emit = defineEmits<{ (event: "open"): void }>();

const failed = ref(false);
const url = computed(() => thumbUrl(props.base, props.image, 320));

const statusLabel = computed(() => {
  switch (props.image.parseStatus) {
    case "error":
      return "解析失败";
    case "no_meta":
      return "无元数据";
    case "partial":
      return "部分解析";
    default:
      return "";
  }
});
</script>

<template>
  <button
    class="image-card"
    :class="{ active }"
    :title="image.relPath"
    type="button"
    @click="emit('open')"
  >
    <img
      v-if="!failed"
      class="image-card-thumb"
      :src="url"
      :alt="image.relPath"
      loading="lazy"
      decoding="async"
      @error="failed = true"
    />
    <span v-else class="image-card-fallback">无法预览</span>
    <span class="image-card-footer">
      <span class="image-card-name">{{ image.relPath }}</span>
      <span v-if="statusLabel" class="image-card-badge">{{ statusLabel }}</span>
    </span>
  </button>
</template>
