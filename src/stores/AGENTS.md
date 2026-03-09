# 状态管理 (Stores)

**生成时间:** 2026-03-09

## 概述

Zustand 5.x 状态管理，每个 store 一个文件。使用 `create` + `selectors` 模式。共 7 个 stores。

## 结构

```
stores/
├── index.ts          # 统一导出 (hooks + selectors + types)
├── flow.ts           # 当前编辑流程 (flow, nodes, edges, isDirty, breakpoints)
├── execution.ts      # 执行状态 (isExecuting, report, history)
├── flowList.ts       # 流程列表 (flowMetas, isLoading)
├── ui.ts             # UI 状态 (面板开关, 对话框)
├── settings.ts       # 全局设置 (报告/日志目录)
├── debug.ts          # 调试状态 (isPaused, currentNodeUuid, isDebugging)
└── console.ts        # 控制台日志
```

## 快速定位

| 任务 | 文件 |
|------|------|
| 修改流程状态 | `flow.ts` |
| 添加执行逻辑 | `execution.ts` |
| 修改面板状态 | `ui.ts` |
| 添加全局设置 | `settings.ts` |
| 调试状态管理 | `debug.ts` |
| 控制台日志 | `console.ts` |

## 约定

### Store 模式
```typescript
export const useXxxStore = create<XxxState>((set, get) => ({
  // State
  value: null,
  
  // Actions
  setValue: (v) => set({ value: v }),
  reset: () => set(initialState),
  
  // Computed
  getValue: () => get().value,
}));

// Selectors (导出用于性能优化)
export const selectValue = (state: XxxState) => state.value;
```

### 使用方式
```typescript
// 组件中
const { flow, updateNode } = useFlowStore();
const isExecuting = useExecutionStore(selectIsExecuting);
const isPaused = useDebugStore(selectIsPaused);

// 外部调用
useFlowStore.getState().reset();
useDebugStore.getState().continueExecution();
```

### 重置规则
- 所有 store 必须实现 `reset()` 方法
- 测试 `beforeEach` 必须调用 `reset()`

## Stores 列表 (7个)

| Store | 用途 | 关键状态 |
|-------|------|----------|
| `flow` | 流程编辑 | flow, nodes, edges, breakpoints |
| `execution` | 执行状态 | isExecuting, report, history |
| `flowList` | 流程列表 | flowMetas, isLoading |
| `ui` | UI 状态 | configPanelOpen, showConfirm |
| `settings` | 全局设置 | reportsDir, logsDir |
| `debug` | 调试状态 | isPaused, currentNodeUuid, isDebugging |
| `console` | 控制台 | logs |
