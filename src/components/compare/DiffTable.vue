<script setup lang="ts">
import { computed } from "vue";
import type { ImageDetail } from "@/api/backend";

const props = defineProps<{
  items: ImageDetail[];
}>();

interface Field {
  key: string;
  label: string;
  value: (item: ImageDetail) => string;
}

const fields: Field[] = [
  {
    key: "positive",
    label: "正面提示词",
    value: (item) => item.positivePrompt ?? "",
  },
  {
    key: "negative",
    label: "负面提示词",
    value: (item) => item.negativePrompt ?? "",
  },
  { key: "model", label: "模型", value: (item) => item.modelName ?? "" },
  { key: "sampler", label: "采样器", value: (item) => item.sampler ?? "" },
  {
    key: "steps",
    label: "步数",
    value: (item) => (item.steps === null ? "" : String(item.steps)),
  },
  {
    key: "cfg",
    label: "CFG",
    value: (item) => (item.cfg === null ? "" : String(item.cfg)),
  },
  {
    key: "seed",
    label: "种子",
    value: (item) => (item.seed === null ? "" : String(item.seed)),
  },
  { key: "loras", label: "LoRA", value: (item) => item.loras ?? "" },
];

const rows = computed(() =>
  fields.map((field) => {
    const values = props.items.map(field.value);
    return {
      ...field,
      values,
      different: new Set(values).size > 1,
    };
  }),
);

const differentCount = computed(
  () => rows.value.filter((row) => row.different).length,
);
</script>

<template>
  <section class="diff-table">
    <header class="diff-head">
      <span class="diff-title">参数差异</span>
      <span class="diff-summary">{{ differentCount }} 项不同</span>
    </header>
    <div class="diff-scroll">
      <table>
        <thead>
          <tr>
            <th class="diff-field">参数</th>
            <th
              v-for="item in items"
              :key="item.id"
              :title="item.relPath"
            >
              {{ item.relPath }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="row in rows"
            :key="row.key"
            :class="{ different: row.different }"
          >
            <th class="diff-field">{{ row.label }}</th>
            <td
              v-for="(value, index) in row.values"
              :key="index"
              :title="value"
            >
              <span class="diff-value">{{ value || "—" }}</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
