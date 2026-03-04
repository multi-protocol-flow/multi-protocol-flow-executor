# 项目知识库

**生成时间:** 2026-03-04
**Commit:** 0afc6fa
**Branch:** master

## 概述

多协议流程编排与执行引擎。Tauri 2.x 桌面应用 + CLI 工具。
- **前端:** React 19 + TypeScript + Vite 6 + TailwindCSS 4 + Zustand + React Flow
- **后端:** Rust (Tauri 2.x) + async-trait + reqwest + evalexpr

## 结构

```
multi-protocol-flow-executor/
├── src/                    # 前端源码 (React/TypeScript)
│   ├── components/         # UI 组件 (详见 src/components/AGENTS.md)
│   ├── stores/             # Zustand 状态管理
│   ├── services/           # Tauri IPC 服务 + Mock
│   ├── types/              # TypeScript 类型 (与 Rust 对齐)
│   └── __tests__/          # Vitest 单元测试
├── src-tauri/              # Rust 后端 (Workspace)
│   ├── src/                # Tauri 适配层 + IPC 命令 (详见 src-tauri/src/AGENTS.md)
│   ├── crates/flow-engine/ # 独立执行引擎 crate
│   └── bin/cli.rs          # CLI 二进制
├── tests/                  # CLI 测试数据 + fixtures
└── docs/                   # 架构文档
```

## 快速定位

| 任务 | 位置 |
|------|------|
| 添加新节点类型 | `src-tauri/crates/flow-engine/src/` + `src/types/node.ts` |
| 修改执行逻辑 | `src-tauri/crates/flow-engine/src/executor.rs` |
| 添加前端组件 | `src/components/` |
| 修改状态管理 | `src/stores/` |
| HTTP 请求节点 | `src-tauri/crates/flow-engine/src/http_executor.rs` |
| 条件分支节点 | `src-tauri/crates/flow-engine/src/conditional_executor.rs` |
| IPC 命令 | `src-tauri/src/` (Tauri 适配层) |

## 代码规范

### TypeScript
- `strict: true`, `noUnusedLocals`, `noUnusedParameters`
- 路径别名: `@/*` → `src/*`
- **禁止** `as any`, `@ts-ignore`, `@ts-expect-error`

### Rust
- **禁止** `unsafe_code` (forbid)
- `correctness = "deny"` - 正确性问题报错
- 格式化: `cargo fmt`, Lint: `cargo clippy`
- 无 `unwrap()` 调用（使用 `?` 或 `expect`）

### 通用
- 禁止空 catch 块
- 禁止删除失败测试来"通过"
- 注释使用中文

## 回归测试 (强制)

**任何功能变更必须执行回归测试。**

| 变更类型 | 命令 |
|----------|------|
| 前端 | `npm run typecheck && npm run test` |
| 后端 | `cd src-tauri && cargo test && cargo clippy` |
| CLI | `cd src-tauri && cargo test cli_tests` |
| 完整回归 | `npm run typecheck && npm run test && cd src-tauri && cargo test && cargo clippy` |

**禁止:** 跳过测试直接提交、删除失败测试、忽略 clippy 警告

## 反模式 (本项目禁止)

| 模式 | 位置 | 说明 |
|------|------|------|
| `unsafe_code` | Rust | forbid 级别禁止 |
| `as any` | TypeScript | 类型安全红线 |
| `@ts-ignore` | TypeScript | 禁止压制类型错误 |
| CSS `!important` | FlowCanvas.tsx | 仅用于覆盖 ReactFlow 样式 |

## 技术债务

| 位置 | 内容 | 优先级 |
|------|------|--------|
| `src/components/canvas/utils.ts:101` | TODO: 添加更多连接验证逻辑 | 中 |
| `HttpConfig.tsx:128` | console.error 调试代码 | 低 |
| `ConditionalConfig.tsx:111` | console.error 调试代码 | 低 |

## 命令

```bash
# 前端开发
npm run dev          # Vite 开发服务器 (0.0.0.0:5173)
npm run build        # tsc + vite build
npm run test         # Vitest 测试
npm run typecheck    # TypeScript 类型检查

# 后端开发
cd src-tauri
cargo build          # 调试构建
cargo build --release --bin cli  # 生产 CLI
cargo test           # Rust 测试
cargo clippy         # Lint 检查

# Tauri 桌面应用
npm run tauri dev    # 开发模式
npm run tauri build  # 构建安装包
```

## 注意事项

1. **双二进制输出**: `src-tauri/Cargo.toml` 定义了 `app` (Tauri 桌面应用) 和 `cli` (独立 CLI 工具，支持 `cli -e -f flow.json`)

2. **TailwindCSS 4**: 使用 CSS-based 配置 (`@import "tailwindcss"` + `@theme`)，无 `tailwind.config.js`

3. **类型同步**: 前端 `src/types/` 与后端 `src-tauri/src/` 结构体必须保持一致:
   - `node.ts` ↔ `node.rs` + `node_config.rs`
   - `flow.ts` ↔ `flow.rs`
   - `connection.ts` ↔ `connection.rs`

4. **Tauri IPC Mock**: `src/services/tauri/mock.ts` 用于非 Tauri 环境测试

5. **缺失 CI/CD**: 无 `.github/workflows`，需手动运行测试

6. **HTTP Body Types**: 支持 `none`、`body`、`form-data`、`x-www-form-urlencoded`、`binary`，Content-Type 自动设置，用户手动设置优先

7. **HTTP 节点增强**: Content-Type 选择器、form-data 文件上传、超时配置 (`timeout_ms`，默认 30000ms，范围 1000-300000ms)

8. **内置流程模板**: 首次启动时自动加载示例流程 (`src/templates/builtinFlows.ts`)，变量引用使用顶级键 `status`、`body`、`headers`

9. **全局设置**: 报告/日志目录配置，持久化使用 tauri-plugin-store（Tauri）/ localStorage（Mock），默认位置 `{appDataDir}/reports/` 和 `{appDataDir}/logs/`

## 子目录知识库

- [src/components/AGENTS.md](./src/components/AGENTS.md) - 前端组件架构
- [src-tauri/src/AGENTS.md](./src-tauri/src/AGENTS.md) - Tauri 应用层
- [src-tauri/crates/flow-engine/src/AGENTS.md](./src-tauri/crates/flow-engine/src/AGENTS.md) - Flow Engine 核心库
- [src/stores/AGENTS.md](./src/stores/AGENTS.md) - Zustand 状态管理
- [src/services/AGENTS.md](./src/services/AGENTS.md) - Tauri IPC 服务
- [src/types/AGENTS.md](./src/types/AGENTS.md) - TypeScript 类型定义
- [src/__tests__/AGENTS.md](./src/__tests__/AGENTS.md) - 前端测试
