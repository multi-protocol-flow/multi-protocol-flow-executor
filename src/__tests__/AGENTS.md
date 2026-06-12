# 测试基础设施

**生成时间:** 2026-06-12 | **Branch:** dev

## OVERVIEW

Frontend unit test suite. Vitest 3.x + Testing Library 16.x, jsdom. ~74 test files across 11 subdirectories. Coverage thresholds: 80% statements, 75% branches, 80% functions, 80% lines.

## STRUCTURE

```
__tests__/
├── components/       40 files (root 25 + execution/10 + panels/5)
│   ├── execution/    viewers (HttpNodeViewer, GrpcViewer, etc.)
│   └── panels/       config panels (Assertion, Conditional, Grpc*)
├── stores/           10 files — one per Zustand store
├── services/         8 files — Tauri IPC mock + command tests
├── types/            13 files — type validation
├── e2e/              2 files — debug.e2e.test.ts, settings.test.ts
├── hooks/            1 file — useKeyboardShortcuts
├── lib/              2 files — flowDiff, restoreFlow
├── layouts/          1 file — MainLayout
├── i18n/             1 file — viewer-skeleton
├── setup.ts          global setup (i18n mock, ResizeObserver, matchMedia)
├── setup.test.tsx    App rendering sanity test
└── templates.test.ts DEMO_FLOW TDD contract
```

## WHERE TO LOOK

| Task | File |
|------|------|
| Canvas interaction | `components/FlowCanvas.test.tsx`, `FlowCanvas.drop.test.tsx` |
| Config panels | `components/panels/` |
| Execution viewers | `components/execution/` |
| Store logic | `stores/<name>.test.ts` |
| Tauri IPC mock | `services/tauri.test.ts` + `services/tauri/` |
| Type correctness | `types/` — one file per domain |
| Settings journey | `e2e/settings.test.ts` |
| New test | match subdirectory under `components/`, `stores/`, etc. |

## CONVENTIONS

- **3-layer mocks:** (1) Global jsdom polyfills in `setup.ts` (ResizeObserver, matchMedia). (2) `vi.mock('@/i18n/i18n-util')` returns zh-CN translation proxy. (3) Tauri API mock at `@/services/tauri/mock.ts` covers IPC commands.
- **Factory functions** in `src/test-utils/` (nodeFactory, connectionFactory, flowFactory).
- **Store isolation**: `useXxxStore.getState().reset()` in `beforeEach`.
- **Events**: prefer `userEvent` over `fireEvent`.
- **E2E naming**: `*.e2e.test.ts`.
- **Coverage exclusion**: `exclude: ['src/__tests__/**']`.

## NOTES

- jsdom globals pre-configured in `setup.ts`.
- i18n mock returns real zh-CN strings with `{param}` substitution.
- Mock service covers 49 IPC commands (flow, debug, settings, dataset).
- Adding a store? Add `stores/<name>.test.ts` and test `reset()`.
