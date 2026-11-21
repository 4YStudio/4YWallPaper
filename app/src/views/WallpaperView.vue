<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  listWallpapers,
  listMonitors,
  listSourceDirs,
  importVideo,
  importFolder,
  deleteWallpaper,
  openFolder,
  addSourceDir,
  removeSourceDir,
  getConfig,
  applyWallpapers,
  stopMonitor,
  plasmaStatus,
} from "../api";
import type { Monitor, MonitorAssignment, WallpaperEntry } from "../types";
import Icon from "../components/Icon.vue";

const emit = defineEmits<{ (e: "refresh-status"): void }>();

const wallpapers = ref<WallpaperEntry[]>([]);
const monitors = ref<Monitor[]>([]);
const sourceDirs = ref<string[]>([]);
const assignments = ref<MonitorAssignment[]>([]);
const activeScreens = ref<string[]>([]);

const loading = ref(false);
const busy = ref(false);
const applying = ref<string | null>(null);
const msg = ref<string | null>(null);
const msgKind = ref<"ok" | "err">("ok");
const preview = ref<string | null>(null);
const filter = ref("");

/** 当前选中的「目标显示器」：点选上方显示器卡片切换 */
const target = ref<string | null>(null);

const scalingOptions = [
  { value: "default", label: "默认" },
  { value: "stretch", label: "拉伸" },
  { value: "fit", label: "适应" },
  { value: "fill", label: "填充" },
];

const visibleWallpapers = computed(() => {
  const q = filter.value.trim().toLowerCase();
  if (!q) return wallpapers.value;
  return wallpapers.value.filter((w) => w.title.toLowerCase().includes(q));
});

function assignmentOf(monitorId: string): MonitorAssignment | undefined {
  return assignments.value.find((a) => a.monitor === monitorId);
}

/** 某张壁纸正在被哪些显示器使用 */
function monitorsUsing(wallpaperId: string): string[] {
  return assignments.value
    .filter((a) => a.wallpaper_id === wallpaperId)
    .map((a) => a.monitor);
}

function wallpaperById(id: string): WallpaperEntry | undefined {
  return wallpapers.value.find((w) => w.id === id);
}

function isRunning(monitorId: string): boolean {
  return activeScreens.value.includes(monitorId);
}

function fmtSize(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function assetUrl(path: string): string {
  return convertFileSrc(path);
}

function dirOf(p: string): string {
  const i = p.lastIndexOf("/");
  return i > 0 ? p.slice(0, i) : p;
}

function flash(text: string, kind: "ok" | "err" = "ok") {
  msg.value = text;
  msgKind.value = kind;
  window.setTimeout(() => {
    if (msg.value === text) msg.value = null;
  }, 5000);
}

async function refresh() {
  const [ws, ms, dirs, cfg, st] = await Promise.all([
    listWallpapers(),
    listMonitors(),
    listSourceDirs(),
    getConfig(),
    plasmaStatus(),
  ]);
  wallpapers.value = ws;
  monitors.value = ms.filter((m) => m.enabled && m.connected !== false);
  sourceDirs.value = dirs;
  assignments.value = cfg.assignments;
  activeScreens.value = st.active_screens;
  // 目标显示器默认取第一个（或上次仍在列表里的那个）
  if (!target.value || !monitors.value.some((m) => m.id === target.value)) {
    target.value = monitors.value[0]?.id ?? null;
  }
}

/** 把 assignments 写回后端并立即应用到桌面 */
async function commit(notice: string) {
  const valid = assignments.value.filter((a) => a.wallpaper_id);
  if (valid.length === 0) {
    flash("请先为至少一个显示器选择壁纸", "err");
    return;
  }
  applying.value = target.value;
  try {
    await applyWallpapers(
      valid.map((a) => ({
        monitor: a.monitor,
        wallpaperId: a.wallpaper_id,
        scaling: a.scaling,
      })),
    );
    flash(notice);
    // plasmashell 重启约 2 秒，等它起来再刷新状态
    await new Promise((r) => setTimeout(r, 3200));
    await refresh();
    emit("refresh-status");
  } catch (e) {
    flash(`应用失败: ${e}`, "err");
  } finally {
    applying.value = null;
  }
}

/** 点卡片上的「设为壁纸」：直接写给目标显示器并立刻应用 */
async function setWallpaper(w: WallpaperEntry) {
  const mon = target.value;
  if (!mon) {
    flash("请先在上方选择一块显示器", "err");
    return;
  }
  const existing = assignmentOf(mon);
  if (existing) {
    existing.wallpaper_id = w.id;
  } else {
    assignments.value.push({ monitor: mon, wallpaper_id: w.id, scaling: "default" });
  }
  await commit(`已将「${w.title}」设为 ${mon} 的壁纸`);
}

async function setScaling(monitorId: string, scaling: string) {
  const a = assignmentOf(monitorId);
  if (!a) {
    flash("这块显示器还没有壁纸", "err");
    return;
  }
  a.scaling = scaling as MonitorAssignment["scaling"];
  await commit(`已更新 ${monitorId} 的缩放模式`);
}

async function onStop(monitorId: string) {
  await stopMonitor(monitorId);
  await new Promise((r) => setTimeout(r, 1200));
  await refresh();
  emit("refresh-status");
  flash(`已停止 ${monitorId} 的动态壁纸`);
}

async function onImportVideo() {
  const sel = await open({
    multiple: false,
    filters: [
      {
        name: "视频文件",
        extensions: ["mp4", "webm", "mkv", "mov", "avi", "m4v", "flv", "ts"],
      },
    ],
  });
  if (!sel || typeof sel !== "string") return;
  busy.value = true;
  try {
    await importVideo(sel);
    await refresh();
    emit("refresh-status");
  } catch (e) {
    flash(`导入失败: ${e}`, "err");
  } finally {
    busy.value = false;
  }
}

async function onImportFolder() {
  const sel = await open({ multiple: false, directory: true });
  if (!sel || typeof sel !== "string") return;
  busy.value = true;
  try {
    await importFolder(sel);
    await refresh();
    emit("refresh-status");
  } catch (e) {
    flash(`导入失败: ${e}`, "err");
  } finally {
    busy.value = false;
  }
}

async function onAddSourceDir() {
  const sel = await open({ multiple: false, directory: true });
  if (!sel || typeof sel !== "string") return;
  busy.value = true;
  try {
    await addSourceDir(sel);
    await refresh();
    emit("refresh-status");
  } catch (e) {
    flash(`添加目录失败: ${e}`, "err");
  } finally {
    busy.value = false;
  }
}

async function onRemoveSourceDir(dir: string) {
  if (!confirm(`取消引用目录「${dir}」？\n只移除列表里的壁纸条目，不会删除任何文件。`))
    return;
  await removeSourceDir(dir);
  await refresh();
  emit("refresh-status");
}

async function onOpenDir(w: WallpaperEntry) {
  await openFolder(w.type === "video" && w.video ? dirOf(w.video) : w.path);
}

async function onDelete(w: WallpaperEntry) {
  if (w.source === "external") {
    flash("这是外部目录中的壁纸。请用「移除壁纸目录」取消引用，文件不会被删除。", "err");
    return;
  }
  if (!confirm(`确定要删除壁纸「${w.title}」吗？此操作不可恢复。`)) return;
  await deleteWallpaper(w.id);
  await refresh();
}

const typeLabel: Record<string, string> = {
  scene: "场景",
  video: "视频",
  web: "网页",
  folder: "文件夹",
};

onMounted(refresh);
</script>

<template>
  <div>
    <div class="header">
      <h1>壁纸</h1>
      <div class="actions">
        <button :disabled="busy" @click="onImportVideo">
          <Icon name="plus" :size="14" />
          {{ busy ? "导入中…" : "导入视频" }}
        </button>
        <button :disabled="busy" @click="onImportFolder">
          <Icon name="folder" :size="14" /> 导入文件夹
        </button>
        <button :disabled="busy" @click="onAddSourceDir">
          <Icon name="external" :size="14" /> 引用目录
        </button>
      </div>
    </div>

    <p v-if="msg" class="msg" :class="msgKind">
      <Icon :name="msgKind === 'ok' ? 'check' : 'alert'" :size="14" />
      <span>{{ msg }}</span>
    </p>

    <!-- ① 显示器：选目标屏 + 当前壁纸 + 缩放 -->
    <section class="panel">
      <div class="panel-head">
        <h2><Icon name="monitor" :size="15" /> 显示器</h2>
        <span class="panel-hint">
          先点选一块屏，再在下方点壁纸卡片上的「设为壁纸」
        </span>
      </div>
      <div v-if="monitors.length === 0" class="empty-line">
        未检测到显示器（需要 kscreen-doctor，KDE Plasma 自带）
      </div>
      <div v-else class="monitor-row">
        <div
          v-for="m in monitors"
          :key="m.id"
          class="monitor-card"
          :class="{ target: target === m.id }"
          @click="target = m.id"
        >
          <div class="m-top">
            <span class="m-name">{{ m.name || m.id }}</span>
            <span class="m-state" :class="{ on: isRunning(m.id) }">
              {{ isRunning(m.id) ? "运行中" : "未运行" }}
            </span>
          </div>
          <div class="m-spec">
            {{ m.width }}×{{ m.height }} @ {{ m.refresh }}Hz
          </div>

          <div class="m-current">
            <template v-if="assignmentOf(m.id)?.wallpaper_id">
              <div class="m-thumb">
                <img
                  v-if="wallpaperById(assignmentOf(m.id)!.wallpaper_id)?.thumb"
                  :src="assetUrl(wallpaperById(assignmentOf(m.id)!.wallpaper_id)!.thumb!)"
                  alt=""
                />
                <div v-else class="m-thumb-empty"><Icon name="image" :size="16" /></div>
              </div>
              <div class="m-current-text">
                <span class="m-current-title">
                  {{
                    wallpaperById(assignmentOf(m.id)!.wallpaper_id)?.title ??
                    assignmentOf(m.id)!.wallpaper_id
                  }}
                </span>
                <span class="m-current-sub">
                  缩放：{{ scalingOptions.find((o) => o.value === assignmentOf(m.id)!.scaling)?.label }}
                </span>
              </div>
            </template>
            <div v-else class="m-current-text muted">未设置壁纸</div>
          </div>

          <div class="m-actions" @click.stop>
            <select
              :value="assignmentOf(m.id)?.scaling ?? 'default'"
              :disabled="!assignmentOf(m.id)?.wallpaper_id"
              @change="setScaling(m.id, ($event.target as HTMLSelectElement).value)"
            >
              <option v-for="o in scalingOptions" :key="o.value" :value="o.value">
                {{ o.label }}
              </option>
            </select>
            <button
              class="danger"
              :disabled="!isRunning(m.id)"
              @click="onStop(m.id)"
            >
              <Icon name="stop" :size="13" /> 停止
            </button>
          </div>

          <div v-if="target === m.id" class="m-target-badge">壁纸目标</div>
        </div>
      </div>
    </section>

    <!-- ② 壁纸库 -->
    <section class="panel">
      <div class="panel-head">
        <h2><Icon name="gallery" :size="15" /> 壁纸库（{{ wallpapers.length }}）</h2>
        <div class="search">
          <Icon name="search" :size="13" />
          <input v-model="filter" type="text" placeholder="搜索壁纸…" />
        </div>
      </div>

      <div v-if="sourceDirs.length" class="sources">
        <span class="sources-label">引用的目录：</span>
        <span v-for="d in sourceDirs" :key="d" class="source-chip">
          <span class="source-path" :title="d">{{ d }}</span>
          <button class="chip-x" title="取消引用" @click="onRemoveSourceDir(d)">
            <Icon name="close" :size="11" />
          </button>
        </span>
      </div>

      <div v-if="loading" class="empty">加载中…</div>
      <div v-else-if="visibleWallpapers.length === 0" class="empty">
        {{
          wallpapers.length === 0
            ? "壁纸库为空，点右上角「导入视频」或「引用目录」开始"
            : "没有匹配的壁纸"
        }}
      </div>

      <div v-else class="grid">
        <div
          v-for="w in visibleWallpapers"
          :key="w.id"
          class="card"
          :class="{ used: monitorsUsing(w.id).length > 0 }"
        >
          <div class="thumb" @dblclick="w.thumb && (preview = w.thumb)">
            <img v-if="w.thumb" :src="assetUrl(w.thumb)" alt="" />
            <div v-else class="thumb-placeholder">
              <Icon name="image" :size="22" />
            </div>
            <span class="badge">{{ typeLabel[w.type] ?? w.type }}</span>
            <span v-if="w.source === 'external'" class="badge-ext">外部</span>
            <span v-if="monitorsUsing(w.id).length" class="badge-used">
              {{ monitorsUsing(w.id).join(" · ") }}
            </span>
          </div>
          <div class="info">
            <div class="title" :title="w.title">{{ w.title }}</div>
            <div class="meta">{{ fmtSize(w.size) }}</div>
          </div>
          <div class="card-actions">
            <button
              class="primary set-btn"
              :disabled="!target || applying !== null"
              @click="setWallpaper(w)"
            >
              <Icon name="play" :size="12" />
              {{ target ? `设为 ${target}` : "先选显示器" }}
            </button>
            <button class="icon-btn" title="打开目录" @click="onOpenDir(w)">
              <Icon name="folder" :size="14" />
            </button>
            <button
              class="icon-btn danger-icon"
              title="删除"
              :disabled="w.source === 'external'"
              @click="onDelete(w)"
            >
              <Icon name="trash" :size="14" />
            </button>
          </div>
        </div>
      </div>
    </section>

    <div v-if="preview" class="preview-mask" @click="preview = null">
      <img :src="assetUrl(preview)" alt="" />
    </div>
  </div>
</template>

<style scoped>
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}
h1 {
  font-size: 20px;
  font-weight: 600;
}
h2 {
  font-size: 14px;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 7px;
  color: var(--accent-hover);
}
.actions {
  display: flex;
  gap: 8px;
}
.actions button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.msg {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  padding: 8px 12px;
  border-radius: 6px;
  background: var(--bg-2);
  border: 1px solid var(--green);
  font-size: 13px;
}
.msg.err {
  border-color: var(--red);
}
.msg.err :deep(.icon) {
  color: var(--red);
}
.msg :deep(.icon) {
  color: var(--green);
}

.panel {
  background: var(--bg-1);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 14px 16px 16px;
  margin-bottom: 16px;
}
.panel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
  gap: 12px;
}
.panel-hint {
  color: var(--text-2);
  font-size: 12px;
}
.search {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--bg-2);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 0 8px;
  color: var(--text-2);
}
.search input {
  background: transparent;
  border: none;
  outline: none;
  color: var(--text-0);
  font: inherit;
  font-size: 13px;
  padding: 6px 0;
  width: 160px;
  user-select: text;
}
.empty-line {
  color: var(--text-2);
  font-size: 13px;
}
.empty {
  color: var(--text-2);
  text-align: center;
  padding: 50px 0;
}

/* ---- 显示器 ---- */
.monitor-row {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 12px;
}
.monitor-card {
  position: relative;
  border: 1px solid var(--border);
  border-radius: 9px;
  background: var(--bg-2);
  padding: 12px;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
}
.monitor-card:hover {
  border-color: var(--accent);
}
.monitor-card.target {
  border-color: var(--accent);
  background: var(--bg-3);
  box-shadow: 0 0 0 1px var(--accent) inset;
}
.m-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.m-name {
  font-weight: 600;
  font-size: 14px;
}
.m-state {
  font-size: 11px;
  padding: 1px 8px;
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
  margin: 2px 0 10px;
}
.m-current {
  display: flex;
  align-items: center;
  gap: 9px;
  min-height: 44px;
}
.m-thumb {
  width: 74px;
  height: 42px;
  border-radius: 5px;
  overflow: hidden;
  background: var(--bg-1);
  flex-shrink: 0;
}
.m-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.m-thumb-empty {
  width: 100%;
  height: 100%;
  display: grid;
  place-items: center;
  color: var(--text-2);
}
.m-current-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.m-current-text.muted {
  color: var(--text-2);
  font-size: 13px;
}
.m-current-title {
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.m-current-sub {
  font-size: 11px;
  color: var(--text-2);
}
.m-actions {
  display: flex;
  gap: 8px;
  margin-top: 10px;
  cursor: default;
}
.m-actions select {
  flex: 1;
  min-width: 0;
  padding: 4px 8px;
  font-size: 12px;
}
.m-actions button {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  padding: 4px 10px;
}
.m-target-badge {
  position: absolute;
  top: -8px;
  right: 10px;
  background: var(--accent);
  color: #fff;
  font-size: 10px;
  padding: 1px 8px;
  border-radius: 8px;
}

/* ---- 目录 chips ---- */
.sources {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 12px;
  font-size: 12px;
}
.sources-label {
  color: var(--text-2);
}
.source-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  max-width: 440px;
  padding: 2px 3px 2px 10px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-2);
}
.source-path {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: monospace;
  color: var(--text-2);
}
.chip-x {
  display: grid;
  place-items: center;
  padding: 3px;
  background: transparent;
  border: none;
  color: var(--text-2);
  cursor: pointer;
  border-radius: 50%;
}
.chip-x:hover {
  color: var(--red);
  background: var(--bg-3);
}

/* ---- 壁纸卡片 ---- */
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(215px, 1fr));
  gap: 13px;
}
.card {
  background: var(--bg-2);
  border: 1px solid var(--border);
  border-radius: 10px;
  overflow: hidden;
  transition: border-color 0.15s, transform 0.15s;
}
.card:hover {
  border-color: var(--accent);
  transform: translateY(-2px);
}
.card.used {
  border-color: rgba(108, 123, 255, 0.55);
}
.thumb {
  position: relative;
  aspect-ratio: 16 / 9;
  background: var(--bg-1);
}
.thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.thumb-placeholder {
  width: 100%;
  height: 100%;
  display: grid;
  place-items: center;
  color: var(--text-2);
}
.badge {
  position: absolute;
  top: 7px;
  right: 7px;
  background: rgba(18, 20, 28, 0.82);
  border: 1px solid var(--border);
  color: var(--text-1);
  font-size: 10px;
  padding: 1px 7px;
  border-radius: 10px;
}
.badge-ext {
  position: absolute;
  top: 7px;
  left: 7px;
  background: rgba(56, 130, 246, 0.85);
  color: #fff;
  font-size: 10px;
  padding: 1px 7px;
  border-radius: 10px;
}
.badge-used {
  position: absolute;
  bottom: 7px;
  left: 7px;
  background: rgba(108, 123, 255, 0.9);
  color: #fff;
  font-size: 10px;
  padding: 1px 7px;
  border-radius: 10px;
  max-width: calc(100% - 14px);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.info {
  padding: 9px 11px 5px;
}
.title {
  font-weight: 500;
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.meta {
  color: var(--text-2);
  font-size: 11px;
  margin-top: 2px;
}
.card-actions {
  display: flex;
  gap: 6px;
  padding: 7px 11px 11px;
}
.set-btn {
  flex: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 5px;
  padding: 5px 0;
  font-size: 12px;
  min-width: 0;
  overflow: hidden;
  white-space: nowrap;
}
.icon-btn {
  display: grid;
  place-items: center;
  width: 30px;
  padding: 0;
  flex-shrink: 0;
}
.danger-icon:hover {
  border-color: var(--red);
  color: var(--red);
}
.preview-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.78);
  display: grid;
  place-items: center;
  z-index: 100;
  cursor: zoom-out;
}
.preview-mask img {
  max-width: 90%;
  max-height: 90%;
  border-radius: 8px;
}
</style>
