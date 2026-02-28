# 前端组件

## OVERVIEW

React 19 UI 组件库。画布编辑器 + 配置面板 + 执行报告。基于 React Flow + shadcn/ui。

## STRUCTURE

```
components/
├── canvas/           # 流程画布
│   ├── FlowCanvas.tsx      # 主画布，拖拽 + 连线
│   ├── nodes/              # EntryNode, HttpNode, ConditionalNode
│   ├── edges/CustomEdge.tsx
│   └── utils.ts            # flow ↔ ReactFlow 转换
├── panels/           # 节点配置面板
│   ├── NodeConfigPanel.tsx # 配置容器，按类型分发
│   ├── EntryConfig.tsx / HttpConfig.tsx / ConditionalConfig.tsx
│   └── schemas.ts          # Zod 验证 schema
├── execution/        # ExecuteButton, ExecutionReport, ExecutionHistory, NodeExecutionDetails
├── sidebar/          # FlowListPanel, NodeLibrary
├── dialogs/          # NewFlowDialog
└── ui/               # shadcn/ui 基础组件
```

## WHERE TO LOOK

| 任务 | 文件 |
|------|------|
| 添加新节点类型 | `canvas/nodes/XxxNode.tsx` + `panels/XxxConfig.tsx` + `schemas.ts` |
| 修改节点样式 | `canvas/nodes/*.tsx` |
| 修改连线 | `canvas/edges/CustomEdge.tsx` |
| 画布交互 | `canvas/FlowCanvas.tsx` |
| 节点类型注册 | `FlowCanvas.tsx` → `nodeTypes` 对象 |
| 执行报告 | `execution/ExecutionReport.tsx` |
| 流程列表 | `sidebar/FlowListPanel.tsx` |

## CONVENTIONS

### React Flow 节点注册
```tsx
const nodeTypes = { entry: EntryNode, http: HttpNode, conditional: ConditionalNode };
```

### Store 使用
```tsx
const { flow, nodes, edges } = useFlowStore();
const { executeFlow, isExecuting } = useExecutionStore();
```

### 配置面板
- `NodeConfigPanel.tsx` 按 `node.type` 分发
- 使用 `react-hook-form` + Zod schema
- 变更通过 `useFlowStore().updateNode()` 同步

## 注意事项

1. **新增节点需同步 4 处**: `nodes/XxxNode.tsx`, `panels/XxxConfig.tsx`, `schemas.ts`, `FlowCanvas.tsx` 的 `nodeTypes`
2. **Flow 转换**: `canvas/utils.ts` 处理 `Flow` ↔ `ReactFlow` 互转
3. **类型对齐**: 节点 `data` 需与 `src/types/node.ts` 一致
