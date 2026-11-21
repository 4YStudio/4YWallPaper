<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import WallpaperView from "./views/WallpaperView.vue";
import SettingsView from "./views/SettingsView.vue";
import LogView from "./views/LogView.vue";
import Icon from "./components/Icon.vue";
import type { IconName } from "./components/icon-names";
import logoUrl from "./assets/logo.svg";
import { plasmaStatus, stopAll } from "./api";
import type { PlasmaStatus } from "./types";

type Tab = "wallpaper" | "settings" | "logs";

const tab = ref<Tab>("wallpaper");
const status = ref<PlasmaStatus | null>(null);
let timer: number | undefined;

const tabs: { key: Tab; label: string; icon: IconName }[] = [
  { key: "wallpaper", label: "壁纸", icon: "gallery" },
  { key: "settings", label: "设置", icon: "settings" },
  { key: "logs", label: "日志", icon: "logs" },
];

const activeCount = computed(() => status.value?.active_screens.length ?? 0);

async function refreshStatus() {
  try {
    status.value = await plasmaStatus();
  } catch {
    /* ignore */
  }
}

async function onStopAll() {
  await stopAll();
  await refreshStatus();
}

onMounted(() => {
  refreshStatus();
  timer = window.setInterval(refreshStatus, 3000);
});
onUnmounted(() => window.clearInterval(timer));
</script>

<template>
  <div class="layout">
    <aside class="sidebar">
      <div class="brand">
        <img class="brand-logo" :src="logoUrl" alt="" />
        <div class="brand-text">
          <div class="brand-name">4YWallPaper</div>
          <div class="brand-sub">动态壁纸</div>
        </div>
      </div>
      <nav>
        <button
          v-for="t in tabs"
          :key="t.key"
          class="nav-btn"
          :class="{ active: tab === t.key }"
          @click="tab = t.key"
        >
          <Icon :name="t.icon" :size="17" />
          <span>{{ t.label }}</span>
        </button>
      </nav>
      <div class="sidebar-footer">
        <div class="run-state" :class="{ on: activeCount > 0 }">
          <span class="dot"></span>
          {{ activeCount > 0 ? `${activeCount} 屏运行中` : "无运行中壁纸" }}
        </div>
        <button
          class="danger full"
          :disabled="activeCount === 0"
          @click="onStopAll"
        >
          <Icon name="stop" :size="13" /> 停止全部
        </button>
      </div>
    </aside>
    <main class="content">
      <WallpaperView v-if="tab === 'wallpaper'" @refresh-status="refreshStatus" />
      <SettingsView v-else-if="tab === 'settings'" />
      <LogView v-else />
    </main>
  </div>
</template>

<style scoped>
.layout {
  display: flex;
  height: 100%;
}
.sidebar {
  width: 196px;
  flex-shrink: 0;
  background: var(--bg-1);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  padding: 14px 10px;
}
.brand {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 2px 6px 16px;
}
.brand-logo {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  display: block;
}
.brand-text {
  min-width: 0;
}
.brand-name {
  font-weight: 600;
  font-size: 14px;
  line-height: 1.25;
}
.brand-sub {
  font-size: 11px;
  color: var(--text-2);
  line-height: 1.3;
}
nav {
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.nav-btn {
  display: flex;
  align-items: center;
  gap: 10px;
  text-align: left;
  background: transparent;
  border-color: transparent;
  padding: 9px 12px;
  border-radius: 8px;
  color: var(--text-1);
}
.nav-btn:hover {
  background: var(--bg-2);
  border-color: transparent;
  color: var(--text-0);
}
.nav-btn.active {
  background: var(--bg-3);
  color: var(--accent-hover);
}
.sidebar-footer {
  margin-top: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.run-state {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-2);
  font-size: 12px;
  padding: 0 4px;
}
.run-state .dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-2);
}
.run-state.on {
  color: var(--green);
}
.run-state.on .dot {
  background: var(--green);
  box-shadow: 0 0 6px var(--green);
}
.full {
  width: 100%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
}
.content {
  flex: 1;
  overflow: auto;
  padding: 20px 24px;
}
</style>
