#!/usr/bin/env bash
# 构建 linux-wallpaperengine 引擎并安装到 ~/.local/share/4ywallpaper/bin/
#
# 引擎是外部项目（Almamu/linux-wallpaperengine, GPLv3），不在本仓库内。
# 本脚本负责：克隆上游 → 打本项目补丁 → 编译 → 安装。
#
# 说明：
#   * 引擎产物是「主程序 + liblinux-wallpaperengine-lib.so + libkissfft」的组合，
#     必须整个目录一起安装，单独拷主程序会报
#     `error while loading shared libraries: liblinux-wallpaperengine-lib.so`。
#   * 安装后用 patchelf 把 RUNPATH 改成 $ORIGIN，这样目录可以整体搬走，
#     之后源码目录删掉也不影响运行。
#   * Web(CEF) 壁纸默认关闭：CEF 二进制包很大且部分网络环境下载不到，
#     视频/场景壁纸不需要它。需要时用 WPENGINE_ENABLE_WEB=ON 重新构建。
#
# 环境变量：
#   WPENGINE_ENABLE_WEB=ON|OFF   是否编译 Web(CEF) 壁纸支持（默认 OFF）
#   WPENGINE_DEPS_PREFIX=<path>  额外依赖前缀（如离线解包的 mpv/ffmpeg 开发包）
#   WPENGINE_REPO=<url>          引擎仓库地址（默认 GitHub 上游）
#   WPENGINE_COMMIT=<sha>        检出的上游提交（默认下面钉住的版本）
#   WPENGINE_DIR=<path>          引擎源码目录（默认仓库内 linux-wallpaperengine/）
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

WPENGINE_ENABLE_WEB="${WPENGINE_ENABLE_WEB:-OFF}"
DEPS_PREFIX="${WPENGINE_DEPS_PREFIX:-}"
ENGINE_REPO="${WPENGINE_REPO:-https://github.com/Almamu/linux-wallpaperengine.git}"
# 补丁（patches/engine-optional-cef.patch）基于这个提交，升级上游需同步 rebase
ENGINE_COMMIT="${WPENGINE_COMMIT:-a0ddce5}"
ENGINE_DIR="${WPENGINE_DIR:-$ROOT/linux-wallpaperengine}"
PATCH_FILE="$ROOT/patches/engine-optional-cef.patch"

echo "==> 0/5 准备引擎源码: $ENGINE_DIR"
if [ ! -d "$ENGINE_DIR/.git" ]; then
  echo "    未找到引擎源码，从上游克隆（约 100MB）..."
  git clone "$ENGINE_REPO" "$ENGINE_DIR"
  git -C "$ENGINE_DIR" checkout "$ENGINE_COMMIT"
fi
cd "$ENGINE_DIR"

echo "==> 1/5 初始化子模块"
git submodule update --init --recursive

echo "==> 2/5 应用本项目补丁（CEF 可选化）"
if [ -f "$PATCH_FILE" ]; then
  if git apply --check "$PATCH_FILE" 2>/dev/null; then
    git apply "$PATCH_FILE"
    echo "    已应用 $PATCH_FILE"
  elif git apply --reverse --check "$PATCH_FILE" 2>/dev/null; then
    echo "    补丁已应用过，跳过"
  else
    echo "!! 补丁无法应用（上游可能已更新），请手动检查: $PATCH_FILE" >&2
    exit 1
  fi
else
  echo "!! 找不到补丁文件 $PATCH_FILE（若上游已内置 WPENGINE_ENABLE_WEB 开关可忽略）" >&2
fi

CMAKE_ARGS=(-DCMAKE_BUILD_TYPE=Release -DWPENGINE_ENABLE_WEB="$WPENGINE_ENABLE_WEB")
if [ -n "$DEPS_PREFIX" ]; then
  CMAKE_ARGS+=(-DCMAKE_PREFIX_PATH="$DEPS_PREFIX")
  export PKG_CONFIG_PATH="$DEPS_PREFIX/lib/pkgconfig:${PKG_CONFIG_PATH:-}"
fi

echo "==> 3/5 CMake 配置 (WPENGINE_ENABLE_WEB=$WPENGINE_ENABLE_WEB)"
cmake -B build "${CMAKE_ARGS[@]}"

echo "==> 4/5 编译 (使用 $(nproc) 线程)"
cmake --build build -j "$(nproc)"

echo "==> 5/5 安装到 ~/.local/share/4ywallpaper/bin/"
DEST="$HOME/.local/share/4ywallpaper/bin"
rm -rf "$DEST"
mkdir -p "$DEST/assets"   # 引擎启动时强制挂载 assets 目录，空目录即可（视频壁纸不需要 WE 资产）

cp build/output/linux-wallpaperengine "$DEST/"
cp build/output/liblinux-wallpaperengine-lib.so "$DEST/"
cp -P build/lib/libkissfft-float.so* "$DEST/" 2>/dev/null || true

# 安装时把 RUNPATH 固定为 $ORIGIN，避免依赖构建目录的绝对路径
if command -v patchelf >/dev/null 2>&1; then
  patchelf --set-rpath '$ORIGIN' "$DEST/linux-wallpaperengine"
  patchelf --set-rpath '$ORIGIN' "$DEST/liblinux-wallpaperengine-lib.so"
else
  echo "!! 未安装 patchelf，安装后的引擎会依赖构建目录路径（sudo apt install patchelf）" >&2
fi

if command -v ldd >/dev/null 2>&1; then
  MISSING="$(ldd "$DEST/linux-wallpaperengine" | grep -c 'not found' || true)"
  if [ "$MISSING" != "0" ]; then
    echo "!! 警告：仍有 $MISSING 个动态库未找到："
    ldd "$DEST/linux-wallpaperengine" | grep 'not found' || true
  fi
fi

echo "==> 完成: $DEST"
echo "    引擎源码目录（$ENGINE_DIR）现在可以删除，安装产物已自包含。"
echo "    如构建提示缺少依赖，先执行:"
echo "    sudo apt install libmpv-dev libglew-dev freeglut3-dev libsdl2-dev liblz4-dev \\"
echo "      libavcodec-dev libavformat-dev libavutil-dev libswscale-dev libglm-dev \\"
echo "      libglfw3-dev libpulse-dev libfftw3-dev libfreetype-dev libdbus-1-dev \\"
echo "      libwayland-dev wayland-protocols patchelf"
