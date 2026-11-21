<script setup lang="ts">
import { ref, onMounted } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  listWallpapers,
  importVideo,
  importFolder,
  deleteWallpaper,
  openFolder,
} from "../api";
import type { WallpaperEntry } from "../types";

const emit = defineEmits<{ (e: "refresh-status"): void }>();

const wallpapers = ref<WallpaperEntry[]>([]);
const loading = ref(false);
const busy = ref(false);
const preview = ref<string | null>(null);

async function refresh() {
  loading.value = true;
  try {
    wallpapers.value = await listWallpapers();
  } finally {
    loading.value = false;
  }
}

function fmtSize(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

async function onImportVideo() {
  const sel = await open({
    multiple: false,
    filters: [
      { name: "视频文件", extensions: ["mp4", "webm", "mkv", "mov", "avi", "m4v", "flv", "ts"] },
    ],
  });
  if (!sel || typeof sel !== "string") return;
  busy.value = true;
  try {
    await importVideo(sel);
    await refresh();
    emit("refresh-status");
  } catch (e) {
    alert(`导入失败: ${e}`);
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
    alert(`导入失败: ${e}`);
  } finally {
    busy.value = false;
  }
}

async function onDelete(w: WallpaperEntry) {
  if (!confirm(`确定要删除壁纸「${w.title}」吗？此操作不可恢复。`)) return;
  await deleteWallpaper(w.id);
  await refresh();
}

function assetUrl(path: string): string {
  return convertFileSrc(path);
}

const typeLabel: Record<string, string> = {
  scene: "场景",
  video: "视频",
  web: "网页",
};

onMounted(refresh);
</script>

<template>
  <div>
    <div class="header">
      <h1>壁纸库</h1>
      <div class="actions">
        <button :disabled="busy" @click="onImportVideo">
          {{ busy ? "导入中…" : "＋ 导入视频壁纸" }}
        </button>
        <button :disabled="busy" @click="onImportFolder">
          ＋ 导入壁纸文件夹
        </button>
      </div>
    </div>
    <p class="hint">
      导入视频壁纸会自动把本地 mp4/webm/mkv 打包成引擎可识别的壁纸文件夹；也可导入包含
      project.json 的 Wallpaper Engine 壁纸文件夹。
    </p>

    <div v-if="loading" class="empty">加载中…</div>
    <div v-else-if="wallpapers.length === 0" class="empty">
      壁纸库为空，点击右上角「导入视频壁纸」开始使用
    </div>

    <div v-else class="grid">
      <div v-for="w in wallpapers" :key="w.id" class="card" @dblclick="preview = w.thumb">
        <div class="thumb">
          <img v-if="w.thumb" :src="assetUrl(w.thumb)" alt="" />
          <div v-else class="thumb-placeholder">{{ typeLabel[w.type] ?? w.type }}</div>
          <span class="badge">{{ typeLabel[w.type] ?? w.type }}</span>
        </div>
        <div class="info">
          <div class="title" :title="w.title">{{ w.title }}</div>
          <div class="meta">{{ fmtSize(w.size) }}</div>
        </div>
        <div class="card-actions">
          <button v-if="w.type === 'video'" @click="openFolder(w.path)">打开目录</button>
          <button class="danger" @click="onDelete(w)">删除</button>
        </div>
      </div>
    </div>

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
  margin-bottom: 8px;
}
h1 {
  font-size: 20px;
  font-weight: 600;
}
.actions {
  display: flex;
  gap: 8px;
}
.hint {
  color: var(--text-2);
  font-size: 12px;
  margin-bottom: 18px;
}
.empty {
  color: var(--text-2);
  text-align: center;
  padding: 80px 0;
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
  gap: 14px;
}
.card {
  background: var(--bg-1);
  border: 1px solid var(--border);
  border-radius: 10px;
  overflow: hidden;
  transition: border-color 0.15s, transform 0.15s;
}
.card:hover {
  border-color: var(--accent);
  transform: translateY(-2px);
}
.thumb {
  position: relative;
  aspect-ratio: 16 / 9;
  background: var(--bg-2);
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
  font-size: 15px;
}
.badge {
  position: absolute;
  top: 8px;
  right: 8px;
  background: rgba(18, 20, 28, 0.8);
  border: 1px solid var(--border);
  color: var(--text-1);
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 10px;
}
.info {
  padding: 10px 12px 6px;
}
.title {
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.meta {
  color: var(--text-2);
  font-size: 12px;
  margin-top: 3px;
}
.card-actions {
  display: flex;
  gap: 6px;
  padding: 8px 12px 12px;
}
.card-actions button {
  flex: 1;
  padding: 5px 0;
  font-size: 12px;
}
.preview-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.75);
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
