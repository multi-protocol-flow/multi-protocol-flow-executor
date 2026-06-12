# Tauri 应用层

**生成时间:** 2026-06-12 | **Branch:** dev

## OVERVIEW

本节是 `src-tauri/src/` — Tauri 应用的二合一入口（GUI + CLI 同一二进制）和 IPC 命令适配层。

`main.rs` 检测参数数量：有参数 → 派发 `cli::run_cli()`，无参数 → 启动 Tauri GUI（Windows 上 `FreeConsole()` 分离控制台）。`lib.rs` 是 Tauri Builder 配置点：插件注册（dialog/opener/store/process/log/updater）、状态注入（DebugSessionManager / ExecutionSessionManager / DatasetExecutionSessionManager / GrpcCancelSessionManager / StressTestSessionManager）、以及 72+ IPC 命令的 `invoke_handler`。

业务逻辑不在此层 — 所有命令委托给 `flow-engine::commands::*`（见 `src-tauri/crates/flow-engine/src/commands/`）。

## STRUCTURE

| 文件 | 职责 |
|------|------|
| `main.rs` | 二进制入口，参数检测 → GUI (`app_lib::run()`) 或 CLI (`app_lib::cli::run_cli()`) |
| `lib.rs` | 库入口，Tauri Builder 组装、插件注册、77 个 `#[tauri::command]` 注册、re-export |
| `tauri_commands.rs` | IPC 命令实现（4200+ 行），5 个 State 管理器，`#[tauri::command]` 函数定义 |
| `tauri_adapters.rs` | Tauri trait 实现：`UnderlyingLogSender` / `ExecutionEventSender` / `DebugEventSender` / `DirectoryOpener` / `AppDirProvider` |
| `cli.rs` | Clap CLI 定义（Cli/Commands/Subcommands） + `run_cli()` 入口 + 日志初始化 |
| `cli/run.rs` | `mpe run` 子命令执行 |
| `cli/validate.rs` | `mpe validate` 子命令执行 |
| `cli/debug_cmd.rs` | `mpe debug` 子命令执行（NDJSON 双向调试协议） |
| `cli/stress.rs` | `mpe stress` 子命令执行 |
| `cli/flow.rs` + `flow_edit.rs` | `mpe flow` 子命令（list/show/create/copy/delete/import/edit） |
| `cli/report.rs` | `mpe report` 子命令（list/show/delete） |
| `cli/config.rs` | `mpe config` 子命令（配置读写） |
| `cli/output.rs` | CLI 输出格式化（JSON / table / error） |

## WHERE TO LOOK

| 任务 | 位置 |
|------|------|
| 查找 IPC 命令入口 | `lib.rs` invoke_handler + `tauri_commands.rs` |
| 添加新 IPC 命令 | `tauri_commands.rs` 实现函数 → `lib.rs` 注册 + re-export |
| 理解命令委托逻辑 | `tauri_commands.rs` → 按 `flow_engine::commands::*` 追踪 |
| 修改 CLI 参数 | `cli.rs`（clap derive） |
| 添加 CLI 子命令 | `cli.rs` + 新建 `cli/xxx.rs` + `cli.rs` pub mod |
| 调试 Tauri 适配器 | `tauri_adapters.rs`（日志/事件/目录操作） |
| Tauri 插件 / 状态管理 | `lib.rs` setup 闭包 |
| 二进制入口逻辑 | `main.rs`（参数判断 + FreeConsole） |

## CONVENTIONS

- **命令委托**: 每个 `#[tauri::command]` 函数从 `tauri::State<T>` 提取管理器，调用 `flow_engine::commands::*` 业务函数，返回 `Result<T, String>`。
- **适配器模式**: `tauri_adapters.rs` 实现 `flow_engine::traits` 中的 trait（日志发送/目录打开/事件发送），使 flow-engine crate 不依赖 Tauri。
- **Channel 日志**: 执行日志通过 `Channel<LogEvent>` 推送到前端，使用 `QueuedLogSender`（调试模式同步，发布模式异步）。
- **State 管理**: 5 个 Session Manager 通过 `app.manage(Arc::new(Mutex::new(...)))` 注入，`tauri::State<T>` 提取。各 Manager 内部维护 `CancelSender` / `GrpcProvider` 等运行态引用。
- **条件编译**: 协议相关命令（gRPC/Kafka/MCP/MQTT 等）使用 `#[cfg(feature = "protocol-*")]` 门控。
- **CLI 输出**: 支持 `--format json|table`，统一通过 `cli/output.rs` 控制。

## IPC COMMAND CATEGORIES

根 AGENTS.md 已有完整 72 命令表。本节说明代码映射：

| 分类 | 文件位置 | 注册点 |
|------|----------|--------|
| Flow (6) | `tauri_commands.rs` | `lib.rs` invoke_handler |
| Reports (4) | `tauri_commands.rs` | `lib.rs` invoke_handler |
| Debug (5) | `tauri_commands.rs` | `lib.rs` invoke_handler |
| Stress (7) | `tauri_commands.rs` | `lib.rs` invoke_handler |
| Flow Files (5) | `tauri_commands.rs` | `lib.rs` invoke_handler |
| Folders (6) | `tauri_commands.rs` | `lib.rs` invoke_handler |
| gRPC (8) | `tauri_commands.rs` + `#[cfg]` | `lib.rs` 条件注册 |
| Kafka (4) | `tauri_commands.rs` + `#[cfg]` | `lib.rs` 条件注册 |
| OAuth2 (3) | `tauri_commands.rs` + `#[cfg]` | `lib.rs` 条件注册 |
| Dataset (8) | `tauri_commands.rs` | `lib.rs` invoke_handler |
| 其他 (16) | `tauri_commands.rs` + `#[cfg]` | `lib.rs` 条件注册 |

## ARCHITECTURE

```
┌──────────────────────────────────────────────────────┐
│                    React 前端                          │
│  Tauri IPC Client  ──→  Channel<LogEvent>            │
│                  ──→  Channel<ExecutionEvent>         │
└────────────────────────┬─────────────────────────────┘
                         │ invoke()
┌────────────────────────▼─────────────────────────────┐
│  ┌──────────────────────────────────────────────┐     │
│  │  lib.rs: tauri::generate_handler![...]       │     │
│  │  + 5 State managers (Arc<Mutex<T>>)          │     │
│  │  + Plugins (dialog/opener/store/log/updater) │     │
│  └──────┬───────────────────────────────────────┘     │
│         │ #[tauri::command]                           │
│  ┌──────▼───────────────────────────────────────┐     │
│  │  tauri_commands.rs  (IPC wrapper functions)   │     │
│  │  State → args → flow_engine::commands::*      │     │
│  └──────┬───────────────────────────────────────┘     │
│         │ tauri_adapters.rs (trait impls)             │
│  ┌──────▼───────────────────────────────────────┐     │
│  │  flow-engine crate (门面, 业务逻辑)             │     │
│  │  flow-engine-executor, flow-engine-core, ...  │     │
│  └───────────────────────────────────────────────┘     │
│         ^                                              │
│  ┌──────┴──────────┐  on no args: FreeConsole          │
│  │  main.rs        │  on args: cli::run_cli()          │
│  └─────────────────┘                                   │
│              src-tauri/src/                            │
└────────────────────────────────────────────────────────┘
```
