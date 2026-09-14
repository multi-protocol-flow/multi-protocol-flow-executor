#Requires -Version 5.1
param(
    [Parameter(Mandatory=$true, Position=0)]
    [string]$Version,
    
    [switch]$UploadToR2
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)

# === 加载 .env ===
$EnvFile = Join-Path $ProjectRoot ".env"
if (-not (Test-Path $EnvFile)) {
    Write-Host "❌ .env 文件不存在，请复制 .env.example 为 .env 并填写配置" -ForegroundColor Red
    Write-Host "   Copy-Item .env.example .env"
    exit 1
}

Get-Content $EnvFile | ForEach-Object {
    if ($_ -match '^([^#][^=]+)=(.*)$') {
        $name = $Matches[1].Trim()
        $value = $Matches[2].Trim()
        # 移除引号
        $value = $value -replace '^["'']|["'']$', ''
        [Environment]::SetEnvironmentVariable($name, $value, "Process")
    }
}

# 去掉可能的 v 前缀
$Version = $Version -replace '^v', ''

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host " MPE 本地构建 v$Version" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

Set-Location $ProjectRoot

# === 1. 版本注入 ===
Write-Host "==> 注入版本号: $Version" -ForegroundColor Yellow

# package.json
$pkg = Get-Content package.json -Raw | ConvertFrom-Json
$pkg.version = $Version
$pkg | ConvertTo-Json -Depth 10 | Set-Content package.json -Encoding UTF8
Write-Host "  ✓ package.json" -ForegroundColor Green

# tauri.conf.json
$tauri = Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
$tauri.version = $Version
$tauri | ConvertTo-Json -Depth 10 | Set-Content src-tauri/tauri.conf.json -Encoding UTF8
Write-Host "  ✓ tauri.conf.json" -ForegroundColor Green

# Cargo.toml
$cargo = Get-Content src-tauri/Cargo.toml -Raw
$cargo = $cargo -replace '(?m)^version\s*=\s*"[^"]*"', "version = `"$Version`""
Set-Content src-tauri/Cargo.toml $cargo -Encoding UTF8
Write-Host "  ✓ Cargo.toml" -ForegroundColor Green

# === 2. 前端构建 ===
Write-Host "==> 安装前端依赖" -ForegroundColor Yellow
npm ci
if ($LASTEXITCODE -ne 0) { throw "npm ci failed" }

Write-Host "==> 生成 i18n 类型" -ForegroundColor Yellow
npm run typesafe-i18n
if ($LASTEXITCODE -ne 0) { throw "typesafe-i18n failed" }

# === 3. 检查签名环境变量 ===
if ($env:TAURI_SIGNING_PRIVATE_KEY) {
    Write-Host "==> 签名密钥已配置" -ForegroundColor Green
} else {
    Write-Host "⚠️  未配置 TAURI_SIGNING_PRIVATE_KEY，产物将不会签名" -ForegroundColor Yellow
}

# === 4. 构建 ===
$DistDir = Join-Path $ProjectRoot "dist-release"
if (-not (Test-Path $DistDir)) {
    New-Item -ItemType Directory -Path $DistDir | Out-Null
}

Write-Host "==> 构建 Windows 安装包 (NSIS)" -ForegroundColor Yellow
cargo tauri build --bundles nsis --config src-tauri/tauri.conf.ci.json
if ($LASTEXITCODE -ne 0) { throw "tauri build failed" }

$NsisDir = Join-Path $ProjectRoot "src-tauri\target\release\bundle\nsis"
# 仅复制当前版本的产物，避免把 bundle 目录中历史版本的安装包带入发布目录
Get-ChildItem "$NsisDir\*.exe" -ErrorAction SilentlyContinue | Where-Object { $_.Name -like "*$Version*" } | Copy-Item -Destination $DistDir -Force
Get-ChildItem "$NsisDir\*.exe.sig" -ErrorAction SilentlyContinue | Where-Object { $_.Name -like "*$Version*" } | Copy-Item -Destination $DistDir -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "=========================================" -ForegroundColor Green
Write-Host " ✅ 构建完成!" -ForegroundColor Green
Write-Host "=========================================" -ForegroundColor Green
Write-Host ""
Write-Host "产物目录: $DistDir" -ForegroundColor Cyan
Get-ChildItem $DistDir | Format-Table Name, @{N='Size';E={[math]::Round($_.Length/1MB, 2) + ' MB'}} -AutoSize

# === 5. 可选: 上传到 R2 ===
if ($UploadToR2 -and $env:R2_ACCOUNT_ID) {
    Write-Host ""
    Write-Host "==> 上传到 R2" -ForegroundColor Yellow
    $R2Endpoint = "https://$($env:R2_ACCOUNT_ID).r2.cloudflarestorage.com"
    
    Get-ChildItem $DistDir | ForEach-Object {
        # 仅上传属于当前版本的产物，避免把 dist-release 中历史版本文件一并发布
        if ($_.Name -notlike "*$Version*") {
            Write-Host "  跳过历史版本产物: $($_.Name)" -ForegroundColor DarkGray
            return
        }
        Write-Host "  上传: $($_.Name)"
        aws s3 cp $_.FullName "s3://$($env:R2_BUCKET)/releases/v$Version/$($_.Name)" --endpoint-url $R2Endpoint --quiet
    }
    
    Write-Host "✅ 上传完成" -ForegroundColor Green
    Write-Host "   URL: https://mpe-downloads.sunanzhi.com/releases/v$Version/" -ForegroundColor Cyan
}

Write-Host ""
Write-Host "提示: 使用 -UploadToR2 开关可自动上传到 R2" -ForegroundColor Gray
