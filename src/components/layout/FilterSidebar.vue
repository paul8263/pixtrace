<script setup lang="ts">
import { computed } from "vue";
import { useRouter } from "vue-router";
import { useGroupsStore } from "@/stores/groups";
import { useLibraryStore } from "@/stores/library";
import { useWorkspaceStore } from "@/stores/workspace";

const store = useWorkspaceStore();
const library = useLibraryStore();
const groups = useGroupsStore();
const router = useRouter();

const count = computed(() => store.workspace?.imageCount ?? 0);
const modeLabel = computed(() =>
  store.appInfo?.mode === "portable" ? "便携版" : "安装版",
);

const ratingRows = [5, 4, 3, 2, 1];

const allActive = computed(
  () =>
    library.viewMode === "grid" &&
    !library.promptHash &&
    !library.isSearching,
);
const groupsActive = computed(() => library.viewMode === "groups");

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

function ratingCount(stars: number): number {
  return library.ratingMode === "min"
    ? (library.filterOptions.starsMin[stars - 1] ?? 0)
    : (library.filterOptions.starsExact[stars - 1] ?? 0);
}

function goLibrary() {
  void router.push("/library");
}

function showAll() {
  goLibrary();
  library.showAll();
}

function showGroups() {
  goLibrary();
  library.showGroups();
}

function showUnrated() {
  goLibrary();
  library.showUnrated();
}

function showRating(stars: number) {
  goLibrary();
  library.showRating(stars);
}
</script>

<template>
  <aside class="sidebar">
    <div class="nav-group">
      <button
        class="nav-item"
        :class="{ active: allActive }"
        type="button"
        @click="showAll"
      >
        <span>全部图片</span>
        <span class="nav-count">{{ count }}</span>
      </button>
      <button
        class="nav-item"
        :class="{ active: library.unrated }"
        type="button"
        @click="showUnrated"
      >
        <span>未评级</span>
        <span class="nav-count">{{ library.filterOptions.unrated }}</span>
      </button>

      <button
        v-for="stars in ratingRows"
        :key="stars"
        class="nav-item rating-nav-item"
        :class="{ active: library.rating === stars }"
        type="button"
        :title="
          library.ratingMode === 'min' ? `至少 ${stars} 星` : `恰好 ${stars} 星`
        "
        :aria-label="
          library.ratingMode === 'min' ? `至少 ${stars} 星` : `恰好 ${stars} 星`
        "
        @click="showRating(stars)"
      >
        <span class="rating-nav-label">
          <span class="rating-nav-filled">{{ "★".repeat(stars) }}</span>
          <span class="rating-nav-empty">{{ "☆".repeat(5 - stars) }}</span>
          <span v-if="library.ratingMode === 'min'" class="rating-nav-plus">
            +
          </span>
        </span>
        <span class="nav-count">{{ ratingCount(stars) }}</span>
      </button>

      <button
        class="nav-item"
        :class="{ active: groupsActive }"
        type="button"
        @click="showGroups"
      >
        <span>提示词分组</span>
        <span v-if="groups.total" class="nav-count">{{ groups.total }}</span>
      </button>
    </div>

    <div class="sidebar-section">
      <div class="section-title">筛选</div>

      <div class="rating-mode">
        <span class="filter-label">评级匹配</span>
        <div class="segmented">
          <button
            type="button"
            :class="{ active: library.ratingMode === 'min' }"
            @click="library.setRatingMode('min')"
          >
            至少
          </button>
          <button
            type="button"
            :class="{ active: library.ratingMode === 'exact' }"
            @click="library.setRatingMode('exact')"
          >
            等于
          </button>
        </div>
      </div>

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
