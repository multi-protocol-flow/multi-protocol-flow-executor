#!/usr/bin/env bash
set -euo pipefail

# scripts/linux/release.sh
# 一键 Linux 本地构建与发布脚本 (Build + Upload to R2)
#
# 用法:
#   ./scripts/linux/release.sh [version] [--cli-only | --gui-only | --all] [--upload]
#
# 示例:
#   ./scripts/linux/release.sh 0.1.2 --upload
#   ./scripts/linux/release.sh --cli-only --upload

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

VERSION_ARG=""
TARGET_MODE="all"
AUTO_UPLOAD=false

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
    --upload)
      AUTO_UPLOAD=true
      ;;
    -h|--help)
      echo "用法: $0 [version] [--cli-only | --gui-only | --all] [--upload]"
      echo ""
      echo "选项:"
      echo "  [version]     版本号 (如 0.1.2)，默认读取 package.json"
      echo "  --cli-only    仅构建 CLI"
      echo "  --gui-only    仅构建 GUI"
      echo "  --all         构建全部 (默认)"
      echo "  --upload      构建完成后自动上传至 Cloudflare R2"
      exit 0
      ;;
    *)
      if [[ -z "$VERSION_ARG" && ! "$arg" =~ ^-- ]]; then
        VERSION_ARG="${arg#v}"
      fi
      ;;
  esac
done

cd "$PROJECT_ROOT"

CURRENT_VERSION=$(node -e "console.log(JSON.parse(require('fs').readFileSync('package.json', 'utf8')).version)")
VERSION="${VERSION_ARG:-$CURRENT_VERSION}"

echo "========================================="
echo " 🚀 MPE Linux 本地发布工作流"
echo " 版本: v${VERSION}"
echo " 模式: ${TARGET_MODE}"
echo " 上传: $([[ "$AUTO_UPLOAD" == "true" ]] && echo '是 (R2)' || echo '否 (仅本地)')"
echo "========================================="

# === 1. 预检查 ===
if [[ "$AUTO_UPLOAD" == "true" ]]; then
  if ! command -v aws &>/dev/null; then
    echo "❌ 指定了 --upload 但系统未安装 aws-cli！"
    echo "   请先安装: sudo apt install -y awscli"
    exit 1
  fi
  if [[ ! -f "$PROJECT_ROOT/.env" && -z "${R2_BUCKET:-}" ]]; then
    echo "❌ 指定了 --upload 但项目根目录未找到 .env 且未设置 R2 环境变量！"
    exit 1
  fi
fi

# === 2. 执行构建 ===
BUILD_ARGS=()
[[ -n "$VERSION_ARG" ]] && BUILD_ARGS+=("$VERSION_ARG")
BUILD_ARGS+=("--${TARGET_MODE}")

"$SCRIPT_DIR/build.sh" "${BUILD_ARGS[@]}"

# === 3. 上传流程 ===
if [[ "$AUTO_UPLOAD" == "true" ]]; then
  echo ""
  "$SCRIPT_DIR/upload-r2.sh" "$VERSION"
else
  echo ""
  echo "💡 提示: 若要将产物上传到 Cloudflare R2，可执行:"
  echo "   $SCRIPT_DIR/upload-r2.sh $VERSION"
  echo "   或者下次直接携带参数: $0 $VERSION --upload"
fi

echo ""
echo "========================================="
echo " 🎉 Linux 发布流程结束"
echo "========================================="
