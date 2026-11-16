<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";
import type { PromptToken } from "@/api/backend";
import { useLibraryStore } from "@/stores/library";
import PromptText from "./PromptText.vue";

const props = defineProps<{
  positive: string | null;
  negative: string | null;
  positiveTokens: PromptToken[];
  negativeTokens: PromptToken[];
}>();

const library = useLibraryStore();
const copied = ref<"positive" | "negative" | null>(null);
const notice = ref<string | null>(null);
let timer: number | undefined;
let noticeTimer: number | undefined;

async function copy(kind: "positive" | "negative") {
  const text = kind === "positive" ? props.positive : props.negative;
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
    copied.value = kind;
    window.clearTimeout(timer);
    timer = window.setTimeout(() => {
      copied.value = null;
    }, 1500);
  } catch {
    copied.value = null;
  }
}

function pick(term: string, exclude: boolean) {
  const action = library.addSearchTerm(term, exclude);
  if (action === "empty") return;
  if (action === "exists") {
    notice.value = exclude ? `已在排除词中：${term}` : `已在搜索中：${term}`;
  } else if (action === "replaced") {
    notice.value = exclude ? `已改为排除：${term}` : `已改为包含：${term}`;
  } else {
    notice.value = exclude ? `已排除：${term}` : `已加入搜索：${term}`;
  }
  window.clearTimeout(noticeTimer);
  noticeTimer = window.setTimeout(() => {
    notice.value = null;
  }, 1800);
}

onBeforeUnmount(() => {
  window.clearTimeout(timer);
  window.clearTimeout(noticeTimer);
});
</script>

<template>
  <div class="prompt-panel">
    <section class="prompt-section">
      <header class="prompt-header">
        <h3>正面提示词</h3>
        <button
          class="link-button"
          :disabled="!positive"
          @click="copy('positive')"
        >
          {{ copied === "positive" ? "已复制" : "复制" }}
        </button>
      </header>
      <PromptText
        v-if="positive"
        :text="positive"
        :tokens="positiveTokens"
        :query="library.query"
        @pick="pick"
      />
      <p v-else class="muted">未提取到正面提示词。</p>
    </section>

    <section class="prompt-section">
      <header class="prompt-header">
        <h3>负面提示词</h3>
        <button
          class="link-button"
          :disabled="!negative"
          @click="copy('negative')"
        >
          {{ copied === "negative" ? "已复制" : "复制" }}
        </button>
      </header>
      <PromptText
        v-if="negative"
        :text="negative"
        :tokens="negativeTokens"
        :query="library.query"
        @pick="pick"
      />
      <p v-else class="muted">未提取到负面提示词。</p>
    </section>

    <p v-if="notice" class="prompt-notice">{{ notice }}</p>
  </div>
</template>
