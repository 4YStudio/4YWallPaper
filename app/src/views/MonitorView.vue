<script setup lang="ts">
import { ref, onMounted } from "vue";
import {
  listMonitors,
  listWallpapers,
  getConfig,
  applyWallpapers,
  stopMonitor,
  plasmaStatus,
} from "../api";
import type { Monitor, WallpaperEntry, MonitorAssignment } from "../types";

const emit = defineEmits<{ (e: "refresh-status"): void }>();

const monitors = ref<Monitor[]>([]);
const wallpapers = ref<WallpaperEntry[]>([]);
const assignments = ref<MonitorAssignment[]>([]);
const applying = ref(false);
const msg = ref<string | null>(null);

const scalingOptions = [
  { value: "default", label: "默认" },
  { value: "stretch", label: "拉伸" },
  { value: "fit", label: "适应（保持比例留边）" },
  { value: "fill", label: "填充（保持比例裁剪）" },
];

function assignmentOf(monitorId: string): MonitorAssignment | undefined {
  return assignments.value.find((a) => a.monitor === monitorId);
}

function setAssignment(monitorId: string, wallpaperId: string) {
  const existing = assignmentOf(monitorId);
  if (existing) {
    existing.wallpaper_id = wallpaperId;
  } else {
    assignments.value.push({
      monitor: monitorId,
      wallpaper_id: wallpaperId,
      scaling: "default",
    });
  }
}

function setScaling(monitorId: string, scaling: string) {
  const existing = assignmentOf(monitorId);
  if (existing) {
    existing.scaling = scaling as MonitorAssignment["scaling"];
  } else {
    assignments.value.push({
      monitor: monitorId,
      wallpaper_id: "",
      scaling: scaling as MonitorAssignment["scaling"],
    });
  }
}

async function refresh() {
  const [ms, ws, cfg, st] = await Promise.all([
    listMonitors(),
    listWallpapers(),
    getConfig(),
    plasmaStatus(),
  ]);
  monitors.value = ms.filter((m) => m.enabled && m.connected !== false);
  wallpapers.value = ws;
  assignments.value = cfg.assignments;
  // 标记正在播放动态壁纸的显示器（由 Plasma 插件状态得出）
  const active = new Set(st.active_screens);
  for (const m of monitors.value) {
    (m as Monitor & { running?: boolean }).running = active.has(m.id);
  }
}

async function onApply() {
  const valid = assignments.value.filter((a) => a.wallpaper_id);
  if (valid.length === 0) {
    msg.value = "请先为至少一个显示器选择壁纸";
    return;
  }
  applying.value = true;
  msg.value = null;
  try {
    await applyWallpapers(
      valid.map((a) => ({
        monitor: a.monitor,
        wallpaperId: a.wallpaper_id,
        scaling: a.scaling,
      })),
    );
    msg.value = "✅ 壁纸已应用（正在重启 plasmashell，约 2 秒后生效）";
    await new Promise((r) => setTimeout(r, 3500));
    await refresh();
    emit("refresh-status");
  } catch (e) {
    msg.value = `❌ 应用失败: ${e}`;
  } finally {
    applying.value = false;
    setTimeout(() => (msg.value = null), 5000);
  }
}

async function onStop(monitor: string) {
  await stopMonitor(monitor);
  await new Promise((r) => setTimeout(r, 1500));
  await refresh();
  emit("refresh-status");
}

onMounted(refresh);
</script>

<template>
  <div>
    <div class="header">
      <h1>显示器与壁纸分配</h1>
      <button class="primary" :disabled="applying" @click="onApply">
        {{ applying ? "应用中…" : "应用壁纸" }}
      </button>
    </div>
    <p v-if="msg" class="msg" :class="{ ok: msg.startsWith('✅') }">{{ msg }}</p>

    <div v-if="monitors.length === 0" class="empty">
      未检测到显示器（需要 kscreen-doctor，KDE Plasma 自带）
    </div>

    <div v-for="m in monitors" :key="m.id" class="monitor-card">
      <div class="m-head">
        <div class="m-title">
          <span class="m-name">{{ m.name || m.id }}</span>
          <span class="m-id">{{ m.id }}</span>
          <span
            class="m-state"
            :class="{ on: (m as any).running }"
          >{{ (m as any).running ? "运行中" : "未运行" }}</span>
        </div>
        <div class="m-spec">
          {{ m.width }}×{{ m.height }} @ {{ m.refresh }}Hz · 位置 ({{ m.x }}, {{ m.y }})
        </div>
      </div>
      <div class="m-body">
        <label>
          <span class="lbl">壁纸</span>
          <select
            :value="assignmentOf(m.id)?.wallpaper_id ?? ''"
            @change="setAssignment(m.id, ($event.target as HTMLSelectElement).value)"
          >
            <option value="">（不使用动态壁纸）</option>
            <option v-for="w in wallpapers" :key="w.id" :value="w.id">
              [{{ w.type }}] {{ w.title }}
            </option>
          </select>
        </label>
        <label>
          <span class="lbl">缩放</span>
          <select
            :value="assignmentOf(m.id)?.scaling ?? 'default'"
            @change="setScaling(m.id, ($event.target as HTMLSelectElement).value)"
          >
            <option v-for="o in scalingOptions" :key="o.value" :value="o.value">
              {{ o.label }}
            </option>
          </select>
        </label>
        <button class="danger" @click="onStop(m.id)">停止此屏</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 14px;
}
h1 {
  font-size: 20px;
  font-weight: 600;
}
.msg {
  margin-bottom: 12px;
  padding: 8px 12px;
  border-radius: 6px;
  background: var(--bg-2);
  border: 1px solid var(--border);
}
.msg.ok {
  border-color: var(--green);
}
.empty {
  color: var(--text-2);
  text-align: center;
  padding: 80px 0;
}
.monitor-card {
  background: var(--bg-1);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 16px;
  margin-bottom: 14px;
}
.m-head {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 12px;
}
.m-title {
  display: flex;
  align-items: center;
  gap: 10px;
}
.m-name {
  font-weight: 600;
  font-size: 15px;
}
.m-id {
  color: var(--text-2);
  font-size: 12px;
  font-family: monospace;
}
.m-state {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 10px;
  background: var(--bg-3);
  color: var(--text-2);
}
.m-state.on {
  background: rgba(74, 222, 128, 0.15);
  color: var(--green);
}
.m-spec {
  color: var(--text-2);
  font-size: 12px;
}
.m-body {
  display: flex;
  gap: 12px;
  align-items: center;
  flex-wrap: wrap;
}
.m-body label {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 240px;
}
.lbl {
  color: var(--text-1);
  font-size: 13px;
  flex-shrink: 0;
}
.m-body select {
  flex: 1;
  min-width: 0;
}
</style>
