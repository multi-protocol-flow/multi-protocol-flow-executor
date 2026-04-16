# 前端组件

**生成时间:** 2026-04-16

## OVERVIEW

React 19 UI 组件库 (97 文件)。画布编辑器 + 配置面板 + 执行报告 + 调试模式 + 压力测试。基于 React Flow + shadcn/ui + react-hook-form + Zod。

## STRUCTURE

```
components/
├── canvas/               # 流程画布
│   ├── FlowCanvas.tsx    # 主画布，拖拽 + 连线 + 节点注册
│   ├── CanvasToolbar.tsx # 工具栏
│   ├── DebugToolbar.tsx  # 调试工具栏 (继续/单步/跳过/结束)
│   ├── BreakpointMarker.tsx # 断点标记
│   ├── nodes/            # 14 种节点: Entry, Http, Conditional, Script, Assertion, Ws*, Db*
│   ├── edges/CustomEdge.tsx
│   └── utils.ts          # Flow ↔ ReactFlow 转换
├── panels/               # 节点配置面板 (20 文件)
│   ├── NodeConfigPanel.tsx   # 按 node.type 分发
│   ├── HttpConfig.tsx / ConditionalConfig.tsx / EntryConfig.tsx / ScriptConfig.tsx
│   ├── AssertionConfig.tsx   # 断言配置
│   ├── WsConnectConfig.tsx / WsSendWaitConfig.tsx / WsCloseConfig.tsx
│   ├── DbConnectConfig.tsx / DbQueryConfig.tsx / DbBeginConfig.tsx
│   ├── DbCommitConfig.tsx / DbRollbackConfig.tsx / DbCloseConfig.tsx
│   ├── VariablePanel.tsx     # 变量面板
│   ├── DebugPanel.tsx        # 调试面板
│   └── schemas.ts            # Zod 验证 schema
├── execution/            # 执行相关 (7 文件)
│   ├── ExecuteButton.tsx, ExecutionReport.tsx
│   ├── ExecutionHistory.tsx, NodeExecutionDetails.tsx
│   ├── ConsolePanel.tsx      # 控制台输出
│   ├── WsStreamsPanel.tsx    # WebSocket 消息流
│   └── viewers/              # 节点执行结果查看器 (9 文件)
├── sidebar/              # FlowListPanel, NodeLibrary, ConnectionsPanel
├── dialogs/              # NewFlowDialog, ConfirmDialog, DebugNodeConfigDialog, SkipNodeDialog, ConnectionDialog
├── settings/             # SettingsDialog (报告/日志目录)
├── stress-test/          # 压力测试 UI (StressTestConfigDialog, StressTestMonitor, StressTestResult, StressTestCharts)
├── debug/                # WsMessageStreamPanel
└── ui/                   # shadcn/ui + 自定义 (VariableInput, VariableSelector, SqlEditor, DbResultPreview 等)
```

## WHERE TO LOOK

| 任务 | 文件 |
|------|------|
| 添加新节点类型 | `canvas/nodes/XxxNode.tsx` + `panels/XxxConfig.tsx` + `schemas.ts` + `execution/viewers/XxxViewer.tsx` |
| 修改节点样式 | `canvas/nodes/*.tsx` |
| 画布交互/节点注册 | `canvas/FlowCanvas.tsx` → `nodeTypes` |
| 节点配置表单 | `panels/*Config.tsx` |
| 变量输入组件 | `ui/VariableInput.tsx`, `ui/VariableSelector.tsx` |
| SQL 编辑器 | `ui/SqlEditor.tsx` |
| 数据库结果预览 | `ui/DbResultPreview.tsx` |
| 执行报告 | `execution/ExecutionReport.tsx` |
| 调试工具栏 | `canvas/DebugToolbar.tsx` |
| 压力测试 | `stress-test/*.tsx` |
| 流程列表 | `sidebar/FlowListPanel.tsx` |
| 数据库连接管理 | `sidebar/ConnectionsPanel.tsx`, `dialogs/ConnectionDialog.tsx` |
| 全局设置 | `settings/SettingsDialog.tsx` |

## CONVENTIONS

- **Store**: `useFlowStore()` 管理流程/节点/边，`useExecutionStore()` 管理执行状态，`useDebugStore()` 管理调试状态，`useStressTestStore()` 管理压测状态，`useConnectionsStore()` 管理数据库连接
- **配置面板**: `react-hook-form` + Zod schema，变更通过 `updateNode()` 同步
- **节点注册**: `FlowCanvas.tsx` 中 `nodeTypes` 对象
- **断点**: 通过 `useFlowStore().toggleBreakpoint()` 管理，`BreakpointMarker` 组件渲染

## 节点类型 (14 种)

| 类型 | 组件 | 配置面板 | 颜色 |
|------|------|----------|------|
| Entry | EntryNode.tsx | EntryConfig.tsx | 绿色 |
| Http | HttpNode.tsx | HttpConfig.tsx | 蓝色 |
| Conditional | ConditionalNode.tsx | ConditionalConfig.tsx | 橙色 |
| Script | ScriptNode.tsx | ScriptConfig.tsx | 紫色 |
| Assertion | AssertionNode.tsx | AssertionConfig.tsx | 红色 |
| WsConnect | WsConnectNode.tsx | WsConnectConfig.tsx | 琥珀色 |
| WsSendWait | WsSendWaitNode.tsx | WsSendWaitConfig.tsx | 青色 |
| WsClose | WsCloseNode.tsx | WsCloseConfig.tsx | 玫红色 |
| DbConnect | DbConnectNode.tsx | DbConnectConfig.tsx | 靛蓝色 |
| DbQuery | DbQueryNode.tsx | DbQueryConfig.tsx | 靛蓝色 |
| DbBegin | DbBeginNode.tsx | DbBeginConfig.tsx | 靛蓝色 |
| DbCommit | DbCommitNode.tsx | DbCommitConfig.tsx | 靛蓝色 |
| DbRollback | DbRollbackNode.tsx | DbRollbackConfig.tsx | 靛蓝色 |
| DbClose | DbCloseNode.tsx | DbCloseConfig.tsx | 靛蓝色 |

## 注意事项

1. **新增节点需同步 5 处**: `canvas/nodes/`, `panels/`, `schemas.ts`, `FlowCanvas.tsx` 的 `nodeTypes`, `execution/viewers/`
2. **Flow 转换**: `canvas/utils.ts` 处理 `Flow` ↔ `ReactFlow` 互转
3. **类型对齐**: 节点 `data` 需与 `src/types/node.ts` 一致
4. **变量引用**: 使用 `{{path.to.value}}` 语法，`VariableInput` 组件提供自动补全
5. **调试状态**: `useDebugStore()` 管理断点、暂停、当前节点等状态
6. **压力测试**: `useStressTestStore()` 管理压测配置和状态
