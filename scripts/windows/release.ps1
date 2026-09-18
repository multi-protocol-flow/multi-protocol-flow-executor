#Requires -Version 5.1
<#
.SYNOPSIS
    一键 Windows 本地构建与发布脚本 (Build + Upload to R2)
.DESCRIPTION
    用法:
      .\scripts\windows\release.ps1 [version] [-Upload]
    示例:
      .\scripts\windows\release.ps1 0.2.0-beta.1 -Upload
#>

[CmdletBinding()]
param(
    [Parameter(Position=0)]
    [string]$Version,

    [switch]$Upload
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
Write-Host " 🚀 MPE Windows 本地发布工作流 (完整安装包)" -ForegroundColor Cyan
Write-Host " 版本: v$Version" -ForegroundColor Cyan
Write-Host " 上传: $(if ($Upload) { '是 (R2)' } else { '否 (仅本地)' })" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

# === 1. 上传预检查 ===
if ($Upload) {
    if (-not (Get-Command aws -ErrorAction SilentlyContinue)) {
        Write-Host "❌ 指定了 -Upload 但系统未安装 aws-cli！" -ForegroundColor Red
        Write-Host "   请先安装 AWS CLI v2: https://awscli.amazonaws.com/AWSCLIV2.msi" -ForegroundColor Yellow
        exit 1
    }
    $EnvFile = Join-Path $ProjectRoot ".env"
    if (-not (Test-Path $EnvFile) -and -not $env:R2_BUCKET) {
        Write-Host "❌ 指定了 -Upload 但项目根目录未找到 .env 且未设置 R2 环境变量！" -ForegroundColor Red
        exit 1
    }
    # 校验并准备 CHANGELOG.md（无更新说明且 [Unreleased] 为空时不给发布；若有 [Unreleased] 则自动转换）
    $PrepareScript = Join-Path $ProjectRoot "scripts\prepare-changelog.js"
    node $PrepareScript $Version | Out-Null
    if ($LASTEXITCODE -ne 0) {
        exit 1
    }
}

# === 2. 执行构建 ===
$BuildScript = Join-Path $ScriptDir "build.ps1"
& $BuildScript -Version $Version
if ($LASTEXITCODE -ne 0) { throw "构建步骤执行失败" }

# === 3. 执行上传 ===
if ($Upload) {
    Write-Host ""
    $UploadScript = Join-Path $ScriptDir "upload-r2.ps1"
    & $UploadScript -Version $Version
    if ($LASTEXITCODE -ne 0) { throw "上传步骤执行失败" }
} else {
    Write-Host ""
    Write-Host "💡 提示: 若要将产物上传到 Cloudflare R2，可执行:" -ForegroundColor Yellow
    Write-Host "   .\scripts\windows\upload-r2.ps1 $Version" -ForegroundColor White
    Write-Host "   或者下次直接携带参数: .\scripts\windows\release.ps1 $Version -Upload" -ForegroundColor White
}

Write-Host ""
Write-Host "=========================================" -ForegroundColor Green
Write-Host " 🎉 Windows 发布流程结束" -ForegroundColor Green
Write-Host "=========================================" -ForegroundColor Green
