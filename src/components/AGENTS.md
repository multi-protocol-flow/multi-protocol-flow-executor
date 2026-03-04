# 前端组件

## OVERVIEW

React 19 UI 组件库。画布编辑器 + 配置面板 + 执行报告。基于 React Flow + shadcn/ui + react-hook-form + Zod。

## STRUCTURE

```
components/
├── canvas/               # 流程画布
│   ├── FlowCanvas.tsx    # 主画布，拖拽 + 连线 + 节点注册
│   ├── CanvasToolbar.tsx # 工具栏
│   ├── nodes/            # EntryNode, HttpNode, ConditionalNode
│   ├── edges/CustomEdge.tsx
│   └── utils.ts          # Flow ↔ ReactFlow 转换
├── panels/               # 节点配置面板
│   ├── NodeConfigPanel.tsx   # 按 node.type 分发
│   ├── HttpConfig.tsx / ConditionalConfig.tsx / EntryConfig.tsx
│   ├── VariablePanel.tsx     # 变量面板
│   └── schemas.ts            # Zod 验证 schema
├── execution/            # 执行相关
│   ├── ExecuteButton.tsx, ExecutionReport.tsx
│   ├── ExecutionHistory.tsx, NodeExecutionDetails.tsx
│   └── ConsolePanel.tsx      # 控制台输出
├── sidebar/              # FlowListPanel, NodeLibrary
├── dialogs/              # NewFlowDialog, ConfirmDialog
├── settings/             # SettingsDialog (报告/日志目录)
└── ui/                   # shadcn/ui + 自定义 (VariableInput, VariableSelector)
```

## WHERE TO LOOK

| 任务 | 文件 |
|------|------|
| 添加新节点类型 | `canvas/nodes/XxxNode.tsx` + `panels/XxxConfig.tsx` + `schemas.ts` |
| 修改节点样式 | `canvas/nodes/*.tsx` |
| 画布交互/节点注册 | `canvas/FlowCanvas.tsx` → `nodeTypes` |
| 节点配置表单 | `panels/*Config.tsx` |
| 变量输入组件 | `ui/VariableInput.tsx`, `ui/VariableSelector.tsx` |
| 执行报告 | `execution/ExecutionReport.tsx` |
| 流程列表 | `sidebar/FlowListPanel.tsx` |
| 全局设置 | `settings/SettingsDialog.tsx` |

## CONVENTIONS

- **Store**: `useFlowStore()` 管理流程/节点/边，`useExecutionStore()` 管理执行状态
- **配置面板**: `react-hook-form` + Zod schema，变更通过 `updateNode()` 同步
- **节点注册**: `FlowCanvas.tsx` 中 `nodeTypes` 对象

## 注意事项

1. **新增节点需同步 4 处**: `canvas/nodes/`, `panels/`, `schemas.ts`, `FlowCanvas.tsx` 的 `nodeTypes`
2. **Flow 转换**: `canvas/utils.ts` 处理 `Flow` ↔ `ReactFlow` 互转
3. **类型对齐**: 节点 `data` 需与 `src/types/node.ts` 一致
4. **变量引用**: 使用 `{{path.to.value}}` 语法，`VariableInput` 组件提供自动补全
