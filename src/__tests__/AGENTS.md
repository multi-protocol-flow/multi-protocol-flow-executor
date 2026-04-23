# 测试知识库

**生成时间:** 2026-04-23

## 概述

前端单元测试，Vitest 3.x + Testing Library 16.x。全局 jsdom 环境（vitest.config.ts 配置，无需逐文件标记）。共 ~74 个测试文件。

## 结构

```
src/__tests__/
├── components/   # React 组件测试 (~37 个文件)
│   ├── canvas/   # 画布相关组件 (FlowCanvas, CanvasToolbar, DebugToolbar, BreakpointMarker)
│   ├── panels/   # 配置面板 (AssertionConfig, ConditionalConfig, GrpcCallConfig)
│   ├── execution/ # 执行相关 (HttpNodeViewer, ConditionalViewer, GenericViewer, WsConnectViewer, WsSendCollectViewer, WsCloseViewer, GrpcViewer, RawJsonViewer, NodeExecutionDetails)
│   └── ui.test.tsx 等
├── stores/       # Zustand Store 测试 (9 个文件: flow, execution, flowList, ui-panels, ui-settings, settings, debug, stressTest, console)
├── services/     # 服务层测试 (7 个文件)
│   ├── tauri/    # IPC 子模块测试 (flow, debug, settings)
│   └── storage/events/reportStorage
├── types/        # 类型测试 (12 个文件: node, flow, report, variable, connection, port, error, ipc, debug, settings, events, websocket)
├── hooks/        # Hooks 测试 (2 个文件: useKeyboardShortcuts, useFullscreen)
├── e2e/          # 端到端测试 (debug.e2e.test.ts, settings.test.ts)
├── layouts/      # 布局测试 (1 个文件: MainLayout)
├── lib/          # 工具函数测试 (flowDiff, restoreFlow)
├── templates.test.ts
├── setup.test.tsx
└── setup.ts      # Polyfills (ResizeObserver, matchMedia, getComputedStyle)
```

## 快速定位

| 目标 | 位置 |
|------|------|
| 组件 | `components/*.test.tsx` |
| Store | `stores/*.test.ts` |
| IPC | `services/tauri/*.test.ts` |
| 变量 | `types/variable.test.ts` |
| E2E | `e2e/*.e2e.test.ts` |
| gRPC 类型 | `types/node.test.ts` |
| gRPC 配置面板 | `components/panels/GrpcCallConfig.test.tsx` |
| gRPC 查看器 | `components/execution/GrpcViewer.test.tsx` |

## 约定

### Mock 三层体系

| 层级 | 用途 | 方式 |
|------|------|------|
| Full mock service | 组件/Store 测试 | `import from '@/services/tauri/mock'` |
| 模块级 vi.mock | 隔离服务层 | `vi.mock('@/services/tauri', () => ({...}))` |
| Tauri API mock | 测试真实 IPC 包装 | `vi.mock('@tauri-apps/api/core', () => ({invoke: vi.fn()}))` |

### Store 测试
```ts
beforeEach(() => {
  useXxxStore.getState().reset();
  vi.clearAllMocks();
});
```

### React Flow Mock
```ts
vi.mock('@xyflow/react', async () => ({
  ...await vi.importActual('@xyflow/react'),
  useReactFlow: () => ({ screenToFlowPosition: vi.fn(), fitView: vi.fn() })
}));
```

### 其他 Mock
- sonner toast: `vi.mock('sonner', () => ({ toast: { error: vi.fn(), success: vi.fn() } }))`
- TestWrapper: `<ReactFlowProvider>{children}</ReactFlowProvider>`

## 命令

```bash
npm run test       # 运行测试
npm run typecheck  # 类型检查
```

## 注意

1. jsdom 环境已全局配置，组件测试无需单独标记 `/** @vitest-environment jsdom */`
2. Store 测试 `beforeEach` 调用 `reset()` + `vi.clearAllMocks()`
3. 用 `userEvent` 不用 `fireEvent`
4. E2E 测试文件命名 `*.e2e.test.ts`
5. 覆盖率阈值: 80% statements, 75% branches, 80% functions, 80% lines
6. gRPC 类型测试包含在 `types/node.test.ts` 中
