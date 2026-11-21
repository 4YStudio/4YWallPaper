#!/usr/bin/env bash
# 一键安装/更新 4YWallPaper 到当前用户环境（不需要 root）
#
# 做的事：
#   1. 构建前端 + Tauri 应用（release）
#   2. 安装 Plasma 壁纸插件到 ~/.local/share/plasma/wallpapers/org.foury.videowall
#   3. 按现有配置应用壁纸（重启 plasmashell 使插件生效）
#
# 用法：
#   ./scripts/install.sh              # 构建 + 安装 + 应用
#   ./scripts/install.sh --no-apply   # 只构建安装，不动当前壁纸
set -euo pipefail
cd "$(dirname "$0")/.."

# node 可能装在 nvm 里
if ! command -v node >/dev/null 2>&1; then
  for d in "$HOME"/.nvm/versions/node/*/bin; do
    [ -x "$d/node" ] && export PATH="$d:$PATH" && break
  done
fi
export PATH="$HOME/.cargo/bin:$PATH"

echo "==> 1/3 构建应用"
(cd app && npm install --silent && npm run tauri build -- --no-bundle)

BIN="app/src-tauri/target/release/fourywallpaper"
[ -x "$BIN" ] || { echo "!! 构建产物缺失: $BIN"; exit 1; }

echo "==> 2/3 安装 Plasma 壁纸插件并应用壁纸"
"$PWD/$BIN" --apply

echo "==> 3/3 完成"
echo "    GUI 启动: $PWD/$BIN"
echo "    打包 deb: cd app && npm run tauri build"
