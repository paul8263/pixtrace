<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";

defineProps<{
  label: string;
  count: number;
  busy: boolean;
  error: string | null;
}>();

const emit = defineEmits<{
  (event: "cancel"): void;
  (event: "confirm", permanent: boolean): void;
}>();

const mode = ref<"trash" | "permanent">("trash");

function cancel() {
  emit("cancel");
}

function confirm() {
  emit("confirm", mode.value === "permanent");
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    cancel();
  }
}

onMounted(() => window.addEventListener("keydown", onKeydown));
onBeforeUnmount(() => window.removeEventListener("keydown", onKeydown));
</script>

<template>
  <div class="delete-backdrop" @click.self="cancel">
    <section
      class="delete-dialog"
      role="dialog"
      aria-modal="true"
      aria-label="删除图片"
    >
      <header class="delete-head">
        <h2>{{ count > 1 ? `删除 ${count.toLocaleString()} 张图片` : "删除图片" }}</h2>
        <p :title="label">{{ label }}</p>
      </header>

      <div class="delete-body">
        <p class="delete-lead">请选择删除方式：</p>
        <label class="delete-option" :class="{ active: mode === 'trash' }">
          <input v-model="mode" type="radio" value="trash" />
          <span class="delete-option-text">
            <strong>移到回收站</strong>
            <small>默认方式，可从系统回收站恢复。</small>
          </span>
        </label>
        <label
          class="delete-option danger"
          :class="{ active: mode === 'permanent' }"
        >
          <input v-model="mode" type="radio" value="permanent" />
          <span class="delete-option-text">
            <strong>永久删除</strong>
            <small>直接从磁盘移除，无法恢复。</small>
          </span>
        </label>
        <p class="delete-hint">
          删除的是工作目录中的原始图片文件，评级与索引会同步更新。
          <template v-if="count > 1">本次共 {{ count.toLocaleString() }} 张。</template>
        </p>
        <p v-if="error" class="delete-error">{{ error }}</p>
      </div>

      <footer class="delete-foot">
        <button
          class="button ghost"
          type="button"
          :disabled="busy"
          @click="cancel"
        >
          取消
        </button>
        <button
          class="button danger"
          type="button"
          :disabled="busy"
          @click="confirm"
        >
          {{ busy ? "删除中…" : "删除" }}
        </button>
      </footer>
    </section>
  </div>
</template>
