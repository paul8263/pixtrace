<script setup lang="ts">
import { computed, ref } from "vue";
import { thumbUrl, type ImageSummary } from "@/api/backend";
import RatingStars from "@/components/common/RatingStars.vue";

const props = defineProps<{
  image: ImageSummary;
  base: string;
  active?: boolean;
  selected?: boolean;
  selectable?: boolean;
  cacheEpoch?: number;
}>();

const emit = defineEmits<{
  (event: "open"): void;
  (event: "toggle-select"): void;
  (event: "rate", stars: number | null): void;
}>();

const failed = ref(false);
const url = computed(() =>
  thumbUrl(props.base, props.image, 320, props.cacheEpoch ?? 0),
);

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
  <div
    class="image-card"
    :class="{ active, selected }"
  >
    <button
      class="image-card-open"
      type="button"
      :title="image.relPath"
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
    </button>

    <label
      class="image-card-select"
      title="选择用于对比"
      @click.stop
    >
      <input
        type="checkbox"
        :checked="selected"
        :disabled="!selectable && !selected"
        @change="emit('toggle-select')"
      />
    </label>

    <div
      class="image-card-rating"
      :class="{ visible: image.stars !== null }"
      @click.stop
    >
      <RatingStars
        :model-value="image.stars"
        small
        @update:model-value="emit('rate', $event)"
      />
    </div>

    <span class="image-card-footer">
      <span class="image-card-name">{{ image.relPath }}</span>
      <span v-if="statusLabel" class="image-card-badge">{{ statusLabel }}</span>
    </span>
  </div>
</template>
