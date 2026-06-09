# Tauri 应用层

**生成时间:** 2026-06-09
**Branch:** dev

## OVERVIEW

Tauri 应用入口 + IPC 命令适配层。业务逻辑委托给 `crates/flow-engine/`。

## STRUCTURE

```
src/
├── lib.rs              # 模块导出 + Tauri 应用启动 (插件注册/日志配置)
├── main.rs             # 应用入口 (调用 lib::run())
├── tauri_adapters.rs   # Tauri 适配器 (LogSender/DirectoryOpener/AppDirProvider)
└── tauri_commands.rs   # IPC 命令 (72 个 #[tauri::command] 包装器)
```

## WHERE TO LOOK

| 任务 | 文件 |
|------|------|
| 添加 IPC 命令 | `tauri_commands.rs` |
| 修改日志/插件配置 | `lib.rs` → `run()` |
| 添加 Tauri 适配器 | `tauri_adapters.rs` |
| 修改应用启动逻辑 | `lib.rs` → `setup` |
| 执行器/节点/流程等 | `crates/flow-engine/` |

## IPC 命令 (72 个)

| 分类 | 数量 | 命令 |
|------|------|------|
| Flow | 6 | create_flow, execute_flow, cancel_flow, update_node, delete_node, validate_flow |
| Reports | 4 | save/load/list/delete_flow_report |
| Debug | 5 | start_debug, send_debug_command, end_debug, get_debug_status, force_end_debug |
| Stress Test | 3 | start/stop/get_stress_test_status |
| Stress Reports | 4 | save/load/list/delete_stress_test_report |
| gRPC | 8 | discover_services, generate_request_skeleton, generate_from_reflection, validate_request, scan_proto_directory, read_proto_files, parse_descriptor_set, channelz_info |
| MCP | 1 | mcp_discover_tools |
| Kafka | 4 | test_connection, list_topics, list_consumer_groups, discover_schema |
| Email OAuth2 | 3 | authorize, refresh, revoke |
| Database | 1 | test_db_connection |
| MQTT | 1 | mqtt_test_connection |
| SSE | 1 | sse_test_connection |
| AMQP | 1 | amqp_test_connection |
| SMTP | 1 | test_smtp_connection |
| IMAP | 1 | test_imap_connection |
| TCP | 2 | tcp_list_templates, tcp_get_template |
| GraphQL | 1 | graphql_introspect_schema |
| Settings | 2 | get_app_data_dir, open_directory |
| CSV | 1 | save_csv_template |
| Flow Files | 5 | list/load/save/delete/copy_flow_file |
| Folders | 6 | list/create/delete/rename_folder, move_flow_to_folder, move_folder |
| Registry | 3 | get_registered_node_types, get_node_config_schema, list_registered_plugins |
| Dataset | 8 | validate, execute, cancel, save/load/list/delete_report, load_row_detail |

## CONVENTIONS

- **命令委托**: IPC 命令仅做类型转换，调用 `flow_engine::commands::*`
- **适配器模式**: 实现 `flow_engine::traits::*` 接口桥接 Tauri
- **Channel 日志**: `TauriLogSender` 包装 `Channel<LogEvent>` 发送到前端
- **状态管理**: `DebugSessionManager` + `ExecutionSessionManager` (cancel 支持) 通过 `tauri::State` 共享
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

