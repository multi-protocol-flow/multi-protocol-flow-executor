# execution/ — Execution Result Display

**生成时间:** 2026-06-12 | **文件数:** 18 组件 + 20 viewers

## OVERVIEW

Execution result display components. Shows flow execution progress, per-node results, console logs, variable state, WS message timelines, dataset execution, stress test history, and flow comparison. Viewers/ subdirectory has 20 dedicated result viewer components for every protocol.

## STRUCTURE

```
execution/
├── ExecuteButton.tsx         触发执行，管理按钮状态
├── ExecutionReport.tsx       完整报告弹窗（统计 + 节点列表）
├── ExecutionPanel.tsx        执行面板容器（报告 + 控制台）
├── ConsolePanel.tsx          实时日志面板
├── NodeExecutionDetails.tsx  单节点详情（structured/raw Tab）
├── CanvasReportViewer.tsx    画布内嵌报告
├── FlowCompareDialog.tsx     两次执行结果对比
├── VariableTreeView.tsx      变量树查看器
├── WsMessageTimeline.tsx     WS 消息时间线
├── DatasetExecutionPanel.tsx 数据集批量执行
├── StressTestHistory.tsx     压测历史
└── viewers/ (20 文件)
```

| Viewer | Node types | |
|--------|------------|---|
| `HttpNodeViewer` | http | `WsConnectViewer`/`WsSendCollectViewer`/`WsCloseViewer` | ws_* |
| `McpViewer` | mcp_* | `TcpUdpViewer` | tcp_*/udp_* |
| `SseViewer` | sse_* | `GraphqlViewer` | graphql_* |
| `AssertionViewer` | assertion | `ConditionalViewer` | conditional |
| `RecordingViewer` | recording | `GenericViewer` | (兜底) |
| `RawJsonViewer` / `VariableChangesCard` | 子组件，被所有 viewer 复用 |

## WHERE TO LOOK

| Task | File |
|------|------|
| 查看/修改协议执行结果展示 | `viewers/<Protocol>Viewer.tsx` |
| 注册新 viewer | `@/lib/viewerRegistry.tsx` |
| 修改节点展开详情结构 | `NodeExecutionDetails.tsx` |
| 修改实时日志 | `ConsolePanel.tsx` |
| 修改执行报告弹窗 | `ExecutionReport.tsx` |
| 修改变量树 | `VariableTreeView.tsx` |
| 修改 WS 消息时间线 | `WsMessageTimeline.tsx` |

## CONVENTIONS

- **Registration**: All viewers register in `@/lib/viewerRegistry.tsx` mapping `node_type` → `ComponentType<{ report: NodeExecutionReport }>`. Unregistered types fall back to `GenericViewer`.
- **Collapsible section pattern**: Viewers use bordered cards with ChevronDown/ChevronRight toggles for request/response/timing/variable sections.
- **Data flow**: `ExecutionReport.node_reports` (keyed by node UUID) → `NodeExecutionDetails` → `getViewerComponent(nodeType)` → viewer. Viewers never call IPC.
- **Sub-viewers**: `RawJsonViewer` (monospace JSON) and `VariableChangesCard` (diff-style variable list) shared across all viewers.
- **i18n**: All viewer text uses `getLL()`.
- **Fallback**: SMTP, Redis, proxy, variable_extractor use `GenericViewer`.

## ANTI-PATTERNS

- Adding viewer without registering in `viewerRegistry.tsx` → component exists but `getViewerComponent()` returns `GenericViewer`.
- Direct IPC calls from viewers → data is already in `NodeExecutionReport`.
- Viewer toggle state in execution store → collapsible state is local to `NodeExecutionDetails`.
