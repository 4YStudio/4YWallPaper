use crate::config::{self, MonitorAssignment, Settings};
use serde::Serialize;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// 引擎可执行文件的默认候选路径
pub fn candidate_engine_paths() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(data) = dirs::data_dir() {
        v.push(data.join("4ywallpaper/bin/linux-wallpaperengine"));
    }
    // 相对当前 exe 的常见构建输出位置
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            // 打包后 exe 在 app/bin 旁
            v.push(parent.join("linux-wallpaperengine"));
            v.push(parent.join("../../linux-wallpaperengine/build/output/linux-wallpaperengine"));
            v.push(parent.join("../../../linux-wallpaperengine/build/output/linux-wallpaperengine"));
        }
    }
    v.push(PathBuf::from("linux-wallpaperengine")); // $PATH
    v
}

pub fn detect_engine_path() -> Option<String> {
    for p in candidate_engine_paths() {
        // $PATH 里的裸名字特殊处理
        if !p.is_absolute() {
            if Command::new("which")
                .arg(&p)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                let abs = Command::new("which")
                    .arg(&p)
                    .output()
                    .ok()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_default();
                if !abs.is_empty() {
                    return Some(abs);
                }
            }
            continue;
        }
        if p.is_file() {
            // 确认可执行
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&p) {
                if meta.permissions().mode() & 0o111 != 0 {
                    return Some(p.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineProcessInfo {
    pub pid: u32,
    pub monitor: Option<String>,
    pub wallpaper: Option<String>,
    pub cmdline: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    pub ts: u64,
    pub stream: String,
    pub text: String,
}

#[derive(Default)]
pub struct LogBuffer(pub Mutex<VecDeque<LogLine>>);

const LOG_CAP: usize = 1000;

pub fn push_log(buf: &Arc<LogBuffer>, stream: &str, text: String) {
    if let Ok(mut q) = buf.0.lock() {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        q.push_back(LogLine {
            ts,
            stream: stream.to_string(),
            text,
        });
        while q.len() > LOG_CAP {
            q.pop_front();
        }
    }
}

/// 扫描 /proc 找到 linux-wallpaperengine 渲染进程
pub fn find_engine_processes() -> Vec<EngineProcessInfo> {
    let mut result = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return result;
    };
    for entry in entries.flatten() {
        let fname = entry.file_name().to_string_lossy().to_string();
        let Ok(pid) = fname.parse::<u32>() else {
            continue;
        };
        let Ok(cmdline) = std::fs::read_to_string(entry.path().join("cmdline")) else {
            continue;
        };
        let args: Vec<String> = cmdline
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        if args.is_empty() {
            continue;
        }
        let exe_name = PathBuf::from(&args[0])
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        if exe_name != "linux-wallpaperengine" {
            continue;
        }
        // 必须是渲染进程（带 --bg / --screen-root），跳过 --list-properties 之类
        let is_render = args.iter().any(|a| a == "--bg" || a == "--screen-root");
        if !is_render {
            continue;
        }
        let monitor = args
            .iter()
            .position(|a| a == "--screen-root")
            .and_then(|i| args.get(i + 1).cloned());
        let wallpaper = args
            .iter()
            .position(|a| a == "--bg")
            .and_then(|i| args.get(i + 1).cloned());
        result.push(EngineProcessInfo {
            pid,
            monitor,
            wallpaper,
            cmdline: args.join(" "),
        });
    }
    result
}

fn kill_pid(pid: u32) {
    let _ = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .output();
}

fn kill_pid_force(pid: u32) {
    let _ = Command::new("kill")
        .args(["-KILL", &pid.to_string()])
        .output();
}

pub fn stop_all_engines() {
    let pids: Vec<u32> = find_engine_processes().into_iter().map(|p| p.pid).collect();
    for pid in &pids {
        kill_pid(*pid);
    }
    if !pids.is_empty() {
        thread::sleep(Duration::from_millis(800));
        let alive: Vec<u32> = find_engine_processes()
            .into_iter()
            .map(|p| p.pid)
            .filter(|pid| pids.contains(pid))
            .collect();
        for pid in alive {
            kill_pid_force(pid);
        }
    }
}

pub fn stop_engine_by_monitor(monitor: &str) {
    let targets: Vec<u32> = find_engine_processes()
        .into_iter()
        .filter(|p| p.monitor.as_deref() == Some(monitor))
        .map(|p| p.pid)
        .collect();
    for pid in &targets {
        kill_pid(*pid);
    }
    if !targets.is_empty() {
        thread::sleep(Duration::from_millis(800));
        let alive: Vec<u32> = find_engine_processes()
            .into_iter()
            .map(|p| p.pid)
            .filter(|pid| targets.contains(pid))
            .collect();
        for pid in alive {
            kill_pid_force(pid);
        }
    }
}

/// 根据 config 构建引擎命令行参数。
/// 参数顺序遵循引擎约定：全局选项在前，每个屏幕依次
/// `--screen-root M --scaling X --bg <path>`。
/// 注意：`--scaling` 作用于「前一个」--screen-root/--screen-span，
/// 因此必须写在 --screen-root **之后**，否则多屏不同缩放模式会串到上一屏。
pub fn build_engine_args(settings: &Settings, assignments: &[MonitorAssignment]) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    if settings.mute {
        args.push("--silent".into());
    }
    if let Some(vol) = settings.volume {
        // UI 0-100 → 引擎 0-128
        let v = ((vol as f64) / 100.0 * 128.0).round().clamp(0.0, 128.0) as i32;
        args.push("--volume".into());
        args.push(v.to_string());
    }
    if !settings.automute {
        args.push("--noautomute".into());
    }
    if settings.disable_mouse {
        args.push("--disable-mouse".into());
    }
    if settings.disable_parallax {
        args.push("--disable-parallax".into());
    }
    if !settings.fullscreen_pause {
        args.push("--no-fullscreen-pause".into());
    }
    if let Some(fps) = settings.fps {
        args.push("--fps".into());
        args.push(fps.to_string());
    }
    if let Some(assets) = &settings.assets_dir {
        if !assets.is_empty() {
            args.push("--assets-dir".into());
            args.push(assets.clone());
        }
    }

    for a in assignments {
        if a.wallpaper_id.is_empty() {
            continue;
        }
        // 外部目录里的裸视频没有 project.json，引擎后端无法直接使用
        let Some(bg_dir) = crate::library::resolve_bg_dir(&a.wallpaper_id) else {
            continue;
        };
        let scaling = if a.scaling.is_empty() {
            settings.scaling.clone()
        } else {
            a.scaling.clone()
        };
        args.push("--screen-root".into());
        args.push(a.monitor.clone());
        // --scaling 必须紧跟 --screen-root，才作用到这一屏
        if !scaling.is_empty() {
            args.push("--scaling".into());
            args.push(scaling);
        }
        args.push("--bg".into());
        args.push(bg_dir.to_string_lossy().to_string());
    }
    args
}

/// 引擎日志文件路径
pub fn engine_log_path() -> PathBuf {
    config::logs_dir().join("engine.log")
}

/// 日志文件超过这个大小就轮转一次，避免无限增长
const LOG_FILE_MAX: u64 = 4 * 1024 * 1024;

/// 从日志文件尾部加载最近若干行到日志缓冲（GUI 启动时用）
pub fn load_recent_logs_from_file(logs: &Arc<LogBuffer>, limit: usize) {
    let path = engine_log_path();
    let Ok(content) = std::fs::read(&path) else {
        return;
    };
    let text = String::from_utf8_lossy(&content);
    let mut lines: Vec<&str> = text.lines().collect();
    if lines.len() > limit {
        lines = lines.split_off(lines.len() - limit);
    }
    for line in lines {
        push_log(logs, "stdout", line.to_string());
    }
}

/// 持续跟踪日志文件新增内容，写入共享日志缓冲。
/// 只跟随当前文件末尾，不重复读取历史内容。
fn spawn_log_tail(logs: Arc<LogBuffer>) {
    let path = engine_log_path();
    thread::spawn(move || {
        use std::io::{Read, Seek, SeekFrom};
        let mut offset = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let mut pending = String::new();
        loop {
            thread::sleep(Duration::from_millis(300));
            let Ok(mut f) = std::fs::File::open(&path) else {
                continue;
            };
            let len = f.metadata().map(|m| m.len()).unwrap_or(0);
            if len < offset {
                // 文件被轮转/截断
                offset = 0;
            }
            if len == offset {
                continue;
            }
            if f.seek(SeekFrom::Start(offset)).is_err() {
                continue;
            }
            let mut buf = String::new();
            if f.read_to_string(&mut buf).is_err() {
                continue;
            }
            offset = len;
            pending.push_str(&buf);
            while let Some(idx) = pending.find('\n') {
                let line: String = pending.drain(..=idx).collect();
                let line = line.trim_end_matches(['\n', '\r']).to_string();
                if !line.is_empty() {
                    push_log(&logs, "stdout", line);
                }
            }
        }
    });
}

/// 启动引擎进程（分离式：引擎脱离本进程独立运行，GUI 退出后壁纸持续显示）。
///
/// 关键点：引擎的 stdout/stderr **不能**接管道。管道会随 GUI 退出而关闭，
/// 引擎下一次写日志就会收到 SIGPIPE 直接死掉（实测 rc=-13），
/// 表现为「关掉控制面板壁纸就没了」。所以这里重定向到日志文件。
pub fn spawn_engine(
    settings: &Settings,
    assignments: &[MonitorAssignment],
    logs: &Arc<LogBuffer>,
) -> Result<(), String> {
    let exe = match &settings.engine_path {
        Some(p) if !p.is_empty() => {
            let pb = PathBuf::from(p);
            if pb.is_absolute() && !pb.exists() {
                return Err(format!("引擎路径不存在: {p}"));
            }
            p.clone()
        }
        _ => detect_engine_path()
            .ok_or("未找到 linux-wallpaperengine，请在设置中指定引擎路径或先构建引擎")?,
    };

    let args = build_engine_args(settings, assignments);
    if !args.iter().any(|a| a == "--bg") {
        return Err("没有有效的壁纸分配".into());
    }

    // 先停掉同屏旧进程，避免叠加
    for a in assignments {
        stop_engine_by_monitor(&a.monitor);
    }

    // 准备日志文件（超过上限先轮转）
    let log_path = engine_log_path();
    if let Some(dir) = log_path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("创建日志目录失败: {e}"))?;
    }
    if std::fs::metadata(&log_path).map(|m| m.len()).unwrap_or(0) > LOG_FILE_MAX {
        let _ = std::fs::rename(&log_path, log_path.with_extension("log.1"));
    }

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("打开日志文件失败: {e}"))?;
    let log_file_err = log_file
        .try_clone()
        .map_err(|e| format!("复制日志句柄失败: {e}"))?;

    let mut cmd = Command::new(&exe);
    cmd.args(&args)
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_file_err))
        .stdin(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0); // 分离进程组，避免被 GUI 的 Ctrl+C 波及
    }

    let header = format!("$ {exe} {}", args.join(" "));
    push_log(logs, "stdout", header.clone());
    if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&log_path) {
        use std::io::Write;
        let _ = writeln!(f, "\n=== {header}");
    }

    let mut child = cmd.spawn().map_err(|e| format!("启动引擎失败: {e}"))?;

    // 后台线程跟随日志文件，把新内容送进 GUI 日志视图
    spawn_log_tail(logs.clone());

    push_log(logs, "stdout", format!("引擎已启动 pid={}", child.id()));

    // 立刻回收僵尸进程句柄：子进程脱离后由 init 接管
    thread::spawn(move || {
        let _ = child.wait();
    });

    Ok(())
}

/// --autostart 模式：读配置并启动引擎，不启动 GUI
pub fn run_autostart() {
    let cfg = config::load();
    let logs = Arc::new(LogBuffer::default());
    if cfg.assignments.is_empty() {
        eprintln!("4ywallpaper: 没有壁纸分配，退出");
        return;
    }
    // plasma 后端：壁纸配置存在 Plasma 自己的 appletsrc 里，登录后 plasmashell
    // 会自动恢复，不需要额外动作；这里只确保插件已安装。
    if cfg.settings.use_plasma() {
        match crate::plasma::ensure_plugin_installed() {
            Ok(p) => println!("4ywallpaper: Plasma 插件就绪 ({})", p.display()),
            Err(e) => eprintln!("4ywallpaper: 插件安装失败: {e}"),
        }
        return;
    }
    match spawn_engine(&cfg.settings, &cfg.assignments, &logs) {
        Ok(()) => println!("4ywallpaper: 壁纸已恢复"),
        Err(e) => {
            eprintln!("4ywallpaper: 启动失败: {e}");
            std::process::exit(1);
        }
    }
}
