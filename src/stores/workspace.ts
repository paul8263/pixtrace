import { defineStore } from "pinia";
import { computed, ref } from "vue";
import {
  getAppInfo,
  getWorkspaceState,
  listImages,
  openWorkspace as invokeOpenWorkspace,
  pickDirectory,
  rescanWorkspace,
  type AppInfo,
  type ImageSummary,
  type WorkspaceInfo,
} from "@/api/backend";

export const useWorkspaceStore = defineStore("workspace", () => {
  const appInfo = ref<AppInfo | null>(null);
  const workspace = ref<WorkspaceInfo | null>(null);
  const images = ref<ImageSummary[]>([]);
  const busy = ref(false);
  const error = ref<string | null>(null);
  const initialized = ref(false);

  const hasWorkspace = computed(() => workspace.value !== null);

  async function init() {
    if (initialized.value) return;
    initialized.value = true;
    try {
      appInfo.value = await getAppInfo();
      workspace.value = await getWorkspaceState();
      if (workspace.value) {
        await refreshImages();
      }
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
      await refreshImages();
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
      await refreshImages();
    } catch (cause) {
      error.value = String(cause);
    } finally {
      busy.value = false;
    }
  }

  async function refreshImages() {
    images.value = await listImages(24);
  }

  function clearError() {
    error.value = null;
  }

  return {
    appInfo,
    workspace,
    images,
    busy,
    error,
    hasWorkspace,
    init,
    chooseWorkspace,
    openWorkspaceAt,
    rescan,
    refreshImages,
    clearError,
  };
});
