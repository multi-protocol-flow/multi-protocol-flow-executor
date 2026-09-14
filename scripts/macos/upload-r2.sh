#!/usr/bin/env bash
set -euo pipefail

# scripts/macos/upload-r2.sh
# 将本地构建的 macOS DMG 完整版产物上传到 Cloudflare R2，并增量更新 manifest.json 与 changelog.json
#
# 用法:
#   ./scripts/macos/upload-r2.sh [version]
#
# 示例:
#   ./scripts/macos/upload-r2.sh 0.2.0-beta.1

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# === 1. 依赖工具检查 ===
if ! command -v aws &>/dev/null; then
  echo "❌ 未检测到 aws-cli！"
  echo "   请先在 macOS 上安装 aws-cli，例如:"
  echo "   brew install awscli 或从 AWS 官网下载 pkg 安装包"
  exit 1
fi

if ! command -v node &>/dev/null; then
  echo "❌ 未检测到 node 环境！"
  exit 1
fi

# === 2. 加载环境变量 ===
ENV_FILE="$PROJECT_ROOT/.env"
if [[ -f "$ENV_FILE" ]]; then
  set -a
  source "$ENV_FILE"
  set +a
fi

MISSING_VARS=()
[[ -z "${R2_ACCOUNT_ID:-}" ]] && MISSING_VARS+=("R2_ACCOUNT_ID")
[[ -z "${R2_ACCESS_KEY_ID:-}" ]] && MISSING_VARS+=("R2_ACCESS_KEY_ID")
[[ -z "${R2_SECRET_ACCESS_KEY:-}" ]] && MISSING_VARS+=("R2_SECRET_ACCESS_KEY")
[[ -z "${R2_BUCKET:-}" ]] && MISSING_VARS+=("R2_BUCKET")

if [[ ${#MISSING_VARS[@]} -gt 0 ]]; then
  echo "❌ 缺少必要的 R2 环境变量配置: ${MISSING_VARS[*]}"
  echo "   请在项目根目录 .env 中配置，参考 .env.example"
  exit 1
fi

export AWS_ACCESS_KEY_ID="${R2_ACCESS_KEY_ID}"
export AWS_SECRET_ACCESS_KEY="${R2_SECRET_ACCESS_KEY}"
export AWS_DEFAULT_REGION="auto"

R2_ENDPOINT="https://${R2_ACCOUNT_ID}.r2.cloudflarestorage.com"
DOWNLOAD_BASE="${DOWNLOAD_BASE_URL:-https://download.mpe.run}"

# === 3. 确定版本号 ===
SPECIFIED_VERSION="${1:-}"
if [[ -z "$SPECIFIED_VERSION" ]]; then
  SPECIFIED_VERSION=$(node -e "console.log(JSON.parse(require('fs').readFileSync('package.json', 'utf8')).version)")
fi
VERSION="${SPECIFIED_VERSION#v}"
TAG="v${VERSION}"
PREFIX="releases/${TAG}"

DIST_DIR="$PROJECT_ROOT/dist-release"
if [[ ! -d "$DIST_DIR" ]]; then
  echo "❌ 产物目录不存在: $DIST_DIR，请先运行构建脚本！"
  exit 1
fi

echo "========================================="
echo "  MPE macOS 产物上传到 Cloudflare R2"
echo "  版本: ${TAG}"
echo "  Bucket: ${R2_BUCKET}"
echo "  下载基址: ${DOWNLOAD_BASE}"
echo "========================================="

# === 4. 扫描待上传文件 ===
DMG_FILE=$(find "$DIST_DIR" -maxdepth 1 -name "*_${VERSION}_*.dmg" 2>/dev/null | head -n 1 || true)
CHECKSUM_FILE=$(find "$DIST_DIR" -maxdepth 1 -name "SHA256SUMS-macos.txt" 2>/dev/null | head -n 1 || true)

if [[ -z "$DMG_FILE" ]]; then
  echo "❌ 在 $DIST_DIR 中未找到版本 ${VERSION} 的 DMG，拒绝上传！"
  echo "   现有文件:"
  ls -lh "$DIST_DIR"
  exit 1
fi

UPLOAD_FILES=()
if [[ -f "$DMG_FILE" ]]; then
  UPLOAD_FILES+=("$DMG_FILE")
fi
if [[ -n "$CHECKSUM_FILE" && -f "$CHECKSUM_FILE" ]]; then
  UPLOAD_FILES+=("$CHECKSUM_FILE")
fi

# 包含签名文件
if [[ -n "$DMG_FILE" && -f "${DMG_FILE}.sig" ]]; then
  UPLOAD_FILES+=("${DMG_FILE}.sig")
fi

if [[ ${#UPLOAD_FILES[@]} -eq 0 ]]; then
  echo "⚠️  在 $DIST_DIR 中未找到版本 ${VERSION} 相关的 macOS 产物！"
  exit 1
fi

echo "==> 准备上传以下文件:"
for f in "${UPLOAD_FILES[@]}"; do
  echo "  - $(basename "$f")"
done

# === 5. 执行文件上传 ===
echo ""
echo "========================================="
echo "==> 上传产物至 R2 (${PREFIX}/)"
echo "========================================="
for f in "${UPLOAD_FILES[@]}"; do
  filename=$(basename "$f")
  dest_key="${PREFIX}/${filename}"
  echo "  --> 正在上传: ${filename} ..."
  aws s3 cp "$f" "s3://${R2_BUCKET}/${dest_key}" \
    --endpoint-url "${R2_ENDPOINT}" \
    --quiet
  echo "      ✓ https://${DOWNLOAD_BASE#https://}/${dest_key}"
done

# === 6. 增量更新 manifest.json ===
echo ""
echo "========================================="
echo "==> 增量更新 Tauri manifest.json"
echo "========================================="

TEMP_DIR=$(mktemp -d)
REMOTE_MANIFEST="$TEMP_DIR/manifest.json"

if ! aws s3 cp "s3://${R2_BUCKET}/manifest.json" "$REMOTE_MANIFEST" --endpoint-url "${R2_ENDPOINT}" --quiet 2>/dev/null; then
  echo "  ℹ️  远程未找到现有 manifest.json，初始化全新清单"
  echo '{"version":"","notes":"","pub_date":"","platforms":{},"assets":{}}' > "$REMOTE_MANIFEST"
fi

DMG_NAME=$([[ -n "$DMG_FILE" ]] && basename "$DMG_FILE" || echo "")
DMG_SIG=""
if [[ -n "$DMG_FILE" && -f "${DMG_FILE}.sig" ]]; then
  DMG_SIG=$(cat "${DMG_FILE}.sig" | tr -d '\r\n')
fi

node -e "
const fs = require('fs');
const manifestPath = '$REMOTE_MANIFEST';
let manifest = {};
try {
  manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
} catch (e) {
  manifest = {};
}

manifest.version = '$VERSION';
manifest.notes = manifest.notes || 'Release v$VERSION';
manifest.pub_date = new Date().toISOString();
manifest.platforms = manifest.platforms || {};
manifest.assets = manifest.assets || {};

const downloadBase = '$DOWNLOAD_BASE';
const prefix = '$PREFIX';
const dmgName = '$DMG_NAME';
const dmgSig = '$DMG_SIG';

if (dmgName) {
  const dmgUrl = \`\${downloadBase}/\${prefix}/\${dmgName}\`;
  
  // Tauri Updater: Universal DMG 覆盖 aarch64 与 x86_64 两端
  manifest.platforms['darwin-aarch64'] = {
    signature: dmgSig,
    url: dmgUrl
  };
  manifest.platforms['darwin-x86_64'] = {
    signature: dmgSig,
    url: dmgUrl
  };
  manifest.assets.macos = dmgUrl;
}

fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2) + '\n');
console.log('  ✓ 增量合并 macOS 平台数据完成:');
console.log('    已包含平台: ' + Object.keys(manifest.platforms).join(', '));
"

aws s3 cp "$REMOTE_MANIFEST" "s3://${R2_BUCKET}/manifest.json" \
  --endpoint-url "${R2_ENDPOINT}" \
  --cache-control "no-cache, must-revalidate" \
  --quiet
aws s3 cp "$REMOTE_MANIFEST" "s3://${R2_BUCKET}/${PREFIX}/manifest.json" \
  --endpoint-url "${R2_ENDPOINT}" \
  --cache-control "no-cache, must-revalidate" \
  --quiet
echo "  ✓ manifest.json 上传成功"

# === 7. 增量更新 changelog.json ===
echo ""
echo "========================================="
echo "==> 增量更新 changelog.json"
echo "========================================="

REMOTE_CHANGELOG="$TEMP_DIR/changelog.json"
if ! aws s3 cp "s3://${R2_BUCKET}/changelog.json" "$REMOTE_CHANGELOG" --endpoint-url "${R2_ENDPOINT}" --quiet 2>/dev/null; then
  echo "  ℹ️  远程未找到现有 changelog.json，初始化空列表"
  echo '[]' > "$REMOTE_CHANGELOG"
fi

node -e "
const fs = require('fs');
const path = require('path');
const changelogPath = '$REMOTE_CHANGELOG';
const rootChangelog = path.resolve('$PROJECT_ROOT', 'CHANGELOG.md');
let changelog = [];
try {
  changelog = JSON.parse(fs.readFileSync(changelogPath, 'utf8'));
  if (!Array.isArray(changelog)) changelog = [];
} catch (e) {
  changelog = [];
}

const version = '$VERSION';
const tag = '$TAG';
const downloadBase = '$DOWNLOAD_BASE';
const prefix = '$PREFIX';
const dmgName = '$DMG_NAME';

let notes = 'Release ' + tag;
if (fs.existsSync(rootChangelog)) {
  try {
    const md = fs.readFileSync(rootChangelog, 'utf8');
    const cleanVer = version.replace(/^v/, '').replace(/\./g, '\\\\.');
    const re = new RegExp('## \\\\[(?:v)?' + cleanVer + '\\\\][^\\\\n]*\\\\n([\\\\s\\\\S]*?)(?=\\\\n## |$)');
    const match = md.match(re);
    if (match && match[1].trim()) {
      notes = match[1].trim();
    }
  } catch (err) {
    console.warn('  ⚠️ 读取 CHANGELOG.md 失败:', err.message);
  }
}

let entry = changelog.find(item => item.version === version || item.tag === tag);
const isNew = !entry;

if (!entry) {
  entry = {
    version: version,
    tag: tag,
    date: new Date().toISOString(),
    notes: notes,
    prerelease: /(beta|alpha)/.test(version),
    downloads: {}
  };
  changelog.unshift(entry);
} else if (notes && (!entry.notes || entry.notes.startsWith('Release '))) {
  entry.notes = notes;
}

entry.downloads = entry.downloads || {};
if (dmgName) {
  entry.downloads.macos = \`\${downloadBase}/\${prefix}/\${dmgName}\`;
}

fs.writeFileSync(changelogPath, JSON.stringify(changelog, null, 2) + '\n');
console.log('  ✓ changelog.json 已更新: ' + (isNew ? ('新增 v' + version) : ('合并 v' + version + ' 下载链接')));
"

aws s3 cp "$REMOTE_CHANGELOG" "s3://${R2_BUCKET}/changelog.json" \
  --endpoint-url "${R2_ENDPOINT}" \
  --cache-control "no-cache, must-revalidate" \
  --quiet
aws s3 cp "$REMOTE_CHANGELOG" "s3://${R2_BUCKET}/${PREFIX}/changelog.json" \
  --endpoint-url "${R2_ENDPOINT}" \
  --cache-control "no-cache, must-revalidate" \
  --quiet
echo "  ✓ changelog.json 上传成功"

rm -rf "$TEMP_DIR"

echo ""
echo "========================================="
echo " ✅ 全部上传与更新完成!"
echo "========================================="
