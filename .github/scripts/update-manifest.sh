#!/bin/bash
# update-manifest.sh — Incremental manifest generation for MPE releases
#
# Called by each platform workflow after uploading artifacts.
# Downloads existing manifest.json and changelog.json, merges the current
# platform's artifacts and signatures, and re-uploads to R2.
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
DOWNLOAD_BASE="https://download.mpe.run"

echo "update-manifest: platform=${PLATFORM} version=${VERSION}"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

# ── 1. Fetch existing manifest.json or initialize ─────────────────────
aws s3 cp "s3://${R2_BUCKET}/manifest.json" "$TMP_DIR/manifest.json" \
  --endpoint-url "${R2_ENDPOINT}" --quiet 2>/dev/null || \
  echo '{"version":"","notes":"","pub_date":"","platforms":{}}' > "$TMP_DIR/manifest.json"

# ── 2. Update platform entry in manifest.json ─────────────────────────
python3 - << 'PYEOF' "$TMP_DIR/manifest.json" "$PLATFORM" "$VERSION" "$R2_ENDPOINT" "$R2_BUCKET" "$PREFIX" "$DOWNLOAD_BASE"
import json, sys, subprocess

manifest_path, platform, version, endpoint, bucket, prefix, download_base = sys.argv[1:8]

with open(manifest_path, 'r', encoding='utf-8') as f:
    try:
        manifest = json.load(f)
    except Exception:
        manifest = {}

manifest["version"] = version
manifest["notes"] = f"Release v{version}"
manifest.setdefault("platforms", {})

def fetch_sig(key):
    cmd = ["aws", "s3", "cp", f"s3://{bucket}/{prefix}/{key}", "-", "--endpoint-url", endpoint]
    res = subprocess.run(cmd, capture_output=True, text=True)
    return res.stdout.strip() if res.returncode == 0 else ""

if platform == "windows":
    sig = fetch_sig(f"mpe-{version}-x64-setup.exe.sig")
    manifest["platforms"]["windows-x86_64"] = {
        "signature": sig,
        "url": f"{download_base}/{prefix}/mpe-{version}-x64-setup.exe"
    }
elif platform == "macos":
    sig = fetch_sig(f"mpe-{version}-universal.dmg.sig")
    manifest["platforms"]["darwin-aarch64"] = {
        "signature": sig,
        "url": f"{download_base}/{prefix}/mpe-{version}-universal.dmg"
    }
    manifest["platforms"]["darwin-x86_64"] = {
        "signature": sig,
        "url": f"{download_base}/{prefix}/mpe-{version}-universal.dmg"
    }
elif platform == "linux":
    # Check for AppImage signature if present
    sig = fetch_sig(f"mpe_{version}_amd64.AppImage.sig")
    linux_url = f"{download_base}/{prefix}/mpe_{version}_amd64.AppImage" if sig else f"{download_base}/{prefix}/mpe-v{version}-linux-x86_64.tar.gz"
    manifest["platforms"]["linux-x86_64"] = {
        "signature": sig,
        "url": linux_url
    }

from datetime import datetime, timezone
manifest["pub_date"] = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

with open(manifest_path, 'w', encoding='utf-8') as f:
    json.dump(manifest, f, indent=2, ensure_ascii=False)

print(f"  ✓ manifest.json updated with platform: {platform}")
PYEOF

# ── 3. Upload manifest.json ───────────────────────────────────────────
aws s3 cp "$TMP_DIR/manifest.json" "s3://${R2_BUCKET}/manifest.json" \
  --endpoint-url "${R2_ENDPOINT}" --cache-control "no-cache, must-revalidate" --quiet
aws s3 cp "$TMP_DIR/manifest.json" "s3://${R2_BUCKET}/${PREFIX}/manifest.json" \
  --endpoint-url "${R2_ENDPOINT}" --cache-control "no-cache, must-revalidate" --quiet
echo "  ✓ manifest.json uploaded"

# ── 4. Fetch and update changelog.json ─────────────────────────────────
aws s3 cp "s3://${R2_BUCKET}/changelog.json" "$TMP_DIR/changelog.json" \
  --endpoint-url "${R2_ENDPOINT}" --quiet 2>/dev/null || echo "[]" > "$TMP_DIR/changelog.json"

python3 - << 'PYEOF' "$TMP_DIR/changelog.json" "$PLATFORM" "$VERSION" "$PREFIX" "$DOWNLOAD_BASE" "$R2_ENDPOINT" "$R2_BUCKET"
import json, sys, re, os, subprocess
from datetime import datetime, timezone

changelog_path, platform, version, prefix, download_base, endpoint, bucket = sys.argv[1:8]

def fetch_sig(key):
    cmd = ["aws", "s3", "cp", f"s3://{bucket}/{prefix}/{key}", "-", "--endpoint-url", endpoint]
    res = subprocess.run(cmd, capture_output=True, text=True)
    return res.stdout.strip() if res.returncode == 0 else ""

with open(changelog_path, 'r', encoding='utf-8') as f:
    try:
        changelog = json.load(f)
        if not isinstance(changelog, list):
            changelog = []
    except Exception:
        changelog = []

tag = f"v{version}"
entry = next((item for item in changelog if item.get("version") == version or item.get("tag") == tag), None)

notes = None
repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
changelog_md = os.path.join(repo_root, "CHANGELOG.md")
if not os.path.exists(changelog_md):
    # 尝试当前执行目录
    changelog_md = os.path.abspath("CHANGELOG.md")

if os.path.exists(changelog_md):
    try:
        with open(changelog_md, "r", encoding="utf-8") as f:
            md_text = f.read()
        clean_ver = re.escape(version.lstrip("v"))
        m = re.search(rf"## \[(?:v)?{clean_ver}\][^\r\n]*(?:\r?\n)([\s\S]*?)(?=(?:\r?\n)## |\Z)", md_text)
        if m and m.group(1).strip():
            notes = m.group(1).strip()
        else:
            # 检查 [Unreleased] 是否有内容，若有则自动转换为目标版本小节
            unreleased_m = re.search(r"## \[Unreleased\][^\r\n]*(?:\r?\n)([\s\S]*?)(?=(?:\r?\n)## |\Z)", md_text)
            if unreleased_m and unreleased_m.group(1).strip():
                notes = unreleased_m.group(1).strip()
                today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
                eol = "\r\n" if "\r\n" in md_text else "\n"
                new_section = f"## [Unreleased]{eol}{eol}## [{version.lstrip('v')}] - {today}{eol}{eol}{notes}{eol}"
                new_md = re.sub(r"## \[Unreleased\][^\r\n]*(?:\r?\n)[\s\S]*?(?=(?:\r?\n)## |\Z)", new_section, md_text, count=1)
                with open(changelog_md, "w", encoding="utf-8") as f:
                    f.write(new_md)
                print(f"  ✓ CHANGELOG.md: [Unreleased] auto-promoted to [{version.lstrip('v')}] - {today}")
    except Exception as e:
        print(f"  ⚠️ Failed to parse CHANGELOG.md: {e}", file=sys.stderr)

if not notes:
    print(f"❌ Error: CHANGELOG.md does not contain notes for [{version}] and [Unreleased] is empty! Refusing to release.", file=sys.stderr)
    sys.exit(1)
if not entry:
    entry = {
        "version": version,
        "tag": tag,
        "date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "notes": notes,
        "prerelease": bool(re.search(r'(beta|alpha)', version)),
        "downloads": {},
        "signatures": {}
    }
    changelog.insert(0, entry)
elif notes and (not entry.get("notes") or entry.get("notes", "").startswith("Release ")):
    entry["notes"] = notes

entry.setdefault("downloads", {})
entry.setdefault("signatures", {})

if platform == "windows":
    entry["downloads"]["windows"] = f"{download_base}/{prefix}/mpe-{version}-x64-setup.exe"
    sig = fetch_sig(f"mpe-{version}-x64-setup.exe.sig")
    if sig:
        entry["signatures"]["windows"] = sig
elif platform == "macos":
    entry["downloads"]["macos"] = f"{download_base}/{prefix}/mpe-{version}-universal.dmg"
    sig = fetch_sig(f"mpe-{version}-universal.dmg.sig")
    if sig:
        entry["signatures"]["macos"] = sig
elif platform == "linux":
    entry["downloads"]["linux"] = f"{download_base}/{prefix}/mpe-v{version}-linux-x86_64.tar.gz"
    entry["downloads"]["linux_deb"] = f"{download_base}/{prefix}/mpe_{version}_amd64.deb"
    entry["downloads"]["linux_appimage"] = f"{download_base}/{prefix}/mpe_{version}_amd64.AppImage"
    sig = fetch_sig(f"mpe_{version}_amd64.AppImage.sig")
    if sig:
        entry["signatures"]["linux"] = sig

with open(changelog_path, 'w', encoding='utf-8') as f:
    json.dump(changelog, f, indent=2, ensure_ascii=False)

print(f"  ✓ changelog.json updated for platform: {platform}")
PYEOF

# ── 5. Upload changelog.json ──────────────────────────────────────────
aws s3 cp "$TMP_DIR/changelog.json" "s3://${R2_BUCKET}/changelog.json" \
  --endpoint-url "${R2_ENDPOINT}" --cache-control "no-cache, must-revalidate" --quiet
echo "  ✓ changelog.json uploaded"
echo "update-manifest: done"
