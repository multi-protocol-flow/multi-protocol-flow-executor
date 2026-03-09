# Tauri 应用层

**生成时间:** 2026-03-09
**Commit:** cc2bf47

## OVERVIEW

Tauri 应用入口 + IPC 命令适配层。业务逻辑委托给 `crates/flow-engine/`。

## STRUCTURE

```
src/
├── lib.rs              # 模块导出 + Tauri 应用启动 (插件注册/日志配置)
├── main.rs             # 应用入口 (调用 lib::run())
├── tauri_adapters.rs   # Tauri 适配器 (LogSender/DirectoryOpener/AppDirProvider)
└── tauri_commands.rs   # IPC 命令 (15 个 #[tauri::command] 包装器)
```

## WHERE TO LOOK

| 任务 | 文件 |
|------|------|
| 添加 IPC 命令 | `tauri_commands.rs` |
| 修改日志/插件配置 | `lib.rs` → `run()` |
| 添加 Tauri 适配器 | `tauri_adapters.rs` |
| 修改应用启动逻辑 | `lib.rs` → `setup` |
| 执行器/节点/流程等 | `crates/flow-engine/` |

## IPC 命令 (15个)

### 流程命令
| 命令 | 用途 |
|------|------|
| `create_flow` | 创建流程 |
| `execute_flow` | 执行流程 (带 Channel 日志) |
| `update_node` | 更新节点 |
| `delete_node` | 删除节点 |

### 报告命令
| 命令 | 用途 |
|------|------|
| `save_flow_report` | 保存执行报告 |
| `load_flow_report` | 加载报告 |
| `list_flow_reports` | 列出所有报告 |
| `delete_flow_report` | 删除报告 |

### 设置命令
| 命令 | 用途 |
|------|------|
| `get_app_data_dir` | 获取应用数据目录 |
| `open_directory` | 打开目录 |

### 调试命令
| 命令 | 用途 |
|------|------|
| `start_debug` | 启动调试会话 (双 Channel) |
| `send_debug_command` | 发送调试命令 (Continue/StepOver) |
| `end_debug` | 结束调试会话 |
| `get_debug_status` | 获取调试状态 |
| `force_end_debug` | 强制结束 (页面刷新恢复) |

## CONVENTIONS

- **命令委托**: IPC 命令仅做类型转换，调用 `flow_engine::commands::*`
- **适配器模式**: 实现 `flow_engine::traits::*` 接口桥接 Tauri
- **Channel 日志**: `TauriLogSender` 包装 `Channel<LogEvent>` 发送到前端
- **状态管理**: `DebugSessionManager` 通过 `tauri::State` 共享
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

