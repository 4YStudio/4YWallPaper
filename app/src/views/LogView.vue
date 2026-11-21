<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from "vue";
import { recentLogs, clearLogs } from "../api";
import type { LogLine } from "../types";

const logs = ref<LogLine[]>([]);
const filter = ref("");
const autoScroll = ref(true);
const container = ref<HTMLElement | null>(null);
let timer: number | undefined;

async function refresh() {
  try {
    const all = await recentLogs(500);
    logs.value = all;
    if (autoScroll.value) {
      await nextTick();
      container.value?.scrollTo({ top: container.value.scrollHeight });
    }
  } catch {
    /* ignore */
  }
}

function fmtTs(ts: number): string {
  const d = new Date(ts * 1000);
  return d.toLocaleTimeString("zh-CN", { hour12: false });
}

onMounted(() => {
  refresh();
  timer = window.setInterval(refresh, 1500);
});
onUnmounted(() => window.clearInterval(timer));
</script>

<template>
  <div class="log-page">
    <div class="header">
      <h1>引擎日志</h1>
      <div class="actions">
        <label class="check">
          <input type="checkbox" v-model="autoScroll" />
          自动滚动
        </label>
        <button @click="clearLogs">清空</button>
      </div>
    </div>
    <input
      v-model="filter"
      type="text"
      placeholder="过滤日志内容…"
      class="filter"
    />
    <div class="log-box" ref="container">
      <div
        v-for="(l, i) in logs.filter(
          (l) => !filter || l.text.toLowerCase().includes(filter.toLowerCase()),
        )"
        :key="i"
        class="line"
        :class="l.stream"
      >
        <span class="ts">{{ fmtTs(l.ts) }}</span>
        <span class="text">{{ l.text }}</span>
      </div>
      <div v-if="logs.length === 0" class="empty">暂无日志</div>
    </div>
  </div>
</template>

<style scoped>
.log-page {
  display: flex;
  flex-direction: column;
  height: 100%;
}
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}
h1 {
  font-size: 20px;
  font-weight: 600;
}
.actions {
  display: flex;
  align-items: center;
  gap: 10px;
}
.check {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  font-size: 13px;
  color: var(--text-1);
}
.check input {
  accent-color: var(--accent);
}
.filter {
  margin-bottom: 10px;
  width: 320px;
}
.log-box {
  flex: 1;
  overflow: auto;
  background: var(--bg-1);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 12px;
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  font-size: 12px;
  line-height: 1.6;
}
.line {
  display: flex;
  gap: 10px;
}
.line.stderr .text {
  color: var(--red);
}
.line.stdout .text {
  color: var(--text-1);
}
.ts {
  color: var(--text-2);
  flex-shrink: 0;
}
.text {
  white-space: pre-wrap;
  word-break: break-all;
  user-select: text;
}
.empty {
  color: var(--text-2);
  text-align: center;
  padding: 40px 0;
}
</style>
