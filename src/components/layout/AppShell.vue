<script setup lang="ts">
import { onMounted } from "vue";
import { RouterView } from "vue-router";
import TopToolbar from "./TopToolbar.vue";
import FilterSidebar from "./FilterSidebar.vue";
import StatusBar from "./StatusBar.vue";
import { useWorkspaceStore } from "@/stores/workspace";

const store = useWorkspaceStore();

onMounted(() => {
  void store.init();
});
</script>

<template>
  <div class="app-shell">
    <TopToolbar />
    <FilterSidebar />
    <main class="app-main">
      <div v-if="store.error" class="error-banner">
        <span>{{ store.error }}</span>
        <button class="link-button" @click="store.clearError()">关闭</button>
      </div>
      <RouterView />
    </main>
    <StatusBar />
  </div>
</template>
