#Requires -Version 5.1
<#
.SYNOPSIS
    将本地构建的 Windows 完整版产物上传到 Cloudflare R2，并增量更新 manifest.json 与 changelog.json
.DESCRIPTION
    用法:
      .\scripts\windows\upload-r2.ps1 [version]
    示例:
      .\scripts\windows\upload-r2.ps1 0.2.0-beta.1
#>

[CmdletBinding()]
param(
    [Parameter(Position=0)]
    [string]$Version
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $ScriptDir)

# === 1. 检查依赖 ===
if (-not (Get-Command aws -ErrorAction SilentlyContinue)) {
    Write-Host "❌ 未检测到 aws-cli！" -ForegroundColor Red
    Write-Host "   请先在 Windows 上安装 AWS CLI v2: https://awscli.amazonaws.com/AWSCLIV2.msi" -ForegroundColor Yellow
    exit 1
}

# === 2. 加载 .env 环境变量 ===
$EnvFile = Join-Path $ProjectRoot ".env"
if (Test-Path $EnvFile) {
    Get-Content $EnvFile | ForEach-Object {
        if ($_ -match '^\s*([^#][^=]+?)\s*=\s*(.*)$') {
            $name = $Matches[1].Trim()
            $val = $Matches[2].Trim() -replace '^["'']|["'']$', ''
            [Environment]::SetEnvironmentVariable($name, $val, "Process")
        }
    }
}

$R2AccountId = $env:R2_ACCOUNT_ID
$R2AccessKey = $env:R2_ACCESS_KEY_ID
$R2SecretKey = $env:R2_SECRET_ACCESS_KEY
$Bucket = $env:R2_BUCKET

if (-not $R2AccountId -or -not $R2AccessKey -or -not $R2SecretKey -or -not $Bucket) {
    Write-Host "❌ 缺少必要的 R2 环境变量配置 (R2_ACCOUNT_ID, R2_ACCESS_KEY_ID, R2_SECRET_ACCESS_KEY, R2_BUCKET)！" -ForegroundColor Red
    Write-Host "   请在项目根目录 .env 中进行配置。" -ForegroundColor Yellow
    exit 1
}

$env:AWS_ACCESS_KEY_ID = $R2AccessKey
$env:AWS_SECRET_ACCESS_KEY = $R2SecretKey
$env:AWS_DEFAULT_REGION = "auto"

$R2Endpoint = "https://$($R2AccountId).r2.cloudflarestorage.com"
$DownloadBase = if ($env:DOWNLOAD_BASE_URL) { $env:DOWNLOAD_BASE_URL.TrimEnd('/') } else { "https://download.mpe.run" }

# === 3. 确定版本号 ===
if ([string]::IsNullOrWhiteSpace($Version)) {
    $PkgJson = Get-Content (Join-Path $ProjectRoot "package.json") -Raw | ConvertFrom-Json
    $Version = $PkgJson.version
}
$Version = $Version -replace '^v', ''
$Tag = "v$Version"
$Prefix = "releases/$Tag"

$DistDir = Join-Path $ProjectRoot "dist-release"
if (-not (Test-Path $DistDir)) {
    Write-Host "❌ 产物目录不存在: $DistDir，请先运行构建脚本！" -ForegroundColor Red
    exit 1
}

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "  MPE Windows 产物上传到 Cloudflare R2" -ForegroundColor Cyan
Write-Host "  版本: $Tag" -ForegroundColor Cyan
Write-Host "  Bucket: $Bucket" -ForegroundColor Cyan
Write-Host "  下载基址: $DownloadBase" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

# === 4. 扫描待上传文件 ===
$UploadFiles = @()

$ExeFile = Get-ChildItem $DistDir -File | Where-Object { $_.Name -like "*$Version*.exe" } | Select-Object -First 1
$ChecksumFile = Join-Path $DistDir "SHA256SUMS-windows.txt"

if (-not $ExeFile) {
    Write-Host "❌ 在 $DistDir 中未找到版本 $Version 的安装包，拒绝上传！" -ForegroundColor Red
    Write-Host "   现有文件:" -ForegroundColor Yellow
    Get-ChildItem $DistDir -File | ForEach-Object { Write-Host "   - $($_.Name)" }
    exit 1
}

# 校验安装包内部版本号，杜绝把历史版本的安装包当成新版本发布出去
$ExeVersion = $ExeFile.VersionInfo.ProductVersion
if (-not [string]::IsNullOrWhiteSpace($ExeVersion)) {
    $ExeVersion = $ExeVersion -replace '^v', ''
    if ($ExeVersion -ne $Version) {
        Write-Host "❌ 安装包内部版本 ($ExeVersion) 与目标版本 ($Version) 不一致！" -ForegroundColor Red
        Write-Host "   文件: $($ExeFile.Name)" -ForegroundColor Yellow
        exit 1
    }
}

$UploadFiles += $ExeFile.FullName
$SigPath = "$($ExeFile.FullName).sig"
if (Test-Path $SigPath) { $UploadFiles += $SigPath }
if (Test-Path $ChecksumFile) { $UploadFiles += $ChecksumFile }

if ($UploadFiles.Count -eq 0) {
    Write-Host "⚠️  在 $DistDir 中未找到版本 $Version 相关的 Windows 产物！" -ForegroundColor Yellow
    exit 1
}

Write-Host "==> 准备上传以下文件:" -ForegroundColor Yellow
$UploadFiles | ForEach-Object { Write-Host "  - $(Split-Path -Leaf $_)" -ForegroundColor Cyan }

# === 5. 执行文件上传 ===
Write-Host ""
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "==> 上传产物至 R2 ($Prefix/)" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

foreach ($file in $UploadFiles) {
    $name = Split-Path -Leaf $file
    $destKey = "$Prefix/$name"
    Write-Host "  --> 正在上传: $name ..." -ForegroundColor Gray
    aws s3 cp $file "s3://$Bucket/$destKey" --endpoint-url $R2Endpoint --quiet
    if ($LASTEXITCODE -ne 0) { throw "上传 $name 失败" }
    Write-Host "      ✓ $DownloadBase/$destKey" -ForegroundColor Green
}

# === 6. 增量更新 manifest.json ===
Write-Host ""
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "==> 增量更新 Tauri manifest.json" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

$TempDir = Join-Path $env:TEMP ("mpe_manifest_" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $TempDir | Out-Null
$LocalManifest = Join-Path $TempDir "manifest.json"

try {
    aws s3 cp "s3://$Bucket/manifest.json" $LocalManifest --endpoint-url $R2Endpoint --quiet 2>$null
    if (Test-Path $LocalManifest) {
        $Manifest = Get-Content $LocalManifest -Raw -Encoding UTF8 | ConvertFrom-Json
    } else {
        $Manifest = [PSCustomObject]@{
            version = ""
            notes = ""
            pub_date = ""
            platforms = [PSCustomObject]@{}
            assets = [PSCustomObject]@{}
        }
    }

    $Manifest.version = $Version
    if (-not $Manifest.notes) { $Manifest.notes = "Release $Tag" }
    $Manifest.pub_date = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
    if (-not $Manifest.platforms) { $Manifest | Add-Member -NotePropertyName "platforms" -NotePropertyValue ([PSCustomObject]@{}) -Force }
    if (-not $Manifest.assets) { $Manifest | Add-Member -NotePropertyName "assets" -NotePropertyValue ([PSCustomObject]@{}) -Force }

    $ExeName = if ($ExeFile) { Split-Path -Leaf $ExeFile.FullName } else { "" }

    # 读取 Windows 签名
    $WinSig = ""
    if ($ExeFile -and (Test-Path "$($ExeFile.FullName).sig")) {
        $WinSig = (Get-Content "$($ExeFile.FullName).sig" -Raw).Trim()
    }

    if ($ExeName) {
        $platformObj = [PSCustomObject]@{
            signature = $WinSig
            url = "$DownloadBase/$Prefix/$ExeName"
        }
        $Manifest.platforms | Add-Member -NotePropertyName "windows-x86_64" -NotePropertyValue $platformObj -Force
        $Manifest.assets | Add-Member -NotePropertyName "windows" -NotePropertyValue "$DownloadBase/$Prefix/$ExeName" -Force
    }

    $ManifestJson = $Manifest | ConvertTo-Json -Depth 10
    [System.IO.File]::WriteAllText($LocalManifest, $ManifestJson, (New-Object System.Text.UTF8Encoding $false))

    aws s3 cp $LocalManifest "s3://$Bucket/manifest.json" --endpoint-url $R2Endpoint --cache-control "no-cache, must-revalidate" --quiet
    aws s3 cp $LocalManifest "s3://$Bucket/$Prefix/manifest.json" --endpoint-url $R2Endpoint --cache-control "no-cache, must-revalidate" --quiet
    Write-Host "  ✓ manifest.json 增量更新并上传成功" -ForegroundColor Green

    # === 7. 增量更新 changelog.json ===
    Write-Host ""
    Write-Host "=========================================" -ForegroundColor Cyan
    Write-Host "==> 增量更新 changelog.json" -ForegroundColor Cyan
    Write-Host "=========================================" -ForegroundColor Cyan

    $LocalChangelog = Join-Path $TempDir "changelog.json"
    try {
        aws s3 cp "s3://$Bucket/changelog.json" $LocalChangelog --endpoint-url $R2Endpoint --quiet 2>$null
        $Changelog = Get-Content $LocalChangelog -Raw -Encoding UTF8 | ConvertFrom-Json
        if ($Changelog -isnot [System.Array]) { $Changelog = @($Changelog) }
    } catch {
        $Changelog = @()
    }

    $entry = $Changelog | Where-Object { $_.version -eq $Version -or $_.tag -eq $Tag } | Select-Object -First 1
    $isNew = ($null -eq $entry)

    $Notes = "Release $Tag"
    $ChangelogMdPath = Join-Path $ProjectRoot "CHANGELOG.md"
    if (Test-Path $ChangelogMdPath) {
        $ChangelogMd = Get-Content $ChangelogMdPath -Raw -Encoding UTF8
        $CleanVer = [regex]::Escape($Version.TrimStart('v'))
        $Pattern = "(?s)## \[(?:v)?$CleanVer\][^\n]*\n(.*?)(?=\n## |\Z)"
        if ($ChangelogMd -match $Pattern) {
            $ExtractedNotes = $Matches[1].Trim()
            if ($ExtractedNotes) {
                $Notes = $ExtractedNotes
            }
        }
    }

    if ($isNew) {
        $isPrerelease = ($Version -match '(beta|alpha)')
        $entry = [PSCustomObject]@{
            version = $Version
            tag = $Tag
            date = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
            notes = $Notes
            prerelease = [bool]$isPrerelease
            downloads = [PSCustomObject]@{}
        }
        $Changelog = @($entry) + @($Changelog)
    } elseif ($Notes -and (-not $entry.notes -or $entry.notes.StartsWith("Release "))) {
        $entry.notes = $Notes
    }
    if (-not $entry.downloads) {
        $entry | Add-Member -NotePropertyName "downloads" -NotePropertyValue ([PSCustomObject]@{}) -Force
    }
    if ($ExeName) {
        $entry.downloads | Add-Member -NotePropertyName "windows" -NotePropertyValue "$DownloadBase/$Prefix/$ExeName" -Force
    }

    $ChangelogJson = $Changelog | ConvertTo-Json -Depth 10
    [System.IO.File]::WriteAllText($LocalChangelog, $ChangelogJson, (New-Object System.Text.UTF8Encoding $false))

    aws s3 cp $LocalChangelog "s3://$Bucket/changelog.json" --endpoint-url $R2Endpoint --cache-control "no-cache, must-revalidate" --quiet
    aws s3 cp $LocalChangelog "s3://$Bucket/$Prefix/changelog.json" --endpoint-url $R2Endpoint --cache-control "no-cache, must-revalidate" --quiet
    Write-Host "  ✓ changelog.json 已增量更新并上传成功" -ForegroundColor Green
} finally {
    Remove-Item $TempDir -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "=========================================" -ForegroundColor Green
Write-Host " ✅ 全部上传与更新完成!" -ForegroundColor Green
Write-Host "=========================================" -ForegroundColor Green
