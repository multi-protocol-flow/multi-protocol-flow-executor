#Requires -Version 5.1
<#
.SYNOPSIS
    Windows 本地构建脚本：构建完整安装包 (GUI + CLI 二合一 NSIS)
.DESCRIPTION
    用法:
      .\scripts\windows\build.ps1 [version]
    示例:
      .\scripts\windows\build.ps1 0.2.0-beta.1
#>

[CmdletBinding()]
param(
    [Parameter(Position=0)]
    [string]$Version
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $ScriptDir)

Set-Location $ProjectRoot

# 读取当前版本
$PkgJson = Get-Content "package.json" -Raw | ConvertFrom-Json
$CurrentVersion = $PkgJson.version

if ([string]::IsNullOrWhiteSpace($Version)) {
    $Version = $CurrentVersion
} else {
    $Version = $Version -replace '^v', ''
}

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "  MPE Windows 本地构建 (完整安装包)" -ForegroundColor Cyan
Write-Host "  版本: v$Version" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

# === 1. 版本号同步注入 ===
if ($Version -ne $CurrentVersion) {
    Write-Host "==> 同步注入版本号: $Version" -ForegroundColor Yellow

    # package.json
    $PkgJson.version = $Version
    $PkgJson | ConvertTo-Json -Depth 10 | Set-Content "package.json" -Encoding UTF8
    Write-Host "  ✓ package.json" -ForegroundColor Green

    # src-tauri/tauri.conf.json
    $TauriJson = Get-Content "src-tauri/tauri.conf.json" -Raw | ConvertFrom-Json
    $TauriJson.version = $Version
    $TauriJson | ConvertTo-Json -Depth 10 | Set-Content "src-tauri/tauri.conf.json" -Encoding UTF8
    Write-Host "  ✓ src-tauri/tauri.conf.json" -ForegroundColor Green

    # src-tauri/Cargo.toml
    $CargoToml = Get-Content "src-tauri/Cargo.toml" -Raw
    $CargoToml = $CargoToml -replace '(?m)^(\[package\][\s\S]*?version\s*=\s*)"[^"]*"', "`$1`"$Version`""
    Set-Content "src-tauri/Cargo.toml" $CargoToml -Encoding UTF8

    # mpe-cli/Cargo.toml
    $CliToml = Get-Content "src-tauri/crates/mpe-cli/Cargo.toml" -Raw
    $CliToml = $CliToml -replace '(?m)^(\[package\][\s\S]*?version\s*=\s*)"[^"]*"', "`$1`"$Version`""
    Set-Content "src-tauri/crates/mpe-cli/Cargo.toml" $CliToml -Encoding UTF8
    Write-Host "  ✓ Cargo.toml (app & mpe-cli)" -ForegroundColor Green
}

$DistDir = Join-Path $ProjectRoot "dist-release"
if (-not (Test-Path $DistDir)) {
    New-Item -ItemType Directory -Path $DistDir | Out-Null
}

# === 2. 构建完整版 (Tauri NSIS 安装包，内置 GUI + CLI) ===
Write-Host ""
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "==> 构建完整版安装包 (Tauri NSIS)" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

if (-not (Test-Path "node_modules")) {
    Write-Host "==> 安装前端依赖..." -ForegroundColor Yellow
    npm ci
    if ($LASTEXITCODE -ne 0) { throw "npm ci 失败" }
}

Write-Host "==> 生成 i18n 类型..." -ForegroundColor Yellow
npm run typesafe-i18n
if ($LASTEXITCODE -ne 0) { throw "typesafe-i18n 失败" }

Write-Host "==> 构建前端产物..." -ForegroundColor Yellow
npm run build
if ($LASTEXITCODE -ne 0) { throw "npm run build 失败" }

Write-Host "==> 执行 Tauri 打包 (NSIS)..." -ForegroundColor Yellow
npx tauri build --bundles nsis --config src-tauri/tauri.conf.ci.json
if ($LASTEXITCODE -ne 0) { throw "tauri build 失败" }

$NsisDir = Join-Path $ProjectRoot "src-tauri\target\release\bundle\nsis"
Get-ChildItem "$NsisDir\*.exe" | ForEach-Object {
    Copy-Item $_.FullName -Destination $DistDir -Force
    Write-Host "  ✓ 复制安装包: $($_.Name)" -ForegroundColor Green
}
Get-ChildItem "$NsisDir\*.exe.sig" -ErrorAction SilentlyContinue | ForEach-Object {
    Copy-Item $_.FullName -Destination $DistDir -Force
    Write-Host "  ✓ 复制签名: $($_.Name)" -ForegroundColor Green
}

# === 3. 生成 SHA256 校验和 ===
Write-Host ""
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "==> 生成 SHA256 校验清单" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

$ChecksumFile = Join-Path $DistDir "SHA256SUMS-windows.txt"
$Hashes = @()
Get-ChildItem $DistDir -File | Where-Object { $_.Name -match '\.exe$' } | ForEach-Object {
    $hash = (Get-FileHash -Path $_.FullName -Algorithm SHA256).Hash.ToLower()
    $Hashes += "$hash  $($_.Name)"
}
$Hashes | Set-Content $ChecksumFile -Encoding UTF8

Write-Host ""
Write-Host "=========================================" -ForegroundColor Green
Write-Host " ✅ Windows 构建完成!" -ForegroundColor Green
Write-Host "=========================================" -ForegroundColor Green
Write-Host "产物目录: $DistDir" -ForegroundColor Cyan
Get-ChildItem $DistDir | Format-Table Name, @{N='Size (MB)';E={[math]::Round($_.Length/1MB, 2)}} -AutoSize
