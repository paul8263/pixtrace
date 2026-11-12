<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { testImageUrl } from "@/api/backend";
import { useWorkspaceStore } from "@/stores/workspace";

const store = useWorkspaceStore();
const state = ref<"idle" | "loading" | "ok" | "fail">("idle");

const url = computed(() =>
  store.appInfo ? testImageUrl(store.appInfo.protocolBase) : "",
);

watch(
  url,
  (value) => {
    state.value = value ? "loading" : "idle";
  },
  { immediate: true },
);

const label = computed(() => {
  switch (state.value) {
    case "ok":
      return "通过";
    case "fail":
      return "失败";
    case "loading":
      return "检测中";
    default:
      return "未检测";
  }
});
</script>

<template>
  <div class="protocol-check">
    <span class="protocol-label">自定义协议</span>
    <span :class="['protocol-badge', state]">{{ label }}</span>
    <img
      v-if="url"
      :src="url"
      class="protocol-probe"
      alt=""
      @load="state = 'ok'"
      @error="state = 'fail'"
    />
  </div>
</template>
