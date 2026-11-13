import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export interface AppInfo {
  version: string;
  mode: "installed" | "portable";
  dataDir: string;
  logDir: string;
  webviewDataDir: string;
  settingsFile: string;
  protocolBase: string;
  lastWorkspace: string | null;
  warning: string | null;
}

export interface WorkspaceInfo {
  root: string;
  imageCount: number;
  dbPath: string;
  schemaVersion: number;
  scannedAt: number;
}

export interface ImageSummary {
  id: number;
  relPath: string;
  fileSize: number;
  mtimeNs: number;
  width: number | null;
  height: number | null;
  parseStatus: string;
  modelName: string | null;
}

export interface ImagePage {
  items: ImageSummary[];
  total: number;
}

export type SortKey = "relevance" | "mtime" | "path";

export interface SearchParams {
  query?: string;
  model?: string | null;
  sampler?: string | null;
  hasWorkflow?: boolean;
  sort?: SortKey;
  offset?: number;
  limit?: number;
}

export interface FilterOptions {
  models: string[];
  samplers: string[];
}

export interface ImageDetail {
  id: number;
  relPath: string;
  fileSize: number;
  mtimeNs: number;
  width: number | null;
  height: number | null;
  format: string;
  parseStatus: string;
  parseError: string | null;
  positivePrompt: string | null;
  negativePrompt: string | null;
  modelName: string | null;
  loras: string | null;
  sampler: string | null;
  steps: number | null;
  cfg: number | null;
  seed: number | null;
  hasPromptJson: boolean;
  hasWorkflowJson: boolean;
  hasRawText: boolean;
  indexedAt: number | null;
}

export interface ImageAsset {
  kind: string;
  text: string | null;
}

export interface IndexProgress {
  running: boolean;
  total: number;
  done: number;
  failed: number;
  phase: string;
}

export type AssetKind = "prompt" | "workflow" | "raw";

export const getAppInfo = () => invoke<AppInfo>("get_app_info");

export const getWorkspaceState = () =>
  invoke<WorkspaceInfo | null>("get_workspace_state");

export const openWorkspace = (path: string) =>
  invoke<WorkspaceInfo>("open_workspace", { path });

export const rescanWorkspace = () => invoke<WorkspaceInfo>("rescan_workspace");

export const searchImages = (params: SearchParams = {}) =>
  invoke<ImagePage>("search_images", { params });

export const getFilterOptions = () =>
  invoke<FilterOptions>("get_filter_options");

export const getImageDetail = (id: number) =>
  invoke<ImageDetail>("get_image_detail", { id });

export const getImageAsset = (id: number, kind: AssetKind) =>
  invoke<ImageAsset>("get_image_asset", { id, kind });

export const getIndexProgress = () =>
  invoke<IndexProgress>("get_index_progress");

export const cancelIndexing = () => invoke<void>("cancel_indexing");

export async function pickDirectory(): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "选择工作目录",
  });
  return typeof selected === "string" ? selected : null;
}

export function imageUrl(base: string, image: ImageSummary): string {
  return `${base}/image/${image.id}?v=${image.mtimeNs}`;
}

export function thumbUrl(base: string, image: ImageSummary, size = 320): string {
  return `${base}/thumb/${image.id}?v=${image.mtimeNs}&size=${size}`;
}

export function testImageUrl(base: string): string {
  return `${base}/test`;
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let index = 0;
  while (value >= 1024 && index < units.length - 1) {
    value /= 1024;
    index += 1;
  }
  return `${value.toFixed(value >= 10 ? 0 : 1)} ${units[index]}`;
}
