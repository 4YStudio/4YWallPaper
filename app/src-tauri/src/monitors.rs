use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

/// 解析 `kscreen-doctor -o` 输出，返回显示器列表。
/// 这是 KDE Plasma 自带工具，深度适配 KDE 的多屏枚举。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KScreenOutput {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub connected: bool,
    pub width: u32,
    pub height: u32,
    pub refresh: f64,
    pub x: i32,
    pub y: i32,
}

pub fn list_monitors() -> Result<Vec<KScreenOutput>, String> {
    let output = Command::new("kscreen-doctor")
        .arg("-o")
        .output()
        .map_err(|e| format!("无法运行 kscreen-doctor: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "kscreen-doctor 退出码 {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(parse_kscreen_output(&text))
}

/// 去掉 ANSI 颜色码（kscreen-doctor -o 默认带颜色，会让正则匹配失败）
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // CSI 序列：ESC [ ... 终止于 @..~
            if chars.peek() == Some(&'[') {
                chars.next();
                for c2 in chars.by_ref() {
                    if ('@'..='~').contains(&c2) {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

pub fn parse_kscreen_output(text: &str) -> Vec<KScreenOutput> {
    let stripped = strip_ansi(text);
    let text = stripped.as_str();
    let re_output = regex::Regex::new(r"Output:\s*\d+\s+(\S+)\s+(enabled|disabled)").unwrap();
    let re_geometry =
        regex::Regex::new(r"Geometry:\s*(-?\d+),(-?\d+)\s+(\d+)x(\d+)").unwrap();
    let re_mode = regex::Regex::new(r"(\d+)x(\d+)@([\d.]+)").unwrap();
    let re_preferred =
        regex::Regex::new(r"(\d+)x(\d+)@([\d.]+)[^\n]*preferred").unwrap();

    // 按 "Output:" 分块
    let mut blocks: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in text.lines() {
        if line.contains("Output:") && !current.is_empty() {
            blocks.push(std::mem::take(&mut current));
        }
        current.push_str(line);
        current.push('\n');
    }
    if !current.is_empty() {
        blocks.push(current);
    }

    let mut result = Vec::new();
    for block in blocks {
        let Some(out_caps) = re_output.captures(&block) else {
            continue;
        };
        let name = out_caps[1].to_string();
        let enabled = &out_caps[2] == "enabled";
        let connected = block.contains("connected");

        // 几何与模式取该块内第一个匹配
        let (x, y, width, height) = re_geometry
            .captures(&block)
            .map(|c| {
                (
                    c[1].parse::<i32>().unwrap_or(0),
                    c[2].parse::<i32>().unwrap_or(0),
                    c[3].parse::<u32>().unwrap_or(0),
                    c[4].parse::<u32>().unwrap_or(0),
                )
            })
            .unwrap_or((0, 0, 0, 0));

        // 优先取标记 preferred 的模式，否则取第一个
        let (width, height, refresh) = re_preferred
            .captures(&block)
            .or_else(|| re_mode.captures(&block))
            .map(|c| {
                (
                    c[1].parse::<u32>().unwrap_or(width),
                    c[2].parse::<u32>().unwrap_or(height),
                    c[3].parse::<f64>().unwrap_or(60.0),
                )
            })
            .unwrap_or((width, height, 60.0));

        result.push(KScreenOutput {
            id: name.clone(),
            name,
            enabled,
            connected,
            width,
            height,
            refresh,
            x,
            y,
        });
    }
    result
}

/// 非交互回退：通过 /sys/class/drm 枚举连接器（kscreen-doctor 不可用时）。
pub fn list_monitors_fallback() -> Vec<KScreenOutput> {
    let mut result = Vec::new();
    let Ok(entries) = fs::read_dir("/sys/class/drm") else {
        return result;
    };
    for entry in entries.flatten() {
        let fname = entry.file_name().to_string_lossy().to_string();
        // card0-eDP-1、card0-HDMI-A-1 等
        let Some((_card, conn)) = fname.split_once('-') else {
            continue;
        };
        let status_path = entry.path().join("status");
        let Ok(status) = fs::read_to_string(&status_path) else {
            continue;
        };
        let connected = status.trim() == "connected";
        if !connected {
            continue;
        }
        // 读当前模式
        let modes_path = entry.path().join("modes");
        let (width, height, refresh) = fs::read_to_string(&modes_path)
            .ok()
            .and_then(|m| {
                let first = m.lines().next()?.trim().to_string();
                // 格式: 1920x1080 60.00
                let mut it = first.split_whitespace();
                let res = it.next()?;
                let (w, h) = res.split_once('x')?;
                let rf = it.next()?.parse::<f64>().ok()?;
                Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?, rf))
            })
            .unwrap_or((0, 0, 60.0));
        result.push(KScreenOutput {
            id: conn.to_string(),
            name: conn.to_string(),
            enabled: true,
            connected,
            width,
            height,
            refresh,
            x: 0,
            y: 0,
        });
    }
    result
}

/// 判断路径是否为有效的显示器 ID（避免把任意字符串当屏幕名传给引擎）
pub fn is_valid_monitor_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && Path::new(id).file_name().map(|f| f.to_string_lossy() == id).unwrap_or(false)
}
