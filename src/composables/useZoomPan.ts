import { computed, ref, type Ref } from "vue";

const MIN_SCALE = 0.05;
const MAX_SCALE = 24;

export function useZoomPan(container: Ref<HTMLElement | null>) {
  const scale = ref(1);
  const tx = ref(0);
  const ty = ref(0);
  const naturalWidth = ref(0);
  const naturalHeight = ref(0);
  const containerWidth = ref(0);
  const containerHeight = ref(0);
  const fitted = ref(true);
  const dragging = ref(false);

  let startX = 0;
  let startY = 0;
  let startTx = 0;
  let startTy = 0;

  const percent = computed(() => Math.round(scale.value * 100));
  const pixelated = computed(() => scale.value >= 2);
  const transformStyle = computed(() => ({
    transform: `translate(${tx.value}px, ${ty.value}px) scale(${scale.value})`,
  }));

  function clamp(value: number, min: number, max: number) {
    return Math.min(max, Math.max(min, value));
  }

  function setNaturalSize(width: number, height: number) {
    naturalWidth.value = width;
    naturalHeight.value = height;
    if (fitted.value) {
      fit();
    }
  }

  function setContainerSize(width: number, height: number) {
    containerWidth.value = width;
    containerHeight.value = height;
    if (fitted.value) {
      fit();
    }
  }

  function fit() {
    if (!naturalWidth.value || !naturalHeight.value) return;
    if (!containerWidth.value || !containerHeight.value) return;
    const next = Math.min(
      containerWidth.value / naturalWidth.value,
      containerHeight.value / naturalHeight.value,
    );
    scale.value = clamp(next, MIN_SCALE, MAX_SCALE);
    tx.value = (containerWidth.value - naturalWidth.value * scale.value) / 2;
    ty.value = (containerHeight.value - naturalHeight.value * scale.value) / 2;
    fitted.value = true;
  }

  function oneToOne() {
    if (!naturalWidth.value || !naturalHeight.value) return;
    scale.value = 1;
    tx.value = (containerWidth.value - naturalWidth.value) / 2;
    ty.value = (containerHeight.value - naturalHeight.value) / 2;
    fitted.value = false;
  }

  function zoomAt(factor: number, x: number, y: number) {
    const next = clamp(scale.value * factor, MIN_SCALE, MAX_SCALE);
    const ratio = next / scale.value;
    tx.value = x - (x - tx.value) * ratio;
    ty.value = y - (y - ty.value) * ratio;
    scale.value = next;
    fitted.value = false;
  }

  function zoomBy(factor: number) {
    zoomAt(factor, containerWidth.value / 2, containerHeight.value / 2);
  }

  function onWheel(event: WheelEvent) {
    event.preventDefault();
    const element = container.value;
    if (!element) return;
    const rect = element.getBoundingClientRect();
    zoomAt(
      Math.exp(-event.deltaY * 0.0015),
      event.clientX - rect.left,
      event.clientY - rect.top,
    );
  }

  function onPointerDown(event: PointerEvent) {
    if (event.button !== 0) return;
    dragging.value = true;
    startX = event.clientX;
    startY = event.clientY;
    startTx = tx.value;
    startTy = ty.value;
    container.value?.setPointerCapture(event.pointerId);
  }

  function onPointerMove(event: PointerEvent) {
    if (!dragging.value) return;
    tx.value = startTx + event.clientX - startX;
    ty.value = startTy + event.clientY - startY;
  }

  function onPointerUp(event: PointerEvent) {
    if (!dragging.value) return;
    dragging.value = false;
    if (container.value?.hasPointerCapture(event.pointerId)) {
      container.value.releasePointerCapture(event.pointerId);
    }
    clampTranslation();
  }

  function clampTranslation() {
    if (!naturalWidth.value || !naturalHeight.value) return;
    const imageWidth = naturalWidth.value * scale.value;
    const imageHeight = naturalHeight.value * scale.value;
    const width = containerWidth.value;
    const height = containerHeight.value;

    if (imageWidth <= width) {
      tx.value = (width - imageWidth) / 2;
    } else {
      const margin = width * 0.25;
      tx.value = clamp(tx.value, width - imageWidth - margin, margin);
    }

    if (imageHeight <= height) {
      ty.value = (height - imageHeight) / 2;
    } else {
      const margin = height * 0.25;
      ty.value = clamp(ty.value, height - imageHeight - margin, margin);
    }
  }

  function reset() {
    naturalWidth.value = 0;
    naturalHeight.value = 0;
    scale.value = 1;
    tx.value = 0;
    ty.value = 0;
    fitted.value = true;
  }

  function toggleOneToOne() {
    if (fitted.value) {
      oneToOne();
    } else {
      fit();
    }
  }

  return {
    scale,
    tx,
    ty,
    percent,
    pixelated,
    transformStyle,
    dragging,
    fitted,
    setNaturalSize,
    setContainerSize,
    fit,
    oneToOne,
    zoomBy,
    onWheel,
    onPointerDown,
    onPointerMove,
    onPointerUp,
    reset,
    toggleOneToOne,
  };
}
