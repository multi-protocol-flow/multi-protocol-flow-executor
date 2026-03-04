# Tauri 应用层

**生成时间:** 2026-03-04
**Commit:** 0afc6fa

## OVERVIEW

Tauri 应用入口 + IPC 命令适配层。业务逻辑委托给 `crates/flow-engine/`。

## STRUCTURE

```
src/
├── lib.rs              # 模块导出 + Tauri 应用启动 (插件注册/日志配置)
├── main.rs             # 应用入口 (调用 lib::run())
├── tauri_adapters.rs   # Tauri 适配器 (LogSender/DirectoryOpener/AppDirProvider)
└── tauri_commands.rs   # IPC 命令 (#[tauri::command] 包装器)
```

## WHERE TO LOOK

| 任务 | 文件 |
|------|------|
| 添加 IPC 命令 | `tauri_commands.rs` |
| 修改日志/插件配置 | `lib.rs` → `run()` |
| 添加 Tauri 适配器 | `tauri_adapters.rs` |
| 修改应用启动逻辑 | `lib.rs` → `setup` |
| 执行器/节点/流程等 | `crates/flow-engine/` |

## CONVENTIONS

- **命令委托**: IPC 命令仅做类型转换，调用 `flow_engine::commands::*`
- **适配器模式**: 实现 `flow_engine::traits::*` 接口桥接 Tauri
- **Channel 日志**: `TauriLogSender` 包装 `Channel<LogEvent>` 发送到前端
- **测试**: Windows 跳过单元测试 (Tauri 依赖问题)

## 与 flow-engine 关系

```
前端 (React)
    ↓ Tauri IPC
tauri_commands.rs  ←  #[tauri::command] 包装
    ↓ 调用
flow_engine::commands  ←  业务逻辑
    ↓
flow_engine::executor  ←  执行引擎核心
```

