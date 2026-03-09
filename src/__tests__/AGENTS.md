# 测试知识库

**生成时间:** 2026-03-09

## 概述

前端单元测试，Vitest 3.x + Testing Library 16.x。

## 结构

```
src/__tests__/
├── components/   # React 组件测试 (55 个文件)
│   ├── canvas/   # 画布相关组件
│   ├── panels/   # 配置面板
│   ├── execution/ # 执行相关
│   └── ...
├── stores/       # Zustand Store 测试
├── services/     # 服务层测试
├── types/        # 类型测试
├── hooks/        # Hooks 测试
├── e2e/          # 端到端测试
├── layouts/      # 布局组件测试
└── setup.ts      # Polyfills (ResizeObserver, matchMedia)
```

## 快速定位

| 目标 | 位置 |
|------|------|
| 组件 | `components/*.test.tsx` |
| Store | `stores/*.test.ts` |
| IPC | `services/tauri.test.ts` |
| 变量 | `types/variable.test.ts` |
| E2E | `e2e/*.e2e.test.ts` |

## 约定

### 组件测试
```tsx
/** @vitest-environment jsdom */
vi.mock('@xyflow/react', async (importActual) => ({
  ...await importActual(), useReactFlow: () => ({ fitView: vi.fn() })
}));
```

### Store 测试
```ts
beforeEach(() => useFlowStore.getState().reset());
```

### E2E 测试
```tsx
/** @vitest-environment jsdom */
// 测试完整用户流程
```

### Mock 规则
- React Flow: Mock `useReactFlow`/`useNodes`/`useEdges`
- Tauri IPC: 使用 `services/tauri/mock.ts`
- 浏览器 API: 在 `setup.ts` 添加

## 命令

```bash
npm run test       # 运行测试
npm run typecheck  # 类型检查
```

## 注意

1. 组件测试标记 `@vitest-environment jsdom`
2. Store 测试 `beforeEach` 调用 `reset()`
3. 用 `userEvent` 不用 `fireEvent`
4. 末尾 `vi.restoreAllMocks()`
5. E2E 测试文件命名 `*.e2e.test.ts`
