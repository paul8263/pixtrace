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
}

export const getAppInfo = () => invoke<AppInfo>("get_app_info");

export const getWorkspaceState = () =>
  invoke<WorkspaceInfo | null>("get_workspace_state");

export const openWorkspace = (path: string) =>
  invoke<WorkspaceInfo>("open_workspace", { path });

export const rescanWorkspace = () => invoke<WorkspaceInfo>("rescan_workspace");

export const listImages = (limit = 24) =>
  invoke<ImageSummary[]>("list_images", { limit });

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

export function testImageUrl(base: string): string {
  return `${base}/test`;
}
