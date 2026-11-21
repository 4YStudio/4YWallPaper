export interface WallpaperEntry {
  /** 唯一 ID = 库内文件夹名 */
  id: string;
  /** project.json 里的 title，回退到文件夹名 */
  title: string;
  /** scene | video | web */
  type: "scene" | "video" | "web";
  /** 缩略图绝对路径（可能不存在） */
  thumb: string | null;
  /** 壁纸文件夹绝对路径 */
  path: string;
  /** 文件夹大小（字节） */
  size: number;
  /** project.json 支持的自定义属性列表 */
  properties: WallpaperProperty[];
}

export interface WallpaperProperty {
  name: string;
  type: string;
  value: unknown;
  text?: string;
}

export interface Monitor {
  /** kscreen-doctor 输出名，如 eDP-1 */
  id: string;
  /** 显示器友好描述 */
  name: string;
  enabled: boolean;
  connected: boolean;
  /** 分辨率宽 */
  width: number;
  /** 分辨率高 */
  height: number;
  /** 刷新率 */
  refresh: number;
  /** 几何位置 x */
  x: number;
  /** 几何位置 y */
  y: number;
}

export interface Settings {
  /** linux-wallpaperengine 可执行文件路径 */
  engine_path: string | null;
  /** Wallpaper Engine 官方 assets 目录（可选） */
  assets_dir: string | null;
  /** 音量 0-100，null = 跟随系统 */
  volume: number | null;
  /** 限制帧率，null = 不限制 */
  fps: number | null;
  /** 静音 */
  mute: boolean;
  /** 其他应用播放声音时自动静音 */
  automute: boolean;
  /** 禁用鼠标交互 */
  disable_mouse: boolean;
  /** 禁用视差效果 */
  disable_parallax: boolean;
  /** 全屏应用运行时暂停壁纸 */
  fullscreen_pause: boolean;
  /** 默认缩放模式 */
  scaling: "default" | "stretch" | "fit" | "fill";
  /** 开机自启 */
  autostart: boolean;
  /** 渲染后端：plasma = KDE 原生壁纸插件（默认）；engine = layer-shell 引擎 */
  backend: "plasma" | "engine";
  /** 应用后自动重启 plasmashell（使 QML 缓存失效） */
  restart_shell: boolean;
}

export interface MonitorAssignment {
  monitor: string;
  wallpaper_id: string;
  scaling: "default" | "stretch" | "fit" | "fill";
}

export interface AppConfig {
  settings: Settings;
  assignments: MonitorAssignment[];
}

export interface EngineProcessInfo {
  pid: number;
  monitor: string | null;
  wallpaper: string | null;
  cmdline: string;
}

export interface LogLine {
  ts: number;
  stream: "stdout" | "stderr";
  text: string;
}

export interface PlasmaStatus {
  installed: boolean;
  plugin_dir: string;
  /** 当前正在播放动态壁纸的显示器名（如 eDP-1） */
  active_screens: string[];
  error: string | null;
}
