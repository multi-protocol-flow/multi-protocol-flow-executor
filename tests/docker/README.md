# gRPC 功能验证环境

## 启动

```bash
cd tests/docker

# 启动所有服务
docker compose up -d

# 只启动 gRPC 相关服务
docker compose up -d protocol-test-server grpcbin-1 grpcbin-2
```

## 服务端口

| 服务 | 端口 | 说明 |
|------|------|------|
| protocol-test-server (明文) | `localhost:50051` | 基础 gRPC 测试 |
| protocol-test-server (TLS) | `localhost:50052` | TLS gRPC (自签名证书) |
| protocol-test-server (mTLS) | `localhost:50053` | mTLS gRPC (需客户端证书) |
| grpcbin-1 (明文) | `localhost:9092` | gRPC + Server Reflection |
| grpcbin-1 (TLS) | `localhost:9093` | TLS (自签名证书) + Server Reflection |
| grpcbin-2 (明文) | `localhost:9094` | 第二个实例，用于负载均衡 |

## 功能验证指南

### 前提

在界面中创建流程时，**确保节点的数据被正确保存**（失焦或手动触发保存后，等几秒再执行）。

---

### 1. 压缩编码 (gzip)

**修复说明**: 运行 `cargo build` 重新编译后端，已添加 `async-compression` 依赖。

**测试步骤**:

1. 加 `grpc_connect` 节点：
   - URL: `grpcbin-1:9092`
   - 开启 **Server Reflection** → 点击 **测试连接**
   - 压缩编码: 选择 **Gzip**
2. 加 `grpc_call` 节点：
   - 服务: `grpcbin.GRPCBin`
   - 方法: `Index`
   - 请求: `{}`
3. 执行

**预期**: Index 方法返回服务器信息，其中包括 `gRPC Server: go-grpcbin`。**注意**: 如果 still 报 gzip 解压错误，可能是旧的 cargo 缓存导致，清理重试：

```bash
cd src-tauri
cargo clean -p flow-engine-grpc
cargo build -p flow-engine-grpc
```

---

### 2. TLS 高级设置

#### 标准 TLS

1. `grpc_connect`：URL `grpcbin-1:9093` → 开启 **TLS**（不跳过验证）
2. 点击 **测试连接**
3. **预期**: 连接失败 ❌（因为自签名证书，标准验证会失败）

#### 跳过证书验证

1. `grpc_connect`：URL `grpcbin-1:9093` → 开启 **TLS** → 开启 **跳过证书验证**
2. 点击 **测试连接**
3. **预期**: 连接成功 ✅，发现 `grpcbin.GRPCBin` 等服务

#### 标准 TLS（验证证书）

protocol-test-server 已内置自签名证书，但需**跳过验证**才能连。

想测完整证书链验证？用 `tests/certs/` 下的证书：

1. 方式一（volume 覆盖）：修改 `docker-compose.yml`，取消环境变量注释：
   ```yaml
   environment:
     - TLS_CERT_PATH=/app/custom-certs/server.pem
     - TLS_KEY_PATH=/app/custom-certs/server.key
   ```
   `docker compose up -d` 后会挂载 `tests/certs/` 到 `/app/custom-certs/`。

2. 方式二（本地测试，不用 Docker）：
   直接运行协议测试服务器的二进制，通过环境变量指定证书路径：
   ```bash
   cd tests/docker
   TLS_CERT_PATH=../certs/server.pem TLS_KEY_PATH=../certs/server.key cargo run
   ```

#### 自定义 CA / mTLS

这些字段填入 grpc_connect 的 **TLS 高级配置**（文件路径，需绝对路径）：

| 字段 | 值 |
|------|-----|
| **CA 证书** | `D:\projects\multi-protocol-flow-executor\tests\certs\ca.pem` |
| **客户端证书** | `D:\projects\multi-protocol-flow-executor\tests\certs\client.pem` |
| **客户端私钥** | `D:\projects\multi-protocol-flow-executor\tests\certs\client.key` |
| **SNI 覆盖** | `localhost` |

1. 启动 `protocol-test-server`，mTLS 端口是 `50053`
2. `grpc_connect`: URL `localhost:50053` → 开启 TLS → 填入上表 4 个字段
3. 点击**测试连接**
4. **预期**: 连接成功 ✅，发现 `testserver.TestService` 等服务

> **注意**: TLS 文件路径字段接收的是**文件路径**（非文件内容），后端会自动读取。

---

### 3. PING / Keepalive

**无法直接观察效果**，但可以用抓包验证。

1. `grpc_connect` → `grpcbin-1:9092`（明文）
2. 在 **连接设置** 中：
   - PING 间隔: `5000`（5 秒）
   - PING 超时: `5000`（5 秒）
   - 开启 **无流时允许 PING**
3. 连接成功后不执行任何调用
4. **预期**: 用 Wireshark 过滤 `http2`，每 5 秒应看到 HTTP/2 PING 帧

---

### 4. Channelz 内省

1. `grpc_connect` → `grpcbin-1:9092` → 连接成功
2. 在 grpc_connect 配置面板拉到 **Channelz 内省**
3. 连接 ID 留空（使用当前节点 UUID）
4. 点击 **查询连接状态**
5. **预期**: 显示 `READY` 状态 + 运行时长 + 0 个活跃调用
6. 执行一个 `grpc_call`，在调用期间再次查询 Channelz
7. **预期**: 活跃调用数变成 1

---

### 5. 负载均衡 (轮询)

1. 确保 `grpcbin-1`（9092）和 `grpcbin-2`（9094）都在运行
2. `grpc_connect`：
   - URL: `grpcbin-1:9092`
   - 负载均衡 → 添加端点: `grpcbin-2:9094`
3. 连续执行多次 `grpc_call`（调用 `Index` 方法查看服务端信息）
4. **预期**: 部分请求由 grpcbin-1 处理，部分由 grpcbin-2 处理

> **注意**: 负载均衡模式不支持跳过证书验证或自定义 TLS。若需要 TLS + 负载均衡，只能使用标准 TLS 模式（非自签名证书）。

---

### 6. 基本 gRPC 调用（protocol-test-server）

`protocol-test-server` 提供 `testserver.TestService`（不支持 Reflection，需手动传 proto 文件）：

```protobuf
syntax = "proto3";
package testserver;

service TestService {
  rpc SayHello(HelloRequest) returns (HelloResponse);
  rpc Echo(EchoRequest) returns (EchoResponse);
  rpc Calculate(CalcRequest) returns (CalcResponse);
  // ... etc
}
```

Proto 文件路径: `tests/docker/protocol-test-server/proto/test_service.proto`

已有测试流程: `tests/docker/flows/gRPC Unary ????.mpf`

---

## 端口冲突处理

若端口冲突，修改 `docker-compose.yml` 中对应端口映射（左边为主机端口），然后重新启动：

```bash
docker compose up -d
```
