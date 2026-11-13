import { reactive } from "vue";

export interface SyncedViewport {
  cx: number;
  cy: number;
  scale: number;
}

const MIN_SCALE = 1;
const MAX_SCALE = 8;

export function createViewport(): SyncedViewport {
  return reactive({ cx: 0.5, cy: 0.5, scale: 1 });
}

export function resetViewport(viewport: SyncedViewport) {
  viewport.cx = 0.5;
  viewport.cy = 0.5;
  viewport.scale = 1;
}

export function zoomViewport(
  viewport: SyncedViewport,
  factor: number,
  px: number,
  py: number,
  rect: DOMRect,
  baseWidth: number,
  baseHeight: number,
) {
  if (
    baseWidth <= 0 ||
    baseHeight <= 0 ||
    rect.width <= 0 ||
    rect.height <= 0
  ) {
    return;
  }
  const next = Math.min(
    MAX_SCALE,
    Math.max(MIN_SCALE, viewport.scale * factor),
  );
  if (next === viewport.scale) return;

  const dx = (px - 0.5) * rect.width;
  const dy = (py - 0.5) * rect.height;
  const nx = viewport.cx + dx / (baseWidth * viewport.scale);
  const ny = viewport.cy + dy / (baseHeight * viewport.scale);

  viewport.scale = next;
  if (next <= MIN_SCALE) {
    viewport.cx = 0.5;
    viewport.cy = 0.5;
    return;
  }
  viewport.cx = nx - dx / (baseWidth * next);
  viewport.cy = ny - dy / (baseHeight * next);
  clampCenter(viewport);
}

export function panViewport(
  viewport: SyncedViewport,
  dx: number,
  dy: number,
  baseWidth: number,
  baseHeight: number,
) {
  if (viewport.scale <= MIN_SCALE || baseWidth <= 0 || baseHeight <= 0) return;
  viewport.cx -= dx / (baseWidth * viewport.scale);
  viewport.cy -= dy / (baseHeight * viewport.scale);
  clampCenter(viewport);
}

function clampCenter(viewport: SyncedViewport) {
  viewport.cx = Math.min(1, Math.max(0, viewport.cx));
  viewport.cy = Math.min(1, Math.max(0, viewport.cy));
}
