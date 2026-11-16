export const NODE_HEADER_HEIGHT = 30;
export const PORT_TOP = 6;
export const PORT_ROW_HEIGHT = 18;
export const GRAPH_PADDING = 24;

const NODE_MIN_WIDTH = 220;
const API_NODE_WIDTH = 240;
const LAYER_GAP = 90;
const NODE_GAP = 28;

export interface WorkflowPort {
  name: string;
  type: string;
  slot: number;
}

export interface WorkflowWidget {
  name: string;
  value: string;
}

export interface WorkflowNode {
  id: string;
  type: string;
  title: string;
  category: string;
  x: number;
  y: number;
  width: number;
  height: number;
  inputs: WorkflowPort[];
  outputs: WorkflowPort[];
  widgets: WorkflowWidget[];
  widgetsTop: number;
}

export interface WorkflowEdge {
  id: string;
  from: string;
  fromSlot: number;
  to: string;
  toSlot: number;
  type: string;
}

export interface WorkflowGraph {
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  width: number;
  height: number;
  source: "ui" | "api";
}

type JsonRecord = Record<string, unknown>;

export function buildWorkflowGraph(value: unknown): WorkflowGraph | null {
  if (!isRecord(value)) return null;
  if (Array.isArray(value.nodes)) {
    return buildUiGraph(value);
  }

  const entries = Object.entries(value).filter(
    (entry): entry is [string, JsonRecord] =>
      isRecord(entry[1]) &&
      typeof entry[1].class_type === "string" &&
      isRecord(entry[1].inputs),
  );
  if (entries.length > 0) {
    return buildApiGraph(entries);
  }
  return null;
}

function buildUiGraph(root: JsonRecord): WorkflowGraph | null {
  const rawNodes = (root.nodes as unknown[]).filter(isRecord);
  if (rawNodes.length === 0) return null;

  const nodes: WorkflowNode[] = [];
  const nodeIds = new Set<string>();
  rawNodes.forEach((raw, index) => {
    const id = String(raw.id ?? index);
    nodeIds.add(id);
    const type = typeof raw.type === "string" ? raw.type : "Node";
    const inputs = normalizeUiPorts(raw.inputs, "in");
    const outputs = normalizeUiPorts(raw.outputs, "out");
    const position = readPoint(raw.pos) ?? { x: 0, y: 0 };
    const size = readPoint(raw.size);
    const width = Math.max(NODE_MIN_WIDTH, size?.x ?? NODE_MIN_WIDTH);
    const givenHeight = size?.y ?? 0;
    const height = nodeHeight(inputs.length, outputs.length, 0, givenHeight);
    nodes.push({
      id,
      type,
      title:
        typeof raw.title === "string" && raw.title.trim()
          ? raw.title
          : type,
      category: nodeCategory(type),
      x: position.x,
      y: position.y,
      width,
      height,
      inputs,
      outputs,
      widgets: [],
      widgetsTop: nodeWidgetsTop(inputs.length, outputs.length, 0),
    });
  });

  const edges: WorkflowEdge[] = [];
  const rawLinks = Array.isArray(root.links) ? root.links : [];
  for (const rawLink of rawLinks) {
    const edge = normalizeLink(rawLink);
    if (!edge || !nodeIds.has(edge.from) || !nodeIds.has(edge.to)) continue;
    edges.push(edge);
  }

  return finalizeGraph(nodes, edges, "ui");
}

function buildApiGraph(entries: [string, JsonRecord][]): WorkflowGraph | null {
  const nodeIds = new Set(entries.map(([id]) => id));
  const nodes: WorkflowNode[] = [];
  const edges: WorkflowEdge[] = [];
  const outputSlots = new Map<string, Set<number>>();

  for (const [id, raw] of entries) {
    const type = String(raw.class_type ?? "Node");
    const rawInputs = isRecord(raw.inputs) ? raw.inputs : {};
    const inputs: WorkflowPort[] = [];
    const widgets: WorkflowWidget[] = [];

    for (const [name, value] of Object.entries(rawInputs)) {
      if (isApiLink(value, nodeIds)) {
        const targetSlot = inputs.length;
        const fromId = String(value[0]);
        const fromSlot = toNumber(value[1]);
        inputs.push({ name, type: "", slot: targetSlot });
        edges.push({
          id: `${fromId}:${fromSlot}->${id}:${name}`,
          from: fromId,
          fromSlot,
          to: id,
          toSlot: targetSlot,
          type: name,
        });
        const slots = outputSlots.get(fromId) ?? new Set<number>();
        slots.add(fromSlot);
        outputSlots.set(fromId, slots);
      } else {
        widgets.push({ name, value: formatValue(value) });
      }
    }

    const meta = isRecord(raw._meta) ? raw._meta : null;
    const title =
      meta && typeof meta.title === "string" && meta.title.trim()
        ? meta.title
        : type;
    nodes.push({
      id,
      type,
      title,
      category: nodeCategory(type),
      x: 0,
      y: 0,
      width: API_NODE_WIDTH,
      height: 0,
      inputs,
      outputs: [],
      widgets,
      widgetsTop: 0,
    });
  }

  const nodeMap = new Set(nodes.map((node) => node.id));
  const edgesWithEndpoints = edges.filter(
    (edge) => nodeMap.has(edge.from) && nodeMap.has(edge.to),
  );

  for (const node of nodes) {
    const slots = outputSlots.get(node.id);
    node.outputs = slots
      ? [...slots]
          .sort((left, right) => left - right)
          .map((slot) => ({ name: `#${slot}`, type: "", slot }))
      : [];
    node.height = nodeHeight(
      node.inputs.length,
      node.outputs.length,
      node.widgets.length,
      0,
    );
    node.widgetsTop = nodeWidgetsTop(
      node.inputs.length,
      node.outputs.length,
      node.widgets.length,
    );
  }

  layoutApiNodes(nodes, edgesWithEndpoints);
  return finalizeGraph(nodes, edgesWithEndpoints, "api");
}

function layoutApiNodes(nodes: WorkflowNode[], edges: WorkflowEdge[]) {
  const dependencies = new Map<string, string[]>();
  for (const node of nodes) dependencies.set(node.id, []);
  for (const edge of edges) {
    const list = dependencies.get(edge.to);
    if (list && !list.includes(edge.from)) list.push(edge.from);
  }

  const layers = new Map<string, number>();
  const visiting = new Set<string>();
  function layerOf(id: string): number {
    const cached = layers.get(id);
    if (cached !== undefined) return cached;
    if (visiting.has(id)) return 0;
    visiting.add(id);
    let layer = 0;
    for (const dependency of dependencies.get(id) ?? []) {
      layer = Math.max(layer, layerOf(dependency) + 1);
    }
    visiting.delete(id);
    layers.set(id, layer);
    return layer;
  }
  for (const node of nodes) layerOf(node.id);

  const maxLayer = Math.max(0, ...layers.values());
  const groups: WorkflowNode[][] = Array.from(
    { length: maxLayer + 1 },
    () => [],
  );
  for (const node of nodes) {
    groups[layers.get(node.id) ?? 0].push(node);
  }
  for (const group of groups) {
    group.sort((left, right) =>
      left.id.localeCompare(right.id, undefined, { numeric: true }),
    );
  }

  let x = GRAPH_PADDING;
  for (const group of groups) {
    let y = GRAPH_PADDING;
    for (const node of group) {
      node.x = x;
      node.y = y;
      y += node.height + NODE_GAP;
    }
    x += API_NODE_WIDTH + LAYER_GAP;
  }
}

function finalizeGraph(
  nodes: WorkflowNode[],
  edges: WorkflowEdge[],
  source: "ui" | "api",
): WorkflowGraph {
  const minX = Math.min(...nodes.map((node) => node.x));
  const minY = Math.min(...nodes.map((node) => node.y));
  const shiftX = GRAPH_PADDING - minX;
  const shiftY = GRAPH_PADDING - minY;
  for (const node of nodes) {
    node.x += shiftX;
    node.y += shiftY;
  }
  return {
    nodes,
    edges,
    width: Math.max(...nodes.map((node) => node.x + node.width)) + GRAPH_PADDING,
    height:
      Math.max(...nodes.map((node) => node.y + node.height)) + GRAPH_PADDING,
    source,
  };
}

function normalizeUiPorts(value: unknown, prefix: string): WorkflowPort[] {
  if (!Array.isArray(value)) return [];
  return value.filter(isRecord).map((port, index) => ({
    name:
      typeof port.name === "string" && port.name.trim()
        ? port.name
        : `${prefix} ${index}`,
    type: typeof port.type === "string" ? port.type : "",
    slot: typeof port.slot_index === "number" ? port.slot_index : index,
  }));
}

function normalizeLink(value: unknown): WorkflowEdge | null {
  if (Array.isArray(value) && value.length >= 5) {
    const [id, from, fromSlot, to, toSlot, type] = value;
    if (from === undefined || to === undefined) return null;
    return {
      id: String(id ?? `${String(from)}:${String(to)}`),
      from: String(from),
      fromSlot: toNumber(fromSlot),
      to: String(to),
      toSlot: toNumber(toSlot),
      type: typeof type === "string" ? type : "",
    };
  }
  if (isRecord(value)) {
    const from = value.origin_id ?? value.from;
    const to = value.target_id ?? value.to;
    if (from === undefined || to === undefined) return null;
    return {
      id: String(value.id ?? `${String(from)}:${String(to)}`),
      from: String(from),
      fromSlot: toNumber(value.origin_slot ?? value.from_slot),
      to: String(to),
      toSlot: toNumber(value.target_slot ?? value.to_slot),
      type: typeof value.type === "string" ? value.type : "",
    };
  }
  return null;
}

function nodeHeight(
  inputs: number,
  outputs: number,
  widgets: number,
  given: number,
): number {
  const natural =
    nodeWidgetsTop(inputs, outputs, widgets) +
    widgets * PORT_ROW_HEIGHT +
    6;
  return Math.max(given, natural);
}

function nodeWidgetsTop(inputs: number, outputs: number, widgets: number) {
  const portRows = Math.max(inputs, outputs);
  return (
    NODE_HEADER_HEIGHT +
    PORT_TOP +
    portRows * PORT_ROW_HEIGHT +
    (widgets > 0 ? 2 : 0)
  );
}

function nodeCategory(type: string): string {
  const lower = type.toLowerCase();
  if (lower.includes("text") || lower.includes("prompt")) return "text";
  if (
    lower.includes("loader") ||
    lower.includes("checkpoint") ||
    lower.includes("unet") ||
    lower.includes("vae") ||
    lower.includes("clip")
  ) {
    return "loader";
  }
  if (
    lower.includes("sampler") ||
    lower.includes("scheduler") ||
    lower.includes("guider") ||
    lower.includes("noise") ||
    lower.includes("guidance")
  ) {
    return "sampler";
  }
  if (lower.includes("latent") || lower.includes("image")) return "latent";
  return "default";
}

function formatValue(value: unknown): string {
  if (typeof value === "string") {
    const text = value.replace(/\s+/g, " ").trim();
    return text.length > 120 ? `${text.slice(0, 117)}…` : text || "—";
  }
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  if (value === null) return "null";
  try {
    const text = JSON.stringify(value);
    return text.length > 120 ? `${text.slice(0, 117)}…` : text;
  } catch {
    return "—";
  }
}

function isApiLink(
  value: unknown,
  nodeIds: Set<string>,
): value is [string | number, number] {
  return (
    Array.isArray(value) &&
    value.length === 2 &&
    (typeof value[0] === "string" || typeof value[0] === "number") &&
    nodeIds.has(String(value[0])) &&
    typeof value[1] === "number"
  );
}

function readPoint(value: unknown): { x: number; y: number } | null {
  if (
    Array.isArray(value) &&
    value.length >= 2 &&
    isFiniteNumber(value[0]) &&
    isFiniteNumber(value[1])
  ) {
    return { x: value[0], y: value[1] };
  }
  if (isRecord(value)) {
    if (isFiniteNumber(value.x) && isFiniteNumber(value.y)) {
      return { x: value.x, y: value.y };
    }
    if (isFiniteNumber(value["0"]) && isFiniteNumber(value["1"])) {
      return { x: value["0"], y: value["1"] };
    }
  }
  return null;
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function toNumber(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}

function isRecord(value: unknown): value is JsonRecord {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
