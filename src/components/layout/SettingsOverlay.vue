<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { formatBytes } from "@/api/backend";
import { useWorkspaceStore } from "@/stores/workspace";

const store = useWorkspaceStore();
const limitMb = ref(store.appInfo?.thumbCacheLimitMb ?? 2048);

const stats = computed(() => store.cacheStats);
const usedPercent = computed(() => {
  const current = stats.value;
  if (!current || current.limitBytes <= 0) return 0;
  return Math.min(100, Math.round((current.thumbBytes / current.limitBytes) * 100));
});

const scannedLabel = computed(() => {
  const seconds = store.workspace?.scannedAt ?? 0;
  if (!seconds) return "尚未扫描";
  return new Date(seconds * 1000).toLocaleString();
});

watch(
  () => store.appInfo?.thumbCacheLimitMb,
  (value) => {
    if (typeof value === "number") limitMb.value = value;
  },
);

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    store.closeSettings();
  }
}

onMounted(() => window.addEventListener("keydown", onKeydown));
onBeforeUnmount(() => window.removeEventListener("keydown", onKeydown));

async function applyLimit() {
  const value = Math.round(Number(limitMb.value));
  if (!Number.isFinite(value)) return;
  const clamped = Math.max(64, Math.min(102400, value));
  limitMb.value = clamped;
  await store.updateCacheLimit(clamped);
}
</script>

<template>
  <div class="settings-backdrop" @click.self="store.closeSettings()">
    <section
      class="settings-panel"
      role="dialog"
      aria-modal="true"
      aria-label="设置"
    >
      <header class="settings-head">
        <h2>设置</h2>
        <button
          class="settings-close"
          type="button"
          title="关闭（Esc）"
          @click="store.closeSettings()"
        >
          ×
        </button>
      </header>

      <div class="settings-body">
        <section v-if="store.workspace" class="settings-section">
          <h3>工作目录</h3>
          <dl class="settings-facts">
            <dt>目录</dt>
            <dd :title="store.workspace.root">{{ store.workspace.root }}</dd>
            <dt>图片</dt>
            <dd>{{ store.workspace.imageCount.toLocaleString() }} 张</dd>
            <dt>上次扫描</dt>
            <dd>{{ scannedLabel }}</dd>
            <dt>索引文件</dt>
            <dd :title="store.workspace.dbPath">{{ store.workspace.dbPath }}</dd>
          </dl>
          <p v-if="store.workspace.warning" class="settings-warning">
            {{ store.workspace.warning }}
          </p>
          <div class="settings-actions">
            <button
              class="button ghost"
              type="button"
              :disabled="store.settingsBusy || store.busy"
              @click="store.rescan()"
            >
              重新扫描目录
            </button>
            <button
              class="button ghost"
              type="button"
              :disabled="store.settingsBusy || store.busy"
              @click="store.rebuildIndex()"
            >
              重建提示词索引
            </button>
          </div>
          <p class="settings-hint">
            重新扫描只处理新增或修改过的文件；重建索引会重新解析全部图片的提示词。
          </p>
        </section>

        <section class="settings-section">
          <h3>文件监听</h3>
          <label class="settings-switch">
            <input
              type="checkbox"
              :checked="store.watchEnabled"
              :disabled="store.settingsBusy || !store.hasWorkspace"
              @change="
                store.updateWatchEnabled(
                  ($event.target as HTMLInputElement).checked,
                )
              "
            />
            <span>自动监听工作目录的变化</span>
          </label>
          <p class="settings-hint">
            开启后，新增、修改或删除图片会在约 2 秒内自动刷新。网络目录上的监听可能不可靠。
          </p>
        </section>

        <section class="settings-section">
          <h3>缩略图缓存</h3>
          <template v-if="stats">
            <div class="cache-meter">
              <span class="cache-meter-fill" :style="{ width: `${usedPercent}%` }" />
            </div>
            <dl class="settings-facts">
              <dt>缩略图</dt>
              <dd>
                {{ stats.thumbCount.toLocaleString() }} 个 ·
                {{ formatBytes(stats.thumbBytes) }}
              </dd>
              <dt>索引数据库</dt>
              <dd>{{ formatBytes(stats.dbBytes) }}</dd>
              <dt>缓存上限</dt>
              <dd>
                {{ formatBytes(stats.limitBytes) }}（已用 {{ usedPercent }}%）
              </dd>
            </dl>
          </template>
          <p v-else class="settings-hint">正在读取缓存信息…</p>
          <div class="settings-inline">
            <label class="cache-limit-field">
              <span>上限</span>
              <input
                v-model.number="limitMb"
                type="number"
                min="64"
                max="102400"
                step="256"
                :disabled="store.settingsBusy"
              />
              <span>MB</span>
            </label>
            <button
              class="button ghost"
              type="button"
              :disabled="store.settingsBusy"
              @click="applyLimit"
            >
              应用
            </button>
            <button
              class="button ghost"
              type="button"
              :disabled="store.settingsBusy || !store.hasWorkspace"
              @click="store.clearThumbCache()"
            >
              清理缓存
            </button>
          </div>
          <p class="settings-hint">
            超过上限时按最近使用时间自动清理最旧的缩略图。清理后浏览时会重新生成。
          </p>
        </section>

        <section v-if="store.appInfo" class="settings-section">
          <h3>关于与数据位置</h3>
          <dl class="settings-facts">
            <dt>版本</dt>
            <dd>{{ store.appInfo.version }} · {{ store.appInfo.mode === "portable" ? "便携版" : "安装版" }}</dd>
            <dt>数据目录</dt>
            <dd :title="store.appInfo.dataDir">{{ store.appInfo.dataDir }}</dd>
            <dt>日志目录</dt>
            <dd :title="store.appInfo.logDir">{{ store.appInfo.logDir }}</dd>
            <dt>设置文件</dt>
            <dd :title="store.appInfo.settingsFile">{{ store.appInfo.settingsFile }}</dd>
          </dl>
          <p v-if="store.appInfo.warning" class="settings-warning">
            {{ store.appInfo.warning }}
          </p>
        </section>

        <p v-if="store.error" class="settings-error">
          {{ store.error }}
        </p>
        <p v-if="store.settingsError" class="settings-error">
          {{ store.settingsError }}
        </p>
        <p v-if="store.settingsMessage" class="settings-message">
          {{ store.settingsMessage }}
        </p>
      </div>

      <footer class="settings-foot">
        <button class="button primary" type="button" @click="store.closeSettings()">
          完成
        </button>
      </footer>
    </section>
  </div>
</template>
