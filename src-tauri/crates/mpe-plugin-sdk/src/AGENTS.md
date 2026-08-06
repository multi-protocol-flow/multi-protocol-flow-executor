# mpe-plugin-sdk/src

**Generated:** 2026-08-05 | **Branch:** feature/plugins | 11 .rs, ~950 LOC

## OVERVIEW

插件作者 SDK crate。第三方作者用它编写 sidecar 插件进程（Rust 二进制），经 stdio 用 JSON-RPC 2.0 与宿主 `flow-engine-plugin` 通信。刻意只依赖 `mpe-protocol`（共享 wire 契约），**不依赖** `flow-engine-core` / `flow-engine-plugin` —— 插件是独立子进程，禁止继承宿主内部类型。

## STRUCTURE

```
src/
├── lib.rs                    # crate 入口 + __plugin_main (doc(hidden))
├── context.rs                # ExecuteContext: 执行上下文
├── macros.rs                 # mpe_plugin_main! 宏
├── runtime.rs                # Plugin trait + async 事件循环
├── runtime/dispatch.rs       # 请求分发 (describe/execute/validate/shutdown)
├── runtime/frame.rs          # stdio 帧读写 (payload + LF)
├── pool.rs                   # 连接/worker 池
├── prelude.rs                # 公开 re-exports (快速上手)
├── types.rs                  # NodeDescription / ExecuteResult / PortDescription
└── bin/demo_plugin.rs        # 示例二进制 mpe_demo_plugin
tests/
└── roundtrip.rs              # 协议往返集成测试
```

## WHERE TO LOOK

| 任务 | 文件 |
|------|------|
| 写一个插件（quick-start） | `prelude.rs` + `types.rs` |
| 声明节点类型 / 定义执行结果 | `types.rs` (`NodeDescription::new(type_id, name)`) |
| 实现 Plugin trait（describe/execute） | `runtime.rs` |
| execute 上下文（读 config、写输出） | `context.rs` |
| main 生成 / 运行时构建 | `macros.rs` + `lib.rs::__plugin_main` |
| 请求分发逻辑 | `runtime/dispatch.rs` |
| stdio 帧格式 | `runtime/frame.rs` |
| 协议常量 / manifest / wire 格式 | `mpe-protocol` crate |
| 宿主侧行为 / 生命周期 | `flow-engine-plugin` crate |

## KEY LOGIC

**快速上手**（lib.rs 文档）。`#[derive(Default)] struct MyPlugin;` → `impl Plugin`（`describe` 返回 `Vec<NodeDescription>`，`execute(&self, ctx: &mut ExecuteContext)` 返回 `impl Future<Output=ExecuteResult> + Send`）→ `mpe_plugin_main!(MyPlugin)` 收尾。

**mpe_plugin_main!** 生成 `fn main`：构建多线程 tokio runtime（`enable_all`），跑事件循环。内部调 `lib.rs::__plugin_main<P: Plugin + Default>() -> i32`（隐藏公开 API），退出码 0 = 干净关闭。`Plugin` 要求 `Default`；`execute` 是异步。

**事件循环**（`runtime.rs`）。stdin 读 describe/execute/validate/shutdown 请求与 cancel/shutdown 通知，stdout 写帧响应。设计约束：
- 单 writer 任务：响应经 bounded `mpsc` 排队，原子写整帧，绝不交错
- spawn-per-request：execute 每个请求 spawn 一个 task（要求 future `Send`），按 request id 关联，可乱序返回；describe/validate 内联处理
- 坏帧只跳过不致命：超行长或坏 JSON 记 stderr 后继续，仅 EOF / I/O 错误 / shutdown 消息退出
- 背压：execute 响应 await 发送；log 通知 `try_send`（满则丢），防日志洪泛饿死响应

**类型契约**（`types.rs`）。`NodeDescription` 字段名对齐宿主 describe 契约（`type_id`、`display_name` 等），可选字段 `skip_serializing_if` 保证最小插件 wire 干净。`type_id` 建议 `"vendor:name"` 格式，不得与宿主内建类型冲突。SDK 自带 `NodeDescription` 定义，M5b 一致性里程碑时与宿主对账。

## CONVENTIONS

- 作者只依赖 SDK 公开面：`use mpe_plugin_sdk::prelude::*;`
- 文档见 `docs/plugin-sdk.md`（作者指南）+ `docs/plugin-protocol.md`（线协议）
- 新节点类型用 `vendor:name` 命名空间，避免与宿主内建冲突
- 新增 SDK 类型必须经 `mpe-protocol` 对齐后再暴露

## ANTI-PATTERNS

- 在插件中依赖 `flow-engine-core` / `flow-engine-plugin` → 破坏 sidecar 边界
- `execute` 返回非 `Send` future → 无法 spawn-per-request
- 手动写帧协议 → 必须走 `runtime/frame.rs` 的 writer 通道，防交错
- 丢弃 `mpe_plugin_main!` 手写 main → 丢失 tokio 构建与事件循环约定
