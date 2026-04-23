# flow-engine-grpc

**Layer 1** | 7 文件 | ~4,365 行 | 依赖 core

## 概述

gRPC 客户端 (tonic 0.12) + 3 种节点执行器 + NodePlugin 实现。运行时 .proto 解析 (protox + prost-reflect)，无需编译时代码生成。

## 结构

```
flow-engine-grpc/src/
├── lib.rs                      # 模块导出
├── plugin.rs                   # NodePlugin 实现 (555 行)
├── proto_parser.rs             # .proto 文件解析 + DynamicCodec (319 行)
├── grpc_connection_manager.rs  # 连接池管理 + 服务反射 (1,154 行)
├── grpc_connect_executor.rs    # 连接+服务发现执行器 (439 行)
├── grpc_call_executor.rs       # 调用执行器 (1,141 行)
└── grpc_close_executor.rs      # 关闭连接执行器 (279 行)
```

## 快速定位

| 任务 | 文件 |
|------|------|
| NodePlugin 注册 | `plugin.rs` |
| .proto 解析 | `proto_parser.rs` |
| 连接管理/服务发现 | `grpc_connection_manager.rs` |
| 连接节点逻辑 | `grpc_connect_executor.rs` |
| 调用节点逻辑 | `grpc_call_executor.rs` |
| 关闭连接逻辑 | `grpc_close_executor.rs` |

## 节点类型 (3 种)

| 类型 | 执行器 | 端口 | 用途 |
|------|--------|------|------|
| GrpcConnect | `grpc_connect_executor.rs` | true/false | 建立连接 + 服务发现 |
| GrpcCall | `grpc_call_executor.rs` | true/false | 调用 gRPC 方法 |
| GrpcClose | `grpc_close_executor.rs` | out | 关闭连接 |

## 关键设计

### 运行时 Proto 解析
- **protox** 解析 .proto 文件语法
- **prost-reflect** 动态编解码 (DynamicCodec)
- 无需编译时代码生成，支持任意 .proto 文件

### TLS 模式 (3 种)
- **plaintext**: 明文连接
- **standard**: 标准 TLS (系统证书)
- **skip-verify**: 跳过证书验证

### 服务发现
- Server Reflection 协议自动发现服务和方法
- .proto 文件上传解析

### 流式调用 (4 种模式)
- **Unary**: 单请求单响应
- **ServerStreaming**: 单请求，服务端流式响应
- **ClientStreaming**: 客户端流式请求，单响应
- **BidiStreaming**: 双向流式

## 编译影响

修改此 crate → `flow-engine-executor` → `flow-engine` → `app` (~3 crate)

```bash
cd src-tauri
cargo test -p flow-engine-grpc           # 仅测 gRPC
cargo clippy -p flow-engine-grpc         # 仅 Lint gRPC
```
