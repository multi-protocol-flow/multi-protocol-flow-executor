# 状态管理 (Stores)

**生成时间:** 2026-04-23

## 概述

Zustand 5.x 状态管理，每个 store 一个文件。使用 `create` + `selectors` 模式。共 10 个 stores。

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
├── console.ts        # 控制台日志
├── stressTest.ts     # 压力测试状态 (status, config, progress, report)
├── wsStream.ts       # WebSocket 消息流 (sharedStreams, activeConnectionId)
└── zoom.ts           # 页面缩放 (zoom, Tauri native WebView API, localStorage 持久化)
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
| 压力测试状态 | `stressTest.ts` |
| WebSocket 消息流 | `wsStream.ts` |
| 页面缩放 | `zoom.ts` |

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

## Stores 列表 (10 个)

| Store | 用途 | 关键状态 |
|-------|------|----------|
| `flow` | 流程编辑 | flow, nodes, edges, breakpoints, selectedNodeId, isDirty, isLocked |
| `execution` | 执行状态 | isExecuting, report, error, history (最近10条) |
| `flowList` | 流程列表 | flowMetas, isLoading, searchKeyword, sortBy/sortOrder |
| `ui` | UI 状态 | 面板开关、对话框、折叠状态、布局尺寸、canvasZoom |
| `settings` | 全局设置 | reportsDir, logsDir (tauri-plugin-store 持久化) |
| `debug` | 调试状态 | sessionId, status, currentNodeUuid, variables, executionStack, wsStreams, 健康检查 |
| `console` | 控制台 | logs, 过滤条件 (level, nodeUuid, searchKeyword) |
| `stressTest` | 压力测试 | status, config (concurrentUsers, rampUpSeconds, durationSeconds), progress, report |
| `wsStream` | WebSocket 消息流 | wsStreams, activeConnectionId, WsDebugEvent 处理 |
| `zoom` | 页面缩放 | zoom (0.5~2.0), ZOOM_MIN/MAX/STEP/DEFAULT 常量, Tauri WebView API + localStorage 持久化 |

## 注意

- `console` store 未在 `index.ts` 中 re-export，需直接 `import from './console'`
