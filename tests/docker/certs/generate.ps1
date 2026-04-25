# ============================================================
# 生成自签名 TLS 证书 (Windows PowerShell)
# 用途：protocol-test-server 本地集成测试
# ============================================================

$ErrorActionPreference = "Stop"

# 检查 openssl 是否可用
try {
    $null = openssl version 2>&1
} catch {
    Write-Host "错误：未找到 openssl 命令" -ForegroundColor Red
    Write-Host ""
    Write-Host "安装方式（任选其一）："
    Write-Host "  1. 安装 Git for Windows（自带 openssl）：https://git-scm.com/download/win"
    Write-Host "  2. Chocolatey: choco install openssl"
    Write-Host "  3. 下载 OpenSSH for Windows（包含 openssl）"
    exit 1
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptDir

Write-Host "正在生成自签名 TLS 证书..." -ForegroundColor Cyan

openssl req -x509 -newkey rsa:2048 `
    -keyout server.key `
    -out server.crt `
    -days 3650 -nodes `
    -subj "/CN=localhost" `
    -addext "subjectAltName=DNS:localhost,IP:127.0.0.1,IP:0.0.0.0"

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "证书生成成功：" -ForegroundColor Green
    Write-Host "  私钥: $scriptDir\server.key"
    Write-Host "  证书: $scriptDir\server.crt"
} else {
    Write-Host "错误：证书生成失败" -ForegroundColor Red
    exit 1
}
