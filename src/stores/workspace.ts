import { listen } from "@tauri-apps/api/event";
import { defineStore } from "pinia";
import { computed, ref } from "vue";
import {
  getAppInfo,
  getIndexProgress,
  getWorkspaceState,
  openWorkspace as invokeOpenWorkspace,
  pickDirectory,
  rescanWorkspace,
  type AppInfo,
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

  const hasWorkspace = computed(() => workspace.value !== null);

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
    init,
    chooseWorkspace,
    openWorkspaceAt,
    rescan,
    clearError,
  };
});
