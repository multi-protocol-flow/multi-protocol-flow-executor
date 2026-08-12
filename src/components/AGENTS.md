# 前端组件库

**生成时间:** 2026-06-12 | **文件数:** ~112 (.tsx/.ts) | **子目录:** 10

## 概览

React 19 UI 组件层。覆盖流程编辑器全生命周期：画布编辑、节点配置、执行监控、报告查看、调试、压力测试。基于 @xyflow/react + shadcn/ui + react-hook-form + Zod。

## 结构

```
components/
├── canvas/ (12)          主画布
│   ├── FlowCanvas.tsx    注册全部 ~70 节点类型到 nodeTypes
│   ├── CanvasToolbar.tsx 缩放/布局/撤销
│   ├── DebugToolbar.tsx  调试步进/继续/断点
│   ├── NodePalette.tsx   拖拽调色板
│   ├── NodeContextMenu.tsx / BreakpointMarker / ExecutionOverlay
│   ├── nodes/ (94 文件)  每 NodeType 一个 .tsx 组件
│   ├── edges/CustomEdge  含动画执行箭头
│   └── layout.ts / utils.ts
├── panels/ (104)         节点配置面板
│   ├── NodeConfigPanel.tsx 按 nodeType 分发到具体面板
│   ├── Http/Grpc/Mcp/Tcp/Udp/Mqtt/Kafka/Sse/Graphql/Amqp/Imap/Smtp/Redis/Db
│   ├── Assertion/Conditional/Script/Entry/End/Proxy/VariableExtractor
│   ├── SettingsPanel / EnvironmentManager / SchemaConfigPanel
│   ├── schemas.ts        ~1700 行 Zod schema
│   └── shared/           OutputMappingsEditor, ImapShared
├── execution/ (19)       执行结果
│   ├── ConsolePanel / ExecutionReport / ExecutionPanel / ExecutionHistory
│   ├── VariableTreeView / VariableDependencyGraph
│   └── viewers/ (20)     各协议查看器
├── sidebar/ (3)          LeftSidebarPanel + FlowListPanel
├── dialogs/ (9)         NewFlow/Confirm/Debug/ScriptHelp/Shortcuts/Search/
│                         ValidationError/SkipNode/Update/Connection
├── ui/ (25)              shadcn/ui 基座 + VariableInput/VariableSelector/
│                         ConfigSection/SearchableSelect
├── stress-test/ (4)      ConfigDialog/Monitor/Charts/Result
├── reports/ (5)          DatasetReportList/Detail/Dialog/RowDetail
├── welcome/ (2)          WelcomeWizard + GuideHighlighter
└── common/ (1)           HexViewer
```

## 找什么

| 任务 | 位置 |
|------|------|
| 新增节点类型 | `canvas/nodes/` + `panels/` + `execution/viewers/` + 3 注册表 |
| 改节点外观 | `canvas/nodes/<X>Node.tsx`，配色在 `@/lib/NodeRegistry` |
| 改配置面板 | `panels/<X>Config.tsx` + `panels/schemas.ts` |
| 改查看器 | `execution/viewers/<X>Viewer.tsx` |
| 改画布交互 | `canvas/FlowCanvas.tsx` (快捷键/拖拽/剪贴板/布局) |
| 加 shadcn/ui 组件 | `ui/` 下加文件 |
| 加对话框 | `dialogs/` 加文件，注册到 `@/lib/DialogRegistry` |
| 压测 UI | `stress-test/` |
| 数据集报告 | `reports/` |

## 约定

- **Store 分工**: `useFlowStore`(流程/节点)、`useUIStore`(UI 状态)、`useExecutionStore`(执行结果)、`useDebugStore`(调试)、`useFlowListStore`(流程列表)。跨 store 用 `.getState()` 避免订阅。
- **Config 模式**: `react-hook-form` + `zodResolver(schema)` + `onSave` 回调。大协议 schema 放在自身文件头部。
- **节点注册**: 新增节点类型必须同步注册 3 处 — `NodeRegistry.ts`(元数据)、`configPanelRegistry.ts`(面板映射)、`viewerRegistry.tsx`(查看器映射)。
- **断点**: `useDebugStore` 持有 `breakpoints: Set<string>`。`BreakpointMarker` 只负责渲染和点击事件。
- **i18n**: 全部用 `getLL()` 懒加载。新增翻译 key 同步更新 `zh-CN`/`en-US`/`i18n-types.ts`。

## 反模式

- 新增节点跳过任一注册表 → 白屏（配置面板或查看器找不到组件）
- 组件内直接调 Tauri IPC → 必须走 `@/services/tauri/` 服务层
- `useFlowStore` 取 UI 状态 → UI 状态放 `useUIStore`，分开订阅减少重渲染
- 直接修改 `node.data` → 必须通过 `useFlowStore.updateNodeData()`
