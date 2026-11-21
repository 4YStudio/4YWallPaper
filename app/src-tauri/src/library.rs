use crate::config::library_dir;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Serialize)]
pub struct WallpaperEntry {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub wallpaper_type: String,
    pub thumb: Option<String>,
    pub path: String,
    pub size: u64,
    pub properties: Vec<serde_json::Value>,
}

pub const VIDEO_EXTS: &[&str] = &["mp4", "mkv", "webm", "mov", "avi", "m4v", "flv", "wmv"];

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

/// 扫描壁纸库：所有含 project.json 的子目录
pub fn list_wallpapers() -> Result<Vec<WallpaperEntry>, String> {
    let lib = library_dir();
    if !lib.exists() {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    let entries = fs::read_dir(&lib).map_err(|e| e.to_string())?;
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
        result.push(WallpaperEntry {
            id: id.clone(),
            title,
            wallpaper_type,
            thumb: find_thumb(&path),
            path: path.to_string_lossy().to_string(),
            size: dir_size(&path),
            properties: Vec::new(),
        });
    }
    result.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(result)
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
    let ext = src_path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
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

    let video_name = format!(
        "video.{ext}"
    );
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
    })
}

fn generate_video_thumbnail(video: &Path, out: &Path) {
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
    })
}

pub fn delete_wallpaper(id: &str) -> Result<(), String> {
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
