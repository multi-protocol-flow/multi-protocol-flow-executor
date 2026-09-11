#!/usr/bin/env bash
set -euo pipefail

# scripts/linux/build.sh
# Linux 本地构建脚本：支持编译 CLI (mpe-cli) 与 GUI (deb/AppImage)
#
# 用法:
#   ./scripts/linux/build.sh [version] [--cli-only | --gui-only | --all]
#
# 示例:
#   ./scripts/linux/build.sh 0.1.2
#   ./scripts/linux/build.sh v0.1.2 --cli-only
#   ./scripts/linux/build.sh --gui-only

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

TARGET_MODE="all" # all | cli | gui
SPECIFIED_VERSION=""

# === 参数解析 ===
for arg in "$@"; do
  case "$arg" in
    --cli-only)
      TARGET_MODE="cli"
      ;;
    --gui-only)
      TARGET_MODE="gui"
      ;;
    --all)
      TARGET_MODE="all"
      ;;
    -h|--help)
      echo "用法: $0 [version] [--cli-only | --gui-only | --all]"
      echo ""
      echo "选项:"
      echo "  [version]     指定发布版本号 (如 0.1.2 或 v0.1.2)，省略则从 package.json 读取"
      echo "  --cli-only    仅构建 Linux CLI (mpe-cli, tar.gz)"
      echo "  --gui-only    仅构建 Linux GUI (Tauri deb / AppImage)"
      echo "  --all         构建全部 (默认)"
      exit 0
      ;;
    *)
      if [[ -z "$SPECIFIED_VERSION" && ! "$arg" =~ ^-- ]]; then
        SPECIFIED_VERSION="${arg#v}"
      fi
      ;;
  esac
done

cd "$PROJECT_ROOT"

# === 获取当前版本 ===
CURRENT_VERSION=$(node -e "console.log(JSON.parse(require('fs').readFileSync('package.json', 'utf8')).version)")
VERSION="${SPECIFIED_VERSION:-$CURRENT_VERSION}"

echo "========================================="
echo "  MPE Linux 本地构建"
echo "  版本: v${VERSION}"
echo "  模式: ${TARGET_MODE}"
echo "========================================="

# === 1. 版本号同步注入 (若指定了版本号且与当前不一致) ===
if [[ -n "$SPECIFIED_VERSION" && "$SPECIFIED_VERSION" != "$CURRENT_VERSION" ]]; then
  echo "==> 正在同步版本号: $VERSION"

  # package.json
  node -e "
    const fs = require('fs');
    let pkg = JSON.parse(fs.readFileSync('package.json', 'utf8'));
    pkg.version = '$VERSION';
    fs.writeFileSync('package.json', JSON.stringify(pkg, null, 2) + '\n');
  "
  echo "  ✓ package.json"

  # src-tauri/tauri.conf.json
  node -e "
    const fs = require('fs');
    let tauri = JSON.parse(fs.readFileSync('src-tauri/tauri.conf.json', 'utf8'));
    tauri.version = '$VERSION';
    fs.writeFileSync('src-tauri/tauri.conf.json', JSON.stringify(tauri, null, 2) + '\n');
  "
  echo "  ✓ src-tauri/tauri.conf.json"

  # src-tauri/Cargo.toml & mpe-cli/Cargo.toml
  sed -i "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" src-tauri/Cargo.toml
  sed -i "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" src-tauri/crates/mpe-cli/Cargo.toml
  echo "  ✓ Cargo.toml (app & mpe-cli)"
fi

DIST_DIR="$PROJECT_ROOT/dist-release"
mkdir -p "$DIST_DIR"

# === 2. 构建 CLI ===
if [[ "$TARGET_MODE" == "all" || "$TARGET_MODE" == "cli" ]]; then
  echo ""
  echo "========================================="
  echo "==> 1/2 构建 Linux CLI (mpe-cli)"
  echo "========================================="
  (
    cd "$PROJECT_ROOT/src-tauri"
    cargo build -p mpe-cli --release
  )

  CLI_BIN="$PROJECT_ROOT/src-tauri/target/release/mpe-cli"
  if [[ ! -f "$CLI_BIN" ]]; then
    echo "❌ 找不到编译出的 mpe-cli: $CLI_BIN"
    exit 1
  fi

  TMP_CLI_STAGE=$(mktemp -d)
  cp "$CLI_BIN" "$TMP_CLI_STAGE/mpe"
  chmod +x "$TMP_CLI_STAGE/mpe"

  CLI_ARCHIVE="mpe-v${VERSION}-linux-x86_64.tar.gz"
  tar -czf "$DIST_DIR/$CLI_ARCHIVE" -C "$TMP_CLI_STAGE" mpe
  rm -rf "$TMP_CLI_STAGE"

  echo "  ✓ 打包完成: $DIST_DIR/$CLI_ARCHIVE"
fi

# === 3. 构建 GUI (Tauri deb / AppImage) ===
if [[ "$TARGET_MODE" == "all" || "$TARGET_MODE" == "gui" ]]; then
  echo ""
  echo "========================================="
  echo "==> 2/2 构建 Linux GUI (Tauri deb & AppImage)"
  echo "========================================="

  # 检查必要前端环境
  if [[ ! -d "node_modules" ]]; then
    echo "==> 安装前端依赖..."
    npm ci
  fi

  echo "==> 生成 i18n 类型..."
  npm run typesafe-i18n

  echo "==> 构建前端产物..."
  npm run build

  echo "==> 执行 Tauri 打包 (deb, appimage)..."
  npx tauri build --bundles deb,appimage

  # 收集 deb
  DEB_SRC=$(find "$PROJECT_ROOT/src-tauri/target/release/bundle/deb" -name "*.deb" 2>/dev/null | head -n 1 || true)
  if [[ -n "$DEB_SRC" && -f "$DEB_SRC" ]]; then
    cp "$DEB_SRC" "$DIST_DIR/"
    echo "  ✓ 复制 DEB: $(basename "$DEB_SRC")"
  fi

  # 收集 AppImage
  APPIMAGE_SRC=$(find "$PROJECT_ROOT/src-tauri/target/release/bundle/appimage" -name "*.AppImage" 2>/dev/null | head -n 1 || true)
  if [[ -n "$APPIMAGE_SRC" && -f "$APPIMAGE_SRC" ]]; then
    cp "$APPIMAGE_SRC" "$DIST_DIR/"
    echo "  ✓ 复制 AppImage: $(basename "$APPIMAGE_SRC")"
  fi

  # 收集 sig (若存在)
  find "$PROJECT_ROOT/src-tauri/target/release/bundle" -name "*.sig" -exec cp {} "$DIST_DIR/" \; 2>/dev/null || true
fi

# === 4. 生成 SHA256 校验和 ===
echo ""
echo "========================================="
echo "==> 生成 SHA256 校验清单"
echo "========================================="
(
  cd "$DIST_DIR"
  # 仅为当前版本文件或主要产物计算
  sha256sum mpe-* *.deb *.AppImage 2>/dev/null > SHA256SUMS-linux.txt || true
)

echo ""
echo "========================================="
echo " ✅ Linux 构建完成!"
echo "========================================="
echo "产物目录: $DIST_DIR"
ls -lh "$DIST_DIR"
