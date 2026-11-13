<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import type { ImageSummary } from "@/api/backend";

const props = defineProps<{
  image: ImageSummary;
  x: number;
  y: number;
}>();

const emit = defineEmits<{
  (event: "open"): void;
  (event: "delete"): void;
  (event: "close"): void;
}>();

const firstItem = ref<HTMLButtonElement | null>(null);

const position = computed(() => ({
  left: `${Math.max(8, Math.min(props.x, window.innerWidth - 232))}px`,
  top: `${Math.max(8, Math.min(props.y, window.innerHeight - 148))}px`,
}));

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    emit("close");
  }
}

onMounted(async () => {
  window.addEventListener("keydown", onKeydown);
  await nextTick();
  firstItem.value?.focus();
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div
    class="context-menu-backdrop"
    @click="emit('close')"
    @contextmenu.prevent="emit('close')"
  >
    <div
      class="context-menu"
      :style="position"
      role="menu"
      @click.stop
      @contextmenu.prevent
    >
      <div class="context-menu-title" :title="image.relPath">
        {{ image.relPath }}
      </div>
      <button
        ref="firstItem"
        class="context-menu-item"
        type="button"
        role="menuitem"
        @click="emit('open')"
      >
        打开图片
      </button>
      <button
        class="context-menu-item danger"
        type="button"
        role="menuitem"
        @click="emit('delete')"
      >
        删除图片…
      </button>
    </div>
  </div>
</template>
