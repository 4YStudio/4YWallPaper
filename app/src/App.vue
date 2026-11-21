<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import LibraryView from "./views/LibraryView.vue";
import MonitorView from "./views/MonitorView.vue";
import SettingsView from "./views/SettingsView.vue";
import LogView from "./views/LogView.vue";
import { plasmaStatus, stopAll } from "./api";
import type { PlasmaStatus } from "./types";

type Tab = "library" | "monitor" | "settings" | "logs";

const tab = ref<Tab>("library");
const status = ref<PlasmaStatus | null>(null);
let timer: number | undefined;

const tabs: { key: Tab; label: string; icon: string }[] = [
  { key: "library", label: "壁纸库", icon: "🖼️" },
  { key: "monitor", label: "显示器", icon: "🖥️" },
  { key: "settings", label: "设置", icon: "⚙️" },
  { key: "logs", label: "日志", icon: "📜" },
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
        <div class="brand-icon">4Y</div>
        <div class="brand-name">4YWallPaper</div>
      </div>
      <nav>
        <button
          v-for="t in tabs"
          :key="t.key"
          class="nav-btn"
          :class="{ active: tab === t.key }"
          @click="tab = t.key"
        >
          <span class="nav-icon">{{ t.icon }}</span>
          <span>{{ t.label }}</span>
        </button>
      </nav>
      <div class="sidebar-footer">
        <div class="run-state" :class="{ on: activeCount > 0 }">
          <span class="dot"></span>
          {{ activeCount > 0 ? `${activeCount} 屏动态壁纸运行中` : "无运行中壁纸" }}
        </div>
        <button
          class="danger full"
          :disabled="activeCount === 0"
          @click="onStopAll"
        >
          停止全部
        </button>
      </div>
    </aside>
    <main class="content">
      <LibraryView v-if="tab === 'library'" @refresh-status="refreshStatus" />
      <MonitorView v-else-if="tab === 'monitor'" @refresh-status="refreshStatus" />
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
  width: 200px;
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
  gap: 10px;
  padding: 4px 8px 16px;
}
.brand-icon {
  width: 34px;
  height: 34px;
  border-radius: 8px;
  background: var(--accent);
  color: #fff;
  font-weight: 700;
  display: grid;
  place-items: center;
  font-size: 13px;
}
.brand-name {
  font-weight: 600;
  font-size: 15px;
}
nav {
  display: flex;
  flex-direction: column;
  gap: 4px;
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
}
.nav-btn:hover {
  background: var(--bg-2);
  border-color: transparent;
}
.nav-btn.active {
  background: var(--bg-3);
  color: var(--accent-hover);
}
.nav-icon {
  font-size: 15px;
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
}
.content {
  flex: 1;
  overflow: auto;
  padding: 22px 26px;
}
</style>
