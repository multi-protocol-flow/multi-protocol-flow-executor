# flow-engine-script

**Layer 1** | 5 文件 | ~3,500 行 | 依赖 core

## 概述

QuickJS 运行时 + 脚本节点执行器 + NodePlugin 实现。提供 `fn.*` API 供脚本访问变量、响应、控制台等。

## 结构

```
flow-engine-script/src/
├── lib.rs              # 模块导出
├── plugin.rs           # NodePlugin 实现 (88 行)
├── script_executor.rs  # ScriptNodeExecutor (实现 NodeExecutor, ~802 行)
├── script_api.rs       # 脚本 API 绑定 (fn.* API，~2,456 行)
└── script_poc.rs       # 概念验证 (开发参考)
```

## 快速定位

| 任务 | 文件 |
|------|------|
| NodePlugin 注册 | `plugin.rs` |
| 脚本执行逻辑 | `script_executor.rs` |
| API 绑定 (fn.*) | `script_api.rs` |
| 变量操作 API | `script_api.rs` → `fn.variables.*` |
| 响应访问 API | `script_api.rs` → `fn.response.*` |
| 工具函数 API | `script_api.rs` → `fn.util.*` |

## fn.* API 参考

| API | 用途 |
|-----|------|
| `fn.variables.get/set/has` | 变量读写 |
| `fn.response.status/body/headers` | 上一个节点响应 |
| `fn.console.log/warn/error` | 日志输出 |
| `fn.flow.stop/abort/skip` | 流程控制（终止/报错/跳过） |
| `fn.util.encode/decode/hash/uuid/time` | 工具函数 |
| `fn.expect()` | 链式断言 |

## 关键实现

- **QuickJS 运行时**: 每次脚本执行创建独立上下文，隔离变量
- **超时控制**: 脚本执行有超时限制，防止无限循环
- **变量桥接**: `fn.variables.*` 读写 `ExecutionContext` 中的变量存储

## 编译影响

修改此 crate → `flow-engine-executor` → `flow-engine` → `app` (~3 crate)

```bash
cd src-tauri
cargo test -p flow-engine-script          # 仅测脚本
cargo clippy -p flow-engine-script        # 仅 Lint 脚本
```
