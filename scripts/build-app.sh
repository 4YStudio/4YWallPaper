#!/usr/bin/env bash
# 构建 4YWallPaper 桌面应用（Tauri + Vue3）并生成 deb 包
set -euo pipefail
cd "$(dirname "$0")/../app"
npm install
npm run tauri build
echo "==> 产物位于 app/src-tauri/target/release/bundle/"
