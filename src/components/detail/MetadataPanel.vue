<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { getImageAsset, type AssetKind, type ImageDetail } from "@/api/backend";
import FileInfoPanel from "./FileInfoPanel.vue";
import ParameterPanel from "./ParameterPanel.vue";
import PromptPanel from "./PromptPanel.vue";
import WorkflowJsonViewer from "./WorkflowJsonViewer.vue";

const props = defineProps<{
  detail: ImageDetail | null;
  loading: boolean;
  error: string | null;
}>();

type Tab = "prompt" | "params" | "workflow" | "file";

const tabs: { key: Tab; label: string }[] = [
  { key: "prompt", label: "提示词" },
  { key: "params", label: "参数" },
  { key: "workflow", label: "工作流" },
  { key: "file", label: "文件" },
];

const activeTab = ref<Tab>("prompt");
const assets = reactive<Record<string, string | null>>({});
const assetLoading = reactive<Record<string, boolean>>({});
const showRaw = ref(false);

function keyFor(kind: AssetKind): string {
  return props.detail ? `${props.detail.id}:${kind}` : "";
}

async function ensureAsset(kind: AssetKind): Promise<string | null> {
  if (!props.detail) return null;
  const key = keyFor(kind);
  if (key in assets) return assets[key];
  if (assetLoading[key]) return null;
  assetLoading[key] = true;
  try {
    const asset = await getImageAsset(props.detail.id, kind);
    assets[key] = asset.text;
    return asset.text;
  } catch {
    assets[key] = null;
    return null;
  } finally {
    assetLoading[key] = false;
  }
}

async function loadForTab(tab: Tab) {
  if (!props.detail) return;
  if (tab === "workflow") {
    const workflow = await ensureAsset("workflow");
    if (workflow === null) {
      await ensureAsset("prompt");
    }
  }
  if (tab === "prompt" && showRaw.value) {
    await ensureAsset("raw");
  }
}

watch(
  [() => props.detail?.id, activeTab],
  () => {
    void loadForTab(activeTab.value);
  },
  { immediate: true },
);

async function toggleRaw() {
  showRaw.value = !showRaw.value;
  if (showRaw.value) {
    await ensureAsset("raw");
  }
}

const workflowText = computed(() =>
  props.detail
    ? (assets[keyFor("workflow")] ?? assets[keyFor("prompt")] ?? null)
    : null,
);

const workflowLoading = computed(() =>
  props.detail
    ? Boolean(assetLoading[keyFor("workflow")] || assetLoading[keyFor("prompt")])
    : false,
);

const rawText = computed(() =>
  props.detail ? (assets[keyFor("raw")] ?? null) : null,
);

const rawLoading = computed(() =>
  props.detail ? Boolean(assetLoading[keyFor("raw")]) : false,
);
</script>

<template>
  <aside class="metadata-panel">
    <div v-if="loading" class="panel-state">载入元数据…</div>
    <div v-else-if="error" class="panel-state error">{{ error }}</div>
    <template v-else-if="detail">
      <nav class="panel-tabs">
        <button
          v-for="tab in tabs"
          :key="tab.key"
          class="panel-tab"
          :class="{ active: activeTab === tab.key }"
          @click="activeTab = tab.key"
        >
          {{ tab.label }}
        </button>
      </nav>

      <div class="panel-content">
        <template v-if="activeTab === 'prompt'">
          <PromptPanel
            :positive="detail.positivePrompt"
            :negative="detail.negativePrompt"
            :positive-tokens="detail.positiveTokens"
            :negative-tokens="detail.negativeTokens"
          />
          <div v-if="detail.hasRawText" class="raw-block">
            <button class="link-button" @click="toggleRaw">
              {{ showRaw ? "收起原始文本" : "查看原始文本" }}
            </button>
            <p v-if="rawLoading" class="muted">加载中…</p>
            <pre v-else-if="showRaw && rawText" class="json-raw">{{ rawText }}</pre>
            <p v-else-if="showRaw" class="muted">没有原始文本。</p>
          </div>
        </template>

        <ParameterPanel
          v-else-if="activeTab === 'params'"
          :detail="detail"
        />

        <WorkflowJsonViewer
          v-else-if="activeTab === 'workflow'"
          :text="workflowText"
          :loading="workflowLoading"
          empty-text="没有可用的工作流或 prompt JSON。"
        />

        <FileInfoPanel v-else-if="activeTab === 'file'" :detail="detail" />
      </div>
    </template>
    <div v-else class="panel-state">没有选中图像。</div>
  </aside>
</template>
