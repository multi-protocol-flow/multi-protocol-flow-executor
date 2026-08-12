# 类型定义知识库

**生成时间**: 2026-06-12 | **Branch**: dev

## OVERVIEW

TypeScript 类型定义层，与 Rust 后端结构体严格对齐。核心是 `NodeData` 判别联合类型（tagged union，65 个变体覆盖全部 89+ 节点类型），每个变体的 `type` 字段与后端 `NodeType` 枚举一一对应。所有 IPC 调用、Store 状态、组件 Props 均依赖此目录下的类型。

## STRUCTURE

| 文件 | Rust 映射 |
|------|-----------|
| `node.ts` | `flow-engine-core/src/node.rs` (Node/NodeType/Port) + `node_config.rs` (NodeData 变体) |
| `flow.ts` | `flow-engine-core/src/flow.rs` (Flow/EnvironmentSet) |
| `connection.ts` | `flow-engine-core/src/connection.rs` |
| `report.ts` | `flow-engine-core/src/report.rs` + `statistics.rs` |
| `variable.ts` | `flow-engine-core/src/variable.rs` |
| `error.ts` | `flow-engine-core/src/error.rs` |
| `debug.ts` | `flow-engine/src/debug/types.rs` |
| `ipc.ts` | `flow-engine/src/commands/*` (IPC 命令输入/输出类型) |
| `port.ts` | `flow-engine-core/src/node.rs` (Port 结构体) |
| `nodeRegistry.ts` | `flow-engine-core/src/plugin.rs` (NodeMetadata) |
| `hook.ts` | `flow-engine-core/src/hook.rs` |
| `dataset.ts` | `flow-engine/src/dataset/types.rs` + events |
| `events.ts` | `flow-engine-core/src/events.rs` (LogEvent) |
| `execution_event.ts` | `flow-engine-core/src/events.rs` (ExecutionEvent, camelCase tagged union) |
| `stressTest.ts` | `flow-engine/src/stress_test/types.rs` |
| `ws_stream.ts` | `flow-engine/src/debug/ws_stream.rs` |
| `smtp.ts` | `flow-engine-smtp/` crate 结构体 |
| `imap.ts` | `flow-engine-imap/` crate 结构体 |
| `settings.ts` | 前端独有（无 Rust 映射） |
| `updater.ts` | 前端独有（Tauri updater events） |
| `index.ts` | barrel re-export |

## WHERE TO LOOK

| 任务 | 文件 | Rust 对应 |
|------|------|-----------|
| 新增节点类型 | `node.ts` (NodeType + NodeData 变体) | `node.rs` + `node_config.rs` |
| 修改流程结构 | `flow.ts` | `flow.rs` |
| 调整连接模型 | `connection.ts` | `connection.rs` |
| 执行报告字段 | `report.ts` | `report.rs` + `statistics.rs` |
| 变量系统变更 | `variable.ts` | `variable.rs` |
| 错误类型/处理 | `error.ts` | `error.rs` |
| 调试会话 | `debug.ts` | `flow-engine/src/debug/types.rs` |
| IPC 命令签名 | `ipc.ts` | `flow-engine/src/commands/*` |
| 注册表元数据 | `nodeRegistry.ts` | `plugin.rs` (NodeMetadata) |
| 协议特定配置 | `smtp.ts` / `imap.ts` | 对应 `flow-engine-*` crate（redis 已插件化，无专用类型文件） |
| 压力测试 | `stressTest.ts` | `flow-engine/src/stress_test/types.rs` |
| 数据集执行 | `dataset.ts` | `flow-engine/src/dataset/types.rs` |
| App 设置 | `settings.ts` | 无（前端唯一定义） |

## CONVENTIONS

- **判别联合优先**: `NodeData` 使用 `type: '<node_type>'` 字段做运行时区分。禁止揉合成一个松散的 `Record<string, any>`。
- **可选字段**: 所有 `?` 标记的字段对应 Rust 的 `Option<T>`。不存在 Rust 侧有 `Option` 但 TS 侧无 `?` 的情况。
- **类型守卫**: 用 `if (data.type === 'http')` 做窄化，禁用 `as any` 强制转换。
- **导出统一**: 每个文件以 `export type { ... }` 或 `export interface` 为主。`index.ts` 是唯一 barrel file。
- **序列化对齐**: `execution_event.ts` 使用 camelCase 匹配 Rust 的 `#[serde(rename_all = "camelCase")]`。

## ANTI-PATTERNS

- **单侧修改**: 在 `node.ts` 加了一个 `NodeType` 但忘记在后端 `node_config.rs` 添加对应结构体（或反之）→ 运行时反序列化失败。
- **`as any` 逃逸**: 用 `(data as any).someField` 代替类型窄化 → 破坏 TypeScript 严格性，掩盖类型不匹配。
- **`@ts-ignore` / `@ts-expect-error`**: 项目中已禁止。遇到类型错误应修正定义而非跳过检查。
- **内联重复类型**: 在 Store 或组件中重复定义类型而非引用 `types/` → 两端不一致。所有跨组件类型都应在 `types/` 中定义。
- **忘记更新 `index.ts`**: 新增文件后未在 `index.ts` 中 re-export → 导入路径不一致。
