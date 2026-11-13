<script setup lang="ts">
import { computed } from "vue";
import type { ImageDetail } from "@/api/backend";

const props = defineProps<{
  detail: ImageDetail;
}>();

const rows = computed(() => {
  const detail = props.detail;
  const items: { label: string; value: string }[] = [];
  if (detail.modelName) items.push({ label: "模型", value: detail.modelName });
  if (detail.loras) items.push({ label: "LoRA", value: detail.loras });
  if (detail.sampler) items.push({ label: "采样器", value: detail.sampler });
  if (detail.steps !== null) {
    items.push({ label: "步数", value: String(detail.steps) });
  }
  if (detail.cfg !== null) items.push({ label: "CFG", value: String(detail.cfg) });
  if (detail.seed !== null) {
    items.push({ label: "种子", value: String(detail.seed) });
  }
  if (detail.width && detail.height) {
    items.push({ label: "分辨率", value: `${detail.width} × ${detail.height}` });
  }
  return items;
});

const statusNote = computed(() => {
  switch (props.detail.parseStatus) {
    case "partial":
      return "部分元数据未能确定，已回退到文本节点收集。";
    case "no_meta":
      return "该文件没有可解析的生成参数。";
    case "error":
      return props.detail.parseError ?? "解析失败。";
    default:
      return "";
  }
});
</script>

<template>
  <div class="parameter-panel">
    <dl v-if="rows.length" class="parameter-table">
      <div v-for="row in rows" :key="row.label" class="parameter-row">
        <dt>{{ row.label }}</dt>
        <dd>{{ row.value }}</dd>
      </div>
    </dl>
    <p v-else class="muted">未提取到生成参数。</p>
    <p v-if="statusNote" class="parse-note">{{ statusNote }}</p>
  </div>
</template>
