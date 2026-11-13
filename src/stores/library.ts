import { listen } from "@tauri-apps/api/event";
import { defineStore } from "pinia";
import { computed, reactive, ref } from "vue";
import {
  clearRating as invokeClearRating,
  deleteImages,
  getFilterOptions,
  getImageDetail,
  listImageIds,
  searchImages,
  setRating as invokeSetRating,
  type FilterOptions,
  type ImageDetail,
  type ImageSummary,
  type IndexProgress,
  type SearchParams,
  type SortKey,
} from "@/api/backend";
import { useCompareStore } from "./compare";
import { useGroupsStore } from "./groups";
import { useSelectionStore } from "./selection";

const PAGE_SIZE = 200;
const QUERY_DELAY = 280;

export type RatingMode = "min" | "exact";
export type LibraryViewMode = "grid" | "groups";
export type DeleteTarget =
  | { kind: "image"; image: ImageSummary }
  | { kind: "selection" };

const emptyFilterOptions = (): FilterOptions => ({
  models: [],
  samplers: [],
  unrated: 0,
  starsExact: [0, 0, 0, 0, 0],
  starsMin: [0, 0, 0, 0, 0],
});

export const useLibraryStore = defineStore("library", () => {
  const pages = reactive(new Map<number, ImageSummary[]>());
  const loadingPages = new Set<string>();
  const total = ref(0);
  const loaded = ref(false);
  const generation = ref(0);
  const revision = ref(0);
  const error = ref<string | null>(null);
  const activeIndex = ref<number | null>(null);
  const initialized = ref(false);
  const contextMenu = ref<{
    image: ImageSummary;
    index: number;
    x: number;
    y: number;
  } | null>(null);
  const deleteTarget = ref<DeleteTarget | null>(null);
  const deleting = ref(false);
  const deleteError = ref<string | null>(null);
  const deleteNotice = ref<string | null>(null);
  const selection = useSelectionStore();

  const query = ref("");
  const model = ref<string | null>(null);
  const sampler = ref<string | null>(null);
  const hasWorkflow = ref(false);
  const rating = ref<number | null>(null);
  const ratingMode = ref<RatingMode>("min");
  const unrated = ref(false);
  const sort = ref<SortKey>("relevance");
  const promptHash = ref<string | null>(null);
  const viewMode = ref<LibraryViewMode>("grid");
  const filterOptions = ref<FilterOptions>(emptyFilterOptions());

  let queryTimer: ReturnType<typeof setTimeout> | null = null;

  const hasFilters = computed(
    () =>
      Boolean(model.value) ||
      Boolean(sampler.value) ||
      hasWorkflow.value ||
      rating.value !== null ||
      unrated.value,
  );

  const isSearching = computed(
    () => query.value.trim().length > 0 || hasFilters.value,
  );

  const activeItem = computed(() =>
    activeIndex.value === null ? null : itemAt(activeIndex.value),
  );

  const deleteCount = computed(() => {
    if (deleteTarget.value?.kind === "selection") return selection.count;
    return deleteTarget.value ? 1 : 0;
  });

  const deleteLabel = computed(() => {
    if (deleteTarget.value?.kind === "image") {
      return deleteTarget.value.image.relPath;
    }
    if (deleteTarget.value?.kind === "selection") {
      return `选中的 ${deleteCount.value.toLocaleString()} 张图片`;
    }
    return "";
  });

  function itemAt(index: number): ImageSummary | undefined {
    const page = pages.get(Math.floor(index / PAGE_SIZE));
    return page?.[index % PAGE_SIZE];
  }

  function findImage(id: number): ImageSummary | undefined {
    for (const page of pages.values()) {
      const image = page.find((item) => item.id === id);
      if (image) return image;
    }
    return undefined;
  }

  function updateImageStars(id: number, stars: number | null) {
    for (const page of pages.values()) {
      const image = page.find((item) => item.id === id);
      if (image) image.stars = stars;
    }
  }

  function currentParams(
    offset = 0,
    limit = PAGE_SIZE,
  ): SearchParams {
    return {
      query: query.value,
      model: model.value,
      sampler: sampler.value,
      hasWorkflow: hasWorkflow.value,
      rating: ratingMode.value === "exact" ? rating.value : null,
      minRating: ratingMode.value === "min" ? rating.value : null,
      unrated: unrated.value,
      promptHash: promptHash.value,
      sort: sort.value,
      offset,
      limit,
    };
  }

  async function loadPage(pageIndex: number) {
    const current = generation.value;
    const key = `${current}:${pageIndex}`;
    if (pages.has(pageIndex) || loadingPages.has(key)) return;
    loadingPages.add(key);
    try {
      const page = await searchImages(
        currentParams(pageIndex * PAGE_SIZE, PAGE_SIZE),
      );
      if (current !== generation.value) return;
      pages.set(pageIndex, page.items);
      total.value = page.total;
      selection.setTotal(page.total);
      loaded.value = true;
    } catch (cause) {
      if (current === generation.value) {
        error.value = String(cause);
        loaded.value = true;
      }
    } finally {
      loadingPages.delete(key);
    }
  }

  function ensureRange(start: number, end: number) {
    if (total.value === 0 && !pages.has(0)) {
      void loadPage(0);
      return;
    }
    const first = Math.max(0, Math.floor(start / PAGE_SIZE));
    const last = Math.floor(Math.max(start, end) / PAGE_SIZE);
    for (let page = first; page <= last; page += 1) {
      void loadPage(page);
    }
  }

  function ensurePageFor(index: number) {
    return loadPage(Math.floor(index / PAGE_SIZE));
  }

  async function reset() {
    generation.value += 1;
    pages.clear();
    total.value = 0;
    loaded.value = false;
    activeIndex.value = null;
    error.value = null;
    contextMenu.value = null;
    selection.clear();
    selection.setTotal(0);
    useCompareStore().clear();
    await loadPage(0);
  }

  async function resetWorkspace() {
    cancelQueryTimer();
    query.value = "";
    model.value = null;
    sampler.value = null;
    hasWorkflow.value = false;
    rating.value = null;
    unrated.value = false;
    ratingMode.value = "min";
    promptHash.value = null;
    viewMode.value = "grid";
    sort.value = "relevance";
    contextMenu.value = null;
    deleteTarget.value = null;
    deleteError.value = null;
    deleteNotice.value = null;
    await reset();
    await loadFilterOptions();
  }

  async function refresh() {
    generation.value += 1;
    pages.clear();
    revision.value += 1;
    loaded.value = false;
    error.value = null;
    contextMenu.value = null;
    await Promise.all([loadPage(0), loadFilterOptions()]);
  }

  async function loadFilterOptions() {
    try {
      filterOptions.value = await getFilterOptions();
    } catch {
      // Keep the previous options when the workspace is not ready yet.
    }
  }

  function cancelQueryTimer() {
    if (queryTimer !== null) {
      clearTimeout(queryTimer);
      queryTimer = null;
    }
  }

  function setQuery(value: string) {
    if (query.value === value) return;
    query.value = value;
    viewMode.value = "grid";
    cancelQueryTimer();
    queryTimer = setTimeout(() => {
      queryTimer = null;
      void reset();
    }, QUERY_DELAY);
  }

  function applyQuery() {
    cancelQueryTimer();
    viewMode.value = "grid";
    void reset();
  }

  function setModel(value: string | null) {
    const next = value || null;
    if (model.value === next) return;
    model.value = next;
    viewMode.value = "grid";
    void reset();
  }

  function setSampler(value: string | null) {
    const next = value || null;
    if (sampler.value === next) return;
    sampler.value = next;
    viewMode.value = "grid";
    void reset();
  }

  function setHasWorkflow(value: boolean) {
    if (hasWorkflow.value === value) return;
    hasWorkflow.value = value;
    viewMode.value = "grid";
    void reset();
  }

  function setSort(value: SortKey) {
    if (sort.value === value) return;
    sort.value = value;
    void reset();
  }

  function setRatingMode(value: RatingMode) {
    if (ratingMode.value === value) return;
    ratingMode.value = value;
    if (rating.value !== null) {
      void reset();
    }
  }

  function showUnrated() {
    const wasGroups = viewMode.value === "groups";
    unrated.value = true;
    rating.value = null;
    viewMode.value = "grid";
    if (wasGroups) {
      promptHash.value = null;
    }
    void reset();
  }

  function showRating(stars: number) {
    const wasGroups = viewMode.value === "groups";
    rating.value = stars;
    unrated.value = false;
    viewMode.value = "grid";
    if (wasGroups) {
      promptHash.value = null;
    }
    void reset();
  }

  function clearFilters() {
    model.value = null;
    sampler.value = null;
    hasWorkflow.value = false;
    rating.value = null;
    unrated.value = false;
    void reset();
  }

  function clearSearch() {
    cancelQueryTimer();
    query.value = "";
    model.value = null;
    sampler.value = null;
    hasWorkflow.value = false;
    rating.value = null;
    unrated.value = false;
    promptHash.value = null;
    viewMode.value = "grid";
    void reset();
  }

  function showAll() {
    clearSearch();
  }

  function showGroups() {
    cancelQueryTimer();
    query.value = "";
    model.value = null;
    sampler.value = null;
    hasWorkflow.value = false;
    rating.value = null;
    unrated.value = false;
    promptHash.value = null;
    viewMode.value = "groups";
    activeIndex.value = null;
    selection.clear();
    useCompareStore().clear();
  }

  async function openGroup(hash: string) {
    cancelQueryTimer();
    query.value = "";
    model.value = null;
    sampler.value = null;
    hasWorkflow.value = false;
    rating.value = null;
    unrated.value = false;
    promptHash.value = hash;
    viewMode.value = "grid";
    await reset();
  }

  async function setRating(id: number, stars: number | null) {
    const previous = findImage(id)?.stars ?? null;
    if (previous === stars) return;
    updateImageStars(id, stars);
    try {
      if (stars === null) {
        await invokeClearRating(id);
      } else {
        await invokeSetRating(id, stars);
      }
      void loadFilterOptions();
    } catch (cause) {
      updateImageStars(id, previous);
      error.value = String(cause);
    }
  }

  function openContextMenu(
    image: ImageSummary,
    index: number,
    x: number,
    y: number,
  ) {
    contextMenu.value = { image, index, x, y };
  }

  function closeContextMenu() {
    contextMenu.value = null;
  }

  function detailToSummary(detail: ImageDetail): ImageSummary {
    return {
      id: detail.id,
      relPath: detail.relPath,
      fileSize: detail.fileSize,
      mtimeNs: detail.mtimeNs,
      width: detail.width,
      height: detail.height,
      parseStatus: detail.parseStatus,
      modelName: detail.modelName,
      stars: detail.stars,
    };
  }

  async function resolveSelectedIds(): Promise<number[]> {
    if (!selection.selectAllActive) {
      return [...selection.selectedIds];
    }
    const all = await listImageIds(currentParams());
    return all.filter((id) => !selection.excludedIds.has(id));
  }

  async function resolveSelectedImages(): Promise<ImageSummary[]> {
    if (!selection.canCompare) return [];
    if (selection.selectAllActive) {
      await loadPage(0);
      return (pages.get(0) ?? [])
        .filter((item) => selection.isSelected(item.id))
        .slice(0, 4);
    }
    const ids = [...selection.selectedIds].slice(0, 4);
    const loaded = new Map<number, ImageSummary>();
    for (const page of pages.values()) {
      for (const item of page) loaded.set(item.id, item);
    }
    const result: ImageSummary[] = [];
    for (const id of ids) {
      const item = loaded.get(id);
      if (item) {
        result.push(item);
        continue;
      }
      try {
        result.push(detailToSummary(await getImageDetail(id)));
      } catch {
        // Skip images that disappeared since selection.
      }
    }
    return result;
  }

  async function openCompareSelection(): Promise<boolean> {
    if (!selection.canCompare) return false;
    const items = await resolveSelectedImages();
    if (items.length < 2) {
      error.value = "无法加载选中的图片，请重新选择。";
      return false;
    }
    const compare = useCompareStore();
    compare.set(items);
    compare.open();
    return compare.isOpen;
  }

  function requestDelete(image: ImageSummary) {
    contextMenu.value = null;
    deleteTarget.value = { kind: "image", image };
    deleteError.value = null;
    deleteNotice.value = null;
  }

  function requestDeleteSelection() {
    if (selection.count === 0) return;
    contextMenu.value = null;
    deleteTarget.value = { kind: "selection" };
    deleteError.value = null;
    deleteNotice.value = null;
  }

  function cancelDelete() {
    if (deleting.value) return;
    deleteTarget.value = null;
    deleteError.value = null;
  }

  function clearDeleteNotice() {
    deleteNotice.value = null;
  }

  async function confirmDelete(permanent: boolean): Promise<boolean> {
    const target = deleteTarget.value;
    if (!target || deleting.value) return false;
    deleting.value = true;
    deleteError.value = null;
    try {
      const ids =
        target.kind === "image"
          ? [target.image.id]
          : await resolveSelectedIds();
      if (ids.length === 0) {
        deleteError.value = "没有选中的图片。";
        return false;
      }
      const result = await deleteImages(ids, permanent);
      const failedIds = new Set(result.failed.map((failure) => failure.id));
      const deletedIds = ids.filter((id) => !failedIds.has(id));
      if (result.failed.length > 0) {
        const preview = result.failed
          .slice(0, 3)
          .map((failure) => `${failure.relPath}：${failure.error}`)
          .join("；");
        deleteError.value =
          result.failed.length > 3
            ? `${result.failed.length} 张图片删除失败：${preview}…`
            : preview;
      }
      if (deletedIds.length === 0) {
        return false;
      }

      const compare = useCompareStore();
      if (target.kind === "image") {
        selection.remove(target.image.id);
        if (
          compare.selected.some((item) => item.id === target.image.id)
        ) {
          compare.set(
            compare.selected.filter(
              (item) => item.id !== target.image.id,
            ),
          );
        }
        if (activeIndex.value !== null && activeItem.value?.id === target.image.id) {
          activeIndex.value = null;
        }
      } else if (result.failed.length === 0) {
        selection.clear();
      } else {
        selection.set([...failedIds]);
      }

      if (result.failed.length === 0) {
        deleteTarget.value = null;
      }
      deleteNotice.value = permanent
        ? `已永久删除 ${deletedIds.length.toLocaleString()} 张图片。`
        : `已将 ${deletedIds.length.toLocaleString()} 张图片移到回收站。`;
      await refresh();
      await loadFilterOptions();
      void useGroupsStore().refresh();
      return result.failed.length === 0;
    } catch (cause) {
      deleteError.value = String(cause);
      return false;
    } finally {
      deleting.value = false;
    }
  }

  function clearError() {
    error.value = null;
  }

  async function openAt(index: number) {
    await ensurePageFor(index);
    if (itemAt(index)) {
      activeIndex.value = index;
    }
  }

  async function step(delta: number) {
    if (activeIndex.value === null) return;
    const target = activeIndex.value + delta;
    if (target < 0 || target >= total.value) return;
    await ensurePageFor(target);
    if (itemAt(target)) {
      activeIndex.value = target;
    }
  }

  function close() {
    activeIndex.value = null;
  }

  async function init() {
    if (initialized.value) return;
    initialized.value = true;
    await listen<IndexProgress>("index:changed", () => {
      revision.value += 1;
      void refresh();
    });
  }

  return {
    pages,
    total,
    loaded,
    generation,
    revision,
    error,
    activeIndex,
    activeItem,
    contextMenu,
    deleteTarget,
    deleteCount,
    deleteLabel,
    deleting,
    deleteError,
    deleteNotice,
    selection,
    query,
    model,
    sampler,
    hasWorkflow,
    rating,
    ratingMode,
    unrated,
    sort,
    promptHash,
    viewMode,
    filterOptions,
    hasFilters,
    isSearching,
    itemAt,
    ensureRange,
    ensurePageFor,
    reset,
    resetWorkspace,
    refresh,
    loadFilterOptions,
    setQuery,
    applyQuery,
    setModel,
    setSampler,
    setHasWorkflow,
    setSort,
    setRatingMode,
    showUnrated,
    showRating,
    clearFilters,
    clearSearch,
    showAll,
    showGroups,
    openGroup,
    setRating,
    clearError,
    openContextMenu,
    closeContextMenu,
    requestDelete,
    requestDeleteSelection,
    cancelDelete,
    clearDeleteNotice,
    confirmDelete,
    openCompareSelection,
    openAt,
    step,
    close,
    init,
  };
});
