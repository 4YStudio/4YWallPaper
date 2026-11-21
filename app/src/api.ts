import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  EngineProcessInfo,
  LogLine,
  Monitor,
  PlasmaStatus,
  WallpaperEntry,
  WallpaperProperty,
} from "./types";

export async function listWallpapers(): Promise<WallpaperEntry[]> {
  return invoke("list_wallpapers");
}

export async function importVideo(path: string): Promise<WallpaperEntry | null> {
  return invoke("import_video", { path });
}

export async function importFolder(path: string): Promise<WallpaperEntry | null> {
  return invoke("import_folder", { path });
}

export async function deleteWallpaper(id: string): Promise<void> {
  return invoke("delete_wallpaper", { id });
}

/** 登记外部壁纸目录（引用，不复制） */
export async function addSourceDir(path: string): Promise<string[]> {
  return invoke("add_source_dir", { path });
}

/** 取消引用外部壁纸目录 */
export async function removeSourceDir(path: string): Promise<string[]> {
  return invoke("remove_source_dir", { path });
}

export async function listSourceDirs(): Promise<string[]> {
  return invoke("list_source_dirs");
}

export async function listMonitors(): Promise<Monitor[]> {
  return invoke("list_monitors");
}

export async function getConfig(): Promise<AppConfig> {
  return invoke("get_config");
}

export async function saveConfig(cfg: AppConfig): Promise<void> {
  return invoke("save_config", { config: cfg });
}

export async function applyWallpapers(
  assignments: { monitor: string; wallpaperId: string; scaling: string }[],
): Promise<void> {
  return invoke("apply_wallpapers", { assignments });
}

export async function stopAll(): Promise<void> {
  return invoke("stop_all_engines");
}

export async function stopMonitor(monitor: string): Promise<void> {
  return invoke("stop_engine", { monitor });
}

export async function engineStatus(): Promise<EngineProcessInfo[]> {
  return invoke("engine_status");
}

export async function recentLogs(limit = 300): Promise<LogLine[]> {
  return invoke("recent_logs", { limit });
}

export async function clearLogs(): Promise<void> {
  return invoke("clear_logs");
}

export async function listProperties(id: string): Promise<WallpaperProperty[]> {
  return invoke("list_properties", { id });
}

export async function setAutostart(enable: boolean): Promise<boolean> {
  return invoke("set_autostart", { enable });
}

export async function detectEngine(): Promise<string | null> {
  return invoke("detect_engine_path");
}

export async function openFolder(path: string): Promise<void> {
  return invoke("open_folder", { path });
}

export async function plasmaStatus(): Promise<PlasmaStatus> {
  return invoke("plasma_status");
}

export async function installPlasmaPlugin(): Promise<string> {
  return invoke("install_plasma_plugin");
}

export async function restartPlasma(): Promise<void> {
  return invoke("restart_plasma");
}
