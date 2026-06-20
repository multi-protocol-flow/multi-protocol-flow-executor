#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# === 加载 .env ===
ENV_FILE="$PROJECT_ROOT/.env"
if [ ! -f "$ENV_FILE" ]; then
  echo "❌ .env not found"
  exit 1
fi

set -a
source "$ENV_FILE"
set +a

# 设置 AWS 凭证 (R2 兼容 S3)
export AWS_ACCESS_KEY_ID="${R2_ACCESS_KEY_ID}"
export AWS_SECRET_ACCESS_KEY="${R2_SECRET_ACCESS_KEY}"
export AWS_DEFAULT_REGION="auto"

# === 参数 ===
VERSION="${1:?Usage: ./scripts/upload-release.sh <version> [-n notes]}"
NOTES="${3:-Release v$VERSION}"
VERSION="${VERSION#v}"
TAG="v$VERSION"

R2_ENDPOINT="https://${R2_ACCOUNT_ID}.r2.cloudflarestorage.com"
BASE_URL="https://mpe-downloads.sunanzhi.com"

echo "========================================="
echo " Upload to R2: $TAG"
echo "========================================="

# === 根据平台定位产物 ===
DIST_DIR="$PROJECT_ROOT/dist-release"
mkdir -p "$DIST_DIR"

case "$(uname -s)" in
  Linux)
    ARTIFACT_DIR="$PROJECT_ROOT/src-tauri/target/release/bundle/nsis"
    EXE_FILE=$(ls "$ARTIFACT_DIR"/*.exe 2>/dev/null | head -1)
    SIG_FILE=$(ls "$ARTIFACT_DIR"/*.exe.sig 2>/dev/null | head -1)
    ASSET_EXE="mpe-${VERSION}-x64-setup.exe"
    ASSET_SIG="mpe-${VERSION}-x64-setup.exe.sig"
    ;;
  Darwin)
    ARTIFACT_DIR="$PROJECT_ROOT/src-tauri/target/universal-apple-darwin/release/bundle/dmg"
    DMG_FILE=$(ls "$ARTIFACT_DIR"/*.dmg 2>/dev/null | head -1)
    SIG_FILE=$(ls "$ARTIFACT_DIR"/*.dmg.sig 2>/dev/null | head -1)
    ASSET_DMG="mpe-${VERSION}-universal.dmg"
    ASSET_SIG="mpe-${VERSION}-universal.dmg.sig"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    ARTIFACT_DIR="$PROJECT_ROOT/src-tauri/target/release/bundle/nsis"
    EXE_FILE=$(ls "$ARTIFACT_DIR"/*.exe 2>/dev/null | head -1)
    SIG_FILE=$(ls "$ARTIFACT_DIR"/*.exe.sig 2>/dev/null | head -1)
    ASSET_EXE="mpe-${VERSION}-x64-setup.exe"
    ASSET_SIG="mpe-${VERSION}-x64-setup.exe.sig"
    ;;
esac

# === 1. 复制产物 ===
echo "==> Copying artifacts"
if [ -n "$EXE_FILE" ]; then
  cp "$EXE_FILE" "$DIST_DIR/$ASSET_EXE"
  echo "  ✓ $ASSET_EXE"
fi
if [ -n "$SIG_FILE" ]; then
  cp "$SIG_FILE" "$DIST_DIR/$ASSET_SIG"
  echo "  ✓ $ASSET_SIG"
fi
if [ -n "$DMG_FILE" ]; then
  cp "$DMG_FILE" "$DIST_DIR/$ASSET_DMG"
  echo "  ✓ $ASSET_DMG"
fi

# === 2. 上传到 R2 ===
echo "==> Uploading to R2"
R2_KEY_EXE="releases/$TAG/$ASSET_EXE"
R2_KEY_SIG="releases/$TAG/$ASSET_SIG"

if [ -f "$DIST_DIR/$ASSET_EXE" ]; then
  aws s3 cp "$DIST_DIR/$ASSET_EXE" "s3://${R2_BUCKET}/$R2_KEY_EXE" --endpoint-url "$R2_ENDPOINT"
  echo "  ✓ $R2_KEY_EXE"
fi

if [ -f "$DIST_DIR/$ASSET_SIG" ]; then
  aws s3 cp "$DIST_DIR/$ASSET_SIG" "s3://${R2_BUCKET}/$R2_KEY_SIG" --endpoint-url "$R2_ENDPOINT"
  echo "  ✓ $R2_KEY_SIG"
fi

# macOS .dmg 上传
if [ -n "$DMG_FILE" ]; then
  R2_KEY_DMG="releases/$TAG/$ASSET_DMG"
  aws s3 cp "$DIST_DIR/$ASSET_DMG" "s3://${R2_BUCKET}/$R2_KEY_DMG" --endpoint-url "$R2_ENDPOINT"
  echo "  ✓ $R2_KEY_DMG"
fi

# === 3. 读取签名 ===
WIN_SIG=""
MAC_SIG=""
if [ -f "$DIST_DIR/$ASSET_SIG" ]; then
  WIN_SIG=$(cat "$DIST_DIR/$ASSET_SIG" | tr -d '\n')
fi

# macOS dmg.sig
DMG_SIG_FILE=$(ls "$ARTIFACT_DIR"/*.dmg.sig 2>/dev/null | head -1)
if [ -n "$DMG_SIG_FILE" ]; then
  MAC_SIG=$(cat "$DMG_SIG_FILE" | tr -d '\n')
fi

# === 4. 生成 manifest.json ===
echo "==> Generating manifest.json"
PUB_DATE=$(date -u +%Y-%m-%dT%H:%M:%SZ)

cat > "$DIST_DIR/manifest.json" << EOF
{
  "version": "$VERSION",
  "notes": "$NOTES",
  "pub_date": "$PUB_DATE",
  "platforms": {
    "windows-x86_64": {
      "signature": "$WIN_SIG",
      "url": "$BASE_URL/releases/$TAG/$ASSET_EXE"
    },
    "darwin-aarch64": {
      "signature": "$MAC_SIG",
      "url": "$BASE_URL/releases/$TAG/$ASSET_DMG"
    },
    "darwin-x86_64": {
      "signature": "$MAC_SIG",
      "url": "$BASE_URL/releases/$TAG/$ASSET_DMG"
    }
  }
}
EOF

aws s3 cp "$DIST_DIR/manifest.json" "s3://${R2_BUCKET}/manifest.json" --endpoint-url "$R2_ENDPOINT" --cache-control "no-cache, must-revalidate"
echo "  ✓ manifest.json"

# === 5. 更新 changelog.json ===
echo "==> Updating changelog.json"

# 下载现有 changelog
aws s3 cp "s3://${R2_BUCKET}/changelog.json" "$DIST_DIR/changelog.json" --endpoint-url "$R2_ENDPOINT" 2>/dev/null || echo "[]" > "$DIST_DIR/changelog.json"

# 判断 prerelease
IS_PRERELEASE=false
echo "$TAG" | grep -qE '(beta|alpha)' && IS_PRERELEASE=true

# 构建新条目
cat > "$DIST_DIR/new_entry.json" << ENTRY
{
  "version": "$VERSION",
  "tag": "$TAG",
  "date": "$PUB_DATE",
  "notes": "$NOTES",
  "prerelease": $IS_PRERELEASE,
  "downloads": {
    "windows": "$BASE_URL/releases/$TAG/$ASSET_EXE",
    "macos": "$BASE_URL/releases/$TAG/$ASSET_DMG"
  }
}
ENTRY

# 添加到 changelog 开头
jq -s '.[1] + [.[0]]' "$DIST_DIR/new_entry.json" "$DIST_DIR/changelog.json" > "$DIST_DIR/changelog_new.json"
mv "$DIST_DIR/changelog_new.json" "$DIST_DIR/changelog.json"

aws s3 cp "$DIST_DIR/changelog.json" "s3://${R2_BUCKET}/changelog.json" --endpoint-url "$R2_ENDPOINT" --cache-control "no-cache, must-revalidate"
echo "  ✓ changelog.json"

rm -f "$DIST_DIR/new_entry.json"

# === 完成 ===
echo ""
echo "========================================="
echo " Done!"
echo "========================================="
echo ""
echo "Download: $BASE_URL/releases/$TAG/$ASSET_EXE"
echo "Manifest: $BASE_URL/manifest.json"
echo "Changelog: $BASE_URL/changelog.json"
