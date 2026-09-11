#Requires -Version 5.1
<#
.SYNOPSIS
    Windows 本地构建脚本：支持编译 CLI (mpe-cli) 与 GUI (NSIS 安装包)
.DESCRIPTION
    用法:
      .\scripts\windows\build.ps1 [version] [-CliOnly] [-GuiOnly] [-All]
    示例:
      .\scripts\windows\build.ps1 0.2.0-beta.1
      .\scripts\windows\build.ps1 -CliOnly
#>

[CmdletBinding()]
param(
    [Parameter(Position=0)]
    [string]$Version,

    [switch]$CliOnly,
    [switch]$GuiOnly,
    [switch]$All
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $ScriptDir)

# 确定目标模式
$TargetMode = "all"
if ($CliOnly) { $TargetMode = "cli" }
elseif ($GuiOnly) { $TargetMode = "gui" }

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
Write-Host "  MPE Windows 本地构建" -ForegroundColor Cyan
Write-Host "  版本: v$Version" -ForegroundColor Cyan
Write-Host "  模式: $TargetMode" -ForegroundColor Cyan
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

# === 2. 构建 CLI ===
if ($TargetMode -eq "all" -or $TargetMode -eq "cli") {
    Write-Host ""
    Write-Host "=========================================" -ForegroundColor Cyan
    Write-Host "==> 1/2 构建 Windows CLI (mpe-cli.exe)" -ForegroundColor Cyan
    Write-Host "=========================================" -ForegroundColor Cyan

    Push-Location "src-tauri"
    try {
        cargo build -p mpe-cli --release
        if ($LASTEXITCODE -ne 0) { throw "cargo build mpe-cli 失败" }
    } finally {
        Pop-Location
    }

    $CliBin = Join-Path $ProjectRoot "src-tauri\target\release\mpe-cli.exe"
    if (-not (Test-Path $CliBin)) {
        throw "找不到编译出的 CLI 文件: $CliBin"
    }

    # 打包为 zip
    $TempStage = Join-Path $env:TEMP ("mpe_cli_stage_" + [Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $TempStage | Out-Null
    Copy-Item $CliBin -Destination (Join-Path $TempStage "mpe.exe") -Force

    $CliZip = Join-Path $DistDir "mpe-v$Version-windows-x86_64.zip"
    if (Test-Path $CliZip) { Remove-Item $CliZip -Force }
    Compress-Archive -Path (Join-Path $TempStage "*") -DestinationPath $CliZip -Force
    Remove-Item $TempStage -Recurse -Force

    Write-Host "  ✓ CLI 打包完成: $CliZip" -ForegroundColor Green
}

# === 3. 构建 GUI (NSIS 安装包) ===
if ($TargetMode -eq "all" -or $TargetMode -eq "gui") {
    Write-Host ""
    Write-Host "=========================================" -ForegroundColor Cyan
    Write-Host "==> 2/2 构建 Windows GUI (Tauri NSIS)" -ForegroundColor Cyan
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
}

# === 4. 生成 SHA256 校验和 ===
Write-Host ""
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "==> 生成 SHA256 校验清单" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

$ChecksumFile = Join-Path $DistDir "SHA256SUMS-windows.txt"
$Hashes = @()
Get-ChildItem $DistDir -File | Where-Object { $_.Name -match '\.(exe|zip)$' } | ForEach-Object {
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
