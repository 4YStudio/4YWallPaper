<script setup lang="ts">
import { ref, onMounted } from "vue";
import { getConfig, saveConfig, setAutostart, detectEngine, openFolder, plasmaStatus, installPlasmaPlugin, restartPlasma } from "../api";
import type { Settings, PlasmaStatus } from "../types";

const settings = ref<Settings | null>(null);
const plasma = ref<PlasmaStatus | null>(null);
const saving = ref(false);
const msg = ref<string | null>(null);

async function refresh() {
  const cfg = await getConfig();
  settings.value = cfg.settings;
  plasma.value = await plasmaStatus();
}

async function onInstallPlugin() {
  try {
    const p = await installPlasmaPlugin();
    plasma.value = await plasmaStatus();
    msg.value = `✅ 插件已安装到 ${p}`;
  } catch (e) {
    msg.value = `❌ 安装失败: ${e}`;
  }
  setTimeout(() => (msg.value = null), 6000);
}

async function onRestartPlasma() {
  msg.value = "正在重启 plasmashell…";
  try {
    await restartPlasma();
    msg.value = "✅ plasmashell 已重启";
  } catch (e) {
    msg.value = `❌ 重启失败: ${e}`;
  }
  setTimeout(() => (msg.value = null), 6000);
}

async function onDetectEngine() {
  const p = await detectEngine();
  if (p && settings.value) {
    settings.value.engine_path = p;
    msg.value = `✅ 已找到引擎: ${p}`;
  } else {
    msg.value = "❌ 未找到 linux-wallpaperengine，请先构建引擎或手动指定路径";
  }
  setTimeout(() => (msg.value = null), 6000);
}

async function onPickEngine() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const p = await open({
    multiple: false,
    title: "选择 linux-wallpaperengine 可执行文件",
  });
  if (typeof p === "string" && settings.value) {
    settings.value.engine_path = p;
  }
}

async function onPickAssets() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const p = await open({
    multiple: false,
    directory: true,
    title: "选择 Wallpaper Engine assets 目录",
  });
  if (typeof p === "string" && settings.value) {
    settings.value.assets_dir = p;
  }
}

async function persist() {
  if (!settings.value) return;
  saving.value = true;
  try {
    await saveConfig({ settings: settings.value, assignments: [] });
    // assignments 传空数组时后端只更新 settings
  } finally {
    saving.value = false;
  }
}

async function onToggleAutostart(enable: boolean) {
  if (!settings.value) return;
  try {
    const ok = await setAutostart(enable);
    settings.value.autostart = ok;
    msg.value = ok === enable ? (enable ? "✅ 已开启开机自启" : "已关闭开机自启") : "⚠️ 设置失败";
  } catch (e) {
    settings.value.autostart = !enable;
    msg.value = `⚠️ 设置失败: ${e}`;
  }
  setTimeout(() => (msg.value = null), 5000);
}

function numOrNull(v: string): number | null {
  if (v === "" || v === null) return null;
  const n = Number(v);
  return Number.isFinite(n) ? n : null;
}

onMounted(refresh);
</script>

<template>
  <div v-if="settings">
    <div class="header">
      <h1>设置</h1>
      <button class="primary" :disabled="saving" @click="persist">
        {{ saving ? "保存中…" : "保存设置" }}
      </button>
    </div>
    <p v-if="msg" class="msg">{{ msg }}</p>

    <section>
      <h2>壁纸后端</h2>
      <div class="row">
        <span class="lbl">渲染方式</span>
        <select v-model="settings.backend">
          <option value="plasma">KDE 原生壁纸插件（推荐：桌面图标文字正常）</option>
          <option value="engine">layer-shell 引擎（仅 scene/web 壁纸需要，会遮住桌面图标）</option>
        </select>
      </div>
      <div class="row" v-if="settings.backend === 'plasma'">
        <label class="check">
          <input
            type="checkbox"
            :checked="settings.restart_shell"
            @change="settings.restart_shell = ($event.target as HTMLInputElement).checked"
          />
          应用壁纸后自动重启 plasmashell（否则插件 QML 缓存不会刷新）
        </label>
      </div>
      <div class="row" v-if="settings.backend === 'plasma'">
        <span class="lbl">插件状态</span>
        <span class="hint">
          {{ plasma?.installed ? "已安装" : "未安装" }} · {{ plasma?.plugin_dir }}
          <template v-if="plasma?.active_screens.length">
            · 运行中: {{ plasma.active_screens.join(", ") }}
          </template>
        </span>
        <button @click="onInstallPlugin">安装/修复插件</button>
        <button @click="onRestartPlasma">重启 plasmashell</button>
      </div>
    </section>

    <section>
      <h2>引擎</h2>
      <div class="row">
        <span class="lbl">引擎路径</span>
        <input
          type="text"
          v-model="settings.engine_path"
          placeholder="linux-wallpaperengine 可执行文件路径"
          class="grow"
        />
        <button @click="onDetectEngine">自动检测</button>
        <button @click="onPickEngine">浏览…</button>
      </div>
      <div class="row">
        <span class="lbl">WE assets 目录</span>
        <input
          type="text"
          v-model="settings.assets_dir"
          placeholder="（可选）Wallpaper Engine 的 assets 文件夹"
          class="grow"
        />
        <button @click="onPickAssets">浏览…</button>
      </div>
    </section>

    <section>
      <h2>播放</h2>
      <div class="row">
        <span class="lbl">音量 (0-100)</span>
        <input
          type="number"
          min="0"
          max="100"
          :value="settings.volume ?? ''"
          placeholder="留空 = 引擎默认音量"
          @input="settings.volume = numOrNull(($event.target as HTMLInputElement).value)"
        />
        <label class="check">
          <input
            type="checkbox"
            :checked="settings.mute"
            @change="settings.mute = ($event.target as HTMLInputElement).checked"
          />
          静音
        </label>
        <label class="check">
          <input
            type="checkbox"
            :checked="settings.automute"
            @change="settings.automute = ($event.target as HTMLInputElement).checked"
          />
          其他应用出声时自动静音
        </label>
      </div>
      <div class="row">
        <span class="lbl">帧率上限</span>
        <input
          type="number"
          min="1"
          :value="settings.fps ?? ''"
          placeholder="留空 = 不限制"
          @input="settings.fps = numOrNull(($event.target as HTMLInputElement).value)"
        />
        <span class="lbl">缩放模式</span>
        <select v-model="settings.scaling">
          <option value="default">默认</option>
          <option value="stretch">拉伸</option>
          <option value="fit">适应</option>
          <option value="fill">填充</option>
        </select>
      </div>
    </section>

    <section>
      <h2>行为</h2>
      <div class="row">
        <label class="check">
          <input
            type="checkbox"
            :checked="settings.disable_mouse"
            @change="settings.disable_mouse = ($event.target as HTMLInputElement).checked"
          />
          禁用鼠标交互
        </label>
        <label class="check">
          <input
            type="checkbox"
            :checked="settings.disable_parallax"
            @change="settings.disable_parallax = ($event.target as HTMLInputElement).checked"
          />
          禁用视差效果
        </label>
        <label class="check">
          <input
            type="checkbox"
            :checked="settings.fullscreen_pause"
            @change="settings.fullscreen_pause = ($event.target as HTMLInputElement).checked"
          />
          全屏应用时暂停壁纸
        </label>
      </div>
    </section>

    <section>
      <h2>系统</h2>
      <div class="row">
        <label class="check">
          <input
            type="checkbox"
            :checked="settings.autostart"
            @change="onToggleAutostart(($event.target as HTMLInputElement).checked)"
          />
          开机自启（systemd 用户服务，登录桌面后自动恢复壁纸）
        </label>
      </div>
      <div class="row">
        <button @click="openFolder('~/.local/share/4ywallpaper/library')">打开壁纸库目录</button>
      </div>
    </section>
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
h2 {
  font-size: 14px;
  color: var(--accent-hover);
  margin-bottom: 12px;
  font-weight: 600;
}
section {
  background: var(--bg-1);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 16px;
  margin-bottom: 16px;
}
.row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
  flex-wrap: wrap;
}
.row:last-child {
  margin-bottom: 0;
}
.lbl {
  color: var(--text-1);
  font-size: 13px;
  flex-shrink: 0;
}
.grow {
  flex: 1;
  min-width: 200px;
}
.check {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  color: var(--text-0);
}
.check input {
  accent-color: var(--accent);
  width: 15px;
  height: 15px;
}
.msg {
  margin-bottom: 12px;
  padding: 8px 12px;
  border-radius: 6px;
  background: var(--bg-2);
  border: 1px solid var(--border);
}
.hint {
  color: var(--text-2);
  font-size: 12px;
  font-family: monospace;
  word-break: break-all;
}
input[type="number"] {
  width: 110px;
}
</style>
