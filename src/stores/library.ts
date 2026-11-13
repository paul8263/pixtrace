import { listen } from "@tauri-apps/api/event";
import { defineStore } from "pinia";
import { computed, reactive, ref } from "vue";
import {
  clearRating as invokeClearRating,
  getFilterOptions,
  searchImages,
  setRating as invokeSetRating,
  type FilterOptions,
  type ImageSummary,
  type IndexProgress,
  type SortKey,
} from "@/api/backend";
import { useCompareStore } from "./compare";

const PAGE_SIZE = 200;
const QUERY_DELAY = 280;

export type RatingMode = "min" | "exact";
export type LibraryViewMode = "grid" | "groups";

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

  async function loadPage(pageIndex: number) {
    const current = generation.value;
    const key = `${current}:${pageIndex}`;
    if (pages.has(pageIndex) || loadingPages.has(key)) return;
    loadingPages.add(key);
    try {
      const page = await searchImages({
        query: query.value,
        model: model.value,
        sampler: sampler.value,
        hasWorkflow: hasWorkflow.value,
        rating: ratingMode.value === "exact" ? rating.value : null,
        minRating: ratingMode.value === "min" ? rating.value : null,
        unrated: unrated.value,
        promptHash: promptHash.value,
        sort: sort.value,
        offset: pageIndex * PAGE_SIZE,
        limit: PAGE_SIZE,
      });
      if (current !== generation.value) return;
      pages.set(pageIndex, page.items);
      total.value = page.total;
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
    await reset();
    await loadFilterOptions();
  }

  async function refresh() {
    generation.value += 1;
    pages.clear();
    revision.value += 1;
    loaded.value = false;
    error.value = null;
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
    openAt,
    step,
    close,
    init,
  };
});
