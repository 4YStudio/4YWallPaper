mod autostart;
mod config;
mod engine;
mod library;
mod monitors;
mod plasma;

use engine::LogBuffer;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
fn list_wallpapers() -> Result<Vec<library::WallpaperEntry>, String> {
    library::list_wallpapers()
}

#[tauri::command]
fn import_video(path: String) -> Result<library::WallpaperEntry, String> {
    library::import_video(&path)
}

#[tauri::command]
fn import_folder(path: String) -> Result<library::WallpaperEntry, String> {
    library::import_folder(&path)
}

#[tauri::command]
fn delete_wallpaper(id: String) -> Result<(), String> {
    library::delete_wallpaper(&id)
}

/// 登记一个外部壁纸目录（直接引用，不复制文件）
#[tauri::command]
fn add_source_dir(path: String) -> Result<Vec<String>, String> {
    library::add_source_dir(&path)
}

/// 取消引用外部壁纸目录
#[tauri::command]
fn remove_source_dir(path: String) -> Result<Vec<String>, String> {
    library::remove_source_dir(&path)
}

/// 当前已引用的外部壁纸目录
#[tauri::command]
fn list_source_dirs() -> Vec<String> {
    config::load().settings.source_dirs
}

#[tauri::command]
fn list_monitors() -> Result<Vec<monitors::KScreenOutput>, String> {
    match monitors::list_monitors() {
        Ok(list) if !list.is_empty() => Ok(list),
        _ => Ok(monitors::list_monitors_fallback()),
    }
}

#[tauri::command]
fn get_config() -> config::AppConfig {
    config::load()
}

#[tauri::command]
fn save_config(config: config::AppConfig) -> Result<(), String> {
    // 传空的 assignments 视为"只更新设置"，保留现有分配
    if config.assignments.is_empty() {
        let mut merged = config::load();
        merged.settings = config.settings;
        config::save(&merged)
    } else {
        config::save(&config)
    }
}

#[tauri::command]
fn apply_wallpapers(
    assignments: Vec<MonitorAssignmentInput>,
    state: State<Arc<LogBuffer>>,
) -> Result<(), String> {
    let mut cfg = config::load();
    let mut new_assignments = Vec::new();
    for a in assignments {
        if !monitors::is_valid_monitor_id(&a.monitor) {
            return Err(format!("非法显示器 ID: {}", a.monitor));
        }
        new_assignments.push(config::MonitorAssignment {
            monitor: a.monitor,
            wallpaper_id: a.wallpaper_id,
            scaling: a.scaling,
        });
    }
    cfg.assignments = new_assignments;
    config::save(&cfg)?;

    // plasma 后端：写 KDE 壁纸插件配置（视频由 plasmashell 自己绘制，图标文字不受影响）
    if cfg.settings.use_plasma() {
        // 先停掉可能残留的 layer-shell 引擎，否则它会盖住插件画的壁纸
        engine::stop_all_engines();
        let out = plasma::apply(&cfg)?;
        engine::push_log(
            state.inner(),
            "stdout",
            format!("[plasma] {out}"),
        );
        if cfg.settings.restart_shell {
            // QML 缓存会导致插件不重新加载，必须重启 plasmashell
            if let Err(e) = plasma::restart_plasmashell() {
                engine::push_log(state.inner(), "stderr", format!("[plasma] {e}"));
            }
        }
        return Ok(());
    }

    engine::spawn_engine(&cfg.settings, &cfg.assignments, &state.inner())
}

#[derive(serde::Deserialize)]
pub struct MonitorAssignmentInput {
    pub monitor: String,
    #[serde(rename = "wallpaperId")]
    pub wallpaper_id: String,
    pub scaling: String,
}

#[tauri::command]
fn stop_all_engines() {
    // 两种后端都要停：layer-shell 引擎进程 + Plasma 插件壁纸
    engine::stop_all_engines();
    let mut cfg = config::load();
    if cfg.settings.use_plasma() {
        for a in &cfg.assignments {
            let _ = plasma::clear_screen(&a.monitor);
        }
        // 同时清掉配置里的分配：否则下次给别的屏设壁纸时，
        // 这里被停掉的屏会被重新写入并跟着启动
        cfg.assignments.clear();
        let _ = config::save(&cfg);
    }
}

#[tauri::command]
fn stop_engine(monitor: String) {
    engine::stop_engine_by_monitor(&monitor);
    let _ = plasma::clear_screen(&monitor);
    // 同上：把这块屏的分配从配置里移除，避免下次 apply 时被顺带唤醒
    let mut cfg = config::load();
    let before = cfg.assignments.len();
    cfg.assignments.retain(|a| a.monitor != monitor);
    if cfg.assignments.len() != before {
        let _ = config::save(&cfg);
    }
}

#[tauri::command]
fn engine_status() -> Vec<engine::EngineProcessInfo> {
    engine::find_engine_processes()
}

#[tauri::command]
fn recent_logs(limit: usize, state: State<Arc<LogBuffer>>) -> Vec<engine::LogLine> {
    let q = state.inner().0.lock().unwrap_or_else(|e| e.into_inner());
    let start = q.len().saturating_sub(limit);
    q.iter().skip(start).cloned().collect()
}

/// 引擎日志文件路径（方便用户直接查看完整日志）
#[tauri::command]
fn engine_log_file() -> String {
    engine::engine_log_path().to_string_lossy().to_string()
}

#[tauri::command]
fn clear_logs(state: State<Arc<LogBuffer>>) {
    if let Ok(mut q) = state.inner().0.lock() {
        q.clear();
    }
}

#[tauri::command]
fn list_properties(id: String) -> Result<Vec<serde_json::Value>, String> {
    // v1：暂不通过引擎解析自定义属性（需短暂启动引擎，开销大）
    let _ = id;
    Ok(Vec::new())
}

/// 壁纸插件状态：是否已安装、当前哪些屏在用
#[derive(serde::Serialize)]
struct PlasmaStatus {
    installed: bool,
    plugin_dir: String,
    active_screens: Vec<String>,
    error: Option<String>,
}

#[tauri::command]
fn plasma_status() -> PlasmaStatus {
    let installed = plasma::plugin_installed();
    let dir = plasma::user_plugin_dir().to_string_lossy().to_string();
    match plasma::active_desktops() {
        Ok(v) => PlasmaStatus {
            installed,
            plugin_dir: dir,
            active_screens: v,
            error: None,
        },
        Err(e) => PlasmaStatus {
            installed,
            plugin_dir: dir,
            active_screens: Vec::new(),
            error: Some(e),
        },
    }
}

/// 安装/更新 Plasma 壁纸插件（也可用于修复被删掉的插件）
#[tauri::command]
fn install_plasma_plugin() -> Result<String, String> {
    plasma::ensure_plugin_installed().map(|p| p.to_string_lossy().to_string())
}

/// 重启 plasmashell（让插件 QML 变更生效）
#[tauri::command]
fn restart_plasma() -> Result<(), String> {
    plasma::restart_plasmashell()
}

#[tauri::command]
fn set_autostart(enable: bool) -> Result<bool, String> {
    autostart::set_autostart(enable)
}

#[tauri::command]
fn detect_engine_path() -> Option<String> {
    engine::detect_engine_path()
}

#[tauri::command]
fn open_folder(path: String) -> Result<(), String> {
    // 展开 ~ 前缀
    let path = if let Some(rest) = path.strip_prefix("~") {
        if let Some(home) = dirs::home_dir() {
            format!("{}{}", home.display(), rest)
        } else {
            path
        }
    } else {
        path
    };
    std::process::Command::new("xdg-open")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("无法打开目录: {e}"))?;
    Ok(())
}

/// `--list`：以 JSON 打印壁纸库扫描结果（排查「为什么绑定不上」时很有用）
pub fn run_list() {
    match library::list_wallpapers() {
        Ok(list) => match serde_json::to_string_pretty(&list) {
            Ok(s) => println!("{s}"),
            Err(e) => {
                eprintln!("4ywallpaper: 序列化失败: {e}");
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("4ywallpaper: 扫描失败: {e}");
            std::process::exit(1);
        }
    }
}

/// 同步补齐所有缺失的缩略图：`fourywallpaper --thumbs`
pub fn run_thumbs() {
    let made = library::generate_missing_thumbnails();
    println!("已生成 {made} 个缩略图，缓存目录: {}", config::thumbs_dir().display());
}

/// 一次性应用配置（不开 GUI）：`fourywallpaper --apply`
/// 便于脚本调用、排查问题，以及在没有图形界面时重新应用壁纸。
pub fn run_apply_once() {
    let cfg = config::load();
    // 分配为空不再直接报错：plasma 后端需要跑一次才能把残留的屏复位成静态壁纸
    if cfg.assignments.is_empty() && !cfg.settings.use_plasma() {
        eprintln!("4ywallpaper: 没有壁纸分配，请先在 GUI 里选择壁纸");
        std::process::exit(1);
    }
    if cfg.settings.use_plasma() {
        match plasma::apply(&cfg) {
            Ok(out) => {
                println!("4ywallpaper: {out}");
                if cfg.settings.restart_shell {
                    match plasma::restart_plasmashell() {
                        Ok(()) => println!("4ywallpaper: plasmashell 已重启"),
                        Err(e) => eprintln!("4ywallpaper: {e}"),
                    }
                }
            }
            Err(e) => {
                eprintln!("4ywallpaper: 应用失败: {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    let logs = Arc::new(LogBuffer::default());
    match engine::spawn_engine(&cfg.settings, &cfg.assignments, &logs) {
        Ok(()) => println!("4ywallpaper: 引擎已启动"),
        Err(e) => {
            eprintln!("4ywallpaper: 启动失败: {e}");
            std::process::exit(1);
        }
    }
}

/// --autostart 模式入口（由 main.rs 调用）
pub fn run_autostart() {
    engine::run_autostart();
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(LogBuffer::default()))
        .setup(|app| {
            // 把引擎日志文件的尾部载入日志视图，这样重启 GUI 后也能看到历史日志
            use tauri::Manager;
            if let Some(state) = app.try_state::<Arc<LogBuffer>>() {
                engine::load_recent_logs_from_file(state.inner(), 200);
            }
            // 告诉 plasma 模块去哪里找随包分发的壁纸插件
            if let Ok(resource_dir) = app.path().resource_dir() {
                plasma::set_resource_dir(resource_dir);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_wallpapers,
            import_video,
            import_folder,
            delete_wallpaper,
            add_source_dir,
            remove_source_dir,
            list_source_dirs,
            list_monitors,
            get_config,
            save_config,
            apply_wallpapers,
            stop_all_engines,
            stop_engine,
            engine_status,
            recent_logs,
            engine_log_file,
            plasma_status,
            install_plasma_plugin,
            restart_plasma,
            clear_logs,
            list_properties,
            set_autostart,
            detect_engine_path,
            open_folder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
