#!/usr/bin/env bash
set -euo pipefail

# scripts/macos/build.sh
# macOS 本地构建脚本：构建完整版 DMG 安装包 (内置 GUI + CLI 二合一)
#
# 用法:
#   ./scripts/macos/build.sh [version] [--universal | --current-arch]
#
# 示例:
#   ./scripts/macos/build.sh 0.2.0-beta.1
#   ./scripts/macos/build.sh --current-arch

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

BUILD_TARGET="universal-apple-darwin"
SPECIFIED_VERSION=""

# === 参数解析 ===
for arg in "$@"; do
  case "$arg" in
    --universal)
      BUILD_TARGET="universal-apple-darwin"
      ;;
    --current-arch)
      BUILD_TARGET=""
      ;;
    -h|--help)
      echo "用法: $0 [version] [--universal | --current-arch]"
      echo ""
      echo "选项:"
      echo "  [version]         指定发布版本号 (如 0.2.0-beta.1)，省略则从 package.json 读取"
      echo "  --universal       构建 Universal DMG (支持 Apple Silicon 与 Intel，默认)"
      echo "  --current-arch    仅构建当前主机的架构版本 (速度较快)"
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
echo "  MPE macOS 本地构建 (完整版 DMG)"
echo "  版本: v${VERSION}"
echo "  目标: ${BUILD_TARGET:-当前架构}"
echo "========================================="

# === 1. 版本号同步注入 (Node 实现，避免 macOS BSD sed 语法不兼容) ===
if [[ -n "$SPECIFIED_VERSION" && "$SPECIFIED_VERSION" != "$CURRENT_VERSION" ]]; then
  echo "==> 正在同步版本号: $VERSION"
  node -e "
    const fs = require('fs');
    const version = '$VERSION';

    // package.json
    let pkg = JSON.parse(fs.readFileSync('package.json', 'utf8'));
    pkg.version = version;
    fs.writeFileSync('package.json', JSON.stringify(pkg, null, 2) + '\n');

    // src-tauri/tauri.conf.json
    let tauri = JSON.parse(fs.readFileSync('src-tauri/tauri.conf.json', 'utf8'));
    tauri.version = version;
    fs.writeFileSync('src-tauri/tauri.conf.json', JSON.stringify(tauri, null, 2) + '\n');

    // src-tauri/Cargo.toml
    let cargo = fs.readFileSync('src-tauri/Cargo.toml', 'utf8');
    cargo = cargo.replace(/(\[package\][\s\S]*?version\s*=\s*)\"[^\"]*\"/, '\$1\"' + version + '\"');
    fs.writeFileSync('src-tauri/Cargo.toml', cargo);

    // src-tauri/crates/mpe-cli/Cargo.toml
    let cli = fs.readFileSync('src-tauri/crates/mpe-cli/Cargo.toml', 'utf8');
    cli = cli.replace(/(\[package\][\s\S]*?version\s*=\s*)\"[^\"]*\"/, '\$1\"' + version + '\"');
    fs.writeFileSync('src-tauri/crates/mpe-cli/Cargo.toml', cli);
  "
  echo "  ✓ package.json, tauri.conf.json, Cargo.toml 版本同步完成"
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

# === 2. 检查 Universal 构建环境 ===
if [[ "$BUILD_TARGET" == "universal-apple-darwin" ]]; then
  if command -v rustup &>/dev/null; then
    INSTALLED_TARGETS=$(rustup target list --installed)
    if ! echo "$INSTALLED_TARGETS" | grep -q "aarch64-apple-darwin"; then
      echo "==> 正在为 Universal 安装 aarch64 架构目标..."
      rustup target add aarch64-apple-darwin
    fi
    if ! echo "$INSTALLED_TARGETS" | grep -q "x86_64-apple-darwin"; then
      echo "==> 正在为 Universal 安装 x86_64 架构目标..."
      rustup target add x86_64-apple-darwin
    fi
  fi
fi

# === 3. 前端与 Tauri 打包 ===
if [[ ! -d "node_modules" ]]; then
  echo "==> 安装前端依赖..."
  npm ci
fi

echo "==> 生成 i18n 类型..."
npm run typesafe-i18n

echo "==> 构建前端产物..."
npm run build

echo "==> 清理历史打包产物 (dmg)..."
rm -rf "$PROJECT_ROOT/src-tauri/target"/*/release/bundle/dmg \
       "$PROJECT_ROOT/src-tauri/target/release/bundle/dmg"

echo "==> 执行 Tauri DMG 打包..."
BUILD_CMD=("npx" "tauri" "build" "--bundles" "dmg" "--config" "src-tauri/tauri.conf.ci.json")
if [[ -n "$BUILD_TARGET" ]]; then
  BUILD_CMD+=("--target" "$BUILD_TARGET")
fi

"${BUILD_CMD[@]}"

# === 4. 收集产物 ===
# 严格匹配当前版本，避免把历史版本的镜像当成新版本发布出去
DMG_SRC=$(find "$PROJECT_ROOT/src-tauri/target" -type f -name "*_${VERSION}_*.dmg" -path "*/release/bundle/dmg/*" 2>/dev/null | head -n 1 || true)
if [[ -n "$DMG_SRC" && -f "$DMG_SRC" ]]; then
  cp "$DMG_SRC" "$DIST_DIR/"
  echo "  ✓ 复制 DMG: $(basename "$DMG_SRC")"
  
  # 复制对应签名文件
  if [[ -f "${DMG_SRC}.sig" ]]; then
    cp "${DMG_SRC}.sig" "$DIST_DIR/"
    echo "  ✓ 复制签名: $(basename "${DMG_SRC}.sig")"
  fi
else
  echo "❌ 未找到编译生成的 .dmg 文件！"
  exit 1
fi

# === 5. 生成 SHA256 校验和 ===
echo ""
echo "========================================="
echo "==> 生成 SHA256 校验清单"
echo "========================================="
(
  cd "$DIST_DIR"
  # 仅为当前版本的镜像计算校验和，避免混入历史版本文件
  if command -v shasum &>/dev/null; then
    shasum -a 256 *"_${VERSION}_"*.dmg > SHA256SUMS-macos.txt 2>/dev/null || true
  elif command -v sha256sum &>/dev/null; then
    sha256sum *"_${VERSION}_"*.dmg > SHA256SUMS-macos.txt 2>/dev/null || true
  fi
)

echo ""
echo "========================================="
echo " ✅ macOS 构建完成!"
echo "========================================="
echo "产物目录: $DIST_DIR"
ls -lh "$DIST_DIR"
