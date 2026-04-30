# TLS 功能验证测试

本目录包含 HTTP 节点 TLS 功能的完整测试套件。

## 前置条件

1. **生成测试证书**（一次性）：

```powershell
cd D:\projects\multi-protocol-flow-executor\tests\certs
.\generate-certs.ps1
# 或手动运行 openssl 命令（见下方）
```

2. **编译 mpe CLI**：

```powershell
cd src-tauri
cargo build --bin mpe
```

3. **Node.js** >= 18（用于测试服务器）

## 快速开始

```powershell
# 一键运行所有测试（自动启动/停止服务器）
node tests/run-tls-tests.mjs
```

## 手动测试

### 1. 启动测试服务器

```powershell
# 标准模式（仅服务端证书）
node tests/https-test-server.mjs

# mTLS 模式（需要客户端证书）
node tests/https-test-server.mjs --mtls
```

### 2. 执行测试流程

证书路径在 flow 文件中使用相对于项目根目录的路径，直接从项目根目录执行：

```powershell
# 从项目根目录直接执行（无需打补丁）
cd D:\projects\multi-protocol-flow-executor
src-tauri\target\debug\mpe.exe run -f tests\cli\tls_01_basic_ca_verify.json
```

## 测试用例

| 文件 | 说明 | 服务器模式 | 预期结果 |
|------|------|-----------|---------|
| `tls_01_basic_ca_verify.json` | 正确 CA 证书 | standard | ✅ 200 成功 |
| `tls_02_skip_verify.json` | 跳过证书验证 | standard | ✅ 200 成功 |
| `tls_03_mtls_auth.json` | mTLS 双向认证 | mtls | ✅ 200 成功 |
| `tls_04_ca_missing.json` | 无 CA 证书（预期失败） | standard | ❌ unknown issuer |
| `tls_05_wrong_ca.json` | 错误 CA 证书（预期失败） | standard | ❌ unable to get local issuer |

## 证书文件

`tests/certs/` 目录下的证书（由 `generate-certs.ps1` 生成）：

| 文件 | 说明 |
|------|------|
| `ca.key` / `ca.pem` | CA 根证书（正确） |
| `ca-wrong.key` / `ca-wrong.pem` | 错误的 CA（用于负面测试） |
| `server.key` / `server.pem` | 服务器证书（SAN: localhost, 127.0.0.1） |
| `client.key` / `client.pem` | 客户端证书（用于 mTLS） |


