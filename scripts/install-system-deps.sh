#!/usr/bin/env bash
set -euo pipefail

echo "=== 安装编译与 Tauri 桌面端所需的系统底层依赖 ==="
sudo apt update
sudo apt install -y \
  build-essential \
  pkg-config \
  libssl-dev \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev

echo ""
echo "=== 系统依赖安装成功！==="
