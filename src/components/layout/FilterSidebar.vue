<script setup lang="ts">
import { computed } from "vue";
import { RouterLink } from "vue-router";
import { useLibraryStore } from "@/stores/library";
import { useWorkspaceStore } from "@/stores/workspace";

const store = useWorkspaceStore();
const library = useLibraryStore();
const count = computed(() => store.workspace?.imageCount ?? 0);
const modeLabel = computed(() =>
  store.appInfo?.mode === "portable" ? "便携版" : "安装版",
);

const modelValue = computed({
  get: () => library.model ?? "",
  set: (value: string) => library.setModel(value || null),
});

const samplerValue = computed({
  get: () => library.sampler ?? "",
  set: (value: string) => library.setSampler(value || null),
});

const workflowValue = computed({
  get: () => library.hasWorkflow,
  set: (value: boolean) => library.setHasWorkflow(value),
});
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

      <label class="filter-field">
        <span class="filter-label">模型</span>
        <select v-model="modelValue" class="filter-select">
          <option value="">全部模型</option>
          <option
            v-for="item in library.filterOptions.models"
            :key="item"
            :value="item"
          >
            {{ item }}
          </option>
        </select>
      </label>

      <label class="filter-field">
        <span class="filter-label">采样器</span>
        <select v-model="samplerValue" class="filter-select">
          <option value="">全部采样器</option>
          <option
            v-for="item in library.filterOptions.samplers"
            :key="item"
            :value="item"
          >
            {{ item }}
          </option>
        </select>
      </label>

      <label class="filter-check">
        <input v-model="workflowValue" type="checkbox" />
        <span>仅含工作流</span>
      </label>

      <button
        v-if="library.hasFilters"
        class="link-button filter-clear"
        type="button"
        @click="library.clearFilters()"
      >
        清除筛选
      </button>

      <div
        v-if="
          !library.filterOptions.models.length &&
          !library.filterOptions.samplers.length
        "
        class="sidebar-placeholder"
      >
        索引完成后可筛选模型与采样器
      </div>
    </div>

    <div class="sidebar-footer">
      <div class="section-title">运行模式</div>
      <div class="mode-badge">{{ modeLabel }}</div>
    </div>
  </aside>
</template>
