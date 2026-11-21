use crate::config::{self, library_dir, thumbs_dir};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

#[derive(Debug, Clone, Serialize)]
pub struct WallpaperEntry {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub wallpaper_type: String,
    pub thumb: Option<String>,
    /// 壁纸条目所在路径：库内目录，或外部目录里的视频/文件夹
    pub path: String,
    pub size: u64,
    pub properties: Vec<serde_json::Value>,
    /// "library" = 复制进壁纸库；"external" = 直接引用外部目录
    pub source: String,
    /// 实际视频文件绝对路径（外部目录的条目靠它播放）
    pub video: Option<String>,
}

pub const VIDEO_EXTS: &[&str] = &["mp4", "mkv", "webm", "mov", "avi", "m4v", "flv", "wmv", "ts"];

fn dir_size(path: &Path) -> u64 {
    let mut size = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_file() {
                size += meta.len();
            } else if meta.is_dir() {
                size += dir_size(&entry.path());
            }
        }
    }
    size
}

fn find_thumb(dir: &Path) -> Option<String> {
    for name in ["thumb.jpg", "preview.jpg", "preview.png", "thumb.png", "poster.jpg"] {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p.to_string_lossy().to_string());
        }
    }
    None
}

/// 稳定哈希（FNV-1a 64）：用于给外部壁纸生成不随程序版本变化的 ID / 缩略图名。
fn stable_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn is_video_file(path: &Path) -> bool {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .map(|e| VIDEO_EXTS.contains(&e.as_str()))
        .unwrap_or(false)
}

fn ext_of(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// 外部壁纸缩略图缓存路径（不往用户目录里写文件）
fn cached_thumb_path(key: &str) -> PathBuf {
    thumbs_dir().join(format!("{:016x}.jpg", stable_hash(key)))
}

fn generate_video_thumbnail(video: &Path, out: &Path) {
    if let Some(parent) = out.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = Command::new("ffmpeg")
        .args([
            "-y",
            "-ss",
            "1",
            "-i",
            &video.to_string_lossy(),
            "-frames:v",
            "1",
            "-vf",
            "scale=640:-2",
            &out.to_string_lossy(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// 扫描壁纸库：所有含 project.json 的子目录
fn scan_library() -> Vec<WallpaperEntry> {
    let lib = library_dir();
    if !lib.exists() {
        return Vec::new();
    }
    let mut result = Vec::new();
    let Ok(entries) = fs::read_dir(&lib) else {
        return result;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let project_file = path.join("project.json");
        if !project_file.is_file() {
            continue;
        }
        let Ok(content) = fs::read_to_string(&project_file) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
            continue;
        };
        let id = entry.file_name().to_string_lossy().to_string();
        let title = json
            .get("title")
            .and_then(|t| t.as_str())
            .unwrap_or(&id)
            .to_string();
        let wallpaper_type = json
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("video")
            .to_lowercase();
        let video = json
            .get("file")
            .and_then(|f| f.as_str())
            .map(|f| path.join(f))
            .filter(|p| p.is_file())
            .map(|p| p.to_string_lossy().to_string());
        result.push(WallpaperEntry {
            id: id.clone(),
            title,
            wallpaper_type,
            thumb: find_thumb(&path),
            path: path.to_string_lossy().to_string(),
            size: dir_size(&path),
            properties: Vec::new(),
            source: "library".into(),
            video,
        });
    }
    result
}

/// 扫描一个外部目录：里面的视频文件，以及含 project.json 的子目录
fn scan_external_dir(dir: &Path) -> Vec<WallpaperEntry> {
    let mut result = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return result;
    };
    let mut items: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    items.sort();

    for path in items {
        if path.is_dir() {
            // Wallpaper Engine 风格：目录里有 project.json
            if !path.join("project.json").is_file() {
                continue;
            }
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let json = fs::read_to_string(path.join("project.json"))
                .ok()
                .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok());
            let title = json
                .as_ref()
                .and_then(|j| j.get("title").and_then(|t| t.as_str()))
                .unwrap_or(&name)
                .to_string();
            let wallpaper_type = json
                .as_ref()
                .and_then(|j| j.get("type").and_then(|t| t.as_str()))
                .unwrap_or("video")
                .to_lowercase();
            let video = json
                .as_ref()
                .and_then(|j| j.get("file").and_then(|f| f.as_str()))
                .map(|f| path.join(f))
                .filter(|p| p.is_file())
                .map(|p| p.to_string_lossy().to_string());
            let key = path.to_string_lossy().to_string();
            result.push(WallpaperEntry {
                id: format!("ext_{:016x}", stable_hash(&key)),
                title,
                wallpaper_type,
                thumb: find_thumb(&path).or_else(|| video.as_ref().and_then(|v| {
                    let p = cached_thumb_path(v);
                    p.is_file().then(|| p.to_string_lossy().to_string())
                })),
                path: key,
                size: dir_size(&path),
                properties: Vec::new(),
                source: "external".into(),
                video,
            });
        } else if path.is_file() && is_video_file(&path) {
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let key = path.to_string_lossy().to_string();
            let cached = cached_thumb_path(&key);
            result.push(WallpaperEntry {
                id: format!("ext_{:016x}", stable_hash(&key)),
                title: name,
                wallpaper_type: "video".into(),
                thumb: cached.is_file().then(|| cached.to_string_lossy().to_string()),
                path: key.clone(),
                size: fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
                properties: Vec::new(),
                source: "external".into(),
                video: Some(key),
            });
        }
    }
    result
}

fn scan_external() -> Vec<WallpaperEntry> {
    let cfg = config::load();
    let mut result = Vec::new();
    for dir in cfg.settings.source_dirs {
        let p = PathBuf::from(&dir);
        if p.is_dir() {
            result.extend(scan_external_dir(&p));
        }
    }
    result
}

/// 扫描壁纸库 + 所有外部目录
pub fn list_wallpapers() -> Result<Vec<WallpaperEntry>, String> {
    let mut result = scan_library();
    result.extend(scan_external());
    // 库内条目按 id 排序在前，外部条目按标题排序
    result.sort_by(|a, b| {
        (a.source != "library", a.title.to_lowercase())
            .cmp(&(b.source != "library", b.title.to_lowercase()))
    });
    // 外部视频的缩略图在后台补齐，避免首次扫描卡住界面
    spawn_thumb_generation();
    Ok(result)
}

/// 后台为缺失缩略图的外部视频生成缓存缩略图（同一时刻只跑一个线程）
static THUMB_JOB: AtomicBool = AtomicBool::new(false);

pub fn spawn_thumb_generation() {
    if THUMB_JOB.swap(true, Ordering::SeqCst) {
        return;
    }
    thread::spawn(|| {
        generate_missing_thumbnails();
        THUMB_JOB.store(false, Ordering::SeqCst);
    });
}

/// 同步把所有缺失的缩略图补齐（给 CLI 的 `--thumbs` 用：
/// 后台线程在进程退出时会被杀掉，CLI 需要阻塞式生成）
pub fn generate_missing_thumbnails() -> usize {
    let mut made = 0usize;
    let cfg = config::load();
    for dir in cfg.settings.source_dirs {
        let p = PathBuf::from(&dir);
        if !p.is_dir() {
            continue;
        }
        for entry in scan_external_dir(&p) {
            let Some(video) = entry.video.as_ref() else { continue };
            if entry.thumb.is_some() {
                continue;
            }
            let out = cached_thumb_path(video);
            if out.is_file() {
                continue;
            }
            generate_video_thumbnail(Path::new(video), &out);
            if out.is_file() {
                made += 1;
            }
        }
    }
    made
}

/// 把目录名转成安全的库内 ID（去除路径分隔符等）
fn safe_id(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "wallpaper".to_string()
    } else {
        cleaned
    }
}

/// 库内去重：如果 id 已存在则追加 -2、-3 …
fn dedupe_id(base: &str) -> String {
    let lib = library_dir();
    let mut id = base.to_string();
    let mut n = 2;
    while lib.join(&id).exists() {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

pub fn import_video(src: &str) -> Result<WallpaperEntry, String> {
    let src_path = PathBuf::from(src);
    if !src_path.is_file() {
        return Err(format!("文件不存在: {src}"));
    }
    let ext = ext_of(&src_path);
    if !VIDEO_EXTS.contains(&ext.as_str()) {
        return Err(format!(
            "不支持的视频格式 .{ext}，支持: {}",
            VIDEO_EXTS.join(", ")
        ));
    }
    let stem = src_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "video".to_string());
    let id = dedupe_id(&safe_id(&stem));

    let lib = library_dir();
    fs::create_dir_all(&lib).map_err(|e| e.to_string())?;
    let dest_dir = lib.join(&id);
    fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;

    let video_name = format!("video.{ext}");
    let dest_video = dest_dir.join(&video_name);
    fs::copy(&src_path, &dest_video).map_err(|e| format!("复制视频失败: {e}"))?;

    // 写 project.json（video 类型只需 type + file）
    let project = serde_json::json!({
        "type": "video",
        "file": video_name,
        "title": stem,
    });
    fs::write(
        dest_dir.join("project.json"),
        serde_json::to_string_pretty(&project).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    // 生成缩略图（ffmpeg，失败不影响导入）
    generate_video_thumbnail(&dest_video, &dest_dir.join("thumb.jpg"));

    Ok(WallpaperEntry {
        id,
        title: stem,
        wallpaper_type: "video".into(),
        thumb: find_thumb(&dest_dir),
        path: dest_dir.to_string_lossy().to_string(),
        size: dir_size(&dest_dir),
        properties: Vec::new(),
        source: "library".into(),
        video: Some(dest_video.to_string_lossy().to_string()),
    })
}

/// 递归复制目录
fn copy_dir(src: &Path, dest: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if ty.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

pub fn import_folder(src: &str) -> Result<WallpaperEntry, String> {
    let src_path = PathBuf::from(src);
    if !src_path.is_dir() {
        return Err(format!("目录不存在: {src}"));
    }
    if !src_path.join("project.json").is_file() {
        return Err("所选目录中没有 project.json，不是有效的壁纸文件夹".into());
    }
    let name = src_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "wallpaper".to_string());
    let id = dedupe_id(&safe_id(&name));

    let lib = library_dir();
    fs::create_dir_all(&lib).map_err(|e| e.to_string())?;
    let dest_dir = lib.join(&id);
    copy_dir(&src_path, &dest_dir).map_err(|e| format!("复制文件夹失败: {e}"))?;

    // 读标题
    let title = fs::read_to_string(dest_dir.join("project.json"))
        .ok()
        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
        .and_then(|j| {
            j.get("title")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| id.clone());

    Ok(WallpaperEntry {
        id,
        title,
        wallpaper_type: "folder".into(),
        thumb: find_thumb(&dest_dir),
        path: dest_dir.to_string_lossy().to_string(),
        size: dir_size(&dest_dir),
        properties: Vec::new(),
        source: "library".into(),
        video: None,
    })
}

pub fn delete_wallpaper(id: &str) -> Result<(), String> {
    // 外部目录的条目不能删（文件在用户自己的目录里）
    if id.starts_with("ext_") {
        return Err("这是外部目录中的壁纸，请用「移除壁纸目录」取消引用".into());
    }
    // 安全校验：id 不能含路径分隔符
    if id.contains('/') || id.contains("..") || id.is_empty() {
        return Err("非法的壁纸 ID".into());
    }
    let dir = library_dir().join(id);
    if !dir.is_dir() {
        return Err(format!("壁纸不存在: {id}"));
    }
    fs::remove_dir_all(&dir).map_err(|e| e.to_string())
}

/// 壁纸 ID → 实际视频文件绝对路径（库内 + 外部目录）
pub fn resolve_video_path(wallpaper_id: &str) -> Option<PathBuf> {
    let lib_dir = library_dir().join(wallpaper_id);
    let project = lib_dir.join("project.json");
    if project.is_file() {
        let content = fs::read_to_string(&project).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;
        let file = json.get("file")?.as_str()?;
        let path = lib_dir.join(file);
        return path.is_file().then_some(path);
    }
    let entry = scan_external().into_iter().find(|e| e.id == wallpaper_id)?;
    let video = PathBuf::from(entry.video?);
    video.is_file().then_some(video)
}

/// 壁纸 ID → 引擎可用的壁纸文件夹（含 project.json）；外部裸视频返回 None
pub fn resolve_bg_dir(wallpaper_id: &str) -> Option<PathBuf> {
    let lib_dir = library_dir().join(wallpaper_id);
    if lib_dir.join("project.json").is_file() {
        return Some(lib_dir);
    }
    let entry = scan_external().into_iter().find(|e| e.id == wallpaper_id)?;
    let dir = PathBuf::from(&entry.path);
    dir.join("project.json").is_file().then_some(dir)
}

/// 登记一个外部壁纸目录（不复制任何文件）
pub fn add_source_dir(path: &str) -> Result<Vec<String>, String> {
    let p = PathBuf::from(path);
    if !p.is_dir() {
        return Err(format!("目录不存在: {path}"));
    }
    let canonical = p
        .canonicalize()
        .unwrap_or(p)
        .to_string_lossy()
        .to_string();
    let mut cfg = config::load();
    if !cfg.settings.source_dirs.iter().any(|d| d == &canonical) {
        cfg.settings.source_dirs.push(canonical);
        config::save(&cfg)?;
    }
    Ok(cfg.settings.source_dirs)
}

/// 取消引用某个外部壁纸目录
pub fn remove_source_dir(path: &str) -> Result<Vec<String>, String> {
    let mut cfg = config::load();
    let target = PathBuf::from(path)
        .canonicalize()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string());
    cfg.settings.source_dirs.retain(|d| d != path && d != &target);
    config::save(&cfg)?;
    Ok(cfg.settings.source_dirs)
}
