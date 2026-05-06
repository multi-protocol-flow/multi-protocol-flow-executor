# flow-engine-graphql

GraphQL 协议支持 crate (内省/查询/变更/订阅/WebSocket)。

## 结构

```
src/
├── lib.rs                           # Crate 入口，模块导出
├── plugin.rs                        # 6 个 NodePlugin 实现 + GraphqlPluginGroup 批量注册
├── graphql_client.rs                # reqwest HTTP 客户端 + WebSocket 订阅
├── graphql_connection_manager.rs    # 连接池管理 + GraphqlProvider trait 实现
├── graphql_connect_executor.rs      # graphql_connect 节点执行器
├── graphql_disconnect_executor.rs   # graphql_disconnect 节点执行器
├── graphql_introspect_executor.rs   # graphql_introspect 节点执行器
├── graphql_query_executor.rs        # graphql_query 节点执行器
├── graphql_subscribe_executor.rs    # graphql_subscribe 节点执行器
└── graphql_unsubscribe_executor.rs  # graphql_unsubscribe 节点执行器
```

## 定位

| 节点类型 | 功能 | 执行器文件 |
|----------|------|-----------|
| `graphql_connect` | 建立 HTTP 连接，验证端点可达性 | `graphql_connect_executor.rs` |
| `graphql_query` | 执行 Query/Mutation，支持文件上传 | `graphql_query_executor.rs` |
| `graphql_subscribe` | WebSocket 订阅，流式接收事件 | `graphql_subscribe_executor.rs` |
| `graphql_unsubscribe` | 取消指定订阅 | `graphql_unsubscribe_executor.rs` |
| `graphql_introspect` | Schema 内省，生成查询模板 | `graphql_introspect_executor.rs` |
| `graphql_disconnect` | 断开连接，清理资源 | `graphql_disconnect_executor.rs` |

## 特性

- Query/Mutation: HTTP POST + multipart 文件上传
- Subscription: WebSocket (graphql-transport-ws)
- Introspection: 标准 GraphQL 内省查询
- 认证: Bearer / Cookie / API Key
- 压测: `new_shared()` 共享模式避免互斥关闭
