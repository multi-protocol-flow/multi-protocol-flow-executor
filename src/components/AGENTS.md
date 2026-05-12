# 前端组件

**生成时间:** 2026-04-23

## OVERVIEW

React 19 UI 组件库 (~111 文件)。画布编辑器 + 配置面板 + 执行报告 + 调试模式 + 压力测试。基于 React Flow + shadcn/ui + react-hook-form + Zod。

## STRUCTURE

```
components/
├── canvas/               # 流程画布
│   ├── FlowCanvas.tsx    # 主画布，拖拽 + 连线 + 节点注册
│   ├── CanvasToolbar.tsx # 工具栏
│   ├── DebugToolbar.tsx  # 调试工具栏 (继续/单步/跳过/结束)
│   ├── BreakpointMarker.tsx # 断点标记
│   ├── nodes/            # 36 种节点: Entry, Http, Conditional, Script, Assertion, Ws*, Db*, Grpc*, Mcp*, Tcp*, Udp*, Mqtt*, Amqp*, Graphql*, Imap*, Kafka*, Smtp*, Sse* + DynamicPluginNode
│   ├── edges/CustomEdge.tsx
│   └── utils.ts          # Flow ↔ ReactFlow 转换
├── panels/               # 节点配置面板 (~22 文件)
│   ├── NodeConfigPanel.tsx   # 按 node.type 分发
│   ├── HttpConfig.tsx / ConditionalConfig.tsx / EntryConfig.tsx / ScriptConfig.tsx
│   ├── AssertionConfig.tsx   # 断言配置
│   ├── WsConnectConfig.tsx / WsSendWaitConfig.tsx / WsCloseConfig.tsx
│   ├── DbConnectConfig.tsx / DbQueryConfig.tsx / DbBeginConfig.tsx
│   ├── DbCommitConfig.tsx / DbRollbackConfig.tsx / DbCloseConfig.tsx
│   ├── GrpcConnectConfig.tsx / GrpcCallConfig.tsx / GrpcCloseConfig.tsx
│   ├── McpConnectConfig.tsx / McpCallToolConfig.tsx / McpReadResourceConfig.tsx / McpGetPromptConfig.tsx / McpCloseConfig.tsx
│   ├── TcpConnectConfig.tsx / TcpSendConfig.tsx / TcpCloseConfig.tsx
│   ├── UdpBindConfig.tsx / UdpSendToConfig.tsx / UdpRecvFromConfig.tsx / UdpCloseConfig.tsx
│   ├── MqttConnectConfig.tsx / MqttPublishConfig.tsx / MqttSubscribeConfig.tsx / MqttUnsubscribeConfig.tsx / MqttDisconnectConfig.tsx / MqttWaitMessageConfig.tsx / MqttRequestConfig.tsx
│   ├── AmqpConnectConfig.tsx / AmqpPublishConfig.tsx / AmqpConsumeConfig.tsx
│   ├── GraphqlIntrospectConfig.tsx / GraphqlQueryConfig.tsx / GraphqlSubscribeConfig.tsx / GraphqlUnsubscribeConfig.tsx
│   ├── ImapConnectConfig.tsx / ImapFetchConfig.tsx / ImapSearchConfig.tsx / ImapFolderConfig.tsx / ImapIdleConfig.tsx
│   ├── KafkaProduceConfig.tsx / KafkaConsumeConfig.tsx / KafkaBatchConsumeConfig.tsx / KafkaAdminConfig.tsx
│   ├── SmtpConnectConfig.tsx / SmtpSendConfig.tsx / SmtpCloseConfig.tsx
│   ├── SseConnectConfig.tsx / SseListenConfig.tsx / SseCloseConfig.tsx
│   ├── SchemaConfigPanel.tsx # Schema 配置面板
│   ├── FlowNodeList.tsx      # 流程节点列表
│   ├── VariablePanel.tsx     # 变量面板
│   ├── DebugPanel.tsx        # 调试面板
│   └── schemas.ts            # Zod 验证 schema
├── execution/            # 执行相关 (~12 文件)
│   ├── ExecuteButton.tsx, ExecutionReport.tsx
│   ├── ExecutionHistory.tsx, NodeExecutionDetails.tsx
│   ├── ConsolePanel.tsx      # 控制台输出
│   ├── WsStreamsPanel.tsx    # WebSocket 消息流
│   ├── ExecutionPanel.tsx    # 执行面板
│   ├── ExecutionStackView.tsx # 执行栈视图
│   ├── FlowCompareDialog.tsx # 流程对比对话框
│   ├── VariableTreeView.tsx  # 变量树视图
│   ├── WsMessageTimeline.tsx # WebSocket 消息时间线
│   ├── StressTestHistory.tsx # 压测历史
│   ├── NodeListItem.tsx      # 节点列表项
│   └── viewers/              # 节点执行结果查看器 (19 文件: HttpNodeViewer, ConditionalViewer, GenericViewer, WsConnectViewer, WsSendCollectViewer, WsCloseViewer, GrpcViewer, McpViewer, TcpUdpViewer, MqttViewer, AmqpViewer, GraphqlViewer, ImapViewer, KafkaViewer, SmtpViewer, SseViewer, RawJsonViewer, VariableChangesCard, +1)
├── sidebar/              # FlowListPanel, NodeLibrary, ConnectionsPanel
├── dialogs/              # NewFlowDialog, ConfirmDialog, DebugNodeConfigDialog, SkipNodeDialog, ConnectionDialog, ScriptHelpDialog
├── panels/SettingsPanel.tsx  # 全局设置 (报告/日志目录)
├── stress-test/          # 压力测试 UI (StressTestConfigDialog, StressTestMonitor, StressTestResult, StressTestCharts)
└── ui/                   # shadcn/ui + 自定义 (VariableInput, VariableSelector, SqlEditor, DbResultPreview 等)
```

## WHERE TO LOOK

| 任务 | 文件 |
|------|------|
| 添加新节点类型 | `canvas/nodes/XxxNode.tsx` + `panels/XxxConfig.tsx` + `schemas.ts` + `execution/viewers/XxxViewer.tsx` + `src/types/nodeRegistry.ts` |
| 修改节点样式 | `canvas/nodes/*.tsx` |
| 画布交互/节点注册 | `canvas/FlowCanvas.tsx` → `nodeTypes` |
| 节点配置表单 | `panels/*Config.tsx` |
| gRPC 节点配置 | `panels/GrpcConnectConfig.tsx`, `panels/GrpcCallConfig.tsx`, `panels/GrpcCloseConfig.tsx` |
| gRPC 执行结果 | `execution/viewers/GrpcViewer.tsx` |
| MCP 节点配置 | `panels/McpConnectConfig.tsx`, `panels/McpCallToolConfig.tsx`, `panels/McpReadResourceConfig.tsx`, `panels/McpGetPromptConfig.tsx`, `panels/McpCloseConfig.tsx` |
| MCP 执行结果 | `execution/viewers/McpViewer.tsx` |
| TCP 节点配置 | `panels/TcpConnectConfig.tsx`, `panels/TcpSendConfig.tsx`, `panels/TcpCloseConfig.tsx` |
| UDP 节点配置 | `panels/UdpBindConfig.tsx`, `panels/UdpSendToConfig.tsx`, `panels/UdpRecvFromConfig.tsx`, `panels/UdpCloseConfig.tsx` |
| TCP/UDP 执行结果 | `execution/viewers/TcpUdpViewer.tsx` |
| MQTT 节点配置 | `panels/MqttConnectConfig.tsx`, `panels/MqttPublishConfig.tsx`, `panels/MqttSubscribeConfig.tsx`, `panels/MqttUnsubscribeConfig.tsx`, `panels/MqttDisconnectConfig.tsx`, `panels/MqttWaitMessageConfig.tsx`, `panels/MqttRequestConfig.tsx` |
| MQTT 执行结果 | `execution/viewers/MqttViewer.tsx` |
| AMQP 节点配置 | `panels/AmqpConnectConfig.tsx`, `panels/AmqpPublishConfig.tsx`, `panels/AmqpConsumeConfig.tsx` |
| AMQP 执行结果 | `execution/viewers/AmqpViewer.tsx` |
| GraphQL 节点配置 | `panels/GraphqlIntrospectConfig.tsx`, `panels/GraphqlQueryConfig.tsx`, `panels/GraphqlSubscribeConfig.tsx`, `panels/GraphqlUnsubscribeConfig.tsx` |
| GraphQL 执行结果 | `execution/viewers/GraphqlViewer.tsx` |
| IMAP 节点配置 | `panels/ImapConnectConfig.tsx`, `panels/ImapFetchConfig.tsx`, `panels/ImapSearchConfig.tsx`, `panels/ImapFolderConfig.tsx`, `panels/ImapIdleConfig.tsx` |
| IMAP 执行结果 | `execution/viewers/ImapViewer.tsx` |
| Kafka 节点配置 | `panels/KafkaProduceConfig.tsx`, `panels/KafkaConsumeConfig.tsx`, `panels/KafkaBatchConsumeConfig.tsx`, `panels/KafkaAdminConfig.tsx` |
| Kafka 执行结果 | `execution/viewers/KafkaViewer.tsx` |
| SMTP 节点配置 | `panels/SmtpConnectConfig.tsx`, `panels/SmtpSendConfig.tsx`, `panels/SmtpCloseConfig.tsx` |
| SMTP 执行结果 | `execution/viewers/SmtpViewer.tsx` |
| SSE 节点配置 | `panels/SseConnectConfig.tsx`, `panels/SseListenConfig.tsx`, `panels/SseCloseConfig.tsx` |
| SSE 执行结果 | `execution/viewers/SseViewer.tsx` |
| 变量输入组件 | `ui/VariableInput.tsx`, `ui/VariableSelector.tsx` |
| SQL 编辑器 | `ui/SqlEditor.tsx` |
| 数据库结果预览 | `ui/DbResultPreview.tsx` |
| 执行报告 | `execution/ExecutionReport.tsx` |
| 调试工具栏 | `canvas/DebugToolbar.tsx` |
| 压力测试 | `stress-test/*.tsx` |
| 流程列表 | `sidebar/FlowListPanel.tsx` |
| 数据库连接管理 | `sidebar/ConnectionsPanel.tsx`, `dialogs/ConnectionDialog.tsx` |
| 全局设置 | `panels/SettingsPanel.tsx` |
| 脚本帮助 | `dialogs/ScriptHelpDialog.tsx` |
| 流程对比 | `execution/FlowCompareDialog.tsx` |

## CONVENTIONS

- **Store**: `useFlowStore()` 管理流程/节点/边，`useExecutionStore()` 管理执行状态，`useDebugStore()` 管理调试状态，`useStressTestStore()` 管理压测状态，`useConnectionsStore()` 管理数据库连接，`useWsStreamStore()` 管理 WebSocket 消息流
- **配置面板**: `react-hook-form` + Zod schema，变更通过 `updateNode()` 同步
- **节点注册**: `FlowCanvas.tsx` 中 `nodeTypes` 对象
- **断点**: 通过 `useFlowStore().toggleBreakpoint()` 管理，`BreakpointMarker` 组件渲染

## 节点类型 (29 种)

| 类型 | 组件 | 配置面板 | 颜色 |
|------|------|----------|------|
| Entry | EntryNode.tsx | EntryConfig.tsx | 绿色 |
| Http | HttpNode.tsx | HttpConfig.tsx | 蓝色 |
| Conditional | ConditionalNode.tsx | ConditionalConfig.tsx | 橙色 |
| Script | ScriptNode.tsx | ScriptConfig.tsx | 紫色 |
| Assertion | AssertionNode.tsx | AssertionConfig.tsx | 红色 |
| WsConnect | WsConnectNode.tsx | WsConnectConfig.tsx | 琥珀色 |
| WsSendCollect | WsSendCollectNode.tsx | WsSendCollectConfig.tsx | 青色 |
| WsClose | WsCloseNode.tsx | WsCloseConfig.tsx | 玫红色 |
| DbConnect | DbConnectNode.tsx | DbConnectConfig.tsx | 靛蓝色 |
| DbQuery | DbQueryNode.tsx | DbQueryConfig.tsx | 靛蓝色 |
| DbBegin | DbBeginNode.tsx | DbBeginConfig.tsx | 靛蓝色 |
| DbCommit | DbCommitNode.tsx | DbCommitConfig.tsx | 靛蓝色 |
| DbRollback | DbRollbackNode.tsx | DbRollbackConfig.tsx | 靛蓝色 |
| DbClose | DbCloseNode.tsx | DbCloseConfig.tsx | 靛蓝色 |
| GrpcConnect | GrpcConnectNode.tsx | GrpcConnectConfig.tsx | 紫色 |
| GrpcCall | GrpcCallNode.tsx | GrpcCallConfig.tsx | 紫色 |
| GrpcClose | GrpcCloseNode.tsx | GrpcCloseConfig.tsx | 紫色 |
| McpConnect | McpConnectNode.tsx | McpConnectConfig.tsx | 青绿色 |
| McpCallTool | McpCallToolNode.tsx | McpCallToolConfig.tsx | 青绿色 |
| McpReadResource | McpReadResourceNode.tsx | McpReadResourceConfig.tsx | 青绿色 |
| McpGetPrompt | McpGetPromptNode.tsx | McpGetPromptConfig.tsx | 青绿色 |
| McpClose | McpCloseNode.tsx | McpCloseConfig.tsx | 青绿色 |
| TcpConnect | TcpConnectNode.tsx | TcpConnectConfig.tsx | 棕色 |
| TcpSend | TcpSendNode.tsx | TcpSendConfig.tsx | 棕色 |
| TcpClose | TcpCloseNode.tsx | TcpCloseConfig.tsx | 棕色 |
| UdpBind | UdpBindNode.tsx | UdpBindConfig.tsx | 橄榄色 |
| UdpSendTo | UdpSendToNode.tsx | UdpSendToConfig.tsx | 橄榄色 |
| UdpRecvFrom | UdpRecvFromNode.tsx | UdpRecvFromConfig.tsx | 橄榄色 |
| UdpClose | UdpCloseNode.tsx | UdpCloseConfig.tsx | 橄榄色 |
| MqttConnect | MqttConnectNode.tsx | MqttConnectConfig.tsx | 蓝绿色 |
| MqttPublish | MqttPublishNode.tsx | MqttPublishConfig.tsx | 蓝绿色 |
| MqttSubscribe | MqttSubscribeNode.tsx | MqttSubscribeConfig.tsx | 蓝绿色 |
| MqttUnsubscribe | MqttUnsubscribeNode.tsx | MqttUnsubscribeConfig.tsx | 蓝绿色 |
| MqttDisconnect | MqttDisconnectNode.tsx | MqttDisconnectConfig.tsx | 蓝绿色 |
| MqttWaitMessage | MqttWaitMessageNode.tsx | MqttWaitMessageConfig.tsx | 蓝绿色 |
| MqttRequest | MqttRequestNode.tsx | MqttRequestConfig.tsx | 蓝绿色 |
| AmqpConnect | AmqpConnectNode.tsx | AmqpConnectConfig.tsx | 金黄色 |
| AmqpPublish | AmqpPublishNode.tsx | AmqpPublishConfig.tsx | 金黄色 |
| AmqpConsume | AmqpConsumeNode.tsx | AmqpConsumeConfig.tsx | 金黄色 |
| GraphqlIntrospect | GraphqlIntrospectNode.tsx | GraphqlIntrospectConfig.tsx | 粉红色 |
| GraphqlQuery | GraphqlQueryNode.tsx | GraphqlQueryConfig.tsx | 粉红色 |
| GraphqlSubscribe | GraphqlSubscribeNode.tsx | GraphqlSubscribeConfig.tsx | 粉红色 |
| GraphqlUnsubscribe | GraphqlUnsubscribeNode.tsx | GraphqlUnsubscribeConfig.tsx | 粉红色 |
| ImapConnect | ImapConnectNode.tsx | ImapConnectConfig.tsx | 深蓝色 |
| ImapFetch | ImapFetchNode.tsx | ImapFetchConfig.tsx | 深蓝色 |
| ImapSearch | ImapSearchNode.tsx | ImapSearchConfig.tsx | 深蓝色 |
| ImapFolder | ImapFolderNode.tsx | ImapFolderConfig.tsx | 深蓝色 |
| ImapIdle | ImapIdleNode.tsx | ImapIdleConfig.tsx | 深蓝色 |
| KafkaProduce | KafkaProduceNode.tsx | KafkaProduceConfig.tsx | 深绿色 |
| KafkaConsume | KafkaConsumeNode.tsx | KafkaConsumeConfig.tsx | 深绿色 |
| KafkaBatchConsume | KafkaBatchConsumeNode.tsx | KafkaBatchConsumeConfig.tsx | 深绿色 |
| KafkaAdmin | KafkaAdminNode.tsx | KafkaAdminConfig.tsx | 深绿色 |
| SmtpConnect | SmtpConnectNode.tsx | SmtpConnectConfig.tsx | 天蓝色 |
| SmtpSend | SmtpSendNode.tsx | SmtpSendConfig.tsx | 天蓝色 |
| SmtpClose | SmtpCloseNode.tsx | SmtpCloseConfig.tsx | 天蓝色 |
| SseConnect | SseConnectNode.tsx | SseConnectConfig.tsx | 浅紫色 |
| SseListen | SseListenNode.tsx | SseListenConfig.tsx | 浅紫色 |
| SseClose | SseCloseNode.tsx | SseCloseConfig.tsx | 浅紫色 |

## 注意事项

1. **新增节点需同步 6 处**: `canvas/nodes/`, `panels/`, `schemas.ts`, `FlowCanvas.tsx` 的 `nodeTypes`, `execution/viewers/`, `src/types/nodeRegistry.ts`
2. **Flow 转换**: `canvas/utils.ts` 处理 `Flow` ↔ `ReactFlow` 互转
3. **类型对齐**: 节点 `data` 需与 `src/types/node.ts` 一致
4. **变量引用**: 使用 `{{path.to.value}}` 语法，`VariableInput` 组件提供自动补全
5. **调试状态**: `useDebugStore()` 管理断点、暂停、当前节点等状态
6. **压力测试**: `useStressTestStore()` 管理压测配置和状态
7. **gRPC 服务层**: gRPC IPC 调用已通过 `src/services/tauri/grpc.ts` 抽象，MCP 通过 `src/services/tauri/mcp.ts` 抽象
