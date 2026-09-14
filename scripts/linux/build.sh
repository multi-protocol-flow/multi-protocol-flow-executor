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

# === 1.5 版本一致性校验 ===
# tauri.conf.json 决定安装包版本，Cargo.toml 决定程序自报版本（updater 用 CARGO_PKG_VERSION 比对）。
# 两者不一致会产出“安装包装的是新版、程序自报旧版”的包，导致更新提示反复出现。
CONSISTENCY_FAILED=false
for entry in \
  "package.json|$(node -e "console.log(JSON.parse(require('fs').readFileSync('package.json','utf8')).version)")" \
  "tauri.conf.json|$(node -e "console.log(JSON.parse(require('fs').readFileSync('src-tauri/tauri.conf.json','utf8')).version)")" \
  "Cargo.toml|$(sed -n 's/^version = "\(.*\)"/\1/p' src-tauri/Cargo.toml | head -n 1)" \
  "mpe-cli/Cargo.toml|$(sed -n 's/^version = "\(.*\)"/\1/p' src-tauri/crates/mpe-cli/Cargo.toml | head -n 1)"; do
  name="${entry%%|*}"
  value="${entry#*|}"
  if [[ "$value" != "$VERSION" ]]; then
    echo "❌ 版本号不一致: $name = ${value:-<空>}，目标版本为 $VERSION"
    CONSISTENCY_FAILED=true
  fi
done

if [[ "$CONSISTENCY_FAILED" == "true" ]]; then
  echo "   请显式传入版本号重新运行（会自动同步各文件），或手动修正后再构建。"
  exit 1
fi
echo "==> 版本一致性校验通过: v$VERSION"

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

  # 清理历史打包产物：bundle 目录会跨版本累积文件，若残留旧版本包，
  # 后续按文件名收集时可能误取旧版本产物（曾导致 v0.2.0-beta.2 发布出 beta.1 的 deb）
  echo "==> 清理历史打包产物 (deb, appimage)..."
  rm -rf "$PROJECT_ROOT/src-tauri/target/release/bundle/deb" \
         "$PROJECT_ROOT/src-tauri/target/release/bundle/appimage"

  echo "==> 执行 Tauri 打包 (deb, appimage)..."
  npx tauri build --bundles deb,appimage

  # 收集 deb（严格匹配当前版本，避免误取历史构建产物）
  DEB_SRC=$(find "$PROJECT_ROOT/src-tauri/target/release/bundle/deb" -maxdepth 1 -name "*_${VERSION}_*.deb" 2>/dev/null | head -n 1 || true)
  if [[ -z "$DEB_SRC" ]]; then
    echo "❌ bundle/deb 中未找到版本 ${VERSION} 的 deb 产物，拒绝继续"
    exit 1
  fi
  cp "$DEB_SRC" "$DIST_DIR/"
  echo "  ✓ 复制 DEB: $(basename "$DEB_SRC")"

  # 收集 AppImage（严格匹配当前版本）
  APPIMAGE_SRC=$(find "$PROJECT_ROOT/src-tauri/target/release/bundle/appimage" -maxdepth 1 -name "*_${VERSION}_*.AppImage" 2>/dev/null | head -n 1 || true)
  if [[ -z "$APPIMAGE_SRC" ]]; then
    echo "❌ bundle/appimage 中未找到版本 ${VERSION} 的 AppImage 产物，拒绝继续"
    exit 1
  fi
  cp "$APPIMAGE_SRC" "$DIST_DIR/"
  echo "  ✓ 复制 AppImage: $(basename "$APPIMAGE_SRC")"

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
  # 仅为当前版本的产物计算校验和，避免混入历史版本文件
  CHECKSUM_TARGETS=()
  if [[ -f "mpe-v${VERSION}-linux-x86_64.tar.gz" ]]; then
    CHECKSUM_TARGETS+=("mpe-v${VERSION}-linux-x86_64.tar.gz")
  fi
  for f in *"_${VERSION}_"*.deb *"_${VERSION}_"*.AppImage; do
    if [[ -f "$f" ]]; then
      CHECKSUM_TARGETS+=("$f")
    fi
  done
  if [[ ${#CHECKSUM_TARGETS[@]} -gt 0 ]]; then
    sha256sum "${CHECKSUM_TARGETS[@]}" > SHA256SUMS-linux.txt
  else
    echo "⚠️  未找到版本 ${VERSION} 的产物，跳过校验和生成"
  fi
)

echo ""
echo "========================================="
echo " ✅ Linux 构建完成!"
echo "========================================="
echo "产物目录: $DIST_DIR"
ls -lh "$DIST_DIR"
