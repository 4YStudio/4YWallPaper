# 4YWallPaper

**Debian 13 + KDE Plasma 6 (Wayland)** 专用动态壁纸软件。Tauri 2 + Vue 3 前端，
壁纸由 **KDE 原生 Plasma 壁纸插件**（Qt6 Multimedia）渲染，可选保留
[linux-wallpaperengine](https://github.com/Almamu/linux-wallpaperengine) 作为
scene/web 壁纸后端。

## 为什么不用 layer-shell 直接铺桌面

最初版本用 linux-wallpaperengine 的 `zwlr_layer_shell_v1` 在背景层铺一层不透明 surface。
实测（KDE Plasma 6.3 + KWin Wayland）会出现两个致命问题：

1. **桌面图标和文字被盖住** —— plasmashell 的桌面 containment（图标 + 文字 + 静态壁纸）
   本身就在 background/bottom 层，引擎再铺一层不透明 surface 后，合成器直接不再绘制它，
   于是图标文字全部消失（鼠标悬浮气泡、右键菜单在更高层，所以仍然可见）。
2. **高 DPI 屏幕不适配** —— `wl_output.scale` 只上报整数（eDP-1 实际 1.75 → 上报 2），
   而 layer-shell `configure` 给的是逻辑尺寸，两者混用导致缓冲区尺寸算错、画面裁切错位。

因此默认后端改为 **KDE 原生壁纸插件**：视频由 plasmashell 自己在桌面 containment 内
绘制，图标/文字天然在视频之上，DPI 缩放由 Plasma 处理，多屏各自独立。

## 功能

- 🖼️ **壁纸库管理**：本地视频（mp4/webm/mkv…）一键导入，自动生成 `project.json` + 缩略图
- 📂 **外部壁纸目录**：直接引用已有壁纸文件夹（不复制、不占额外空间），裸视频与
  Wallpaper Engine 文件夹自动识别，缩略图按需缓存
- 🖥️ **多显示器**：通过 `kscreen-doctor` 枚举显示器，每屏独立分配壁纸与缩放模式
- 🎬 **Plasma 壁纸插件**：随程序分发的 `org.foury.videowall` 插件，写入 Plasma 桌面配置
- ⚙️ **播放控制**：缩放（裁剪填满 / 拉伸 / 完整显示）、音量、静音、开机自启
- 📜 **日志面板**：查看引擎日志（`~/.local/share/4ywallpaper/logs/engine.log`）
- 🔁 **可切换后端**：设置里可切回 `engine`（layer-shell，支持 scene/web 壁纸，但会盖住图标）

## 目录结构

```
4YWallPaper/
├── app/                            # Tauri 2 + Vue 3 应用
│   ├── src/                        #   前端（壁纸库 / 显示器 / 设置 / 日志）
│   └── src-tauri/
│       ├── src/                    #   Rust 后端
│       │   ├── plasma.rs           #   ★ Plasma 壁纸插件后端（默认）
│       │   ├── engine.rs           #   linux-wallpaperengine 进程管理（可选）
│       │   ├── monitors.rs         #   kscreen-doctor 解析（已处理 ANSI 颜色码）
│       │   ├── library.rs          #   壁纸库扫描 / 导入 / 缩略图
│       │   ├── config.rs           #   配置读写（损坏自动备份）
│       │   └── autostart.rs        #   XDG autostart
│       └── plugin/org.foury.videowall/   # ★ 随包分发的 Plasma 壁纸插件
├── patches/
│   └── engine-optional-cef.patch   # 引擎补丁：把 CEF(Web 壁纸) 改成可选编译
└── scripts/
    ├── install.sh                  # 一键：构建应用 + 装插件 + 应用壁纸
    ├── build-engine.sh             # 克隆上游引擎 + 打补丁 + 构建安装（可选）
    └── build-app.sh                # 构建 GUI + deb 包
```

> 引擎源码（`linux-wallpaperengine/`）**不在本仓库内**——它是独立的 GPLv3 上游项目，
> 由 `scripts/build-engine.sh` 自动克隆并打补丁。仓库只保留 4 个文件的补丁。

## 快速开始

```bash
# 0. 环境（Debian 13 / KDE Plasma 6；Rust 与 Node 需要已装好）
#    本机 Node 在 nvm 里时：
export PATH="$HOME/.nvm/versions/node/*/bin:$HOME/.cargo/bin:$PATH"

git clone <this-repo> && cd 4YWallPaper

# 1. 一键构建 + 安装 + 应用（不需要 root）
./scripts/install.sh

# 2. 之后随时启动 GUI
app/src-tauri/target/release/fourywallpaper

# 3. 或在无 GUI 时按现有配置直接应用壁纸
app/src-tauri/target/release/fourywallpaper --apply
```

在 GUI 里：**壁纸库** 导入视频 → **显示器** 给每块屏选壁纸并点「应用到桌面」。
应用后会自动重启 plasmashell（约 2 秒），视频壁纸立即生效。

### 引用已有的壁纸文件夹（不复制）

如果壁纸已经放在某个目录里（例如 `/home/you/文档/动态壁纸`），不用导入：

1. **壁纸库** → **＋ 添加壁纸目录** → 选那个目录；
2. 目录里的裸视频（`*.mp4/webm/mkv…`）和含 `project.json` 的壁纸文件夹会直接出现在列表里，
   卡片左上角标「外部」；
3. 缩略图由 ffmpeg 抽帧缓存到 `~/.local/share/4ywallpaper/thumbs/`，原目录不会被写入；
4. 不想用了点目录右侧的 ✕ 取消引用（**不会删除任何文件**）。

命令行等价操作：

```bash
# 查看扫描结果（JSON），排查「为什么绑定不上」
fourywallpaper --list
# 一次性补齐所有外部视频的缩略图
fourywallpaper --thumbs
```

目录引用保存在 `~/.config/4ywallpaper/config.json` 的 `settings.source_dirs`。

## 构建引擎（可选，仅 scene/web 壁纸需要）

默认后端（Plasma 插件）**完全不需要引擎**。只有想用 Wallpaper Engine 的
scene/web 壁纸时才需要构建：

```bash
./scripts/build-engine.sh          # 自动 clone 上游 + 打补丁 + 编译（默认 WPENGINE_ENABLE_WEB=OFF）
```

引擎编译需要 `libmpv-dev` 与 ffmpeg 开发包。**无 root 环境**下可用离线前缀：

```bash
# 以 Debian 13 为例，从镜像下载开发包解包到本地前缀（不需要 sudo）
mkdir -p /tmp/pkgs && cd /tmp/pkgs
for p in libmpv-dev libavcodec-dev libavformat-dev libavutil-dev libswscale-dev libswresample-dev; do
  apt-get download "$p" 2>/dev/null || curl -LO "https://mirrors.tuna.tsinghua.edu.cn/debian/pool/main/..."
done
for f in *.deb; do dpkg -x "$f" "$HOME/opt/wedev"; done

export WPENGINE_DEPS_PREFIX="$HOME/opt/wedev"
export WPENGINE_ENABLE_WEB=OFF
./scripts/build-engine.sh
```

安装产物（`~/.local/share/4ywallpaper/bin/`）已用 `patchelf --set-rpath '$ORIGIN'`
自包含，**构建完成后引擎源码目录可以直接删除**。

### CEF（web 壁纸）说明

`WPENGINE_ENABLE_WEB=ON` 需要 CEF `135.0.17+gcbc1c5b+chromium-135.0.7049.52` 的开发包
（头文件 + `libcef.so` + `libcef_dll` wrapper 源码），上游通过
`https://cef-builds.spotifycdn.com/` 下载。该域名不可达时无法开启 web 壁纸；
**纯视频壁纸完全不需要 CEF**，默认构建即为 `WPENGINE_ENABLE_WEB=OFF`。

## 架构说明

- **plasma 后端（默认）**：`plasma.rs` 把「屏幕 → 视频」写进 Plasma 桌面配置
  （`d.wallpaperPlugin = "org.foury.videowall"` + `writeConfig("videoPath", …)`），
  再 `systemctl --user restart plasma-plasmashell` 让 QML 缓存失效。
- **engine 后端（可选）**：`engine.rs` 以独立进程组启动引擎，stdout/stderr 重定向到
  `~/.local/share/4ywallpaper/logs/engine.log`（**不能用管道**，否则 GUI 退出后引擎会收到
  SIGPIPE 被杀）。
- 壁纸库：`~/.local/share/4ywallpaper/library/<id>/`（`project.json` + 视频 + `thumb.jpg`）
- 配置：`~/.config/4ywallpaper/config.json`（解析失败自动备份为 `config.json.broken`）
- 自启：`~/.config/autostart/4ywallpaper.desktop`（登录后执行 `fourywallpaper --autostart`；
  plasma 后端下只需确保插件已安装，壁纸由 Plasma 自己恢复）
- 插件安装位置：`~/.local/share/plasma/wallpapers/org.foury.videowall/`

## 排错

| 现象 | 原因 / 处理 |
| --- | --- |
| 改了 `main.qml` 不生效 | plasmashell 缓存已编译的 `.qmlc` 并在内存中持有句柄，必须 `systemctl --user restart plasma-plasmashell` |
| 「没有检测到已启用的显示器」 | `kscreen-doctor -o` 输出带 ANSI 颜色码，已修复；若仍出现请贴输出 |
| 壁纸盖住桌面图标 | 说明用的是 engine 后端，设置里切回 `plasma` |
| 引擎起不来 / `cannot open shared object file` | 引擎产物是「主程序 + `liblinux-wallpaperengine-lib.so` + `libkissfft-float.so`」整个目录，`build-engine.sh` 会一起安装并设 `$ORIGIN` RUNPATH |
| 视频不动 / 黑屏 | 确认 `~/.local/share/plasma/wallpapers/org.foury.videowall/contents/ui/main.qml` 存在，然后重启 plasmashell |
| 插件目录被删了 | 设置页点「安装/修复插件」，或跑 `fourywallpaper --apply` |

## 许可证

本项目代码（`app/`、`scripts/`、`patches/`）以 MIT 发布。
动态壁纸引擎 [linux-wallpaperengine](https://github.com/Almamu/linux-wallpaperengine)
为独立项目，遵循 **GPLv3**，通过运行时进程调用 / 源码补丁方式使用，未静态链接。
Wallpaper Engine 的素材与创意工坊内容版权归其各自作者所有。
