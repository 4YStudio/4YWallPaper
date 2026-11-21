use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const APP_DIR_NAME: &str = "4ywallpaper";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// linux-wallpaperengine 可执行文件路径
    pub engine_path: Option<String>,
    /// Wallpaper Engine 官方 assets 目录（可选）
    pub assets_dir: Option<String>,
    /// 音量 0-100，None = 引擎默认 (15/128)
    pub volume: Option<u32>,
    /// 帧率上限
    pub fps: Option<u32>,
    /// 静音
    pub mute: bool,
    /// 其他应用播放声音时自动静音
    pub automute: bool,
    /// 禁用鼠标交互
    pub disable_mouse: bool,
    /// 禁用视差效果
    pub disable_parallax: bool,
    /// 全屏应用运行时暂停壁纸
    pub fullscreen_pause: bool,
    /// 默认缩放模式
    pub scaling: String,
    /// 开机自启
    pub autostart: bool,
    /// 渲染后端："plasma" = KDE 原生壁纸插件（默认，图标文字可见）；
    /// "engine" = linux-wallpaperengine layer-shell（支持 scene/web 壁纸，但会盖住桌面图标）
    pub backend: String,
    /// 应用后是否自动重启 plasmashell（使 QML 缓存失效）
    pub restart_shell: bool,
    /// 外部壁纸目录：直接引用这些目录里的视频 / project.json 文件夹，不复制进库
    pub source_dirs: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            engine_path: None,
            assets_dir: None,
            volume: None,
            fps: None,
            mute: false,
            automute: true,
            disable_mouse: true,
            disable_parallax: false,
            fullscreen_pause: true,
            scaling: "default".into(),
            autostart: false,
            backend: "plasma".into(),
            restart_shell: true,
            source_dirs: Vec::new(),
        }
    }
}

impl Settings {
    /// plasma 后端是否启用
    pub fn use_plasma(&self) -> bool {
        self.backend != "engine"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MonitorAssignment {
    pub monitor: String,
    pub wallpaper_id: String,
    pub scaling: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub settings: Settings,
    pub assignments: Vec<MonitorAssignment>,
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join(APP_DIR_NAME)
        .join("config.json")
}

pub fn library_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from(".local/share"))
        .join(APP_DIR_NAME)
        .join("library")
}

/// 外部壁纸缩略图缓存目录（不污染用户的原目录）
pub fn thumbs_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from(".local/share"))
        .join(APP_DIR_NAME)
        .join("thumbs")
}

/// 引擎日志目录（引擎进程自己写文件，GUI 退出后不影响引擎）
pub fn logs_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from(".local/share"))
        .join(APP_DIR_NAME)
        .join("logs")
}

pub fn load() -> AppConfig {
    let path = config_path();
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<AppConfig>(&content) {
                Ok(cfg) => cfg,
                Err(e) => {
                    // 不要静默丢掉用户配置：把损坏的文件备份下来，方便排查/恢复
                    let backup = path.with_extension("json.broken");
                    let _ = fs::copy(&path, &backup);
                    eprintln!(
                        "4ywallpaper: 配置文件解析失败 ({e})，已备份到 {}，本次使用默认配置",
                        backup.display()
                    );
                    AppConfig::default()
                }
            },
            Err(_) => AppConfig::default(),
        }
    } else {
        AppConfig::default()
    }
}

pub fn save(cfg: &AppConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}
