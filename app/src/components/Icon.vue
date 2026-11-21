<script setup lang="ts">
import type { IconName } from "./icon-names";

/**
 * 统一的矢量图标组件（stroke = currentColor）。
 * 不用 emoji 做图标：emoji 在不同字体/发行版下渲染不一致，也无法跟随主题色。
 */
withDefaults(
  defineProps<{
    name: IconName;
    /** 像素尺寸 */
    size?: number | string;
    /** 描边宽度 */
    stroke?: number;
  }>(),
  { size: 16, stroke: 1.7 },
);
</script>

<template>
  <svg
    class="icon"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    :stroke-width="stroke"
    stroke="currentColor"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <!-- 壁纸：画框 + 山 + 太阳 -->
    <template v-if="name === 'gallery'">
      <rect x="3" y="4" width="18" height="14" rx="2.5" />
      <circle cx="8.2" cy="9" r="1.5" />
      <path d="M4 16.2 9 11l3.4 3.2L15.4 11l4.6 5.2" />
      <path d="M8.5 21h7" />
    </template>
    <!-- 显示器 -->
    <template v-else-if="name === 'monitor'">
      <rect x="2.5" y="4" width="19" height="12.5" rx="2" />
      <path d="M9 20.5h6M12 16.5v4" />
    </template>
    <!-- 设置：滑杆 -->
    <template v-else-if="name === 'settings'">
      <path d="M4 7h9M17 7h3M4 17h3M11 17h9" />
      <circle cx="15" cy="7" r="2.2" />
      <circle cx="9" cy="17" r="2.2" />
    </template>
    <!-- 日志：文档 -->
    <template v-else-if="name === 'logs'">
      <path d="M6 3.5h8.5L19 8v12.5H6z" />
      <path d="M14 3.5V8h4.5" />
      <path d="M9 12.5h6M9 16h4" />
    </template>
    <!-- 文件夹 -->
    <template v-else-if="name === 'folder'">
      <path d="M3.5 7.5A2 2 0 0 1 5.5 5.5h3.2l2 2.2h7.8a2 2 0 0 1 2 2v7.8a2 2 0 0 1-2 2H5.5a2 2 0 0 1-2-2z" />
    </template>
    <!-- 删除 -->
    <template v-else-if="name === 'trash'">
      <path d="M4 7h16M9.5 7V5h5v2M6.5 7l.8 12.2a1.8 1.8 0 0 0 1.8 1.8h5.8a1.8 1.8 0 0 0 1.8-1.8L17.5 7" />
      <path d="M10.2 11v6M13.8 11v6" />
    </template>
    <!-- 加号 -->
    <template v-else-if="name === 'plus'">
      <path d="M12 5v14M5 12h14" />
    </template>
    <!-- 关闭 -->
    <template v-else-if="name === 'close'">
      <path d="M6 6l12 12M18 6L6 18" />
    </template>
    <!-- 对勾 -->
    <template v-else-if="name === 'check'">
      <path d="M4.5 12.5 9.5 17.5 19.5 6.5" />
    </template>
    <!-- 警告 -->
    <template v-else-if="name === 'alert'">
      <circle cx="12" cy="12" r="8.5" />
      <path d="M12 7.8v5.2M12 16.2v.1" />
    </template>
    <!-- 播放 -->
    <template v-else-if="name === 'play'">
      <path d="M8.5 5.5 18 12l-9.5 6.5z" />
    </template>
    <!-- 停止 -->
    <template v-else-if="name === 'stop'">
      <rect x="6" y="6" width="12" height="12" rx="2" />
    </template>
    <!-- 刷新 -->
    <template v-else-if="name === 'refresh'">
      <path d="M20 12a8 8 0 1 1-2.6-5.9" />
      <path d="M20 4.5V10h-5.3" />
    </template>
    <!-- 外部引用 -->
    <template v-else-if="name === 'external'">
      <path d="M13.5 4.5H19.5v6" />
      <path d="M19.5 4.5 11 13" />
      <path d="M18 14.5v3.7a2 2 0 0 1-2 2H5.8a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h3.7" />
    </template>
    <!-- 搜索 -->
    <template v-else-if="name === 'search'">
      <circle cx="11" cy="11" r="6.5" />
      <path d="M16 16l4 4" />
    </template>
    <!-- 图片占位 -->
    <template v-else-if="name === 'image'">
      <rect x="3.5" y="4.5" width="17" height="15" rx="2.5" />
      <circle cx="8.5" cy="9.5" r="1.6" />
      <path d="M4.5 17 10 11.5l3.5 3.4L16.5 12l4 4.4" />
    </template>
    <!-- 下拉箭头 -->
    <template v-else-if="name === 'chevron-down'">
      <path d="M6 9.5 12 15.5 18 9.5" />
    </template>
  </svg>
</template>

<style scoped>
.icon {
  flex-shrink: 0;
  display: block;
}
</style>
