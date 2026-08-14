# 测试基础设施

**生成时间:** 2026-08-14 | **Branch:** feature/plugins-sdk

## OVERVIEW

Frontend unit test suite. Vitest 3.x + Testing Library 16.x, jsdom. **121 test files**. Coverage thresholds: 80% statements, 75% branches, 80% functions, 80% lines（注意：当前全局覆盖率约 lines 57% / branches 74%，阈值未达标，见下方「已知缺口」；新增测试已把先前 0% 覆盖的面板/store/hook 提升至 63–100%）。

## STRUCTURE

```
__tests__/
├── components/       43 files（root 28 + panels/24 + execution/9 + plugin/1 + panels/shared/1 + panels/grpc/1）
│   ├── execution/    viewers (HttpNodeViewer, GrpcViewer, etc.)
│   ├── panels/       config panels（5 个原有 + 19 个 TCP/UDP/SSE/GraphQL/Script/Proxy/VariableExtractor）
│   └── panels/shared/ 共享组件（OnErrorSection）
├── stores/           15 files — one per Zustand store（含从 e2e/ 并入的 settings-journey / debug-journey）
├── services/         9 files — Tauri IPC mock + command tests（5 + services/tauri/ 4）
├── types/            14 files — type validation
├── hooks/            7 files — useKeyboardShortcuts 等 + useFlowVariables/usePanelDrag/useThrottledValue/useTheme/useLocale
├── lib/              5 files — flowDiff, restoreFlow, viewerRegistry, nodeIcons 等
├── browser-compat/   3 files — 浏览器兼容 shim 测试（storage-flowStorage.mock / shim-api-core / platform）
├── layouts/          1 file — MainLayout
├── i18n/             1 file — viewer-skeleton
├── setup.ts          global setup (i18n mock, ResizeObserver, matchMedia, getComputedStyle)
├── setup.test.tsx    App rendering sanity test
└── templates.test.ts DEMO_FLOW TDD contract
```

> e2e/ 目录已废弃并并入 stores/：settings.test.ts → `stores/settings-journey.test.ts`、debug.e2e.test.ts → `stores/debug-journey.test.ts`（均为 store 级集成测试，非真实浏览器 e2e）。

## WHERE TO LOOK

| Task | File |
|------|------|
| Canvas interaction | `components/FlowCanvas.test.tsx`, `FlowCanvas.drop.test.tsx` |
| Config panels | `components/panels/` |
| Execution viewers | `components/execution/` |
| Store logic | `stores/<name>.test.ts` |
| Tauri IPC mock | `services/tauri.test.ts` + `services/tauri/` |
| Type correctness | `types/` — one file per domain |
| Settings journey | `stores/settings-journey.test.ts` |
| New test | match subdirectory under `components/`, `stores/`, etc. |

## CONVENTIONS

- **3-layer mocks:** (1) Global jsdom polyfills in `setup.ts` (ResizeObserver, matchMedia, getComputedStyle；Radix Select 需要的 pointer-capture / scrollIntoView 由相关面板测试文件内 polyfill)。(2) `vi.mock('@/i18n/i18n-util')` returns zh-CN translation proxy。(3) Tauri API mock at `@/services/tauri/mock.ts` covers IPC commands。
- **Factory functions** in `src/test-utils/` (nodeFactory, connectionFactory, flowFactory)。
- **Store isolation**: `useXxxStore.getState().reset()` in `beforeEach`（zoom store 无 reset()，用 `setState({zoom: ZOOM_DEFAULT}) + localStorage.clear()`）。
- **Events**: prefer `userEvent` over `fireEvent`。
- **面板保存断言**: 本仓库面板保存触发有 4 种（unmount cleanup / blur 100ms 防抖 / onValueChange 即时 / flush-config-panel 事件），测试按面板实际实现选择最稳路径，优先 unmount 保存。
- **Coverage exclusion**: `exclude: ['src/__tests__/**', 'src/test-utils/**']` — 测试文件不计入覆盖率。

## 已知缺口（记录在案，勿误以为已覆盖）

- **Canvas 渲染器**（FlowCanvas / NodeContextMenu / NodePalette / CanvasToolbar 等）：覆盖率低（NodeContextMenu 约 4%），补测成本高，暂不补。
- **stress / reports 组件**：无专门测试文件。
- 全局覆盖率阈值（80/75/80/80）在当前代码规模下未达标 —— 结构性缺口即上述两项 + browser-shims 大量 0% 文件。新增面板/store/hook 测试已把对应模块从 0% 提升至 63–100%，但不足以拉平全局数字。

## NOTES

- jsdom globals pre-configured in `setup.ts`。
- i18n mock returns real zh-CN strings with `{param}` substitution。
- Mock service covers 49 IPC commands (flow, debug, settings, dataset)。
- 新增 store 或 hook？补 `stores/<name>.test.ts` / `hooks/<name>.test.ts`，store 必须测 `reset()`（或等价隔离）。
- 面板新增 node type 时同步补 `components/panels/<XxxConfig>.test.tsx`，参照 `GrpcConnectConfig.test.tsx` 模板。
