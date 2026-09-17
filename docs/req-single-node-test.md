# 单节点测试执行 — 需求文档

## 1. 背景与目标

### 1.1 核心场景
AI agent 在编排多协议流程前，需要**快速验证某个服务是否可用/可达**：
- Redis 实例能否连通？
- 数据库服务是否正常？
- MQTT/Kafka broker 是否存活？
- HTTP API 是否可访问？
- gRPC/MCP server 能否响应？

### 1.2 设计原则
- **CLI 为主战场**（AI agent 执行环境），UI 为附加能力
- **不保存**测试报告，展示完即丢弃
- **复用现有执行引擎**，不重复造轮子
- **一次做一个事**，清晰简洁

---

## 2. CLI 设计

### 2.1 命令格式

```bash
# 执行单个节点
mpe run node <json-string> [--var KEY=VALUE]...

# 示例
mpe run node '{"type":"redis:connect","host":"127.0.0.1","port":6379}'
mpe run node '{"type":"http:request","url":"https://api.example.com/health","method":"get"}'
mpe run node '{"type":"db:connect","driver":"postgres","connection_string":"postgres://user:pass@localhost/db"}'
mpe run node '{"type":"mqtt_connect","broker":"tcp://broker.emqx.io:1883"}'
mpe run node '{"type":"kafka_connect","bootstrap_servers":"localhost:9092"}'
mpe run node '{"type":"script","code":"return 1+1;"}'

# 带流程变量
mpe run node '{"type":"redis:connect","host":"{{HOST}}","port":{{PORT}}}' \
  --var HOST=127.0.0.1 \
  --var PORT=6379
```

### 2.2 输出格式

```json
{
  "success": true,
  "data": {
    "node_type": "redis:connect",
    "status": "success",
    "duration_ms": 12,
    "output_data": { "success": true },
    "error": null
  }
}
```

失败时：

```json
{
  "success": false,
  "data": {
    "node_type": "redis:connect",
    "status": "failed",
    "duration_ms": 5003,
    "output_data": null,
    "error": {
      "message": "Connection refused: 127.0.0.1:6379",
      "type": "connection_error"
    }
  }
}
```

### 2.3 已存在的 CLI 架构可复用部分

| 现有能力 | 来源 |
|---------|------|
| JSON 字符串解析 | `flow_edit.rs` 的 `--config <JSON>` |
| `--var` 参数解析 | `run.rs` 的 `parse_vars()` |
| CLI 输出格式化 | `output.rs` 的 `CliOutput<T>` + `output_json()` |
| `FlowExecutor` 构建 | `run.rs` 的 builder 模式 |

---

## 3. 协议支持范围

### 3.1 第一版支持列表（单节点即可完成验证）

| 协议 | 节点类型 | 验证内容 |
|------|---------|---------|
| Redis | `redis:connect`（插件节点） | Redis 实例连通性（需安装 plugins/redis 插件） |
| DB | `db:connect`（插件节点） | MySQL/PostgreSQL/SQLite 可达性（需安装 plugins/db 插件） |
| HTTP | `http:request` | HTTP API 可访问性 |
| MQTT | `mqtt_connect` | Broker 连通性 |
| Kafka | `kafka_connect` | Kafka 集群可达性 |
| AMQP | `amqp_connect` / `amqp10_connect` | RabbitMQ/AMQP broker 可达性 |
| SMTP | `smtp_connect` | SMTP 服务器可达性 |
| IMAP | `imap_connect` | IMAP 服务器可达性 |
| WebSocket | `ws:connect` | WS 端点可达性 |
| SSE | `sse:connect` | SSE 端点可达性 |
| gRPC | `grpc_connect` | gRPC 端点连通性 |
| MCP | `mcp_connect` | MCP server 连通性 |
| GraphQL | `graphql:connect` | GraphQL 端点连通性 |
| TCP | `tcp:connect` | TCP 端口连通性（类似 telnet） |
| Script | `script` | 运行 JS 脚本验证逻辑 |
| Assertion | `assertion` | 数据断言验证（通常配合 --var 注入测试数据） |

> **插件节点接入**：第三方插件（如 `plugins/redis`）的 connect 节点通过
> describe 的 `capabilities.single_node: true` 声明加入单节点路径——宿主
> `execute_single_node` / `mpe run node` 的能力门（
> `NodePlugin::supports_single_node()`，内置白名单常量
> `SUPPORTED_SINGLE_NODE_TYPES` 之外的第二通道）据此放行，无需宿主改代码。
> 例：安装 plugins/redis 后 `mpe run node '{"type":"redis:connect",...}'`
> 即完成 Redis 连通性验证。

### 3.2 暂不支持的类型

| 类别 | 原因 |
|------|------|
| `*_Send / *_Query / *_Publish / *_Consume` | 需要前置 connect 节点建立 session，单节点无意义 |
| `*_Close / *_Disconnect` | 没有打开的连接，无意义 |
| `Entry / End / Conditional / VariableExtractor / Proxy` | 纯流程控制节点，无外部验证价值 |
| `UDP` | 场景较少，待有需求再加 |

### 3.3 关于多节点链式

**第一版不做多节点链式。** 原因：

- `*_Connect` 单节点本身就能验证连通性，满足核心需求
- 链式（如 `DbConnect → DbQuery`）属于**业务逻辑验证**，非"服务可用性验证"
- 如果后续需要，可扩展 `mpe run nodes <json1> <json2>...` 命令，底层构造最小 flow 走 `execute_flow`

---

## 4. 执行方案

### 4.1 技术路线

不走完整 flow 遍历，而是**直接调用 `plugin.execute()`**：

```
解析 JSON → NodeData + Node
            ↓
创建 ExecutionContext（注入 --var 变量）
初始化所有协议 Provider（with_default_providers）
            ↓
从 registry 获取 plugin: registry.get(type_id)
            ↓
调用 plugin.execute(&node, &mut context, &[], &node_map)
            ↓
返回 NodeExecutionReport → CLI 输出
```

### 4.2 变量替换

现成的 `replace_variables()` 函数，执行前将 `--var` 注入 `VariableStorage`，JSON 中的 `{{VAR}}` 自动替换。

### 4.3 连接生命周期

对于 `*_Connect` 节点：
- `execute()` 内部通过对应的 `ConnectionManager` 建立真实连接
- 执行完成后，连接由 Context 清理（`remove_provider`）
- 跟正常 flow 执行的行为完全一致

### 4.4 错误处理

遵循节点自身的 `on_error` 策略：
- 默认 `RouteToFalse` → 失败时返回错误信息
- `Ignore` → 即使失败也标记为成功
- `AbortFlow` → 立即终止

---

## 5. 输出报告

### 5.1 内容

复用现有的 `NodeExecutionReport`，包含：

| 字段 | 说明 |
|------|------|
| `node_type` | 节点类型 |
| `status` | `success` / `failed` |
| `start_time / end_time / duration_ms` | 执行时间 |
| `output_data` | 执行结果（如 HTTP 响应、Redis PONG） |
| `error` | 错误信息（类型 + 消息） |
| `resolved_config` | 变量替换后的最终配置 |

### 5.2 是否保存

**不保存。** CLI 输出完就丢，不调用 `save_report()`。UI 上如果有单节点测试面板，结果只在内存中展示。

---

## 6. UI 附加能力

CLI 为主，UI 为辅。UI 侧的接入点：

- **节点右键菜单** → "测试此节点"（将当前配置面板的 JSON 作为输入）
- **结果展示** → 复用已有的 `NodeExecutionDetails` + `viewerRegistry`（20 个协议 viewer 已就绪）
- **状态管理** → 复用 `executionStore.nodeReportsMap`

UI 不需要新建大量组件，主要是连接现有能力。

---

## 7. 实现范围（第一版）

### 7.1 Rust 侧
- `mpe-cli` crate：新增 `Commands::RunNode` 变体 + `cli/run_node.rs`
- `flow-engine-executor`：新增 `execute_single_node()` 公共方法（直接调用 `plugin.execute()`）

### 7.2 CLI 侧
- `mpe run node <json> [--var...]` 命令
- 输出 `CliOutput<NodeExecutionReport>` JSON

### 7.3 UI 侧（可选，可延后）
- Node config panel 或 context menu 增加 "测试" 按钮
- 复用 `NodeExecutionDetails` 展示结果

### 7.4 不做（第一版）
- ❌ 多节点链式 `mpe run nodes`
- ❌ 保存测试报告
- ❌ 历史记录
- ❌ 批量测试
