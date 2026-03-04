# TypeScript 类型定义

**生成时间:** 2026-03-04

## 概述

前端 TypeScript 类型定义，与 Rust 后端结构体严格对齐。所有类型定义必须与后端保持同步。

## 文件结构

```
src/types/
├── index.ts          # 统一导出入口
├── node.ts           # 节点类型 + 配置 (判别联合)
├── flow.ts           # 流程结构
├── connection.ts     # 节点连接
├── report.ts         # 执行报告 + 统计
├── variable.ts       # 变量系统
├── error.ts          # 错误类型
├── ipc.ts            # Tauri IPC 命令类型
├── settings.ts       # 全局设置
└── port.ts           # 端口定义
```

## 快速定位

| 任务 | 文件 | 对应 Rust 文件 |
|------|------|---------------|
| 修改节点类型 | `node.ts` | `node.rs` + `node_config.rs` |
| 修改流程结构 | `flow.ts` | `flow.rs` |
| 修改连接定义 | `connection.ts` | `connection.rs` |
| 修改执行报告 | `report.ts` | `report.rs` + `statistics.rs` |
| 修改变量系统 | `variable.ts` | `variable.rs` |
| 添加 IPC 命令 | `ipc.ts` | `command.rs` |

## 约定

### 前后端类型同步 (强制)

**任何类型修改必须同时更新前后端。**

对应关系：
- `node.ts` ↔ `src-tauri/src/node.rs` + `src-tauri/src/node_config.rs`
- `flow.ts` ↔ `src-tauri/src/flow.rs`
- `connection.ts` ↔ `src-tauri/src/connection.rs`
- `report.ts` ↔ `src-tauri/src/report.rs` + `src-tauri/src/statistics.rs`
- `variable.ts` ↔ `src-tauri/src/variable.rs`
- `error.ts` ↔ `src-tauri/src/error.rs`

### 类型安全红线

- **禁止** `as any` 类型断言
- **禁止** `@ts-ignore` / `@ts-expect-error`
- 使用判别联合类型 (`NodeData`) 代替松散类型
- 类型守卫函数用于运行时判断 (`isHttpConfig` 等)

### 判别联合类型

`NodeData` 使用判别联合，通过 `type` 字段区分：
```typescript
type NodeData = EntryConfig | HttpConfig | ConditionalConfig
```

### 可选字段

- 可选字段使用 `?:` 标记
- 必须与 Rust 端 `Option<T>` 对应
- 默认值在 Rust 端实现，前端仅声明类型
