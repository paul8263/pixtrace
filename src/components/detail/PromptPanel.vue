<script setup lang="ts">
import { ref } from "vue";

const props = defineProps<{
  positive: string | null;
  negative: string | null;
}>();

const copied = ref<"positive" | "negative" | null>(null);
let timer: number | undefined;

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
      <p v-if="positive" class="prompt-text">{{ positive }}</p>
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
      <p v-if="negative" class="prompt-text">{{ negative }}</p>
      <p v-else class="muted">未提取到负面提示词。</p>
    </section>
  </div>
</template>
