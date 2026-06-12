# src/stores/

## OVERVIEW

Zustand 5 state management layer. 10 stores covering the full application state: flow (active flow graph), execution (runtime lifecycle), flowList (saved flows), ui (panel visibility, modals), settings (user preferences), debug (breakpoints, inspect), console (log output), stressTest (config + state), wsStream (WebSocket message buffer), zoom (canvas transform).

Each store is a single file with `create()` from Zustand and a `reset()` method returning state to initial values. Selectors are exported as memoized getter functions for render performance. Test isolation is achieved by calling `getState().reset()` in test setup.

## WHERE TO LOOK

| Task | Store |
|------|-------|
| Active flow graph (nodes, edges, viewport) | `flowStore.ts` |
| Flow execution state (running, paused, node statuses) | `executionStore.ts` |
| Saved flow files list, search, sort | `flowListStore.ts` |
| Sidebar visibility, modal dialogs, toast | `uiStore.ts` |
| App settings (theme, language, editor prefs) | `settingsStore.ts` |
| Debug session (breakpoints, variable inspector) | `debugStore.ts` |
| Execution log stream (timestamped entries) | `consoleStore.ts` |
| Stress test config, progress, results | `stressTestStore.ts` |
| WebSocket message stream buffer | `wsStreamStore.ts` |
| Canvas zoom level and pan offset | `zoomStore.ts` |

## CONVENTIONS

- All stores use `create<State>()` from Zustand 5 with an `initialState` constant and a `reset()` method that sets all keys back to initial values.
- Selectors are exported as standalone functions (e.g. `selectNodes(s)`) for tree-shaking and to avoid inline selector re-creation.
- Tests import the store directly and call `useXxxStore.getState().reset()` in `beforeEach` to guarantee isolation.

## NOTES

The console store (`consoleStore.ts`) is NOT re-exported from `stores/index.ts` to keep the bundle lean for production. Import it directly: `import { useConsoleStore } from '@/stores/consoleStore'`.
