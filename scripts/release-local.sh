#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# === 加载 .env ===
ENV_FILE="$PROJECT_ROOT/.env"
if [ ! -f "$ENV_FILE" ]; then
  echo "❌ .env 文件不存在，请复制 .env.example 为 .env 并填写配置"
  echo "   cp .env.example .env"
  exit 1
fi

set -a
source "$ENV_FILE"
set +a

# === 参数检查 ===
VERSION="${1:?用法: ./scripts/release-local.sh <version>}"
VERSION="${VERSION#v}"  # 去掉可能的 v 前缀

echo "========================================="
echo " MPE 本地构建 v${VERSION}"
echo "========================================="

# === 校验并准备 CHANGELOG.md（无更新说明且 [Unreleased] 为空时不给发布；若有 [Unreleased] 则自动转换）
node "$PROJECT_ROOT/scripts/prepare-changelog.js" "$VERSION" >/dev/null


# === 1. 版本注入 ===
echo "==> 注入版本号: $VERSION"

# package.json
node -e "
const fs = require('fs');
let pkg = JSON.parse(fs.readFileSync('package.json', 'utf8'));
pkg.version = '$VERSION';
fs.writeFileSync('package.json', JSON.stringify(pkg, null, 2) + '\n');
console.log('  ✓ package.json');
"

# tauri.conf.json
node -e "
const fs = require('fs');
let tauri = JSON.parse(fs.readFileSync('src-tauri/tauri.conf.json', 'utf8'));
tauri.version = '$VERSION';
fs.writeFileSync('src-tauri/tauri.conf.json', JSON.stringify(tauri, null, 2) + '\n');
console.log('  ✓ tauri.conf.json');
"

# Cargo.toml
if [[ "$OSTYPE" == "darwin"* ]]; then
  sed -i '' "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" src-tauri/Cargo.toml
else
  sed -i "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" src-tauri/Cargo.toml
fi
echo "  ✓ Cargo.toml"

# === 2. 前端构建 ===
echo "==> 安装前端依赖"
npm ci

echo "==> 生成 i18n 类型"
npm run typesafe-i18n

# === 3. 检查签名环境变量 ===
if [ -n "$TAURI_SIGNING_PRIVATE_KEY" ]; then
  echo "==> 签名密钥已配置"
else
  echo "⚠️  未配置 TAURI_SIGNING_PRIVATE_KEY，产物将不会签名（不支持自动更新）"
fi

# === 4. 平台构建 ===
DIST_DIR="$PROJECT_ROOT/dist-release"
mkdir -p "$DIST_DIR"

case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*|Windows_NT)
    echo "==> 构建 Windows 安装包 (NSIS)"
    cargo tauri build --bundles nsis --config src-tauri/tauri.conf.ci.json
    
    NSIS_DIR="src-tauri/target/release/bundle/nsis"
    # 仅复制当前版本的产物，避免把 bundle 目录中历史版本的安装包带入发布目录
    cp "$NSIS_DIR"/*"$VERSION"*.exe "$DIST_DIR/" 2>/dev/null || true
    cp "$NSIS_DIR"/*"$VERSION"*.exe.sig "$DIST_DIR/" 2>/dev/null || true

    ARTIFACT=$(ls "$DIST_DIR"/*"$VERSION"*.exe 2>/dev/null | head -1)
    ;;
    
  Darwin)
    echo "==> 构建 macOS DMG (universal)"
    cargo tauri build --target universal-apple-darwin --bundles dmg --config src-tauri/tauri.conf.ci.json
    
    DMG_DIR="src-tauri/target/universal-apple-darwin/release/bundle/dmg"
    # 仅复制当前版本的产物
    cp "$DMG_DIR"/*"$VERSION"*.dmg "$DIST_DIR/" 2>/dev/null || true
    cp "$DMG_DIR"/*"$VERSION"*.dmg.sig "$DIST_DIR/" 2>/dev/null || true

    ARTIFACT=$(ls "$DIST_DIR"/*"$VERSION"*.dmg 2>/dev/null | head -1)
    ;;
    
  Linux)
    echo "==> 构建 Linux CLI"
    cd src-tauri && cargo build -p mpe-cli --release && cd ..
    
    cp src-tauri/target/release/mpe-cli "$DIST_DIR/mpe"
    tar czf "$DIST_DIR/mpe-v${VERSION}-linux-x86_64.tar.gz" -C "$DIST_DIR" mpe
    rm "$DIST_DIR/mpe"
    
    ARTIFACT="$DIST_DIR/mpe-v${VERSION}-linux-x86_64.tar.gz"
    ;;
esac

echo ""
echo "========================================="
echo " ✅ 构建完成!"
echo "========================================="
echo ""
echo "产物目录: $DIST_DIR"
ls -lh "$DIST_DIR"/* 2>/dev/null || true

# === 5. 可选: 上传到 R2 ===
if [ "${UPLOAD_TO_R2:-false}" = "true" ] && [ -n "$R2_ACCOUNT_ID" ]; then
  echo ""
  echo "==> 上传到 R2"
  R2_ENDPOINT="https://${R2_ACCOUNT_ID}.r2.cloudflarestorage.com"
  
  for file in "$DIST_DIR"/*; do
    filename=$(basename "$file")
    # 仅上传属于当前版本的产物，避免把 dist-release 中历史版本文件一并发布
    if [[ "$filename" != *"$VERSION"* ]]; then
      echo "  跳过历史版本产物: $filename"
      continue
    fi
    echo "  上传: $filename"
    aws s3 cp "$file" "s3://${R2_BUCKET}/releases/v${VERSION}/${filename}" \
      --endpoint-url "$R2_ENDPOINT" \
      --quiet
  done
  
  echo "✅ 上传完成"
  echo "   URL: https://mpe-downloads.sunanzhi.com/releases/v${VERSION}/"
fi

echo ""
echo "提示: 设置 UPLOAD_TO_R2=true 可自动上传到 R2"
