<script setup lang="ts">
import { computed, ref } from "vue";
import { useWorkspaceStore } from "@/stores/workspace";
import ProtocolCheck from "@/components/common/ProtocolCheck.vue";
import AboutOverlay from "./AboutOverlay.vue";

const store = useWorkspaceStore();
const count = computed(() => store.workspace?.imageCount ?? 0);
const aboutOpen = ref(false);
</script>

<template>
  <footer class="status-bar">
    <span v-if="store.workspace">
      {{ count.toLocaleString() }} 张图片
    </span>
    <span v-else>未打开工作目录</span>
    <span class="status-separator">·</span>

    <span v-if="store.progress.running" class="status-progress">
      <span>{{ store.progress.phase }}</span>
      <span
        class="progress-track"
        :title="`${store.progress.done} / ${store.progress.total}`"
      >
        <span
          class="progress-fill"
          :style="{ width: `${store.progressPercent}%` }"
        />
      </span>
      <span>{{ store.progressPercent }}%</span>
      <span v-if="store.progress.failed" class="status-warning">
        失败 {{ store.progress.failed }}
      </span>
    </span>
    <span v-else-if="store.progress.total" class="status-muted-inline">
      {{ store.progress.phase }} · {{ store.progress.total.toLocaleString() }} 张已处理
    </span>
    <span v-else-if="store.progress.phase !== '空闲'">
      {{ store.progress.phase }}
    </span>
    <span v-else>{{ store.busy ? "扫描中" : "就绪" }}</span>

    <span class="status-spacer" />
    <span v-if="store.appInfo" class="status-muted">
      {{ store.appInfo.mode === "portable" ? "便携版" : "安装版" }} ·
      {{ store.appInfo.dataDir }}
    </span>
    <ProtocolCheck />
    <button
      class="status-about"
      type="button"
      title="关于 PixTrace"
      @click="aboutOpen = true"
    >
      关于
    </button>
    <AboutOverlay v-if="aboutOpen" @close="aboutOpen = false" />
  </footer>
</template>
