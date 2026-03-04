# 状态管理 (Stores)

## 概述

Zustand 5.x 状态管理，每个 store 一个文件。使用 `create` + `selectors` 模式。

## 结构

```
stores/
├── index.ts          # 统一导出 (hooks + selectors + types)
├── flow.ts           # 当前编辑流程 (flow, nodes, edges, isDirty)
├── execution.ts      # 执行状态 (isExecuting, report, history)
├── flowList.ts       # 流程列表 (flowMetas, isLoading)
├── ui.ts             # UI 状态 (面板开关, 对话框)
├── settings.ts       # 全局设置 (报告/日志目录)
└── console.ts        # 控制台日志
```

## 快速定位

| 任务 | 文件 |
|------|------|
| 修改流程状态 | `flow.ts` |
| 添加执行逻辑 | `execution.ts` |
| 修改面板状态 | `ui.ts` |
| 添加全局设置 | `settings.ts` |

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

// 外部调用
useFlowStore.getState().reset();
```

### 重置规则
- 所有 store 必须实现 `reset()` 方法
- 测试 `beforeEach` 必须调用 `reset()`
