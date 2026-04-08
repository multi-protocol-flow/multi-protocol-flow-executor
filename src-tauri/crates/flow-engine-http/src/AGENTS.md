# flow-engine-http

**Layer 1** | 10 文件 | 4,590 行 | 依赖 core

## 概述

HTTP 客户端 (hyper 1.x + rustls) + HTTP 节点执行器。支持多种请求体、超时、TLS。

## 结构

```
flow-engine-http/src/
├── lib.rs                 # 模块导出
├── http_executor.rs       # HTTP 节点执行器 (2,542 行)
└── http/                  # HTTP 客户端栈
    ├── mod.rs
    ├── client.rs          # HttpClient 实现
    ├── request.rs         # 请求构建
    ├── response.rs        # 响应处理
    ├── body.rs            # 请求体 (none/json/form-data/binary)
    ├── timing.rs          # 性能计时
    ├── connector.rs       # TLS 连接器 (rustls)
    └── error.rs           # HTTP 错误类型
```

## 快速定位

| 任务 | 文件 |
|------|------|
| HTTP 执行逻辑 | `http_executor.rs` |
| HTTP 客户端 | `http/client.rs` |
| 请求体处理 | `http/body.rs` |
| TLS 配置 | `http/connector.rs` |
| 响应处理 | `http/response.rs` |
| 超时/计时 | `http/timing.rs` |

## HTTP Body 类型

支持 `none`、`body` (JSON)、`form-data` (文件上传)、`x-www-form-urlencoded`、`binary`。
Content-Type 自动设置，用户手动设置优先。超时 `timeout_ms` 默认 30000ms (范围 1000-300000ms)。

## 编译影响

修改此 crate → `flow-engine-executor` → `flow-engine` → `app` (~3 crate)

```bash
cd src-tauri
cargo test -p flow-engine-http          # 仅测 HTTP
cargo clippy -p flow-engine-http        # 仅 Lint HTTP
```
