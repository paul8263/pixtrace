<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import {
  chipKey,
  parseSingleTerm,
  parseTerms,
  serializeQuery,
  type SearchChip,
} from "@/utils/searchQuery";

const props = withDefaults(
  defineProps<{
    modelValue: string;
    disabled?: boolean;
  }>(),
  { disabled: false },
);

const emit = defineEmits<{
  (event: "update:modelValue", value: string): void;
  (event: "search"): void;
  (event: "clear"): void;
}>();

const tokens = ref<SearchChip[]>([]);
const draft = ref("");
const editingIndex = ref<number | null>(null);
const editingText = ref("");
const composing = ref(false);
const inputRef = ref<HTMLInputElement | null>(null);
const editorRef = ref<HTMLInputElement | null>(null);
const chipsRef = ref<HTMLDivElement | null>(null);

const hasContent = computed(
  () => tokens.value.length > 0 || draft.value.length > 0,
);
const placeholder = computed(() =>
  tokens.value.length > 0
    ? "继续输入…"
    : "搜索提示词，空格生成词块，-排除",
);

function serialize(): string {
  return serializeQuery(tokens.value, draft.value);
}

function emitQuery() {
  emit("update:modelValue", serialize());
}

function addChip(chip: SearchChip) {
  const key = chipKey(chip);
  if (tokens.value.some((token) => chipKey(token) === key)) return;
  tokens.value = [...tokens.value, chip];
}

function scrollToEnd() {
  void nextTick(() => {
    const element = chipsRef.value;
    if (element) element.scrollLeft = element.scrollWidth;
  });
}

function commitDraft() {
  const text = draft.value.trim();
  if (!text) {
    draft.value = "";
    return;
  }
  for (const chip of parseTerms(text)) addChip(chip);
  draft.value = "";
  emitQuery();
  scrollToEnd();
}

function removeToken(index: number) {
  if (props.disabled) return;
  const next = [...tokens.value];
  next.splice(index, 1);
  tokens.value = next;
  if (editingIndex.value === index) {
    editingIndex.value = null;
    editingText.value = "";
  } else if (
    editingIndex.value !== null &&
    editingIndex.value > index
  ) {
    editingIndex.value -= 1;
  }
  emitQuery();
  emit("search");
  void nextTick(() => inputRef.value?.focus());
}

function clearAll() {
  if (props.disabled) return;
  tokens.value = [];
  draft.value = "";
  editingIndex.value = null;
  editingText.value = "";
  emit("update:modelValue", "");
  emit("clear");
  void nextTick(() => inputRef.value?.focus());
}

function isQuoteOpen(text: string): boolean {
  let open = false;
  for (const char of text) {
    if (char === '"') open = !open;
  }
  return open;
}

function startEditing(index: number) {
  if (props.disabled) return;
  const chip = tokens.value[index];
  if (!chip) return;
  editingIndex.value = index;
  editingText.value = `${chip.exclude ? "-" : ""}${chip.text}`;
  void nextTick(() => {
    const editor = editorRef.value;
    if (!editor) return;
    editor.focus();
    editor.select();
    editor.scrollIntoView({ block: "nearest", inline: "nearest" });
  });
}

function setEditorRef(element: unknown, index: number) {
  if (editingIndex.value === index && element instanceof HTMLInputElement) {
    editorRef.value = element;
  }
}

function commitEditing(search: boolean, refocus = true) {
  const index = editingIndex.value;
  if (index === null) return;
  const chip = parseSingleTerm(editingText.value);
  const next = [...tokens.value];
  if (chip) {
    next[index] = chip;
  } else {
    next.splice(index, 1);
  }
  tokens.value = next;
  editingIndex.value = null;
  editingText.value = "";
  emitQuery();
  if (search) emit("search");
  if (refocus) {
    void nextTick(() => inputRef.value?.focus());
  }
  scrollToEnd();
}

function cancelEditing() {
  editingIndex.value = null;
  editingText.value = "";
  void nextTick(() => inputRef.value?.focus());
}

function onEditorBlur() {
  commitEditing(false, false);
}

function onEditorKeydown(event: KeyboardEvent) {
  if (event.isComposing || composing.value) return;
  if (event.key === "Enter") {
    event.preventDefault();
    commitEditing(true, true);
  } else if (event.key === "Escape") {
    event.preventDefault();
    cancelEditing();
  }
}

function onMainKeydown(event: KeyboardEvent) {
  if (event.isComposing || composing.value) return;
  if (event.key === " ") {
    if (isQuoteOpen(draft.value)) return;
    event.preventDefault();
    commitDraft();
    return;
  }
  if (event.key === "Enter") {
    event.preventDefault();
    commitDraft();
    emit("search");
    return;
  }
  if (
    event.key === "Backspace" &&
    draft.value === "" &&
    tokens.value.length > 0
  ) {
    event.preventDefault();
    startEditing(tokens.value.length - 1);
    return;
  }
  if (event.key === "Escape") {
    event.preventDefault();
    clearAll();
  }
}

function onPaste(event: ClipboardEvent) {
  const text = event.clipboardData?.getData("text") ?? "";
  if (!text || props.disabled) return;
  event.preventDefault();
  const combined = draft.value + text;
  for (const chip of parseTerms(combined)) addChip(chip);
  draft.value = "";
  emitQuery();
  scrollToEnd();
}

function focus() {
  if (props.disabled) return;
  if (editingIndex.value !== null) {
    editorRef.value?.focus();
    editorRef.value?.select();
    return;
  }
  inputRef.value?.focus();
  inputRef.value?.select();
}

watch(draft, () => {
  emitQuery();
  scrollToEnd();
});

watch(
  () => props.modelValue,
  (value) => {
    if (value === serialize()) return;
    tokens.value = parseTerms(value);
    draft.value = "";
    editingIndex.value = null;
    editingText.value = "";
    scrollToEnd();
  },
  { immediate: true },
);

watch(
  () => props.disabled,
  (disabled) => {
    if (disabled) {
      editingIndex.value = null;
      editingText.value = "";
    }
  },
);

defineExpose({ focus });
</script>

<template>
  <div
    class="search-box chip-search"
    :class="{ disabled }"
    role="group"
    aria-label="搜索词"
  >
    <div ref="chipsRef" class="search-chips">
      <template
        v-for="(chip, index) in tokens"
        :key="`${index}-${chipKey(chip)}`"
      >
        <input
          v-if="editingIndex === index"
          :ref="(element) => setEditorRef(element, index)"
          v-model="editingText"
          class="chip-editor"
          type="text"
          :aria-label="`编辑搜索词 ${chip.text}`"
          :style="{
            width: `${Math.max(4, Math.min(28, editingText.length + 2))}ch`,
          }"
          @keydown="onEditorKeydown"
          @blur="onEditorBlur"
          @compositionstart="composing = true"
          @compositionend="composing = false"
        />
        <span
          v-else
          class="search-chip"
          :class="{ exclude: chip.exclude, phrase: chip.phrase }"
          role="button"
          tabindex="0"
          :aria-label="`编辑搜索词 ${chip.text}`"
          :title="chip.phrase ? '短语搜索词，点击编辑' : '点击编辑'"
          @focus="startEditing(index)"
        >
          <span
            v-if="chip.exclude"
            class="search-chip-minus"
            aria-hidden="true"
          >
            −
          </span>
          <span class="search-chip-text">{{ chip.text }}</span>
          <button
            class="search-chip-remove"
            type="button"
            tabindex="-1"
            :aria-label="`删除搜索词 ${chip.text}`"
            @mousedown.prevent
            @click.stop="removeToken(index)"
          >
            ×
          </button>
        </span>
      </template>

      <input
        ref="inputRef"
        v-model="draft"
        class="chip-input"
        type="text"
        :disabled="disabled"
        :placeholder="placeholder"
        aria-label="搜索提示词"
        @keydown="onMainKeydown"
        @paste="onPaste"
        @compositionstart="composing = true"
        @compositionend="composing = false"
        @focus="scrollToEnd"
      />
    </div>

    <button
      v-if="hasContent"
      class="search-clear"
      type="button"
      title="清除搜索"
      @click="clearAll"
    >
      ×
    </button>
  </div>
</template>
