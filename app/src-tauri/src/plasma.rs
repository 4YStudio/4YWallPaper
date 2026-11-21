//! KDE Plasma 壁纸插件后端。
//!
//! 为什么不用 linux-wallpaperengine 的 layer-shell 渲染：它会在桌面最上层铺一张
//! 不透明 surface，把 plasmashell 自己画的桌面图标和文字盖住（图标文字看不见）。
//! 改成 KDE 原生壁纸插件后，视频由 plasmashell 在 desktop containment 内部绘制，
//! 图标/文字天然在视频之上，而且每屏 DPI 缩放由 Plasma 自己处理，不会错位。
//!
//! 本模块负责：
//! 1. 把随程序分发的 Plasma 插件（org.foury.videowall）安装到
//!    ~/.local/share/plasma/wallpapers/
//! 2. 通过 Plasma 的 D-Bus 脚本接口把「哪块屏播哪个视频」写进桌面配置
//! 3. 重启 plasmashell 让 QML 缓存失效（否则改配置不生效）

use crate::config::{self, AppConfig};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// 插件 ID，必须与 metadata.json 里的 KPlugin.Id 一致
pub const PLUGIN_ID: &str = "org.foury.videowall";

static RESOURCE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 由 lib.rs 在 setup 时传入 Tauri 的资源目录（deb 安装后是 /usr/lib/4YWallPaper）
pub fn set_resource_dir(dir: PathBuf) {
    let _ = RESOURCE_DIR.set(dir);
}

/// 插件相对配置组
fn config_group() -> String {
    format!("\"Wallpaper\", \"{PLUGIN_ID}\", \"General\"")
}

/// 用户级插件安装目录
pub fn user_plugin_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from(".local/share"))
        .join("plasma/wallpapers")
        .join(PLUGIN_ID)
}

/// 插件源目录：优先用打包后随 exe 分发的那份，开发时回退到仓库里的那份
fn bundled_plugin_dir() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    // 1. Tauri 资源目录（deb 安装后 /usr/lib/4YWallPaper，AppImage 为挂载点）
    if let Some(dir) = RESOURCE_DIR.get() {
        candidates.push(dir.join("plugin").join(PLUGIN_ID));
        candidates.push(dir.join(PLUGIN_ID));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("plasma-wallpapers").join(PLUGIN_ID));
            candidates.push(dir.join("plugin").join(PLUGIN_ID));
            // deb: /usr/bin/fourywallpaper → /usr/lib/4YWallPaper/plugin/<id>
            candidates.push(dir.join("../lib/4YWallPaper/plugin").join(PLUGIN_ID));
            // target/release/fourywallpaper → 回到 src-tauri/plugin
            candidates.push(dir.join("../../plugin").join(PLUGIN_ID));
        }
    }
    // 编译期路径：开发环境直接读仓库里的插件源码
    candidates.push(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugin")
            .join(PLUGIN_ID),
    );
    candidates.into_iter().find(|p| p.join("metadata.json").is_file())
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| format!("创建 {} 失败: {e}", to.display()))?;
    let entries = fs::read_dir(from).map_err(|e| format!("读取 {} 失败: {e}", from.display()))?;
    for entry in entries.flatten() {
        let src = entry.path();
        let dst = to.join(entry.file_name());
        if src.is_dir() {
            copy_dir(&src, &dst)?;
        } else {
            fs::copy(&src, &dst)
                .map_err(|e| format!("复制 {} 失败: {e}", src.display()))?;
        }
    }
    Ok(())
}

/// 插件是否已安装到用户目录
pub fn plugin_installed() -> bool {
    user_plugin_dir().join("metadata.json").is_file()
        && user_plugin_dir().join("contents/ui/main.qml").is_file()
}

/// 安装/更新插件，返回安装路径
pub fn ensure_plugin_installed() -> Result<PathBuf, String> {
    let target = user_plugin_dir();
    let Some(source) = bundled_plugin_dir() else {
        return Err("找不到随程序分发的 Plasma 壁纸插件（plugin/org.foury.videowall）".into());
    };
    // 已经装好且内容一致就跳过
    let src_main = source.join("contents/ui/main.qml");
    let dst_main = target.join("contents/ui/main.qml");
    let same = match (fs::read(&src_main), fs::read(&dst_main)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    };
    if same && plugin_installed() {
        return Ok(target);
    }
    copy_dir(&source, &target)?;
    Ok(target)
}

/// 通过 kscreen-doctor 得到「Plasma 屏幕索引 → 输出名(如 eDP-1)」的映射。
/// Plasma 的 desktop.screen 是 Qt 的屏幕索引（与 kscreen 的输出顺序一致）。
pub fn desktop_screen_map() -> Result<Vec<(u32, String)>, String> {
    let outputs = crate::monitors::list_monitors()
        .unwrap_or_else(|_| crate::monitors::list_monitors_fallback());
    Ok(outputs
        .into_iter()
        .filter(|o| o.enabled)
        .enumerate()
        .map(|(i, o)| (i as u32, o.id))
        .collect())
}

/// 壁纸库里的壁纸 ID → 实际视频文件绝对路径
pub fn resolve_video_path(wallpaper_id: &str) -> Option<PathBuf> {
    let dir = config::library_dir().join(wallpaper_id);
    let project = dir.join("project.json");
    let content = fs::read_to_string(&project).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let file = json.get("file")?.as_str()?;
    let path = dir.join(file);
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

fn escape_js(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// 生成写入 Plasma 桌面配置的 JS 脚本
pub fn build_script(cfg: &AppConfig, screen_map: &[(u32, String)]) -> String {
    let mut lines = vec![
        "var ds = desktops();".to_string(),
        "var n = 0;".to_string(),
    ];
    for (screen_idx, output) in screen_map {
        let Some(assignment) = cfg
            .assignments
            .iter()
            .find(|a| &a.monitor == output)
        else {
            continue;
        };
        if assignment.wallpaper_id.is_empty() {
            continue;
        }
        let Some(video) = resolve_video_path(&assignment.wallpaper_id) else {
            continue;
        };
        let fill = match assignment.scaling.as_str() {
            "stretch" => 0,
            "fit" => 2,
            _ => 1, // fill / default → 保持比例裁剪填满
        };
        let audio = if cfg.settings.mute { "false" } else { "false" };
        lines.push(format!(
            "for (var i = 0; i < ds.length; i++) {{ var d = ds[i]; \
if (d.screen !== {screen_idx}) continue; \
d.wallpaperPlugin = \"{PLUGIN_ID}\"; \
d.currentConfigGroup = [{group}]; \
d.writeConfig(\"videoPath\", \"{path}\"); \
d.writeConfig(\"fillMode\", {fill}); \
d.writeConfig(\"audioEnabled\", {audio}); \
d.writeConfig(\"volume\", {volume}); \
d.writeConfig(\"paused\", false); \
n++; }}",
            screen_idx = screen_idx,
            group = config_group(),
            path = escape_js(&video.to_string_lossy()),
            fill = fill,
            audio = audio,
            volume = (cfg.settings.volume.unwrap_or(50) as f64 / 100.0).clamp(0.0, 1.0),
        ));
    }
    lines.push("print(\"applied to \" + n + \" desktop(s)\");".to_string());
    lines.join("\n")
}

fn run_plasma_script(script: &str) -> Result<String, String> {
    let output = Command::new("qdbus6")
        .args([
            "org.kde.plasmashell",
            "/PlasmaShell",
            "org.kde.PlasmaShell.evaluateScript",
            script,
        ])
        .output()
        .map_err(|e| format!("无法运行 qdbus6（需要 KDE Plasma）: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() {
        Ok(stdout)
    } else {
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

/// 应用壁纸配置：安装插件 → 写桌面配置
pub fn apply(cfg: &AppConfig) -> Result<String, String> {
    ensure_plugin_installed()?;
    let screen_map = desktop_screen_map()?;
    if screen_map.is_empty() {
        return Err("没有检测到已启用的显示器".into());
    }
    let script = build_script(cfg, &screen_map);
    let out = run_plasma_script(&script)?;
    if out.contains("applied to 0") {
        return Err(format!(
            "没有匹配到任何显示器，脚本输出: {out}（请确认已为显示器选择壁纸）"
        ));
    }
    Ok(out)
}

/// 重启 plasmashell。
/// 必须重启：plasmashell 会把编译后的 QML 缓存在 ~/.cache/plasmashell/qmlcache
/// 并在内存里保留已删除的 .qmlc 句柄，仅改配置或删缓存目录都不会重新加载插件。
pub fn restart_plasmashell() -> Result<(), String> {
    let status = Command::new("systemctl")
        .args(["--user", "restart", "plasma-plasmashell"])
        .status()
        .map_err(|e| format!("无法重启 plasmashell: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("重启 plasmashell 失败（退出码 {status}）"))
    }
}

/// 清除某个屏幕上的动态壁纸，恢复成 KDE 静态壁纸（保持原图片设置）
pub fn clear_screen(monitor: &str) -> Result<(), String> {
    let screen_map = desktop_screen_map()?;
    let Some((idx, _)) = screen_map.iter().find(|(_, name)| name == monitor) else {
        return Err(format!("未知显示器: {monitor}"));
    };
    let script = format!(
        "var ds = desktops(); for (var i = 0; i < ds.length; i++) {{ var d = ds[i]; \
if (d.screen !== {idx}) continue; d.wallpaperPlugin = \"org.kde.image\"; }} \
print(\"cleared {idx}\");"
    );
    run_plasma_script(&script).map(|_| ())
}

/// 当前是否有屏在用我们的插件。返回显示器名（如 eDP-1），与 config 里的 monitor 字段一致
pub fn active_desktops() -> Result<Vec<String>, String> {
    let script = format!(
        "var ds = desktops(); var out = []; for (var i = 0; i < ds.length; i++) \
{{ if (ds[i].wallpaperPlugin === \"{PLUGIN_ID}\") out.push(ds[i].screen); }} \
print(out.join(\",\"));"
    );
    let out = run_plasma_script(&script)?;
    let idxs: Vec<u32> = out
        .split(',')
        .filter_map(|s| s.trim().parse::<u32>().ok())
        .collect();
    // 把 Plasma 的屏幕索引翻译成显示器名，方便前端直接匹配
    let map = desktop_screen_map().unwrap_or_default();
    Ok(idxs
        .into_iter()
        .map(|i| {
            map.iter()
                .find(|(idx, _)| *idx == i)
                .map(|(_, name)| name.clone())
                .unwrap_or_else(|| i.to_string())
        })
        .collect())
}
