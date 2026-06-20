#Requires -Version 5.1
param(
    [Parameter(Mandatory=$true, Position=0)]
    [string]$Version,
    
    [string]$Notes = ""
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)

# === 加载 .env ===
$EnvFile = Join-Path $ProjectRoot ".env"
if (-not (Test-Path $EnvFile)) {
    Write-Host ".env not found" -ForegroundColor Red
    exit 1
}

Get-Content $EnvFile | ForEach-Object {
    if ($_ -match '^([^#][^=]+)=(.*)$') {
        $name = $Matches[1].Trim()
        $value = $Matches[2].Trim()
        $value = $value -replace '^["'']|["'']$', ''
        [Environment]::SetEnvironmentVariable($name, $value, "Process")
    }
}

# 设置 AWS 凭证 (R2 兼容 S3)
$env:AWS_ACCESS_KEY_ID = $env:R2_ACCESS_KEY_ID
$env:AWS_SECRET_ACCESS_KEY = $env:R2_SECRET_ACCESS_KEY
$env:AWS_DEFAULT_REGION = "auto"

# 去掉 v 前缀
$Version = $Version -replace '^v', ''
$Tag = "v$Version"

if (-not $Notes) {
    $Notes = "Release $Tag"
}

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host " 上传到 R2: $Tag" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

# === 配置 ===
$R2Endpoint = "https://$($env:R2_ACCOUNT_ID).r2.cloudflarestorage.com"
$Bucket = $env:R2_BUCKET
$BaseUrl = "https://mpe-downloads.sunanzhi.com"
$NsisDir = Join-Path $ProjectRoot "src-tauri\target\release\bundle\nsis"
$DistDir = Join-Path $ProjectRoot "dist-release"

# 创建 dist-release 目录
if (-not (Test-Path $DistDir)) {
    New-Item -ItemType Directory -Path $DistDir | Out-Null
}

# === 1. 复制产物到 dist-release ===
Write-Host "==> 复制产物" -ForegroundColor Yellow
$ExeFile = Get-ChildItem "$NsisDir\*.exe" | Select-Object -First 1
$SigFile = Get-ChildItem "$NsisDir\*.exe.sig" | Select-Object -First 1

$DestExe = "mpe-$Version-x64-setup.exe"
$DestSig = "mpe-$Version-x64-setup.exe.sig"

Copy-Item $ExeFile.FullName -Destination (Join-Path $DistDir $DestExe) -Force
Copy-Item $SigFile.FullName -Destination (Join-Path $DistDir $DestSig) -Force
Write-Host "  ✓ $DestExe" -ForegroundColor Green
Write-Host "  ✓ $DestSig" -ForegroundColor Green

# === 2. 上传到 R2 ===
Write-Host "==> 上传到 R2" -ForegroundColor Yellow

$ExePath = Join-Path $DistDir $DestExe
$SigPath = Join-Path $DistDir $DestSig
$R2KeyExe = "releases/$Tag/$DestExe"
$R2KeySig = "releases/$Tag/$DestSig"

aws s3 cp $ExePath "s3://$Bucket/$R2KeyExe" --endpoint-url $R2Endpoint --quiet
Write-Host "  ✓ $R2KeyExe" -ForegroundColor Green

aws s3 cp $SigPath "s3://$Bucket/$R2KeySig" --endpoint-url $R2Endpoint --quiet
Write-Host "  ✓ $R2KeySig" -ForegroundColor Green

# === 3. 读取签名 ===
Write-Host "==> 生成 manifest.json" -ForegroundColor Yellow
$WinSig = Get-Content $SigPath -Raw
$WinSig = $WinSig.Trim()

# === 4. 更新 manifest.json ===
$Manifest = @{
    version = $Version
    notes = $Notes
    pub_date = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    platforms = @{
        "windows-x86_64" = @{
            signature = $WinSig
            url = "$BaseUrl/$R2KeyExe"
        }
    }
} | ConvertTo-Json -Depth 10

$ManifestPath = Join-Path $DistDir "manifest.json"
[System.IO.File]::WriteAllText($ManifestPath, $Manifest, (New-Object System.Text.UTF8Encoding $false))

# 上传 manifest.json
aws s3 cp $ManifestPath "s3://$Bucket/manifest.json" --endpoint-url $R2Endpoint --cache-control "no-cache, must-revalidate" --quiet
Write-Host "  ✓ manifest.json" -ForegroundColor Green

# === 5. 更新 changelog.json ===
Write-Host "==> 更新 changelog.json" -ForegroundColor Yellow

$ChangelogPath = Join-Path $DistDir "changelog.json"

# 尝试下载现有的 changelog
try {
    aws s3 cp "s3://$Bucket/changelog.json" $ChangelogPath --endpoint-url $R2Endpoint --quiet 2>$null
    $Changelog = Get-Content $ChangelogPath -Raw | ConvertFrom-Json
} catch {
    $Changelog = @()
}

# 判断是否 prerelease
$IsPrerelease = $Tag -match '(beta|alpha)'

# 新条目
$NewEntry = @{
    version = $Version
    tag = $Tag
    date = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    notes = $Notes
    prerelease = $IsPrerelease
    downloads = @{
        windows = "$BaseUrl/$R2KeyExe"
    }
}

# 添加到 changelog 开头
$Changelog = @($NewEntry) + @($Changelog)

$ChangelogJson = $Changelog | ConvertTo-Json -Depth 10
[System.IO.File]::WriteAllText($ChangelogPath, $ChangelogJson, (New-Object System.Text.UTF8Encoding $false))

aws s3 cp $ChangelogPath "s3://$Bucket/changelog.json" --endpoint-url $R2Endpoint --cache-control "no-cache, must-revalidate" --quiet
Write-Host "  ✓ changelog.json" -ForegroundColor Green

# === 完成 ===
Write-Host ''
Write-Host '=========================================' -ForegroundColor Green
Write-Host ' Upload Complete!' -ForegroundColor Green
Write-Host '=========================================' -ForegroundColor Green
Write-Host ''
$downloadUrl = $BaseUrl + '/' + $R2KeyExe
$manifestUrl = $BaseUrl + '/manifest.json'
$changelogUrl = $BaseUrl + '/changelog.json'
Write-Host 'Download:' -ForegroundColor Cyan
Write-Host ('  ' + $downloadUrl) -ForegroundColor White
Write-Host ''
Write-Host 'Manifest:' -ForegroundColor Cyan
Write-Host ('  ' + $manifestUrl) -ForegroundColor White
Write-Host ''
Write-Host 'Changelog:' -ForegroundColor Cyan
Write-Host ('  ' + $changelogUrl) -ForegroundColor White
