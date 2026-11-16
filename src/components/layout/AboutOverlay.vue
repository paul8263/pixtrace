<script setup lang="ts">
import { onBeforeUnmount, onMounted } from "vue";
import iconUrl from "@/assets/pixtrace-icon.png";
import { useWorkspaceStore } from "@/stores/workspace";

const emit = defineEmits<{ close: [] }>();
const store = useWorkspaceStore();

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    emit("close");
  }
}

onMounted(() => window.addEventListener("keydown", onKeydown));
onBeforeUnmount(() => window.removeEventListener("keydown", onKeydown));
</script>

<template>
  <Teleport to="body">
    <div class="about-backdrop" @click.self="emit('close')">
      <section
        class="about-panel"
        role="dialog"
        aria-modal="true"
        aria-label="关于 PixTrace"
      >
        <header class="about-head">
          <button
            class="settings-close"
            type="button"
            title="关闭（Esc）"
            @click="emit('close')"
          >
            ×
          </button>
        </header>

        <div class="about-body">
          <img class="about-icon" :src="iconUrl" alt="PixTrace 图标" />
          <h2 class="about-title">PixTrace <span>图迹</span></h2>
          <p class="about-subtitle">本地 ComfyUI 图像浏览器</p>
          <p v-if="store.appInfo" class="about-version">
            版本 {{ store.appInfo.version }} ·
            {{ store.appInfo.mode === "portable" ? "便携版" : "安装版" }}
          </p>

          <dl class="about-facts">
            <dt>作者</dt>
            <dd>Yao Zhang</dd>
            <dt>邮箱</dt>
            <dd>
              <a href="mailto:xzhangyao@126.com">xzhangyao@126.com</a>
            </dd>
          </dl>
        </div>

        <footer class="about-foot">
          <button class="button primary" type="button" @click="emit('close')">
            关闭
          </button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>
