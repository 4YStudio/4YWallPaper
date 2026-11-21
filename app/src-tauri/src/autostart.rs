use crate::config::APP_DIR_NAME;
use std::fs;
use std::path::PathBuf;

/// XDG 自适应 KDE：通过 ~/.config/autostart 的 .desktop 文件实现开机自启。
/// KDE Plasma 在 plasmashell 启动后执行该条目，环境变量（WAYLAND_DISPLAY 等）完整继承。
pub fn autostart_file() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("autostart")
        .join(format!("{APP_DIR_NAME}.desktop"))
}

pub fn set_autostart(enable: bool) -> Result<bool, String> {
    let path = autostart_file();
    if enable {
        let exe = std::env::current_exe()
            .map_err(|e| format!("无法获取程序路径: {e}"))?
            .to_string_lossy()
            .to_string();
        let content = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=4YWallPaper\n\
             Comment=4YWallPaper 动态壁纸（登录后自动恢复）\n\
             Exec={exe} --autostart\n\
             Icon=4ywallpaper\n\
             Terminal=false\n\
             Categories=Utility;\n\
             X-KDE-StartupNotify=false\n"
        );
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&path, content).map_err(|e| e.to_string())?;
        Ok(true)
    } else {
        if path.exists() {
            fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        Ok(false)
    }
}

#[allow(dead_code)]
pub fn is_autostart_enabled() -> bool {
    autostart_file().exists()
}
