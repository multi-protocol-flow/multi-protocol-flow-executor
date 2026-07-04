#!/bin/bash
# update-manifest.sh — Idempotent manifest generation for MPE releases
#
# Called by each platform workflow after uploading artifacts.
# 1. Uploads a .done-{platform} marker
# 2. Checks if all 3 platforms (.done-windows, .done-macos, .done-linux) are done
# 3. If all done, generates manifest.json (Tauri auto-updater) + updates changelog.json
#
# Usage: update-manifest.sh <platform> <version> <r2_endpoint> <r2_bucket>
#
# Platform: windows | macos | linux
# Version:  e.g. "0.1.1-beta.1"
#
set -euo pipefail

PLATFORM="$1"
VERSION="$2"
R2_ENDPOINT="$3"
R2_BUCKET="$4"

PREFIX="releases/v${VERSION}"
DOWNLOAD_BASE="https://mpe-downloads.sunanzhi.com"

echo "update-manifest: platform=${PLATFORM} version=${VERSION}"

# ── 1. Upload .done marker ──────────────────────────────────────────────
echo "${VERSION}" | aws s3 cp - "s3://${R2_BUCKET}/${PREFIX}/.done-${PLATFORM}" \
  --endpoint-url "${R2_ENDPOINT}" --quiet

# ── 2. Check if manifest already exists (idempotent) ────────────────────
if aws s3 ls "s3://${R2_BUCKET}/${PREFIX}/manifest.json" \
  --endpoint-url "${R2_ENDPOINT}" &>/dev/null; then
  echo "Manifest already exists at ${PREFIX}/manifest.json, skipping"
  exit 0
fi

# ── 3. Count .done files ────────────────────────────────────────────────
DONE_COUNT=$(aws s3 ls "s3://${R2_BUCKET}/${PREFIX}/.done-" \
  --endpoint-url "${R2_ENDPOINT}" 2>/dev/null | wc -l | tr -d ' ')
echo "Done files: ${DONE_COUNT}/3"
if [ "${DONE_COUNT}" -lt 3 ]; then
  echo "Not all platforms done yet, waiting for others"
  exit 0
fi

echo "All 3 platforms done, generating manifest..."

# ── 4. Download signature files ─────────────────────────────────────────
WIN_SIG=$(aws s3 cp "s3://${R2_BUCKET}/${PREFIX}/mpe-${VERSION}-x64-setup.exe.sig" - \
  --endpoint-url "${R2_ENDPOINT}" 2>/dev/null || echo "")
MAC_SIG=$(aws s3 cp "s3://${R2_BUCKET}/${PREFIX}/mpe-${VERSION}-universal.dmg.sig" - \
  --endpoint-url "${R2_ENDPOINT}" 2>/dev/null || echo "")

# ── 5. Generate manifest.json (Tauri auto-updater) ──────────────────────
cat > /tmp/manifest.json << EOF
{
  "version": "${VERSION}",
  "notes": "Release v${VERSION}",
  "pub_date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "platforms": {
    "windows-x86_64": {
      "signature": "${WIN_SIG}",
      "url": "${DOWNLOAD_BASE}/${PREFIX}/mpe-${VERSION}-x64-setup.exe"
    },
    "darwin-aarch64": {
      "signature": "${MAC_SIG}",
      "url": "${DOWNLOAD_BASE}/${PREFIX}/mpe-${VERSION}-universal.dmg"
    },
    "darwin-x86_64": {
      "signature": "${MAC_SIG}",
      "url": "${DOWNLOAD_BASE}/${PREFIX}/mpe-${VERSION}-universal.dmg"
    }
  }
}
EOF

aws s3 cp /tmp/manifest.json "s3://${R2_BUCKET}/${PREFIX}/manifest.json" \
  --endpoint-url "${R2_ENDPOINT}" --cache-control "no-cache, must-revalidate" --quiet
aws s3 cp /tmp/manifest.json "s3://${R2_BUCKET}/manifest.json" \
  --endpoint-url "${R2_ENDPOINT}" --cache-control "no-cache, must-revalidate" --quiet

echo "manifest.json uploaded"

# ── 6. Update changelog.json ────────────────────────────────────────────
aws s3 cp "s3://${R2_BUCKET}/changelog.json" /tmp/changelog.json \
  --endpoint-url "${R2_ENDPOINT}" 2>/dev/null || echo "[]" > /tmp/changelog.json

IS_PRERELEASE="false"
if echo "${VERSION}" | grep -qE '(beta|alpha)'; then
  IS_PRERELEASE="true"
fi

cat > /tmp/new_entry.json << EOF
{
  "version": "${VERSION}",
  "tag": "v${VERSION}",
  "date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "notes": "Release v${VERSION}",
  "prerelease": ${IS_PRERELEASE},
  "downloads": {
    "windows": "${DOWNLOAD_BASE}/${PREFIX}/mpe-${VERSION}-x64-setup.exe",
    "macos": "${DOWNLOAD_BASE}/${PREFIX}/mpe-${VERSION}-universal.dmg",
    "linux": "${DOWNLOAD_BASE}/${PREFIX}/mpe-v${VERSION}-linux-x86_64.tar.gz"
  }
}
EOF

python3 -c "
import json, sys
changelog = json.load(open(sys.argv[1]))
new_entry = json.load(open(sys.argv[2]))
changelog.insert(0, new_entry)
json.dump(changelog, open(sys.argv[3], 'w'), indent=2, ensure_ascii=False)
" /tmp/changelog.json /tmp/new_entry.json /tmp/changelog_new.json
aws s3 cp /tmp/changelog_new.json "s3://${R2_BUCKET}/changelog.json" \
  --endpoint-url "${R2_ENDPOINT}" --cache-control "no-cache, must-revalidate" --quiet

echo "changelog.json updated"
echo "update-manifest: done"
