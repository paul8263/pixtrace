<script setup lang="ts">
import { computed } from "vue";
import type { PromptToken } from "@/api/backend";
import {
  queryTermState,
  type SearchChipState,
} from "@/utils/searchQuery";

const props = defineProps<{
  text: string | null;
  tokens: PromptToken[];
  query: string;
}>();

const emit = defineEmits<{
  (event: "pick", term: string, exclude: boolean): void;
}>();

interface Segment {
  text: string;
  token?: PromptToken;
}

const segments = computed<Segment[]>(() => {
  const source = props.text ?? "";
  if (!source) return [];
  const result: Segment[] = [];
  let cursor = 0;
  for (const token of props.tokens) {
    if (!token.term || token.start < cursor || token.end <= token.start) {
      continue;
    }
    const start = Math.min(token.start, source.length);
    const end = Math.min(token.end, source.length);
    if (start >= end) continue;
    if (start > cursor) result.push({ text: source.slice(cursor, start) });
    result.push({ text: source.slice(start, end), token });
    cursor = end;
  }
  if (cursor < source.length) result.push({ text: source.slice(cursor) });
  return result;
});

const termStates = computed(() => {
  const states = new Map<string, SearchChipState>();
  for (const token of props.tokens) {
    const state = queryTermState(props.query, token.term);
    if (state) states.set(token.term, state);
  }
  return states;
});

function pick(event: MouseEvent, token: PromptToken) {
  const selection = window.getSelection();
  if (selection && !selection.isCollapsed && selection.toString().trim()) {
    return;
  }
  event.preventDefault();
  emit("pick", token.term, event.ctrlKey || event.metaKey);
}

function onKeydown(event: KeyboardEvent, token: PromptToken) {
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    emit("pick", token.term, event.ctrlKey || event.metaKey);
  }
}
</script>

<template>
  <p class="prompt-text prompt-tokens">
    <template v-for="(segment, index) in segments" :key="index">
      <span
        v-if="segment.token"
        class="prompt-token"
        :class="{
          included: termStates.get(segment.token.term) === 'positive',
          excluded: termStates.get(segment.token.term) === 'negative',
        }"
        role="button"
        tabindex="0"
        :title="`点击加入搜索，Ctrl+点击排除：${segment.token.term}`"
        :aria-label="`搜索词 ${segment.token.term}`"
        @click="pick($event, segment.token)"
        @keydown="onKeydown($event, segment.token)"
      >{{ segment.text }}</span>
      <template v-else>{{ segment.text }}</template>
    </template>
  </p>
</template>
