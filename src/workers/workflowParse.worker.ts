import { buildWorkflowGraph, type WorkflowGraph } from "@/utils/workflowGraph";

export interface WorkflowParseRequest {
  id: number;
  text: string;
}

export interface WorkflowParseResponse {
  id: number;
  graph: WorkflowGraph | null;
  tree: unknown;
  treeTruncated: boolean;
  error: string | null;
}

const PREVIEW_MAX_DEPTH = 10;
const PREVIEW_MAX_NODES = 10_000;
const PREVIEW_MAX_ARRAY_ITEMS = 1_000;
const PREVIEW_MAX_OBJECT_KEYS = 1_000;
const PREVIEW_MAX_STRING = 4_000;

interface PreviewState {
  nodes: number;
  truncated: boolean;
}

function buildPreview(
  value: unknown,
  depth: number,
  state: PreviewState,
): unknown {
  state.nodes += 1;
  if (state.nodes > PREVIEW_MAX_NODES) {
    state.truncated = true;
    return "…（内容过大，已截断）";
  }
  if (typeof value === "string") {
    if (value.length > PREVIEW_MAX_STRING) {
      state.truncated = true;
      return `${value.slice(0, PREVIEW_MAX_STRING)}…（已截断，共 ${value.length.toLocaleString()} 字符）`;
    }
    return value;
  }
  if (value === null || typeof value !== "object") return value;
  if (depth >= PREVIEW_MAX_DEPTH) {
    state.truncated = true;
    return Array.isArray(value) ? `[… ${value.length} 项]` : "{…}";
  }
  if (Array.isArray(value)) {
    const count = Math.min(value.length, PREVIEW_MAX_ARRAY_ITEMS);
    const result: unknown[] = [];
    for (let index = 0; index < count; index += 1) {
      result.push(buildPreview(value[index], depth + 1, state));
    }
    if (value.length > count) {
      state.truncated = true;
      result.push(`…（还有 ${value.length - count} 项）`);
    }
    return result;
  }
  const record = value as Record<string, unknown>;
  const keys = Object.keys(record);
  const count = Math.min(keys.length, PREVIEW_MAX_OBJECT_KEYS);
  const result: Record<string, unknown> = {};
  for (let index = 0; index < count; index += 1) {
    const key = keys[index];
    result[key] = buildPreview(record[key], depth + 1, state);
  }
  if (keys.length > count) {
    state.truncated = true;
    result["…"] = `还有 ${keys.length - count} 个字段`;
  }
  return result;
}

const scope = self as unknown as {
  addEventListener(
    type: "message",
    listener: (event: MessageEvent<WorkflowParseRequest>) => void,
  ): void;
  postMessage(message: WorkflowParseResponse): void;
};

scope.addEventListener("message", (event) => {
  const { id, text } = event.data;
  try {
    const parsed: unknown = JSON.parse(text);
    const graph = buildWorkflowGraph(parsed);
    const state: PreviewState = { nodes: 0, truncated: false };
    const tree = buildPreview(parsed, 0, state);
    scope.postMessage({
      id,
      graph,
      tree,
      treeTruncated: state.truncated,
      error: null,
    });
  } catch (error) {
    scope.postMessage({
      id,
      graph: null,
      tree: null,
      treeTruncated: false,
      error: String(error),
    });
  }
});
