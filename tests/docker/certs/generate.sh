#!/bin/bash
# ============================================================
# 生成自签名 TLS 证书 (Bash / WSL / Linux / macOS)
# 用途：protocol-test-server 本地集成测试
# ============================================================
set -e

# 检查 openssl 是否可用
if ! command -v openssl &>/dev/null; then
    echo "错误：未找到 openssl 命令"
    echo ""
    echo "安装方式："
    echo "  Ubuntu/Debian: sudo apt-get install openssl"
    echo "  macOS:         brew install openssl"
    echo "  Alpine:        apk add openssl"
    exit 1
fi

cd "$(dirname "$0")"

echo "正在生成自签名 TLS 证书..."

openssl req -x509 -newkey rsa:2048 \
    -keyout server.key \
    -out server.crt \
    -days 3650 -nodes \
    -subj "/CN=localhost" \
    -addext "subjectAltName=DNS:localhost,IP:127.0.0.1,IP:0.0.0.0"

echo ""
echo "证书生成成功："
echo "  私钥: $(pwd)/server.key"
echo "  证书: $(pwd)/server.crt"
