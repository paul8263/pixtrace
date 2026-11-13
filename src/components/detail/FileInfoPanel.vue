<script setup lang="ts">
import { computed } from "vue";
import { formatBytes, type ImageDetail } from "@/api/backend";

const props = defineProps<{
  detail: ImageDetail;
}>();

function statusLabel(status: string): string {
  switch (status) {
    case "ok":
      return "已解析";
    case "partial":
      return "部分解析";
    case "no_meta":
      return "无元数据";
    case "error":
      return "解析失败";
    case "pending":
      return "待解析";
    default:
      return status;
  }
}

const rows = computed(() => {
  const detail = props.detail;
  const items: { label: string; value: string; error?: boolean }[] = [
    { label: "相对路径", value: detail.relPath },
    { label: "文件大小", value: formatBytes(detail.fileSize) },
    {
      label: "修改时间",
      value: new Date(Math.floor(detail.mtimeNs / 1_000_000)).toLocaleString(
        "zh-CN",
        { hour12: false },
      ),
    },
    { label: "格式", value: detail.format.toUpperCase() },
    {
      label: "尺寸",
      value:
        detail.width && detail.height
          ? `${detail.width} × ${detail.height}`
          : "未知",
    },
    { label: "解析状态", value: statusLabel(detail.parseStatus) },
  ];
  if (detail.parseError) {
    items.push({ label: "解析信息", value: detail.parseError, error: true });
  }
  if (detail.indexedAt) {
    items.push({
      label: "索引时间",
      value: new Date(detail.indexedAt * 1000).toLocaleString("zh-CN", {
        hour12: false,
      }),
    });
  }
  return items;
});
</script>

<template>
  <dl class="parameter-table file-info">
    <div v-for="row in rows" :key="row.label" class="parameter-row">
      <dt>{{ row.label }}</dt>
      <dd :class="{ error: row.error }">{{ row.value }}</dd>
    </div>
  </dl>
</template>
