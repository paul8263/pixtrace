<script setup lang="ts">
import { useRouter } from "vue-router";
import ProtocolCheck from "@/components/common/ProtocolCheck.vue";
import { useWorkspaceStore } from "@/stores/workspace";

const store = useWorkspaceStore();
const router = useRouter();

async function openDirectory() {
  const opened = await store.chooseWorkspace();
  if (opened) {
    await router.push("/library");
  }
}

async function openRecent() {
  const recent = store.appInfo?.lastWorkspace;
  if (!recent) return;
  const opened = await store.openWorkspaceAt(recent);
  if (opened) {
    await router.push("/library");
  }
}
</script>

<template>
  <section class="welcome-view">
    <div class="welcome-hero">
      <p class="eyebrow">ComfyUI 图像浏览器</p>
      <h1>PixTrace <span>图迹</span></h1>
      <p class="lead">
        打开一个工作目录，浏览、检索、评级并对比你的 ComfyUI 生成图像。索引保存在目录内的
        <code>.pixtrace/</code> 中，可随目录一起迁移。
      </p>
      <div class="welcome-actions">
        <button
          class="button primary large"
          :disabled="store.busy"
          @click="openDirectory"
        >
          {{ store.busy ? "正在扫描…" : "打开工作目录" }}
        </button>
        <button
          v-if="store.appInfo?.lastWorkspace"
          class="button ghost large"
          :disabled="store.busy"
          @click="openRecent"
        >
          打开最近目录
        </button>
      </div>
      <p
        v-if="store.appInfo?.lastWorkspace"
        class="recent-path"
        :title="store.appInfo.lastWorkspace"
      >
        最近：{{ store.appInfo.lastWorkspace }}
      </p>
    </div>

    <div class="welcome-meta">
      <div class="meta-card">
        <span class="meta-label">运行模式</span>
        <strong>{{ store.appInfo?.mode === "portable" ? "便携版" : "安装版" }}</strong>
        <span class="meta-detail">{{ store.appInfo?.dataDir ?? "读取中…" }}</span>
      </div>
      <div class="meta-card">
        <span class="meta-label">当前阶段</span>
        <strong>M3 评级与对比</strong>
        <span class="meta-detail">星级评级、同提示词分组与同步对比</span>
      </div>
      <div class="meta-card protocol-card">
        <span class="meta-label">协议自检</span>
        <ProtocolCheck />
        <span class="meta-detail">pixtrace:// 用于安全读取工作目录内的图像</span>
      </div>
    </div>
  </section>
</template>
