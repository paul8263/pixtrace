import { listen } from "@tauri-apps/api/event";
import { defineStore } from "pinia";
import { computed, ref } from "vue";
import {
  clearCache as invokeClearCache,
  getAppInfo,
  getCacheStats,
  getIndexProgress,
  getWorkspaceState,
  openWorkspace as invokeOpenWorkspace,
  pickDirectory,
  rebuildIndex as invokeRebuildIndex,
  rescanWorkspace,
  setCacheLimit as invokeSetCacheLimit,
  setWatchEnabled as invokeSetWatchEnabled,
  type AppInfo,
  type CacheStats,
  type IndexProgress,
  type WorkspaceInfo,
} from "@/api/backend";
import { useLibraryStore } from "./library";
import { useGroupsStore } from "./groups";

const emptyProgress = (): IndexProgress => ({
  running: false,
  total: 0,
  done: 0,
  failed: 0,
  phase: "空闲",
});

export const useWorkspaceStore = defineStore("workspace", () => {
  const appInfo = ref<AppInfo | null>(null);
  const workspace = ref<WorkspaceInfo | null>(null);
  const progress = ref<IndexProgress>(emptyProgress());
  const busy = ref(false);
  const error = ref<string | null>(null);
  const initialized = ref(false);
  const settingsOpen = ref(false);
  const settingsBusy = ref(false);
  const settingsError = ref<string | null>(null);
  const settingsMessage = ref<string | null>(null);
  const cacheStats = ref<CacheStats | null>(null);
  const cacheEpoch = ref(0);

  const hasWorkspace = computed(() => workspace.value !== null);
  const watchEnabled = computed(() => appInfo.value?.watchWorkspace ?? true);

  const progressPercent = computed(() => {
    if (progress.value.total === 0) return 0;
    return Math.min(
      100,
      Math.round((progress.value.done / progress.value.total) * 100),
    );
  });

  async function init() {
    if (initialized.value) return;
    initialized.value = true;
    try {
      appInfo.value = await getAppInfo();
      workspace.value = await getWorkspaceState();
      progress.value = await getIndexProgress();
      if (workspace.value) {
        await useLibraryStore().resetWorkspace();
        useGroupsStore().reset();
      }
      await listen<IndexProgress>("scan:progress", (event) => {
        progress.value = event.payload;
      });
      await listen<WorkspaceInfo>("workspace:changed", (event) => {
        workspace.value = event.payload;
      });
    } catch (cause) {
      error.value = String(cause);
    }
  }

  async function chooseWorkspace(): Promise<boolean> {
    const selected = await pickDirectory();
    if (!selected) return false;
    return openWorkspaceAt(selected);
  }

  async function openWorkspaceAt(path: string): Promise<boolean> {
    busy.value = true;
    error.value = null;
    try {
      workspace.value = await invokeOpenWorkspace(path);
      progress.value = await getIndexProgress();
      await useLibraryStore().resetWorkspace();
      useGroupsStore().reset();
      return true;
    } catch (cause) {
      error.value = String(cause);
      return false;
    } finally {
      busy.value = false;
    }
  }

  async function rescan() {
    if (!workspace.value) return;
    busy.value = true;
    error.value = null;
    try {
      workspace.value = await rescanWorkspace();
      progress.value = await getIndexProgress();
      const library = useLibraryStore();
      await library.reset();
      void library.loadFilterOptions();
      const groups = useGroupsStore();
      groups.reset();
      void groups.load(true);
    } catch (cause) {
      error.value = String(cause);
    } finally {
      busy.value = false;
    }
  }

  async function openSettings() {
    settingsOpen.value = true;
    settingsError.value = null;
    settingsMessage.value = null;
    await refreshCacheStats();
  }

  function closeSettings() {
    settingsOpen.value = false;
  }

  async function refreshCacheStats() {
    try {
      cacheStats.value = await getCacheStats();
    } catch (cause) {
      settingsError.value = String(cause);
    }
  }

  async function clearThumbCache() {
    settingsBusy.value = true;
    settingsError.value = null;
    settingsMessage.value = null;
    try {
      cacheStats.value = await invokeClearCache();
      cacheEpoch.value += 1;
      settingsMessage.value = "缩略图缓存已清理，重新浏览时会自动生成。";
    } catch (cause) {
      settingsError.value = String(cause);
    } finally {
      settingsBusy.value = false;
    }
  }

  async function updateCacheLimit(limitMb: number) {
    settingsBusy.value = true;
    settingsError.value = null;
    settingsMessage.value = null;
    try {
      cacheStats.value = await invokeSetCacheLimit(limitMb);
      if (appInfo.value) {
        appInfo.value.thumbCacheLimitMb = limitMb;
      }
      settingsMessage.value = "缩略图缓存上限已更新。";
    } catch (cause) {
      settingsError.value = String(cause);
    } finally {
      settingsBusy.value = false;
    }
  }

  async function updateWatchEnabled(enabled: boolean) {
    settingsBusy.value = true;
    settingsError.value = null;
    settingsMessage.value = null;
    try {
      const result = await invokeSetWatchEnabled(enabled);
      if (appInfo.value) {
        appInfo.value.watchWorkspace = result;
      }
      settingsMessage.value = result
        ? "文件监听已开启，目录变化会自动刷新。"
        : "文件监听已关闭，可使用“重新扫描”手动刷新。";
    } catch (cause) {
      settingsError.value = String(cause);
    } finally {
      settingsBusy.value = false;
    }
  }

  async function rebuildIndex() {
    settingsBusy.value = true;
    settingsError.value = null;
    settingsMessage.value = null;
    try {
      await invokeRebuildIndex();
      settingsMessage.value = "已开始重建提示词索引，进度见底部状态栏。";
    } catch (cause) {
      settingsError.value = String(cause);
    } finally {
      settingsBusy.value = false;
    }
  }

  function clearError() {
    error.value = null;
  }

  return {
    appInfo,
    workspace,
    progress,
    progressPercent,
    busy,
    error,
    hasWorkspace,
    watchEnabled,
    settingsOpen,
    settingsBusy,
    settingsError,
    settingsMessage,
    cacheStats,
    cacheEpoch,
    init,
    chooseWorkspace,
    openWorkspaceAt,
    rescan,
    openSettings,
    closeSettings,
    refreshCacheStats,
    clearThumbCache,
    updateCacheLimit,
    updateWatchEnabled,
    rebuildIndex,
    clearError,
  };
});
