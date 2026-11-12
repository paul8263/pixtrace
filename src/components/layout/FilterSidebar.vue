<script setup lang="ts">
import { computed } from "vue";
import { RouterLink } from "vue-router";
import { useWorkspaceStore } from "@/stores/workspace";

const store = useWorkspaceStore();
const count = computed(() => store.workspace?.imageCount ?? 0);
const modeLabel = computed(() =>
  store.appInfo?.mode === "portable" ? "便携版" : "安装版",
);
</script>

<template>
  <aside class="sidebar">
    <div class="nav-group">
      <RouterLink class="nav-item" active-class="active" to="/library">
        <span>全部图片</span>
        <span class="nav-count">{{ count }}</span>
      </RouterLink>
      <button class="nav-item" disabled>
        未评级 <span class="soon">M3</span>
      </button>
      <button class="nav-item" disabled>
        星级筛选 <span class="soon">M3</span>
      </button>
      <button class="nav-item" disabled>
        提示词分组 <span class="soon">M3</span>
      </button>
    </div>

    <div class="sidebar-section">
      <div class="section-title">筛选</div>
      <div class="sidebar-placeholder">模型 / 采样器（M2）</div>
    </div>

    <div class="sidebar-footer">
      <div class="section-title">运行模式</div>
      <div class="mode-badge">{{ modeLabel }}</div>
    </div>
  </aside>
</template>
