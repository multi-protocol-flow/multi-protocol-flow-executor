# 项目知识库

**生成时间:** 2026-02-28
**Commit:** 3a15f83
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
├── src-tauri/              # 后端源码 (Rust)
│   ├── src/                # 执行引擎核心 (详见 src-tauri/src/AGENTS.md)
│   └── bin/cli.rs          # 独立 CLI 二进制
├── tests/                  # CLI 测试数据 + fixtures
└── docs/                   # 架构文档
```

## 快速定位

| 任务 | 位置 | 备注 |
|------|------|------|
| 添加新节点类型 | `src-tauri/src/` + `src/types/node.ts` | 前后端类型需同步 |
| 修改执行逻辑 | `src-tauri/src/executor.rs` | 核心执行器 |
| 添加前端组件 | `src/components/` | 参考 shadcn/ui 风格 |
| 修改状态管理 | `src/stores/` | Zustand 模式 |
| HTTP 请求节点 | `src-tauri/src/http_executor.rs` | reqwest 封装 |
| HTTP body types | `src-tauri/src/node_config.rs` | body_type/content_type/form_fields/file_path |
| 条件分支节点 | `src-tauri/src/conditional_executor.rs` | evalexpr 表达式 |
| 内置流程模板 | `src/templates/builtinFlows.ts` | DEMO_FLOW 示例流程 |
| 首次启动模板加载 | `src/components/sidebar/FlowListPanel.tsx` | 空列表时自动加载 |
| 全局设置 | `src/components/settings/SettingsDialog.tsx` | 报告/日志目录配置 |
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

1. **双二进制输出**: `src-tauri/Cargo.toml` 定义了两个 binary:
   - `app` (Tauri 桌面应用)
   - `cli` (独立 CLI 工具，支持 `cli -e -f flow.json`)

2. **TailwindCSS 4**: 使用 CSS-based 配置 (`@import "tailwindcss"` + `@theme`)，无 `tailwind.config.js`

3. **类型同步**: 前端 `src/types/` 与后端 `src-tauri/src/` 结构体必须保持一致:
   - `node.ts` ↔ `node.rs` + `node_config.rs`
   - `flow.ts` ↔ `flow.rs`
   - `connection.ts` ↔ `connection.rs`

4. **Tauri IPC Mock**: `src/services/tauri/mock.ts` 用于非 Tauri 环境测试

5. **缺失 CI/CD**: 无 `.github/workflows`，需手动运行测试

6. **HTTP Body Types**: 支持多种请求体类型:
   - `none`: 无请求体（GET 请求默认）
   - `body`: 原始请求体（JSON/Text/HTML/XML/JS）
   - `form-data`: multipart 表单（支持文件上传）
   - `x-www-form-urlencoded`: URL 编码表单
   - `binary`: 二进制文件上传
   - Content-Type 自动设置，用户手动设置优先

7. **HTTP 节点增强功能**:
   - **Content-Type 选择器**: `body_type='body'` 时可选择 json/text/html/xml/javascript
   - **form-data 文件上传**: 支持 `FormFieldType::File`，自动检测 MIME 类型
   - **超时配置**: `timeout_ms` 字段，默认 30000ms，范围 1000-300000ms
   - 测试覆盖: `http_executor.rs` 包含完整单元测试

8. **内置流程模板**: 首次启动时自动加载示例流程:
   - 模板文件: `src/templates/builtinFlows.ts`
   - 加载逻辑: `FlowListPanel.tsx` 的 `useEffect` 中检测 `flowMetas.length === 0`
   - 包含节点: Entry → HTTP GET → 条件判断 → HTTP POST (成功分支)
   - 变量引用: HTTP 响应使用顶级键 `status`、`body`、`headers` (非 `http_response.xxx`)
   - 不会覆盖用户已有数据

9. **全局设置功能**: 
   - 设置入口: 顶部工具栏齿轮图标
   - 设置项: 报告目录 + 日志目录
   - 持久化: tauri-plugin-store（Tauri）/ localStorage（Mock）
   - 默认位置: `{appDataDir}/reports/` 和 `{appDataDir}/logs/`
   - 新增命令: `get_app_data_dir()` - 获取应用数据目录
## 子目录知识库

- [src/components/AGENTS.md](./src/components/AGENTS.md) - 前端组件架构
- [src-tauri/src/AGENTS.md](./src-tauri/src/AGENTS.md) - Rust 执行引擎
