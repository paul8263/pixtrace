<script setup lang="ts">
defineOptions({ name: "JsonTreeNode" });

defineProps<{
  name?: string;
  value: unknown;
  depth: number;
}>();

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function entries(value: unknown): [string, unknown][] {
  return isRecord(value) ? Object.entries(value) : [];
}

function summary(value: unknown): string {
  if (Array.isArray(value)) return `[${value.length}]`;
  if (isRecord(value)) return `{${Object.keys(value).length}}`;
  return "";
}

function leaf(value: unknown): string {
  if (typeof value === "string") return value;
  if (value === null) return "null";
  return String(value);
}

function leafClass(value: unknown): string {
  if (typeof value === "string") return "string";
  if (typeof value === "number") return "number";
  if (typeof value === "boolean") return "boolean";
  return "null";
}
</script>

<template>
  <div class="json-node">
    <details v-if="isRecord(value)" :open="depth < 2">
      <summary class="json-summary-row">
        <span v-if="name" class="json-key">{{ name }}</span>
        <span class="json-summary">{{ summary(value) }}</span>
      </summary>
      <div class="json-children">
        <JsonTreeNode
          v-for="[key, child] in entries(value)"
          :key="key"
          :name="Array.isArray(value) ? `[${key}]` : key"
          :value="child"
          :depth="depth + 1"
        />
      </div>
    </details>
    <div v-else class="json-leaf">
      <span v-if="name" class="json-key">{{ name }}</span>
      <span :class="['json-value', leafClass(value)]">{{ leaf(value) }}</span>
    </div>
  </div>
</template>
