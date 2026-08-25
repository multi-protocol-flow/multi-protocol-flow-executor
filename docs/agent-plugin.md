# Agent 智能体插件设计与技术规范文档

> **Status:** 规范设计（Normative Spec / M1）
> **范围:** 基于 `mpe-plugin-sdk` 的通用 Agent 智能体插件（`plugins/agent/`），以 Sidecar 插件形式提供 ReAct 范式自主决策节点 `agent:react`
> **关联文档:** `docs/plugin-architecture.md`（插件架构）、`docs/plugin-protocol.md`（Wire 规范）、`docs/plugin-sdk.md`（SDK 指南）、`docs/grpc-plugin.md`（插件参考实现）
> **仓库位置:** `plugins/agent/`（独立 workspace、独立 `Cargo.lock`；插件自带 React 前端 `plugins/agent/frontend/`）
> **SDK 依赖:** `mpe-plugin-sdk`（扩展支持 `host.call_tool` 双向 RPC 调用）

---

## 0. 概要与架构定位 (Overview & Positioning)

### 0.1 核心定义
在 Multi-Protocol Flow Executor (MPE) 架构中，传统节点（如 HTTP Request、Redis Command、Database Query、gRPC Call 等）均属于**确定性单步执行器**，其执行逻辑由用户在画布中静态编排决定。

**Agent 插件（`agent:react`）** 定位为 MPE 流程中的 **LLM 驱动自主决策大脑与动态调度中心**：
- 采用 **ReAct (Reasoning + Acting)** 循环模式，根据动态输入数据、前序节点输出及给定的目标，自主规划多步行动。
- 通过大模型推理生成思考过程（Chain-of-Thought / Thinking），决定是否调用工具、调用何种工具、构造何种参数。
- 观察工具底层执行反馈，在遇到错误时触发**自我修复（Self-Correction）**机制，直至达成目标并产出最终结论。

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          MPE 画布流程 (Canvas Flow)                          │
│                                                                             │
│   ┌───────────────┐        ┌──────────────────┐        ┌────────────────┐   │
│   │  HTTP Ingress │───────►│   agent:react    │───────►│  Notification  │   │
│   │ (获取告警数据) │        │ (自主排障与修复)  │        │   (发送飞书群)   │   │
│   └───────────────┘        └─────────┬────────┘        └────────────────┘   │
└──────────────────────────────────────┼──────────────────────────────────────┘
                                       │ 动态调度全协议工具 (Dynamic Tools)
        ┌──────────────────────────────┼──────────────────────────────┐
        ▼                              ▼                              ▼
┌───────────────┐              ┌───────────────┐              ┌───────────────┐
│  Redis (查询) │              │  SQL (查日志) │              │  gRPC (重启)  │
└───────────────┘              └───────────────┘              └───────────────┘
```

### 0.2 协议无关 (Protocol-Agnostic) 与全协议 Tools 动态转换
Agent 插件本身不绑定任何特定协议。得益于 MPE 宿主统一的节点执行能力与插件生态，**MPE 中的任何协议节点、独立动作或子流程均可动态映射为 Agent 的 Tool**：
- **内置协议：** HTTP (`http`), WebSocket (`ws:send`), Script (`script`), Assertion (`assertion`), TCP (`tcp:send`), SSE (`sse:subscribe`), GraphQL (`graphql:query`)。
- **插件协议：** Database (`db:query`), Redis (`redis:cmd`), gRPC (`grpc:call`), Kafka (`kafka:produce`), Mongo (`mongo:find`), S3 (`s3:get_object`), MCP (`mcp:call_tool`) 等。
- **复合子流程：** 嵌套子流程（`.mpf` 流程），可作为一个复合高阶 Tool 暴露给 Agent。

---

## 1. 双向调度通信规范 (Bidirectional RPC Specification)

### 1.1 架构拓扑
Agent 插件运行在独立的 Sidecar 进程中，通过基于标准输入输出 (`stdio`) 的 JSON-RPC 2.0 协议与 MPE 宿主进程通信。为了实现 Agent 动态驱动宿主执行工具的能力，通信模型从单向请求扩展为**双向 RPC 交互模型**：

```
MPE 宿主进程 (flow-engine-plugin / executor)                  Agent 插件进程 (mpe-plugin-agent)
┌─────────────────────────────────────────┐                  ┌─────────────────────────────────────────┐
│ FlowExecutor                            │                  │ AgentReActExecutor                      │
│ ┌─────────────────────────────────────┐ │                  │ ┌─────────────────────────────────────┐ │
│ │ 1. execute_flow 调度到 agent:react  │ │                  │ │ 2. 组装 Prompt / Tools              │ │
│ └──────────────────┬──────────────────┘ │                  │ │ 3. 调用 LLM API (OpenAI/Claude/...)  │ │
│                    │                    │   JSON-RPC 2.0   │ │ 4. LLM 产出 ToolCall                │ │
│                    ▼                    │   Method:        │ └──────────────────┬──────────────────┘ │
│ flow-engine-plugin::SidecarCaller       │   "execute"      │                    │                    │
│ ┌─────────────────────────────────────┐ │─────────────────►│                    │                    │
│ │ 发送 execute 请求，阻塞等待最终结果 │ │                  │                    │                    │
│ └──────────────────▲──────────────────┘ │                  │                    │                    │
│                    │                    │   JSON-RPC 2.0   │                    ▼                    │
│                    │ 拦截 incoming      │   Method:        │ mpe_plugin_sdk::HostClient              │
│                    │ host.call_tool 请求│   "host.call_tool"┌─────────────────────────────────────┐ │
│                    │                    │◄─────────────────┤ 5. 发起 host.call_tool 请求           │ │
│                    ▼                    │                  │    (阻塞等待宿主返回工具执行结果)       │ │
│ FlowExecutor::execute_single_node       │                  └──────────────────▲──────────────────┘ │
│ ┌─────────────────────────────────────┐ │                  │                    │                    │
│ │ 6. 提取 ExecutionContext 连接池      │ │   JSON-RPC 2.0   │                    │                    │
│ │ 7. 派发执行对应协议节点 (DB/Redis/gRPC)│ │   Response     │                    │                    │
│ │ 8. 捕获 Raw Output / Error          │ │─────────────────►│ 9. 接收工具执行结果, 注入对话上下文    │ │
│ └─────────────────────────────────────┘ │                  │ 10. LLM 下一轮迭代 / 收敛产出最终结论   │ │
│                                         │                  └──────────────────┬──────────────────┘ │
│                                         │   JSON-RPC 2.0                      │                    │
│                                         │   Response                          ▼                    │
│                                         │◄─────────────────────────────────────────────────────────┘
│ 11. 接收最终执行报告并向后路由连线       │
└─────────────────────────────────────────┘
```

### 1.2 `host.call_tool` Wire 协议规范
当 Agent 在思考循环中决定调用工具时，通过 `stdio` 向上游宿主发送一个标准的 JSON-RPC 2.0 请求（方法名为 `host.call_tool`）。

#### 1.2.1 Request 契约 (Plugin ➔ Host)
```json
{
  "jsonrpc": "2.0",
  "id": "call-tool-9b7e41ac-001",
  "method": "host.call_tool",
  "params": {
    "execution_id": "exec-20260824-001928-a8f2",
    "tool_id": "query_order_db",
    "node_type": "db:query",
    "config": {
      "connection_uuid": "conn-mysql-prod-01",
      "sql": "SELECT order_id, user_id, status, amount FROM t_orders WHERE status = 'PENDING' LIMIT 5;",
      "timeout_ms": 5000
    }
  }
}
```

**字段规范：**
- `execution_id` (`string`, 必填): 当前流程执行的唯一生命周期实例 ID，确保与宿主及其他插件的有状态连接池（如 DB 事务、Redis 连接、gRPC Session）严格对齐。
- `tool_id` (`string`, 必填): 工具在 Agent 配置中的唯一标识名称。
- `node_type` (`string`, 必填): 目标工具映射的 MPE 节点类型（如 `db:query`, `redis:cmd`, `http`, `grpc:call`）。
- `config` (`object`, 必填): 目标节点执行所需的完整配置对象（由固定预设参数与大模型动态入参合并生成）。

#### 1.2.2 Success Response 契约 (Host ➔ Plugin)
```json
{
  "jsonrpc": "2.0",
  "id": "call-tool-9b7e41ac-001",
  "result": {
    "success": true,
    "duration_ms": 42,
    "output": {
      "rows": [
        { "order_id": "ORD-1001", "user_id": 9527, "status": "PENDING", "amount": 199.50 },
        { "order_id": "ORD-1002", "user_id": 9528, "status": "PENDING", "amount": 88.00 }
      ],
      "rows_affected": 2
    },
    "raw_response": "{\"rows\":[{\"order_id\":\"ORD-1001\",\"user_id\":9527,\"status\":\"PENDING\",\"amount\":199.50},{\"order_id\":\"ORD-1002\",\"user_id\":9528,\"status\":\"PENDING\",\"amount\":88.00}],\"rows_affected\":2}"
  }
}
```

#### 1.2.3 Error Response 契约 (Host ➔ Plugin)
当底层工具执行失败（如网络超时、SQL 语法错误、认证失败、连接断开）时，宿主返回业务执行错误结果（注意：使用 JSON-RPC 成功包裹业务失败，以便 Agent 获取详细错误上下文并启动自我修复）：
```json
{
  "jsonrpc": "2.0",
  "id": "call-tool-9b7e41ac-001",
  "result": {
    "success": false,
    "duration_ms": 1250,
    "error": {
      "code": "DB_QUERY_FAILED",
      "message": "Table 'order_center.t_order' doesn't exist",
      "detail": "Error 1146 (42S02): Table 'order_center.t_order' doesn't exist. Did you mean 't_orders'?"
    },
    "raw_response": "{\"code\":\"DB_QUERY_FAILED\",\"message\":\"Table 'order_center.t_order' doesn't exist\"}"
  }
}
```

### 1.3 宿主拦截机制与上下文共享
1. **Sidecar 通信泵拦截：** 宿主 `flow-engine-plugin` 在向 Sidecar 发出 `execute` 并等待响应时，其 Stdio 读取循环具备**双向请求分发器（Demuxer）**。
   - 读取到的帧若包含 `method: "host.call_tool"` 且包含 `id`，立即判定为插件发起的逆向 RPC 请求。
   - 宿主从请求中提取 `execution_id`，定位当前执行上下文 `ExecutionContext`。
2. **连接池穿透与状态共享：**
   - 宿主直接使用该 `ExecutionContext` 调度 `FlowExecutor::execute_single_node(node_type, config)`。
   - 如果目标节点是插件节点（如 `db:query` 调用 `mpe-plugin-db`），`mpe-plugin-db` 依据传入的相同 `execution_id` 直接从其内存池获取已有连接（例如前序 `db:connect` 或 `db:begin` 建立的连接/事务），实现跨插件、无缝的状态共享。
3. **响应回路：** 宿主执行完成后，将节点执行的 `NodeExecutionOutput` 序列化为 JSON-RPC 响应，写回 Agent 插件的标准输入。

---

## 2. 工具抽象与参数切分模型 (Tool Abstraction & Parameter Schema)

### 2.1 工具来源三态定义
在 Agent 配置中，Tool 支持三种来源形态：

```
                           ┌──────────────────────────────────────────────┐
                           │               Agent Tool 来源形态            │
                           └──────────────────────┬───────────────────────┘
                                                  │
              ┌───────────────────────────────────┼──────────────────────────────────┐
              ▼                                   ▼                                  ▼
   【LinkedNode 画布节点】              【StandaloneAction 独立动作】            【Subflow 复合子流程】
   - 引用画布上已存在的节点             - 无需在画布显式建节点                   - 引用独立 .mpf 流程文件
   - 复用其完整预设配置与连线           - 专为 Agent 声明协议动作                - 复合多协议多步骤任务
   - 例如：复用现有 "MySQL 连接池"       - 例如：临时执行 HTTP API 或 Redis       - 例如："用户退款并发送通知"
```

1. **画布连线节点 (`LinkedNode`)：**
   - 用户在画布上拖拽并配置好的节点（如配置了完备连接参数、安全凭证的 `db:query` 或 `http` 节点）。
   - Agent 节点通过 `target_node_id` 引用该节点。
2. **独立协议动作 (`StandaloneAction`)：**
   - 不需要在画布呈现独立节点图形，直接在 Agent 内部声明式定义协议动作（如快捷定义一个只读 Redis 查询或 DNS 解析）。
3. **子流程 (`Subflow`)：**
   - 引用工作空间内已有的 `.mpf` 流程文件，Agent 传入流程变量，由宿主执行整条子流程并汇总最终输出变量作为 Tool 结果返回。

### 2.2 参数切分与合并原理 (Static Preset vs LLM Dynamic Params)
为了兼顾**系统安全性**（防止大模型篡改核心鉴权参数或越权访问危险数据库）与**调用灵活性**，Agent 采用严格的“人类预设参数 + LLM 动态入参”切分模型：

| 参数类别 | 拥有者 | 典型字段 | 安全控制 |
|---|---|---|---|
| **固定预设参数 (`fixed_config`)** | 人类工程师 (配置面板) | `connection_uuid`, `base_url`, `auth_token`, `headers`, `timeout_ms` | 严格锁定，大模型不可见且不可覆写 |
| **动态入参 (`dynamic_args`)** | 大模型 (LLM Tool Calling) | `sql`, `command`, `path`, `query_params`, `body` | 经 Schema 约束，由模型根据推理生成 |

#### 参数合并算法 (`merge_tool_config`)
```rust
pub fn merge_tool_config(
    fixed_config: &serde_json::Value,
    dynamic_args: &serde_json::Value,
    override_allowed_keys: &[String],
) -> Result<serde_json::Value, ToolConfigError> {
    let mut final_config = fixed_config.clone();
    
    if let (Some(base_obj), Some(dyn_obj)) = (final_config.as_object_mut(), dynamic_args.as_object()) {
        for (key, val) in dyn_obj {
            // 仅允许模型填充被显式暴露给 Tool Schema 的白名单字段
            if override_allowed_keys.contains(key) {
                base_obj.insert(key.clone(), val.clone());
            } else {
                return Err(ToolConfigError::SecurityViolation(format!(
                    "LLM attempted to overwrite protected configuration field: {}", key
                )));
            }
        }
    }
    Ok(final_config)
}
```

### 2.3 自动生成 OpenAI 兼容 Function Calling Schema
Agent 插件根据每个 Tool 的定义与动态暴露字段，自动编译生成 OpenAI / Anthropic 兼容的 JSON Schema：

```json
{
  "type": "function",
  "function": {
    "name": "query_order_db",
    "description": "查询订单中心数据库。支持标准只读 SQL 查询，如查询订单状态、支付金额、创建时间等。",
    "parameters": {
      "type": "object",
      "properties": {
        "sql": {
          "type": "string",
          "description": "标准只读 SQL 语句。注意表名为 t_orders, t_order_items，禁止使用 DELETE/UPDATE/DROP 等写操作。"
        }
      },
      "required": ["sql"],
      "additionalProperties": false
    }
  }
}
```

---

## 3. ReAct 运行时执行引擎与自我修复机制 (ReAct Engine & Self-Correction)

### 3.1 运行时状态机 (ReAct Execution Loop)

```
                       ┌─────────────────────────────────────┐
                       │           1. 初始化状态             │
                       │ 组装 System Prompt / Input / Tools  │
                       └──────────────────┬──────────────────┘
                                          │
                                          ▼
                       ┌─────────────────────────────────────┐
                       │           2. 组装会话历史           │◄────────────────────────┐
                       │  [System, User, (Assistant, Tool)*] │                         │
                       └──────────────────┬──────────────────┘                         │
                                          │                                            │
                                          ▼                                            │
                       ┌─────────────────────────────────────┐                         │
                       │        3. 调用大模型 LLM API         │                         │
                       │    (流式接收 Thinking & ToolCalls)  │                         │
                       └──────────────────┬──────────────────┘                         │
                                          │                                            │
                                          ▼                                            │
                       ┌─────────────────────────────────────┐                         │
                       │         4. 判定模型输出类型         │                         │
                       └──────┬───────────────────────┬──────┘                         │
                              │                       │                                │
                 [产出最终答案] │                       │ [决定调用 Tool]                 │
                              ▼                       ▼                                │
              ┌────────────────────────┐  ┌────────────────────────┐                   │
              │     5. 校验收敛条件    │  │   6. 安全护栏与权限检查 │                   │
              │  构造 Agent 最终报告   │  │   (只读检查 / 注入拦截)│                   │
              └───────────────┬────────┘  └───────────┬────────────┘                   │
                              │                       │                                │
                              ▼                       ▼ [检查通过]                     │
              ┌────────────────────────┐  ┌────────────────────────┐                   │
              │     8. 执行成功结束    │  │   7. 发起 host.call_tool│                   │
              │   路由到 'true' 输出端口│  │   派发执行宿主协议节点 │                   │
              └────────────────────────┘  └───────────┬────────────┘                   │
                                                      │                                │
                                                      ▼                                │
                                  ┌────────────────────────────────────────┐           │
                                  │      8. 评估工具结果与自我修复判定     │───────────┘
                                  │ 成功 ➔ 注入 Tool Message 正常迭代      │
                                  │ 失败 ➔ 注入详细 Error 驱动 LLM 修正重试│
                                  └────────────────────────────────────────┘
```

### 3.2 自我修复机制 (Self-Correction Engine)
在复杂的分布式环境与动态接口调用中，LLM 构造的入参可能出现偏差（如表名笔误、字段名不符、JSON 结构缺少必填项、HTTP 接口返回 404 等）。

传统工作流在节点报错后直接中断流程，而 **Agent 插件通过闭环错误反馈实现智能化自我修复**：

```
                                  自我修复循环 (Self-Correction Cycle)
                                  
  ┌─────────────────┐             ┌─────────────────┐             ┌─────────────────┐
  │ LLM 第一次尝试  │  调用报错    │ 宿主捕获底层错误 │  结构化反馈  │ LLM 收到错误上下文│
  │ SELECT * FROM   ├────────────►│ Table 't_order' ├────────────►│ "抱歉，表名错误，  │
  │ t_order LIMIT 1;│             │ doesn't exist   │             │ 应为 t_orders"  │
  └─────────────────┘             └─────────────────┘             └────────┬────────┘
                                                                           │
                                  ┌─────────────────┐                      │
                                  │ 宿主执行成功返回 │      再次发起调用    │
                                  │ 200 OK / 数据集 ◄──────────────────────┘
                                  └─────────────────┘  SELECT * FROM t_orders LIMIT 1;
```

#### 自我修复设计关键准则：
1. **真实底层错误上下文注入：** 宿主捕获到的原生错误（包含协议状态码、错误堆栈、数据库 Error Code、提示信息）原汁原味注入到对话历史中，作为 Tool Message 提供给模型。
2. **状态纯隔离（No Side-Effect on Canvas）：** 动态修复仅修改当前 Agent 执行上下文内的推理历史，**严禁修改画布上已保存的节点静态配置**。
3. **修复尝试次数控制：** 每次工具调用失败均计入 `iteration_count`，受最大迭代轮数 (`max_iterations`) 统一约束，防止模型在无法恢复的致命错误上无限死循环。

### 3.3 安全护栏与收敛控制 (Guardrails)
为保障生产系统稳定性，Agent 运行时内置三重硬核安全护栏：
- **最大迭代轮次 (`max_iterations`)：** 默认 10 轮。当超出阈值仍未收敛，状态机强制终止，将已有思维链与未完成原因输出，并路由至 `false` 端口。
- **全局只读模式 (`read_only_mode`)：** 开启后，插件在派发 `host.call_tool` 前进行静态语法与意图审计。
  - SQL 工具：拦截 `INSERT`, `UPDATE`, `DELETE`, `DROP`, `ALTER`, `TRUNCATE` 等关键字。
  - Redis 工具：仅允许 `GET`, `HGET`, `MGET`, `SCAN`, `KEYS` 等只读命令。
  - HTTP 工具：拦截 `POST`, `PUT`, `DELETE`, `PATCH` 请求。
- **单步与全局硬超时 (`timeout_secs`)：** 监控单次 LLM API 请求与单次 Tool 执行耗时，防止进程挂起。

---

## 4. 节点规格与多语言定义 (Node Specifications)

### 4.1 节点元数据
- **Type ID:** `agent:react`
- **Display Name (zh-CN):** ReAct 智能体
- **Display Name (en-US):** ReAct Agent
- **Category:** `agent`
- **Icon:** `Bot` / `Brain`
- **Input Ports:** `["in"]`
- **Output Ports:** `["true", "false"]`（成功达成目标收敛路由至 `true`；超限或不可逆致命错误路由至 `false`）

### 4.2 节点配置结构 (Rust / TypeScript)

#### 4.2.1 Rust 结构体定义
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReactConfig {
    /// LLM 基础配置
    pub llm: LlmConfig,
    
    /// 系统提示词 (System Prompt)，支持 {{variables}} 插值
    pub system_prompt: String,
    
    /// 用户目标提示词 (User Goal Prompt)，支持 {{steps.x.output}} 插值
    pub user_prompt: String,
    
    /// 注册给 Agent 的工具列表
    pub tools: Vec<AgentToolDefinition>,
    
    /// 最大思考迭代轮数 (默认: 10)
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    
    /// 模型采样温度 (0.0 ~ 2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    
    /// 全局硬超时时间 (秒)
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    
    /// 是否开启只读安全保护模式
    #[serde(default = "default_false")]
    pub read_only_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// 提供商类型: "openai" | "anthropic" | "deepseek" | "ollama" | "custom"
    pub provider: String,
    /// 模型名称: "gpt-4o", "deepseek-chat", "claude-3-5-sonnet", etc.
    pub model: String,
    /// API 密钥 (支持 {{env.OPENAI_API_KEY}})
    pub api_key: String,
    /// API 基础端点 (如 "https://api.deepseek.com/v1")
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolDefinition {
    /// 工具唯一标识名
    pub id: String,
    /// 工具呈现名称
    pub name: String,
    /// 给大模型的工具能力描述
    pub description: String,
    /// 工具类型: "linked_node" | "standalone_action" | "subflow"
    pub tool_type: String,
    /// 目标节点类型 (如 "db:query", "redis:cmd", "http")
    pub target_node_type: String,
    /// 绑定的画布节点 ID (当 tool_type 为 "linked_node" 时必填)
    pub target_node_id: Option<String>,
    /// 人类预设固定配置
    pub fixed_config: serde_json::Value,
    /// 暴露给 LLM 动态填写的入参 Schema
    pub dynamic_schema: serde_json::Value,
}

fn default_max_iterations() -> u32 { 10 }
fn default_temperature() -> f32 { 0.2 }
fn default_timeout_secs() -> u64 { 120 }
fn default_false() -> bool { false }
```

#### 4.2.2 输出结果结构体 (Output & Report Payload)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReactOutput {
    /// 是否成功收敛达成目标
    pub success: bool,
    /// 最终结论文本 (Markdown 格式)
    pub final_answer: String,
    /// 实际迭代轮数
    pub iterations: u32,
    /// 工具调用总次数
    pub tool_call_count: u32,
    /// 自我修复发生次数
    pub self_corrections_count: u32,
    /// Token 消耗统计
    pub token_usage: TokenUsage,
    /// 完整全量时间线记录
    pub timeline: Vec<TimelineStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineStep {
    pub step_index: u32,
    /// 步骤类别: "thought" | "tool_call" | "self_correction" | "final_answer"
    pub step_type: String,
    /// 该轮的思维链推理内容
    pub thought: Option<String>,
    /// 工具调用信息
    pub tool_call: Option<TimelineToolCall>,
    /// 单步消耗的耗时 (毫秒)
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineToolCall {
    pub tool_id: String,
    pub tool_name: String,
    pub node_type: String,
    pub input_args: serde_json::Value,
    pub success: bool,
    pub output_data: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub raw_response: String,
}
```

---

## 5. 嵌入式前端交互与全细节查看器 (Frontend Panels & Viewer)

Agent 插件自带独立的前端产物（基于 `plugins/agent/frontend/` 打包成单文件 HTML，宿主通过 `iframe + postMessage` 双向桥接通信），包含**设计态配置面板**与**运行态 Thinking Timeline 查看器**。

### 5.1 设计态配置面板 (Config Panel)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ ⚙️ 配置 ReAct 智能体 (agent:react)                                           │
├─────────────────────────────────────────────────────────────────────────────┤
│ 1. 大模型配置 (LLM Provider)                                                │
│    提供商: [ DeepSeek (OpenAI-Compatible)  ▼ ]                               │
│    模型名: [ deepseek-chat                 ▼ ]  采样温度: [ 0.2  ]          │
│    Base URL: [ https://api.deepseek.com/v1                                ] │
│    API Key:  [ {{env.DEEPSEEK_API_KEY}}                                   ] │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. 提示词设定 (Prompts)                                                     │
│    System Prompt:                                                           │
│    ┌──────────────────────────────────────────────────────────────────────┐ │
│    │ 你是资深运维与性能调优专家，负责针对告警日志分析异常并执行修复。    │ │
│    └──────────────────────────────────────────────────────────────────────┘ │
│    User Goal Prompt:                                                        │
│    ┌──────────────────────────────────────────────────────────────────────┐ │
│    │ 请排查订单延迟：{{steps.http_ingress.output.body.alert_desc}}       │ │
│    └──────────────────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────────────────┤
│ 3. 工具管理 (Registered Tools)                       [ + 添加协议工具 ]     │
│    ┌──────────────────────────────────────────────────────────────────────┐ │
│    │ 🔧 query_db (db:query) ➔ 引用节点: [ MySQL 订单主库 (node_2)  ▼ ]    │ │
│    │    描述: 查询订单明细与状态                                         │ │
│    │    动态入参: [ ✓ sql ]  (锁定 connection_uuid 与 timeout)            │ │
│    ├──────────────────────────────────────────────────────────────────────┤ │
│    │ 🔧 get_redis_cache (redis:cmd) ➔ 独立动作                             │ │
│    │    描述: 获取用户会话与锁状态                                       │ │
│    │    固定配置: connection_uuid: "redis_cluster_01"                     │ │
│    │    动态入参: [ ✓ command, ✓ key ]                                    │ │
│    └──────────────────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────────────────┤
│ 4. 安全护栏与收敛控制                                                       │
│    最大迭代轮数: [ 10 ]     超时控制: [ 120s ]     只读安全模式: [ 🔘 开启 ] │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 5.2 运行态全细节查看器 (Thinking Timeline Viewer)

Thinking Timeline 必须记录 **100% 完整的思维与通信细节**，杜绝任何黑盒操作，使工程排错与模型决策完全透明：

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🧠 Agent 决策与执行报告 (Thinking Timeline)                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│ 📊 运行指标看板                                                             │
│    [ ⏱️ 总耗时: 3.42s ] [ 🪙 Total Tokens: 1,842 (Prompt: 1420 / Comp: 422) ]│
│    [ 🔄 迭代轮次: 3 轮 ] [ 🛠️ 工具调用: 2 次 ] [ ⚠️ 自我修复: 1 次 ]         │
├─────────────────────────────────────────────────────────────────────────────┤
│ 🎯 最终结论 (Final Answer)                                                  │
│ ┌─────────────────────────────────────────────────────────────────────────┐ │
│ │ 经过排查，订单延迟原因为 `t_orders` 表中存在 2 笔处于 `PENDING` 状态的  │ │
│ │ 阻塞事务。已成功通过缓存比对确认用户状态正常，建议执行重试策略。        │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────────────────┤
│ 📜 决策时间线 (Timeline Detail)                                              │
│                                                                             │
│ ┌─ [Step 1] ──────────────────────────────────────────────────────────────┐ │
│ │ 💭 思维过程 (Thought):                                                  │ │
│ │   "首先需要检查订单主库中是否存在未处理的积压订单，我将执行 SQL 查询。" │ │
│ │ 🛠️ 工具调用: query_db (`db:query`)                                      │ │
│ │   ▶ 底层请求 Payload:                                                   │ │
│ │     { "sql": "SELECT * FROM t_order WHERE status = 'PENDING' LIMIT 5;" }│ │
│ │   ❌ 底层响应 (Error / 耗时 32ms):                                      │ │
│ │     Error 1146: Table 'order_center.t_order' doesn't exist              │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ ┌─ [Step 2 - ⚠️ Self-Correction 智能自我修复] ────────────────────────────┐ │
│ │ 💭 思维过程 (Thought):                                                  │ │
│ │   "上一步提示表 `t_order` 不存在。根据数据库命名规范，表名应为复数      │ │
│ │    `t_orders`。我将修正 SQL 语句后重新发起查询。"                       │ │
│ │ 🛠️ 工具调用: query_db (`db:query`)                                      │ │
│ │   ▶ 底层请求 Payload:                                                   │ │
│ │     { "sql": "SELECT * FROM t_orders WHERE status = 'PENDING' LIMIT 5;"}│ │
│ │   ✅ 底层响应 (Success / 耗时 48ms):                                    │ │
│ │     { "rows": [{ "order_id": "ORD-1001", "status": "PENDING" }], ... }  │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│ ┌─ [Step 3] ──────────────────────────────────────────────────────────────┐ │
│ │ 💭 思维过程 (Thought):                                                  │ │
│ │   "查询成功获取到 2 条积压订单数据，目标达成，开始组织最终排障结论。"   │ │
│ │ 🏁 产出最终答案 (Final Answer Generated / 耗时 820ms)                    │ │
│ └─────────────────────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────────────────┤
│ [ 📋 复制完整 Raw JSON 链路数据 ]                       [ 💾 导出 Markdown ]│
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. CLI 输出规范与集成测试用例 (CLI Output & Verification)

### 6.1 CLI 紧凑 ASCII 树状日志输出
在 `mpe run flow.mpf` 纯命令行模式下，Agent 节点通过标准错误流输出层级清晰、富有表现力的 ASCII 树状进度：

```
[INFO] Executing node: agent_diagnose (agent:react)
[AGENT:START] Goal: "排查订单延迟并确认阻塞原因"
├── 🔄 [Round 1/10] Thinking...
│   ├── 💭 Thought: 需要查询订单主库未结订单数据
│   ├── 🛠️  Call Tool: query_db (node_type: db:query)
│   │   ├── Payload: {"sql": "SELECT * FROM t_order LIMIT 5"}
│   │   └── ❌ Error: Table 'order_center.t_order' doesn't exist (32ms)
├── 🔄 [Round 2/10] ⚠️ Self-Correcting...
│   ├── 💭 Thought: 修正表名为 t_orders 并重试查询
│   ├── 🛠️  Call Tool: query_db (node_type: db:query)
│   │   ├── Payload: {"sql": "SELECT * FROM t_orders LIMIT 5"}
│   │   └── ✅ Success: 2 rows returned (48ms)
└── 🏁 [Round 3/10] Final Answer Converged (Success: true, Tokens: 1842, Duration: 3.42s)
[INFO] Node agent_diagnose completed -> routing to 'true' port
```

### 6.2 单元与端到端集成测试策略

#### 6.2.1 Mock LLM Server (基于 Axum / SSE)
在测试中启动本地轻量 Mock HTTP 服务，模拟 LLM 接口输出标准 OpenAI / DeepSeek SSE 流：
1. **测试用例 1：一次性直接收敛**（无 Tool Call，直接产出结论）。
2. **测试用例 2：标准 Tool 调用与多轮迭代**（LLM 产出 ToolCall ➔ Host 返回结果 ➔ LLM 收敛）。
3. **测试用例 3：自我修复测试**（首次 ToolCall 故意返回数据库错误，验证 Agent 第二轮自主修正 SQL 语句）。
4. **测试用例 4：安全护栏拦截测试**（只读模式下拦截大模型尝试发起的 `DELETE FROM t_orders` 语句）。
5. **测试用例 5：超限终止测试**（模型陷入死循环，验证在达到 `max_iterations = 3` 时平稳收敛至 `false` 分支）。

#### 6.2.2 集成测试示例代码 (`tests/agent_react_test.rs`)
```rust
#[tokio::test]
async fn test_agent_self_correction_flow() {
    // 1. 启动 Mock LLM 驱动自我修复序列
    let mock_server = MockLlmServer::builder()
        .round_1_response_tool_call("query_db", json!({"sql": "SELECT * FROM wrong_table"}))
        .round_2_response_tool_call("query_db", json!({"sql": "SELECT * FROM right_table"}))
        .round_3_response_final_answer("Query succeeded with correct table.")
        .start().await;

    // 2. 构造包含 agent:react 与 mock db:query 的测试流程
    let flow = TestFlowBuilder::new()
        .add_node("agent_node", "agent:react", json!({
            "llm": {
                "provider": "openai",
                "model": "mock-model",
                "api_key": "mock-key",
                "base_url": mock_server.url()
            },
            "system_prompt": "You are a test agent.",
            "user_prompt": "Query the table.",
            "tools": [{
                "id": "query_db",
                "name": "Query Database",
                "description": "Execute SQL query",
                "tool_type": "standalone_action",
                "target_node_type": "db:query",
                "fixed_config": { "connection_uuid": "test-db" },
                "dynamic_schema": {
                    "type": "object",
                    "properties": { "sql": { "type": "string" } },
                    "required": ["sql"]
                }
            }]
        }))
        .build();

    // 3. 执行流程
    let result = FlowExecutor::execute_flow(&flow).await.expect("Flow execution failed");
    
    // 4. 验证自我修复断言
    let agent_output: AgentReactOutput = result.get_node_output("agent_node");
    assert!(agent_output.success);
    assert_eq!(agent_output.iterations, 3);
    assert_eq!(agent_output.self_corrections_count, 1);
    assert_eq!(agent_output.tool_call_count, 2);
    assert!(agent_output.final_answer.contains("Query succeeded"));
}
```

---

## 7. 总结与后续实施路线图

| 阶段 | 核心任务 | 交付物 |
|---|---|---|
| **Phase 1: SDK 扩展** | `mpe-plugin-sdk` 增加 `host.call_tool` 请求派发与反向 RPC 通道支持 | `mpe-plugin-sdk` v0.3.0 |
| **Phase 2: 宿主拦截器** | `flow-engine-plugin` 实现 `host.call_tool` 拦截与 `FlowExecutor` 节点调度 | 宿主双向调度能力 |
| **Phase 3: Agent 核心引擎** | `plugins/agent/` 实现 ReAct 状态机、LLM 流式调用、自我修复与安全护栏 | `mpe_plugin_agent` 二进制 |
| **Phase 4: 前端交互** | `plugins/agent/frontend/` 实现配置面板与 Thinking Timeline Viewer | `panel.html` / `viewer.html` |
| **Phase 5: CLI & 端到端集成** | CLI 树状进度打印、Mock LLM 验证全协议工具调度与自我修复 | 完整回归测试套件 |
