<script setup lang="ts">
import { computed } from "vue";
import { useWorkspaceStore } from "@/stores/workspace";
import ProtocolCheck from "@/components/common/ProtocolCheck.vue";

const store = useWorkspaceStore();
const count = computed(() => store.workspace?.imageCount ?? 0);
</script>

<template>
  <footer class="status-bar">
    <span v-if="store.workspace">
      {{ count.toLocaleString() }} 张图片
    </span>
    <span v-else>未打开工作目录</span>
    <span class="status-separator">·</span>
    <span>{{ store.busy ? "扫描中" : "就绪" }}</span>
    <span class="status-spacer" />
    <span v-if="store.appInfo" class="status-muted">
      {{ store.appInfo.mode === "portable" ? "便携版" : "安装版" }} ·
      {{ store.appInfo.dataDir }}
    </span>
    <ProtocolCheck />
  </footer>
</template>
