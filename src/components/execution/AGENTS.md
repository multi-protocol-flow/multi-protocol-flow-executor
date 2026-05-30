# Execution 组件

**Layer:** 前端组件 | 18 组件 + 19 查看器 (viewers/) | 父知识库: [components/AGENTS.md](../AGENTS.md)

## 概述

执行结果展示组件集。包括执行控制、报告展示、控制台、调试栈、变量树、WebSocket 消息流、数据集执行、压测历史。

## WHERE TO LOOK

| 任务 | 文件 | 说明 |
|------|------|------|
| 执行按钮 | `ExecuteButton.tsx` | 触发流程执行 |
| 执行报告 | `ExecutionReport.tsx` | 完整执行结果报告 |
| 控制台 | `ConsolePanel.tsx` | 执行日志实时输出 |
| 执行栈 | `ExecutionStackView.tsx` | 调试模式下的节点执行栈 |
| 变量树 | `VariableTreeView.tsx` | 变量值层级展示 |
| WS 消息流 | `WsStreamsPanel.tsx` / `WsMessageTimeline.tsx` | WebSocket 消息实时流 + 时间线 |
| 数据集执行 | `DatasetExecutionPanel.tsx` | 数据集驱动的批量执行 |
| 压测历史 | `StressTestHistory.tsx` | 压力测试历史记录 |
| 节点详情 | `NodeExecutionDetails.tsx` | 单节点执行结果详情 |
| 流程对比 | `FlowCompareDialog.tsx` | 执行前后流程对比 |
| 节点列表项 | `NodeListItem.tsx` | 执行历史中的节点条目 |
| 画布报告 | `CanvasReportViewer.tsx` | 在画布上叠加执行状态 |

## Viewers 查看器 (~19 文件)

各协议节点的执行结果渲染，位于 `viewers/` 目录：

| 查看器 | 对应协议 |
|--------|---------|
| `HttpNodeViewer.tsx` | HTTP |
| `GrpcViewer.tsx` | gRPC |
| `McpViewer.tsx` | MCP |
| `WsConnectViewer.tsx`, `WsSendCollectViewer.tsx`, `WsCloseViewer.tsx` | WebSocket |
| `TcpUdpViewer.tsx` | TCP/UDP |
| `MqttViewer.tsx` | MQTT |
| `AmqpViewer.tsx` | AMQP |
| `GraphqlViewer.tsx` | GraphQL |
| `ImapViewer.tsx` | IMAP |
| `KafkaViewer.tsx` | Kafka |
| `SmtpViewer.tsx` | SMTP |
| `SseViewer.tsx` | SSE |
| `AssertionViewer.tsx` | 断言 |
| `ConditionalViewer.tsx` | 条件分支 |
| `EmailViewer.tsx` | 邮件（预留） |
| `GenericViewer.tsx` | 通用兜底 |
| `RawJsonViewer.tsx` | 原始 JSON |
| `RecordingViewer.tsx` | 消息录制回放 |
| `VariableChangesCard.tsx` | 变量变更卡 |

## CONVENTIONS

- **Viewer 注册**: 通过 `viewerRegistry` 注册到 `src/lib/viewerRegistry.tsx`
- **数据来源**: 统一从 `ExecutionReport` 的 `node_reports` 取数据
- **查看器 props**: `{ data: NodeReport, isOpen: boolean, onToggle: () => void }`
- **新增协议查看器**: 在 `viewers/` 下创建 → 注册到 `viewerRegistry` → 父 AGENTS.md 中不需要重复记录
