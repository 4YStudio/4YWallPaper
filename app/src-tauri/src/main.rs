// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 开机自启模式：读取配置启动引擎后退出，不启动 GUI
    if std::env::args().any(|a| a == "--autostart") {
        fourywallpaper_lib::run_autostart();
        return;
    }
    // 一次性应用：不开 GUI，直接按配置写入壁纸（便于脚本/排错）
    if std::env::args().any(|a| a == "--apply") {
        fourywallpaper_lib::run_apply_once();
        return;
    }
    // 打印壁纸库扫描结果（JSON），便于排查目录引用问题
    if std::env::args().any(|a| a == "--list") {
        fourywallpaper_lib::run_list();
        return;
    }
    // 同步生成所有缺失的缩略图（不开 GUI）
    if std::env::args().any(|a| a == "--thumbs") {
        fourywallpaper_lib::run_thumbs();
        return;
    }
    fourywallpaper_lib::run();
}
