<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import {
  searchImages,
  thumbUrlFor,
  type GroupSort,
  type PromptGroup,
} from "@/api/backend";
import { useCompareStore } from "@/stores/compare";
import { useGroupsStore } from "@/stores/groups";
import { useLibraryStore } from "@/stores/library";
import { useWorkspaceStore } from "@/stores/workspace";

const groups = useGroupsStore();
const library = useLibraryStore();
const compare = useCompareStore();
const workspace = useWorkspaceStore();

const base = computed(() => workspace.appInfo?.protocolBase ?? "");

const sortValue = computed({
  get: () => groups.sort,
  set: (value: GroupSort) => groups.setSort(value),
});

onMounted(() => {
  void groups.init();
  void groups.load(true);
});

watch(
  () => workspace.workspace?.root,
  () => {
    void groups.load(true);
  },
);

function openGroup(group: PromptGroup) {
  void library.openGroup(group.promptHash);
}

async function compareGroup(group: PromptGroup) {
  const page = await searchImages({
    promptHash: group.promptHash,
    sort: "path",
    limit: 4,
  });
  compare.set(page.items);
  compare.open();
}
</script>

<template>
  <section class="group-view">
    <div class="library-head">
      <div class="library-title-group">
        <h1 class="library-title">提示词分组</h1>
        <span class="library-count">
          {{ groups.total.toLocaleString() }} 组
        </span>
      </div>
      <label class="sort-control">
        <span>排序</span>
        <select v-model="sortValue" class="sort-select">
          <option value="count">数量</option>
          <option value="recent">最近</option>
        </select>
      </label>
      <span v-if="workspace.progress.running" class="library-index-note">
        {{ workspace.progress.phase }} ·
        {{ workspace.progress.done.toLocaleString() }} /
        {{ workspace.progress.total.toLocaleString() }}
      </span>
      <span
        v-else-if="workspace.progress.failed"
        class="library-index-note warning"
      >
        {{ workspace.progress.failed.toLocaleString() }} 个文件解析失败
      </span>
    </div>

    <div class="group-scroller">
      <div v-if="groups.error" class="library-error">
        <span>{{ groups.error }}</span>
      </div>

      <div v-if="!groups.loaded" class="grid-empty">
        <p class="muted">正在读取提示词分组…</p>
      </div>
      <div
        v-else-if="groups.items.length === 0"
        class="grid-empty"
      >
        <div class="empty-mark" />
        <h2>没有可分组图像</h2>
        <p>提示词解析完成后会按正面提示词聚合。</p>
      </div>

      <div v-else class="group-grid">
        <article
          v-for="group in groups.items"
          :key="group.promptHash"
          class="group-card"
        >
          <button
            class="group-card-open"
            type="button"
            :title="group.sample"
            @click="openGroup(group)"
          >
            <img
              class="group-card-cover"
              :src="
                thumbUrlFor(
                  base,
                  group.coverId,
                  group.latestMtimeNs,
                  480,
                  workspace.cacheEpoch,
                )
              "
              alt=""
              loading="lazy"
              decoding="async"
            />
            <span class="group-card-count">{{ group.count }} 张</span>
            <span class="group-card-body">
              <span class="group-card-prompt" :title="group.sample">
                {{ group.sample }}
              </span>
            </span>
          </button>
          <button
            v-if="group.count >= 2"
            class="button ghost group-card-compare"
            type="button"
            @click.stop="compareGroup(group)"
          >
            对比前 4 张
          </button>
        </article>
      </div>

      <div v-if="groups.hasMore" class="group-more">
        <button
          class="button"
          type="button"
          :disabled="groups.loading"
          @click="groups.load(false)"
        >
          {{ groups.loading ? "加载中…" : "加载更多" }}
        </button>
      </div>
    </div>
  </section>
</template>
