# MPE 插件市场方案（讨论稿 v2.1）

> 目标：让第三方开发者可以独立开发协议插件，用户安装即用，**无需重新编译宿主应用**。
>
> 状态：讨论稿 v2.1。A/B/C/D 组决策点已全部拍板（见第 12 章）。v2.1 基于对抗式评审（5 名评审成员 × 3 轮交叉攻击）对 v2 做了**事实修正与安全修正**，修正点以 🔧 标记就地标注，汇总见第 13 章。实施必须以 v2.1 为准（含第 13 章修正），不得以 v2 原文为准。

---

## 1. 背景与需求

### 1.1 现状痛点

- 项目当前 16 个协议 crate，首次编译需全量编译（AMQP 16k LOC、DB 8k、gRPC 7k、Kafka 6k…）
- 已有 `protocol-*` feature flag，但 `default = ["all-protocols"]`，且**加协议必须重新编译**
- 目标不是"减少自己的编译时间"，而是**开放第三方生态**

### 1.2 需求定义

| 需求 | 说明 |
|---|---|
| 第三方开发 | **语言放开**（不局限 Rust），官方推荐 Rust 开发 |
| 用户安装 | 下载插件 → 放入目录 → 即用，**不重编译宿主** |
| 前端适配 | 插件自带配置面板，宿主动态加载 |
| 插件市场 | 版本管理、分发、未来签名验签 |
| **压测支持** | 压测场景下插件 IPC 不能成为吞吐瓶颈 |

---

## 2. 方案选型

| 方案 | 可行 | 关键问题 |
|---|---|---|
| **Feature flag（现状）** | ✅ | 必须重编译，不满足生态需求 |
| **Rust .dll/.so 动态加载** | ❌ | Rust 无稳定 ABI；`Arc<dyn Any>` provider 穿不过 FFI；每个 .so 自带 tokio/openssl 体积爆炸；跨平台噩梦 |
| **WASM 插件** | ⚠️ | tonic/rdkafka/sqlx 等重型依赖编不到 wasm-wasi；WASI socket/tokio 不成熟 |
| **Sidecar 子进程** | ✅ 正解 | 语言无关；进程隔离；安装即用；VSCode LSP 验证过的模式 |

**结论：Sidecar 子进程 + JSON-RPC 2.0 over stdin/stdout（异步流水线）。**

### 2.1 为什么选 stdin/stdout 而非 TCP

| 选项 | 结论 |
|---|---|
| **stdin/stdout JSON-RPC** | 🏆 最简单。无端口管理、无网络暴露、可 `echo '{}' \| plugin` 独立调试 |
| TCP socket | 需要端口分配，有本地端口暴露风险 |
| Unix socket | Windows 支持差 |
| gRPC | 太重，第三方写 proto 门槛高 |
| HTTP | 插件要内置 HTTP server，门槛高 |

**性能可行性**（详见第 7 章）：压测场景下单管道异步流水线可支撑 5-10 万 RPC/s，对网络 I/O 型插件（HTTP/MQTT/DB…）的 IPC 开销占比 <0.5%，可忽略。

---

## 3. 总体架构

```
┌──────────────────────────────────────────────────────────────┐
│                  宿主 (mpe 桌面应用)                           │
│                                                               │
│  ┌──────────────┐   ┌─────────────────────────────────────┐  │
│  │  NodeRegistry │   │  PluginManager                      │  │
│  │  (内置+插件)  │   │  ├─ 扫描 ~/.mpe/plugins/            │  │
│  │               │   │  ├─ 启动/停止子进程                  │  │
│  │  SidecarPlugin│◄──│  ├─ 异步流水线 RPC（id 关联）        │  │
│  │  (伪装成 Node │   │  ├─ 心跳/超时/崩溃恢复              │  │
│  │   Plugin)     │   │  ├─ 背压控制（按并发上限排队）       │  │
│  │               │   │  └─ 进程池管理                      │  │
│  └──────────────┘   └─────────────────────────────────────┘  │
│         │                          │                          │
│  ┌──────▼──────┐          ┌───────▼────────┐                 │
│  │ 执行引擎     │          │ IframeBridge    │                 │
│  │ (execute_    │          │ (前端 postMessage│                 │
│  │  node 拦截)  │          │  桥接)          │                 │
│  └─────────────┘          └────────────────┘                 │
└──────────────────────────────────────────────────────────────┘
        │ 启动子进程                     │ 返回节点元数据+前端URL
        │ 异步流水线 JSON-RPC            │
        ▼                                ▼
┌─────────────────────────────┐   ┌──────────────────────┐
│  插件进程 (任意语言)         │   │  插件自带前端资源      │
│  mongodb-plugin.exe         │   │  http://localhost:9101│
│  ├─ 异步事件循环读 stdin     │   │  /config-panel.html  │
│  ├─ 每请求 spawn task（并发）│   └──────────────────────┘
│  ├─ describe/execute/...    │
│  ├─ 内部连接池 (插件自管)    │
│  └─ 内置 HTTP 服务(可选)     │
└─────────────────────────────┘
```

**核心思想**：插件进程伪装成一个 `NodePlugin` 注册进宿主 `NodeRegistry`，执行引擎对内置/插件节点**零感知**——只是调用时走一次 IPC 转发。

---

## 4. 插件通信协议（JSON-RPC 2.0 over stdin/stdout）

### 4.1 传输层

- 逐行 JSON（`\n` 分隔），双方随时可发
- **异步流水线**：请求通过 `id` 关联响应，响应不要求按序返回（详见第 7 章）

### 4.2 方法总览

| 方法 | 方向 | 场景 |
|---|---|---|
| `describe` | 宿主→插件 (req) | 插件注册、节点元数据、前端资源 |
| `validate` | 宿主→插件 (req) | 配置验证 |
| `execute` | 宿主→插件 (req→res) | 普通节点执行 |
| `execute_stream` | 宿主→插件 (req→流+final) | 流式执行（SSE 监听/Kafka 消费） |
| `cancel` | 宿主→插件 (req) | 取消进行中的操作 |
| `shutdown` | 宿主→插件 (req) | 优雅关闭 |
| `heartbeat` | 宿主→插件 (req) | 保活探测 |
| `event` | 插件→宿主 (notification) | 流式事件/日志/进度 |
| `log` | 插件→宿主 (notification) | 独立日志通道 |

### 4.3 Describe（发现阶段）

```
→ {"jsonrpc":"2.0","id":1,"method":"describe",
   "params":{"host_version":"0.1.1-beta.1","host_features":["protocol-http","protocol-ws"]}}

← {"jsonrpc":"2.0","id":1,"result":{
     "plugin": {"name","version","author","min_host_version","description"},
     "nodes": [{
       "type_id","display_name","category","icon","color",
       "ports":[{"id","label","position"}],
       "default_config": {...},
       "config_schema": {...},            // JSON Schema，驱动动态表单
       "frontend": {                       // 前端资源（关键）
         "config_panel": {"type":"url"|"inline","value":"..."},
         "viewer":      {"type":"url"|"inline","value":"..."},
         "help": {"zh-CN":"...","en-US":"..."}
       }
     }],
     "capabilities": {
       "connection_pooling": true,
       "streaming": true,
       "lifecycle_hooks": true,
       "max_concurrent_requests": 1000,    // 插件并发上限（背压用）
       "concurrency_model": "async",       // async | thread_per_request
       "supports_pipelining": true
     }
   }}
```

`frontend` 三种形式：`url`（插件 HTTP 服务托管）/ `inline`（内联 HTML）/ `iframe`（外部 URL）。

> **已定（A4）**：首期仅支持 `inline`（内联 HTML）。`url`/`iframe` 形式二期再开放，协议字段预留。

### 4.4 Execute（执行阶段）—— 变量已由宿主替换

```
→ {"jsonrpc":"2.0","id":2,"method":"execute","params":{
     "flow_id":"...","node_instance_id":"...","type_id":"mongodb_find",
     "config": { ... },                    // ← 宿主已替换完的最终值
     "referenced_variables": { ... },      // ← 仅 config 引用的变量
     "providers": { "mongodb_conn": {...} }, // 前置节点注入的连接配置
     "settings": {"timeout_ms":30000,"on_error":"RouteToFalse"}
   }}

← {"jsonrpc":"2.0","id":2,"result":{
     "success": true,
     "next_ports": ["out"],
     "output_data": {...},                 // 下游节点消费（精简）
     "report_data": {...},                 // 给人看的报告（可详尽，可选）
     "variable_updates": {"total": 42},    // 🔧 v2.1: key 必须是扁平顶层变量名（见下）
     "provider_updates": {"mongodb_conn": {...}}, // 供后续节点使用
     "metrics": {"duration_ms": 23, "rows": 42},
     "errors": []
   }}
```

> 🔧 **v2.1 修正（A10，对抗评审确认的契约错误）**：v2 原文示例 `variable_updates: {"runtime.total": 42}` 是**错误的**。现有 `VariableStorage` 是扁平结构（`context.rs:490-532`，按精确名称 keyed），而 `replace_variables` 按 `.` 拆分路径（`flow-engine-core/src/executor.rs:48-52`）查找 `"runtime"` 对象——一个扁平 `"runtime.total"` key **永远无法被 `{{runtime.total}}` 解析到**，占位符会被静默保留（executor.rs:120）。**修正后的契约：`variable_updates` 的 key 必须是扁平顶层变量名（不含 `.`）**。宿主收到点分 key 时：告警 + 跳过该条目（非致命）+ 记入报告。该契约在实现 schema 冻结前必须先跑解析回归测试（见第 13 章 C1）。

### 4.5 Execute_stream（流式执行）

```
→ {"jsonrpc":"2.0","id":3,"method":"execute_stream","params":{...}}

← 流式事件（每行一个 JSON）:
  {"jsonrpc":"2.0","method":"event","params":{"execution_id":"run-1","node_instance_id":"n-3","type":"connected","data":{...}}}
  {"jsonrpc":"2.0","method":"event","params":{"execution_id":"run-1","node_instance_id":"n-3","type":"message","data":{...}}}
  {"jsonrpc":"2.0","method":"log","params":{"execution_id":"run-1","node_instance_id":"n-3","level":"info","message":"..."}}
← 最终结果:
  {"jsonrpc":"2.0","id":3,"result":{...}}

宿主收到事件 → 立即转成前端 ExecutionEvent 推送；
宿主收到取消 → {"method":"cancel","params":{"execution_id":"run-1","node_instance_id":"n-3"}}
```

> 🔧 **v2.1 修正（C5）**：v2 原文事件/日志**缺少执行关联字段**。压测下 N 个虚拟用户共享同一插件进程，无关联字段则宿主无法把事件路由到正确的流程/前端流。**每个 `event`/`log` 通知必须携带 `execution_id` + `node_instance_id`**（冻结在 schema 中，即使 execute_stream 排 P1）；`cancel` 也针对该二元组。执行级关联注册表统一取消关联与事件路由。

---

## 5. 变量处理机制（宿主统一替换）

**原则：插件永远只拿"已替换完的最终值"，插件内部零变量逻辑。**

### 5.1 执行时序

```
执行引擎遇到插件节点
  → 宿主调 resolve_variables_in_json(raw_config, context)   ← 递归替换 {{path}}
  → 把替换后的 config 发给插件
  → 插件直接用最终值执行
  → 插件返回 variable_updates（增量）
  → 宿主 merge 回 VariableStorage
  → 下一个节点继续（同样流程）
```

### 5.2 复用现有工具

`flow-engine-core/src/executor.rs` 已有 `resolve_variables_in_json()`——递归遍历 JSON 树，替换每个字符串里的 `{{path}}`，无需新增逻辑。

### 5.3 变量策略

| 场景 | 策略 | 说明 |
|---|---|---|
| **执行阶段** | 策略 A：只发 config 引用的变量 | 最小数据量、最小泄露面 |
| **配置面板编辑** | 策略 B：全量只读展示 | 面板需要浏览变量，不执行 |

### 5.4 约束

- **禁止嵌套 `{{}}`**（已定 B2）：`resolve_variables_in_json` 单趟替换；validator 增加规则，防止写入含占位符的变量值。
- **写回路径一致**（🔧 v2.1 修正）：v2 原文"插件 `variable_updates` 用原始变量路径 key，宿主按 key merge，不存在回填问题"对**点分 key 不成立**——见 §4.4 修正块。**契约：`variable_updates` 必须使用扁平顶层变量名**；宿主按精确名 merge 到 `VariableStorage`；点分 key 告警+跳过。解析回归测试先行。
- **敏感信息随 config 传递**（已定 B3）：密码/token 经变量替换后直接包含在发给插件的 `config` 中，不做额外过滤。宿主已不再内置加密 crate；插件（如 plugins/smtp、plugins/imap）自行携带 `crypto.rs` 副本，负责内存中的凭据保护。宿主对 `referenced_variables` 中已引用的敏感变量不做拦截。

### 5.5 Provider（连接池）传递

```
mongodb_connect 执行 → 返回 provider_updates.mongodb_conn = {conn_string, db}
                        宿主把该配置编入执行上下文
mongodb_find 执行   → 宿主从上下文取出 provider 配置 → 发进 params.providers
                        插件进程内部用该配置找/建自己的连接池
mongodb_close 执行  → 插件返回 provider_updates.mongodb_conn = null
                        宿主清除上下文中的 provider
```

**原则**：连接池由插件进程自管（跨进程无法共享内存），宿主只负责传递连接配置和生命周期信号。

---

## 6. 报告体系（插件报告如何体现）

**核心结论：插件节点的报告仍然走宿主现有的 `NodeExecutionReport` 扁平结构**，内置/插件统一处理。宿主在构建报告时把插件的 `result` 字段映射进去。

### 6.1 基础映射

| 插件 result 字段 | 宿主 NodeExecutionReport 字段 | 说明 |
|---|---|---|
| `success` / `next_ports` | `status` / `used_port_id` | 宿主在 SidecarPlugin 包装层处理 |
| `output_data` | `output_data` | 直接存入 |
| `report_data` | `plugin_data`（新增字段） | 插件独有的详细报告，不污染 output_data |
| `errors[]` | `error` | 转成 `ErrorDetail` |
| `variable_updates` | `variable_changes` | 宿主 merge 后 diff 生成 |
| `metrics` | `plugin_data.metrics` | 并入报告数据 |

### 6.2 报告结构扩展

```rust
// report.rs — NodeExecutionReport 新增
pub struct NodeExecutionReport {
    // ...现有字段...
    /// 插件节点独有的详细报告数据（插件返回的 report_data）
    /// 内置节点为 None，序列化时跳过
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_data: Option<serde_json::Value>,
}
```

### 6.3 流式事件聚合

复用现有 `FlowExecutionReport.ws_streams` 的**收集钩子**模式（`context.ws_streams: Arc<RwLock<HashMap>>`），新增：

> 🔧 **v2.1 修正**：v2 原文"复用 ws_streams 模式"有误导。现有 `WsConnectionStream`（report.rs:247 + ws_stream.rs）存**全量消息**（上限 MAX 1MB/500 条每段/2000 条总计），**不是**文档所述的"汇总统计 + 前 N 条样例 + 完整流存文件"。后者是**全新存储代码**（文件落盘 + 汇总/采样聚合），实施时必须按新代码预算，而非适配器。

```rust
// report.rs — FlowExecutionReport 新增
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginStreamRecord {
    pub node_uuid: String,
    pub node_type: String,
    pub events: Vec<PluginStreamEvent>,   // 样例事件（前 N 条）
    pub stats: serde_json::Value,          // 插件自己汇总的统计
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginStreamEvent {
    pub type: String,                      // "message" | "connected" | ...
    pub timestamp: u64,
    pub data: serde_json::Value,
}

// FlowExecutionReport
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub plugin_streams: Vec<PluginStreamRecord>,
```

**内存控制**：流式节点可能收到海量消息。报告只存**汇总统计 + 前 N 条样例**；完整流另存文件，报告存路径。

### 6.4 异常场景报告

| 场景 | 报告体现 |
|---|---|
| **插件返回 errors[]** | `status=Failed`，`error=ErrorDetail`，`used_port_id="false"` |
| **插件崩溃/进程退出** | `ErrorDetail`（新增 `ErrorCode::PluginCrashed`）→ `status=Failed` |
| **执行超时** | `cancel` RPC → 插件无响应 → kill → `status=Cancelled` + 超时说明 |
| **用户取消** | `cancel` RPC → 插件返回 Cancelled → `status=Cancelled` |
| **插件未安装** | 现有 `"Unregistered node type"` 错误路径 |

### 6.5 前端 viewer

- `viewerRegistry` 检测到节点 `metadata.frontend` → 渲染 PluginIframe（复用 iframe 桥接）
- 宿主把 `node_report`（含 `plugin_data` + `output_data` + logs）postMessage 进 iframe
- 插件自己渲染专属报告视图

### 6.6 安全与大小控制

- `report_data` 大小设上限（建议 1MB，可配置），防插件塞爆报告存储
- `plugin_data` 建议带 `{plugin_name, plugin_version}` 便于追溯

---

## 7. 性能设计（压测场景专项）

> 压测模型：N 个虚拟用户 = N 个 tokio task 并发跑 `execute_flow`，每个插件节点 = 一次跨进程 RPC。

### 7.1 单次 RPC 开销量级

| 环节 | 耗时 |
|---|---|
| 宿主侧 JSON 序列化（serde_json） | ~2-5μs |
| 管道写入 syscall | ~1-2μs |
| 插件侧读取 + 反序列化 | ~3-6μs |
| 插件内部调度 + 业务执行 | 真正的成本在这 |
| 插件侧序列化 + 写回 | ~3-6μs |
| 宿主读取 + 反序列化 | ~3-6μs |
| **空转 RPC 合计** | **~15-30μs** |

对比压测对象（真实网络操作）：

| 压测目标 | 单次耗时 | IPC 占比 |
|---|---|---|
| HTTP 请求（远程） | 5-100ms | 0.03% |
| HTTP 请求（本机 echo） | 0.5-5ms | 0.5% |
| MQTT publish | 1-10ms | 0.2% |
| DB query | 0.5-50ms | 0.05% |

**结论：压测的是"插件对目标服务的网络操作"，15-30μs 的 IPC 开销相对网络延迟可忽略。**

### 7.2 真正的坑：单管道串行化

```
错误设计（插件进程串行处理）:
  loop { 读取一行请求 → 执行 → 写回一行响应 }   ← 一次只能处理一个！
  6000 RPC/s 会排队，延迟 = N × 单次耗时

正确设计（异步流水线）:
  宿主：异步发送 + id 关联 + 并发等待
  插件：读到的每个请求 spawn 一个 task，响应完成后写回（顺序无关）
```

### 7.3 宿主侧异步流水线

> 🔧 **v2.1 修正（对抗评审确认：v2 以下代码示例存在 3 处致命缺陷，禁止照抄）**：
> 1. **写者竞争**：v2 示例 `writer.write_all` 直接在 `call()` 内执行，若多个并发调用共享 `BufWriter` 会交错写坏帧。**必须采用"单写者任务"**：一个任务独占 `ChildStdin`，调用方把 `(id, bytes)` 推入 mpsc，写者任务串行原子写整帧（互斥锁**不足**——取消可能仍在锁内丢半帧）。
> 2. **reader 遇 JSON-RPC error 响应会崩溃**：v2 示例 `from_str::<Response{result}>` 无法解析 `{"id":N,"error":{...}}`，一旦出现 error 响应（协议正常路径）reader 任务 `?` 退出 → 所有并发 RPC 挂死。**reader 必须解析 result|error|notification 判别联合体**，error → 向对应 pending oneshot 发 `Err(PluginError)`，notification（id=null）→ 先 demux 到处理器再查 pending，**坏行跳过+计数（连续 N 条后杀进程），绝不退出**。
> 3. **pending 永不排空**：v2 无 reader 退出清理。**reader 退出时须清空 pending 表**，所有等待者收到 `Err(PluginCrashed)`；区分干净 EOF（shutdown 后 pending 空）与崩溃（非零退出/pending 非空）。另需**每请求超时**（`select!` oneshot vs sleep）——这是覆盖"卡死但存活"插件的唯一机制（无崩溃、无 EOF 时只有超时能兜底）。

```rust
// 正确设计（v2.1）—— 单写者任务 + reader 判别联合体 + 超时 + 退出排空
struct PluginConnection {
    // 写者任务独占 ChildStdin；调用方经 mpsc 推 (id, bytes)
    writer_tx: tokio::sync::mpsc::Sender<(u64, Vec<u8>)>,
    // reader 任务独占 ChildStdout，负责 demux 与 pending 分发
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, PluginError>>>>>,
    next_id: AtomicU64,
}

impl PluginConnection {
    async fn call(&self, method: &str, params: Value, timeout: Duration) -> Result<Value, PluginError> {
        let id = self.next_id.fetch_add(1);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, PendingRequest { tx });

        let msg = serde_json::json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        self.writer_tx.send((id, format!("{}\n", msg).into_bytes())).await
            .map_err(|_| PluginError::ChannelClosed)?;

        // 每请求超时：覆盖"卡死但存活"的插件（无崩溃、无 EOF）
        tokio::time::timeout(timeout, rx)
            .await
            .map_err(|_| { self.pending.lock().unwrap().remove(&id); PluginError::Timeout })?
            .map_err(|_| PluginError::ChannelClosed)?
    }

    // 后台 reader 任务：解析 result|error|notification 联合体；退出时排空 pending
    fn spawn_reader(&self, mut reader: BufReader<ChildStdout>) {
        tokio::spawn(async move {
            loop {
                // 读行（带 max-line 上限，不做无界 read_line）→ 解析判别联合体
                let resp: InboundMessage = match parse_frame(&mut reader).await {
                    Ok(m) => m,
                    Err(_) => { /* 坏行：跳过 + 计数，连续 N 条后 kill，绝不退出 */ continue; }
                };
                match resp {
                    InboundMessage::Response { id, result, error } => {
                        if let Some(p) = self.pending.lock().unwrap().remove(&id) {
                            let _ = p.send(result.map_err(|e| PluginError::Rpc(e)));
                        }
                    }
                    InboundMessage::Notification { method, params } => {
                        self.notify_handlers(method, params).await; // 事件/日志先于此分发
                    }
                }
            }
            // reader 退出：排空 pending，所有等待者收到 PluginCrashed
            let waiters: Vec<_> = self.pending.lock().unwrap().drain().map(|(_, p)| p).collect();
            for p in waiters { let _ = p.send(Err(PluginError::PluginCrashed)); }
        });
    }
}
```

> 🔧 **补充（v2.1）**：host/plugin 两侧都必须遵守同一套传输正确性要求：单写者任务、reader 判别联合体、每请求超时、退出排空、事件携带 `execution_id`+`node_instance_id`、LF 分帧 + 容忍尾随 `\r` + UTF-8 校验 + 2MB 行上限、批次数组禁止、未知 id 忽略、通知不回响应、stderr 从 spawn 即排空（64KB 满 → 全局停顿）、Windows Job Object `KILL_ON_JOB_CLOSE`（宿主死亡清插件及孙进程）、句柄继承卫生（孙进程不得继承 stdout，否则管道永不 EOF）。详见 `docs/plugin-protocol.md`（规范文档，实施时产出）。

### 7.4 插件侧异步分发

> 🔧 **v2.1 修正**：v2 以下示例同样存在**写者竞争**（每请求 spawn 的 task 各自 `writer.clone().write_all+flush` 于同一 `BufWriter` → 帧交错损坏）。插件侧必须与宿主侧一致采用**单写者任务**（见 §7.3 修正块）。`tokio::io::BufWriter::clone` 复制的是缓冲状态而非句柄，不能作为并发写共享。

```rust
// 正确设计（v2.1）—— 插件侧同样单写者任务
// 读到的每个请求 spawn 一个 task（天然并发），但写回统一走 writer 任务：
let (writer_tx, mut writer_rx) = mpsc::channel::<(u64, Vec<u8>)>(1024);
// writer 任务独占 stdout：
tokio::spawn(async move {
    let mut stdout = BufWriter::new(stdout_handle);
    while let Some((id, bytes)) = writer_rx.recv().await {
        stdout.write_all(&bytes).await?;  // 整帧原子写
        stdout.flush().await?;
    }
});
// 请求处理：
loop {
    let line = stdin_reader.read_line().await?;
    let req: Request = serde_json::from_str(&line)?;
    let registry = registry.clone();
    let writer_tx = writer_tx.clone();
    tokio::spawn(async move {
        let result = registry.dispatch(&req.method, &req.params).await;
        let resp = Response { id: req.id, result };
        writer_tx.send((req.id, format!("{}\n", serde_json::to_string(&resp)?).into_bytes())).await?;
    });
}
```

> 🔧 **补充（v2.1）**：SDK（`mpe-plugin-sdk`）的 `mpe_plugin_main!` 宏移除 v2 §10.1 中的 `frontend_port: 9101` 字段——与已锁定的 A4（仅 inline HTML 前端）矛盾。宏保留（声明式，约 15-30 行），仅删 HTTP server 字段。

### 7.5 流水线吞吐对比

| 处理模型 | 1000 空转 RPC 耗时 | 6000 RPC/s 时延迟 |
|---|---|---|
| 串行（读→做→写） | ~25ms | 锁死在 ~40k RPC/s 上限，排队严重 |
| **异步流水线** | **~1-3ms** | 插件内部并发处理，单次延迟≈业务延迟 |

> 🔧 **v2.1 修正（H1）**：v2 §7.1 的 15-30μs/空转 RPC 与"5-10 万 RPC/s"**偏乐观 2-5 倍**。`tokio::process` 的 stdio 在 Windows 上经阻塞线程池（匿名管道无法非阻塞），每次读写都有跨线程交接 → 现实单次空转 RPC 约 **30-80μs**，吞吐上限约 **12k-33k RPC/s**。但文档的核心结论**仍然成立**：对网络 I/O 型插件（单次 ≥5ms 的操作），IPC 开销占比 <1-2%，可忽略。实施时须先建基准（echo mock），以实测数据为准，且不得因修正后的数字推翻已锁定的传输决策。

### 7.6 压测专项设计

- **连接池复用**：插件进程天然是连接池驻留地，压测时所有虚拟用户共享插件进程内连接池——模拟真实用户池，正是压测想要的。
- **背压控制**：PluginManager 按插件 `capabilities.max_concurrent_requests` 做背压，超过上限排队而非无脑灌管道。
- **极端场景**：若插件本身非网络 I/O bound（罕见），压测配置可支持"每虚拟用户一个插件进程"，但默认单进程复用。

### 7.7 传输层可升级路径

传输层抽象成 trait，将来换实现不动业务代码：

| 传输 | 吞吐 | 代价 |
|---|---|---|
| stdin/stdout + 流水线 | ~5-10万 RPC/s | 零（推荐先做这个） |
| Unix socket / 命名管道 | ~10-20万 RPC/s | Windows 支持有限 |
| TCP loopback（多连接） | ~20万+ RPC/s | 端口管理、连接池、安全 |
| 共享内存 | ~100万+ RPC/s | 跨语言地狱、Windows 兼容差、放弃 |

---

## 8. 插件生命周期

```
宿主启动 → 扫描 get_data_dir()/plugins/*/plugin.json   (🔧 v2.1: 见 §8.3 修正，非 ~/.mpe)
         → 读取清单（执行路径、环境变量、权限）
         → 启动子进程（stdin/stdout pipe）
         → describe → 获取节点元数据 + 前端资源
         → 注册进 NodeRegistry（与内置节点混合）
         → 前端 get_registered_node_types 返回合并列表
         → 用户执行 → execute / execute_stream
         → 宿主退出 → shutdown → 超时则 kill
```

### 8.1 进程生命周期策略（已定 A2）

- **连接型/查询型节点**（如 mongodb_find）：**按需启动 + 空闲回收**。插件进程在首次用到其节点时拉起，空闲超时（建议 60s，可配置）后回收。
- **Streaming 类节点**（SSE 监听 / Kafka 消费 / MQTT 订阅）：**进程常驻**。插件声明 `capabilities.streaming=true` 后保持运行，避免流中断。
- 单一插件进程服务同一插件的所有节点类型（不按节点分进程）。

### 8.2 崩溃恢复策略（已定 A3）

- 插件进程崩溃 → 宿主捕获退出信号 → **自动重启一次**并重新 `describe` 注册。
- 重启后若再次崩溃（执行中）→ **按 `on_error` 策略路由**（RouteToFalse / Ignore / AbortFlow），不再自动重启。
- 崩溃事件在报告中体现为 `ErrorCode::PluginCrashed`（见 6.4）。

### 8.3 分发形式（已定 A5）

- **首期**：用户手动下载插件压缩包，解压到 `get_data_dir()/plugins/<plugin-name>/`（🔧 v2.1：非 `~/.mpe/plugins/`，见 plugin.json 修正块），重启应用或热加载生效。
- **二期**：应用内插件市场，自动下载/更新/签名验签。

### plugin.json 清单

```json
{
  "name": "MongoDB 插件",
  "version": "1.0.0",
  "min_host_version": "0.1.0",
  "entry": {
    "command": "mongodb-plugin",
    "args": []
  },
  "env": { "CACHE_DIR": "$DATA_DIR/cache/mongodb" },
  "permissions": ["network"],
  "capabilities": ["connection_pooling", "streaming"]
}
```

> 🔧 **v2.1 修正（H6/F14）**：
> 1. **`entry` 必须支持 command+args**（如 `{"command":"python","args":["plugin.py"]}`）——解释型插件（Python/Node）无法用单一可执行路径表达，且非 Rust 插件本来就是源码（F14），无解释器则无法启动。解析需限制在插件目录内（禁止绝对路径/`..`）。
> 2. **插件目录约定**：v2 通篇使用 `~/.mpe/plugins/`，但宿主实际数据目录是 `get_data_dir()`（storage.rs:66-70，Windows 为 `%APPDATA%/multi-protocol-flow-executor`）。**统一使用 `get_data_dir()/plugins`**（可用环境变量 `MPE_PLUGIN_DIR` 覆盖），废弃 `~/.mpe` 约定。

---

## 9. 前端 Iframe 桥接

### 9.1 渲染路径

```
ConfigPanel 检测到节点 metadata.frontend !== undefined
  → 渲染 <iframe sandbox="allow-scripts">            (🔧 v2.1: 仅 allow-scripts，禁 allow-same-origin，见 §9.3)
  → src 来自：blob URL（inline HTML；首期仅此一种，url/iframe 二期）
      🔧 v2.1: blob 与当前 CSP 冲突，加载机制另见 §9.3 / 第 13 章 C2
  → usePluginIframe hook 管理双向 postMessage
```

### 9.2 postMessage 协议

> 主题（明暗模式）如何跨 iframe 同步、以及插件 CSS 的改造规范，见
> [plugin-frontend-theme.md](./plugin-frontend-theme.md)。

**宿主 → iframe：**

| type | 场景 |
|---|---|
| `init` | 发送初始 config + 变量快照 + host_api 能力声明 + nodeType + locale + theme |
| `configUpdated` | 宿主侧配置变更时推送 |
| `actionResult` | 响应 iframe 的 requestAction（带 requestId） |
| `themeChanged` | 宿主 resolved 主题（light/dark）变化时推送 |

**iframe → 宿主：**

| type | 场景 |
|---|---|
| `configChanged` | 用户改了表单 → 通知宿主保存 |
| `requestAction` | 请求宿主能力（testConnection / openFileDialog / 自定义 action 透传） |
| `resize` | iframe 自适应高度 |
| `error` | 面板内部错误上报 |
| `ready` | 初始化完成通知 |

### 9.3 安全

> 🔧 **v2.1 修正（C1，对抗评审确认的 RCE 级漏洞）**：v2 原文 `sandbox="allow-scripts allow-same-origin"` **不安全**。宿主 `tauri.conf.json` 设 `withGlobalTauri: true`，`window.__TAURI__` 全局注入；blob URL 继承创建者源（app 源）；`allow-same-origin` 使 iframe 源非不透明 → 插件 inline HTML 与宿主**同源**，可直读 `window.parent.__TAURI__` 触发任意 Tauri IPC（可进而调用 `execute_flow` + script 节点 = RCE 级）。v2"iframe 拿不到 window.__TAURI__"的说法**错误**。

```
// 🔧 v2.1 修正后的安全约束：
sandbox="allow-scripts"                          // 仅 allow-scripts；禁止 allow-same-origin（不透明源）
// 不透明源阻断对父 DOM / window.__TAURI__ 的访问；postMessage 跨源依然可用
宿主 postMessage 处理器必须校验：event.origin 合法 && event.source === iframe.contentWindow
// 防伪造：任何同源 frame 或宿主 XSS 都可伪装插件读取其 config

// 🔧 v2.1 新增——CSP 与加载机制（OD3，P1 设计任务，见第 13 章 C2）：
// 宿主 CSP 为 `default-src 'self'; script-src 'self' ...`（无 frame-src、无 unsafe-inline）。
// (a) blob: iframe 被 `default-src 'self'` 阻断；且 (b) blob 文档继承宿主 CSP，
//     `script-src 'self'`（无 unsafe-inline）会阻断插件 HTML 内的内联 <script>——
//     插件自带 CSP meta 无法覆盖继承的 CSP（CSP 取交集）。
// 结论：blob 方案与当前 CSP 冲突。首选 `mpe-plugin://` 自定义 Tauri 协议
// （自有源，规避 CSP 继承）；备选 blob + `frame-src blob:; script-src blob:`。
// CSP + sandbox + 加载机制 = 一个设计任务，一个 PR 落地。
```

**威胁模型声明（v2.1 新增）**：插件**二进制**是受信实体（B3 向它发送凭据、D2 首期不强制权限），iframe 沙箱保护的是**宿主 UI 免受插件 HTML 影响**，而非宿主免受插件二进制影响。沙箱不是二进制隔离边界——实施时不得把沙箱当作安全边界来宣传。

**补充约束（v2.1）**：
- 插件 HTML 无自带 CSP 的豁免——宿主 CSP 是唯一生效的 CSP。
- inline HTML 走 blob URL 时不暴露源码路径（保留 v2 约束），但需配合 `URL.revokeObjectURL` 在面板关闭时释放。
- `requestAction` 宿主能力白名单保持最小（`testConnection` 是本机 SSRF 原语，逐个审计）。
- 插件 HTML 中的任何外链脚本（如分析脚本）都能窃取 init 载荷中的变量快照（含 B3 敏感信息）——文档明确"插件 HTML 不可信"，不承诺保密性。

### 9.4 前端注册表改造

```
configPanelRegistry.ts：
  内置节点 → 现有硬编码 loader（不动）
  插件节点 → 走 PluginIframe 组件（通过 metadata.frontend 判断）
```

---

## 10. 插件 SDK

### 10.0 语言策略（已定 A1）

- **语言放开**：协议是语言无关的 JSON-RPC，任何语言（Python/Node/Go/C#…）只要实现 stdin/stdout 协议即可成为插件。
- **官方推荐 Rust**：提供完整 SDK（`mpe-plugin-sdk`）最优体验；Python/Node 提供轻量模板（不维护完整 SDK）。
- 文档约定：**使用 Rust SDK 的插件获得一等公民支持**（调试工具、schema 校验、测试夹具）。

### 10.1 目标体验（Rust 推荐路径）

```rust
use mpe_plugin_sdk::prelude::*;

struct MongoFindNode;

impl PluginNode for MongoFindNode {
    fn describe(&self) -> NodeDescription {
        NodeDescription::new("mongodb_find", "MongoDB 查询")
            .category("database").icon("search").color("#10B981")
            .port(Port::output("out")).port(Port::error("error"))
            .config_schema(json_schema!({...}))
    }

    async fn execute(&self, ctx: ExecuteContext) -> NodeResult {
        let conn = ctx.get_provider("mongodb_conn")?;      // 连接配置
        let pool = ctx.get_or_init_pool("mongodb", || {...}).await?;  // 池由 SDK 管
        ctx.log(LogLevel::Info, "查询中".to_string());
        Ok(NodeOutput::new()
            .port("out")
            .data(json!({...}))
            .report_data(json!({                       // 报告数据（新增）
                "query": "...", "execution_stats": {...}
            }))
            .update_variable("runtime.total", 42))
    }
}

mpe_plugin_main! {           // 一行宏：异步事件循环 + JSON-RPC（🔧 v2.1: 移除 frontend_port，见 §7.4 修正）
    nodes: [MongoConnectNode, MongoFindNode, MongoCloseNode],
    plugin_name: "MongoDB 插件",
    plugin_version: "1.0.0",
    min_host_version: "0.2.0",
}
```

### 10.2 SDK 核心类型

| 类型 | 职责 |
|---|---|
| `PluginNode` trait | 插件作者唯一需要实现的接口（describe / validate / execute） |
| `ExecuteContext` | 已替换的 config、providers、log_tx、连接池注册表、变量写回通道 |
| `NodeOutput` / `NodeResult` | 输出端口 + 数据 + report_data + variable_updates + provider_updates |
| `ConnectionPool` | 通用连接池管理（key → `Arc<dyn Any>`） |
| `mpe_plugin_main!` 宏 | 生成 main：tokio runtime + 异步流水线事件循环 + 可选前端 HTTP server |
| `PluginDispatcher` | stdin/stdout 异步读取、每请求 spawn task、id 关联响应 |

### 10.3 cargo generate 模板

```
cargo generate gh:mpe/plugin-template --name mongodb-plugin
→ src/main.rs（用户填逻辑）+ frontend/（可选面板资源）+ plugin.json
→ cargo build --release → mongodb-plugin.exe
→ 丢进 ~/.mpe/plugins/ 即装
```

### 10.4 非 Rust 插件（轻量模板）

Python / Node 模板只提供协议层封装（读 stdin / 写 stdout / id 关联），无 `PluginNode` trait 与连接池管理：

```
python-template/
├── plugin.py        # 实现 JSON-RPC 循环 + describe/execute 分发
├── manifest.json    # plugin.json 清单
└── frontend/        # inline HTML 面板资源
```

```python
# plugin.py（示意）
from mpe_plugin_sdk import PluginRuntime

@PluginRuntime.register("mongodb_find")
def mongodb_find(ctx):
    conn = ctx.get_provider("mongodb_conn")
    ...
    return {"success": True, "next_ports": ["out"], "output_data": {...}}

if __name__ == "__main__":
    PluginRuntime.run()
```

> **已定（A1）**：非 Rust 插件不提供一等支持（无连接池 SDK、无调试工具），但协议保证可运行。

---

## 11. 落地路线

| 阶段 | 内容 | 估算 |
|---|---|---|
| **P0** | JSON-RPC 协议定义 + PluginManager（进程启动/异步流水线/心跳/超时/背压） | 2-3 周 |
| **P0** | SidecarPlugin 包装（伪装 NodePlugin 注册进 registry） | 1 周 |
| **P1** | 前端 iframe 桥接（usePluginIframe + postMessage，首期仅 inline） | 1 周 |
| **P1** | 官方插件 SDK 模板（Rust 完整 SDK + Python/Node 轻量模板） | 1-2 周 |
| **P1** | 报告体系扩展（plugin_data / plugin_streams / ErrorCode::PluginCrashed） | 1 周 |
| **P2** | 插件市场（registry、版本、签名验签） | 2-3 周 |
| **P2** | 热加载（运行时安装/卸载不重启） | 1 周 |

**MVP 约 6-9 周**：核心通信层 3-4 周 + 前端桥接 1 周 + SDK 模板 1-2 周 + 报告扩展 1 周。

---

## 12. 决策记录（全部已拍板）

> 2026-08-04 拍板。A/B/C/D 四组全部落定，进入实施阶段。

### A 组：核心架构

| # | 问题 | 决策 | 说明 |
|---|---|---|---|
| A1 | 插件语言 | **语言放开，推荐 Rust** | 非 Rust 插件可运行但不享一等支持（见 10.4） |
| A2 | 进程生命周期 | **按需启动+空闲回收；streaming 类常驻** | 见 8.1 |
| A3 | 崩溃恢复 | **重启一次，仍失败则路由 on_error** | 见 8.2 |
| A4 | 前端资源首期范围 | **仅 inline HTML** | url/iframe 二期，协议字段预留 |
| A5 | 分发形式 | **手动解压目录** | 市场二期（见 8.3） |

### B 组：变量与数据边界

| # | 问题 | 决策 | 说明 |
|---|---|---|---|
| B1 | 变量快照范围 | **执行 A（仅引用）+ 面板 B（全量只读）** | 见 5.3 |
| B2 | 嵌套 `{{}}` | **禁止，单趟替换** | validator 拦截（见 5.4） |
| B3 | 敏感信息 | **随 config 发给插件** | 宿主加密存储，插件自管内存凭据（见 5.4） |

### C 组：报告体系（按建议采纳）

| # | 问题 | 决策 | 说明 |
|---|---|---|---|
| C1 | plugin_data 位置 | **NodeExecutionReport 可选字段** | 保持扁平结构（见 6.2） |
| C2 | 流式报告存储 | **汇总 + 前 N 条样例 + 完整流存文件** | 报告存路径（见 6.3） |
| C3 | report_data 上限 | **1MB（可配置）** | 防插件塞爆存储（见 6.6） |
| C4 | 崩溃报错区分 | **新增 ErrorCode::PluginCrashed** | 前端可区分（见 6.4） |
| C5 | 报告插件版本 | **plugin_data 带 {name, version}** | 追溯用（见 6.6） |

### D 组：协议与兼容性（按建议采纳）

| # | 问题 | 决策 | 说明 |
|---|---|---|---|
| D1 | 协议版本策略 | **min_host_version 门槛制** | 不兼容拒绝加载 |
| D2 | plugin.json 权限声明 | **首期声明不强制，市场期强制** | 字段预留 |
| D3 | 压测背压策略 | **超过上限排队** | 不拒绝请求（见 7.6） |

---

## 已确定的设计决策（架构级，不再变更）

- ✅ 传输层：JSON-RPC 2.0 over stdin/stdout（异步流水线）
- ✅ 插件模型：Sidecar 子进程（拒绝 Rust .so/WASM）
- ✅ 变量主链路：宿主统一替换 → 插件收最终值 → variable_updates 回写（🔧 v2.1: key 必须扁平顶层名，见 §4.4）
- ✅ 报告主链路：统一走 NodeExecutionReport 扁平结构，report_data → plugin_data
- ✅ 前端桥接：iframe + postMessage，插件自带面板/viewer（🔧 v2.1: sandbox 仅 allow-scripts；加载机制与 CSP 合并设计，见 §9.3）
- ✅ 连接池：插件进程自管，宿主只传配置
- ✅ 决策明细：见第 12 章（A/B/C/D 组全部已拍板）+ 第 13 章（v2.1 修正记录）

---

## 13. v2.1 修正记录（2026-08-04，对抗式评审产出）

> 本节基于 5 名评审成员（架构/传输/集成/范围/质疑）3 轮交叉攻击的结论，全部经**代码级核实**。实施以本节为准。正文对应处已就地标注 🔧。

### A 组：致命修正（照 v2 原文实施会出错）

| # | 原 v2 内容 | 问题 | v2.1 修正 |
|---|---|---|---|
| A1 | §7.3/7.4 代码示例：`call()` 内直接 `write_all`；每请求 `writer.clone()` 并发写 | 写者竞争：帧交错损坏 | **单写者任务**（mpsc 推 `(id, bytes)`，写者独占管道原子写整帧）。互斥锁不足——取消仍可能丢半帧。宿主与插件两侧一致 |
| A2 | §7.3 reader 只解析 `{id, result}` | JSON-RPC `error` 响应（正常路径）使 `from_str` 失败 → reader 退出 → 全部并发 RPC 挂死 | reader 解析 **result\|error\|notification 判别联合体**；error → pending 发 `Err(PluginError)`；notification 先 demux；坏行跳过+计数（连续 N 条杀进程），**绝不退出** |
| A3 | §7.3 pending 表无清理 | reader 退出/崩溃 → 所有在途 `call()` 永久挂起 | reader 退出时**排空 pending**，等待者收 `Err(PluginCrashed)`；区分干净 EOF 与崩溃（`child.wait()/try_wait()`）；另加**每请求超时**（唯一覆盖"卡死但存活"插件的机制） |
| A4 | §9.3 `sandbox="allow-scripts allow-same-origin"`；"iframe 拿不到 window.__TAURI__" | `withGlobalTauri:true` + blob 继承源 + `allow-same-origin` = 插件 HTML 与宿主同源 → 可达 `__TAURI__` → RCE 级 | **sandbox 仅 `allow-scripts`**（不透明源）；postMessage 校验 `event.origin` + `event.source`；威胁模型声明：插件二进制是受信实体 |
| A5 | §4.4/§5.4 `variable_updates: {"runtime.total":42}` | 扁平 `VariableStorage` + 点分路径解析 → 点分 key 永不命中，占位符静默保留 | **契约：variable_updates 的 key 必须扁平顶层名**；点分 key 告警+跳过+记报告；解析回归测试先行（schema 冻结前） |

### B 组：高影响修正

| # | 原 v2 内容 | 问题 | v2.1 修正 |
|---|---|---|---|
| B1 | §4.5 事件/日志无关联字段 | 压测多虚拟用户共享插件进程时无法路由事件 | **每个 `event`/`log` 携带 `execution_id` + `node_instance_id`**；`cancel` 针对该二元组；执行级关联注册表 |
| B2 | §2.1/§7.1 性能 15-30μs / 5-10 万 RPC/s | Windows 阻塞线程池使现实值约 30-80μs / 12k-33k RPC/s（偏乐观 2-5 倍） | 修正数字；**核心结论不变**（网络 I/O 型插件 IPC <1-2%）；实施时建基准实测 |
| B3 | §6.3 "复用 ws_streams 模式" | 现有 ws_streams 存全量消息（有上限），非"汇总+样例+文件" | 明确 plugin_streams 的"汇总+前 N 样例+文件"是**全新存储代码**；收集钩子可复用 |
| B4 | §8.3/通篇 `~/.mpe/plugins/` | 宿主实际数据目录为 `get_data_dir()`（`multi-protocol-flow-executor`） | **统一 `get_data_dir()/plugins`**；`MPE_PLUGIN_DIR` 可覆盖 |
| B5 | §8.3 plugin.json `entry` 为单一路径 | 解释型插件（Python/Node）无法表达解释器调用；非 Rust 插件本就是源码 | `entry` 支持 **command+args**；解析限制在插件目录内 |
| B6 | §10.1 宏含 `frontend_port: 9101` | 与已锁定 A4（仅 inline HTML）矛盾 | **宏保留、删 `frontend_port`** |

### C 组：事实澄清（评审中互相推翻的说法，实施时以此为准）

| # | 结论 | 核实依据 |
|---|---|---|
| C1 | **`mpe run-node` 存在**（评审中 integrator 曾称"不存在"，被推翻） | `mpe-cli/src/cli.rs:132,426-430` → `run_node.rs:54` → `execute_single_node`；但受 `SUPPORTED_SINGLE_NODE_TYPES` 白名单（executor.rs:101-119）与 `parse_node_type`（flow_edit.rs:62）门控，**插件类型被挡**。单节点插件连通性测试排 P1 |
| C2 | **`src-tauri/src/cli/*` 是死代码** | `main.rs:16` 走 `app_lib::cli::run_cli()`，`lib.rs:13` 为 `pub use mpe_cli::cli;` → CLI 实际由 mpe-cli crate 提供；`src-tauri/src/cli/` 9 个文件未被引用 |
| C3 | **tokio `process` feature 已就绪**，无需"启用" | `workspace-hack/Cargo.toml:88,162` 已含 `process`（feature 统一）。新 crate 显式声明即可，非 blocker |
| C4 | 宿主已有"伪装"基础：`NodeData::Plugin(Value)` catch-all（node_config.rs:3796,4108,4216）+ 回环测试 + 字符串型 registry 分发（executor.rs:1182） | 配置解析零改造；**注册表注入**（`with_registry`）才是真缺口：`FlowExecutor::new()` 硬编码 `default_registry()`（executor.rs:154），tauri_commands.rs:3669/3679/3691/3760 每次新建 |
| C5 | `NodeType` serde 为 `lowercase`（node.rs:7），非 AGENTS.md 所述的 `snake_case` | 功能等价，文档勘误 |

### D 组：实施边界（锁定决策不变，仅实施方式明确）

1. **注册表注入 = 最大关键路径**：`with_registry(Arc<NodeRegistry>)` 构建器 + 共享 `Arc<NodeRegistry>`（GUI: `lib.rs` setup `app.manage`；CLI: `run_cli()`）+ 贯穿 10+ 构造点（tauri_commands 6 处 + commands/execute_flow.rs + dataset_executor 2 处 + debug + stress + mpe-cli run.rs）。压测每虚拟用户新建 `FlowExecutor`（stress_test/executor.rs:465）→ **管理器必须共享**，否则每个 VU 各拉起插件进程，§7.6"共享连接池"不成立。
2. **崩溃重启策略归属**：重启一次+限速在 **PluginManager**（机制），`SidecarPlugin::execute` 只是接口（单次调用、单个最终 `Err(PluginCrashed)`），执行器 on_error 路由（RouteToFalse/Ignore/AbortFlow）不感知。
3. **超时归属**：`NodeData::execution_timeout_ms()` 对 Plugin 返回 `None`（node_config.rs:4229）→ 无 tokio 兜底 = 无限挂起。**wrapper 内自持超时 + cancel + kill**；P0 兑现 `cancel_receiver`（select! → 丢弃 pending → Err(Cancelled)），无需 cancel RPC。
4. **生命周期**：驻留与否在 describe 时按 `capabilities.streaming` 决定（streaming=true → 常驻，即使零活跃流）；否则按需启动 + 60s 空闲回收（in-flight==0 时才回收）；扫描（manifest 解析）同步且不 spawn，扫描失败=禁用插件，**绝不在扫描期重启**；describe 崩溃 N 次（默认 3）后隔离。
5. **type_id 冲突**：`NodeRegistry.register` 裸覆盖（registry.rs:29-32，测试断言新胜）→ 内置 id 保留（不可被插件遮蔽）+ 插件间冲突先到先得+告警 + 遮蔽回归测试。
6. **背压（D3）细化**：有界 in-flight 信号量 + 有界队列；溢出=调用方**等待**（阻塞），**绝不拒绝**；慢消费者信号（日志+计数）暴露给压测指标。
7. **大小上限**：除 report_data 1MB 外，`output_data`、流样例事件、完整流文件（建议 50MB 轮转）均需上限。
8. **前端静默损坏**：`createDefaultNodeData`（canvas/utils.ts:369-371）对未知类型回退 `{type:'entry'}` → 插件节点被静默改成 entry。**必须修复**（回退改为 `{type, ...(cachedConfig ?? {})}`，缓存已有 `getCachedDefaultConfig` @178）。
9. **TS 联合类型**：`NodeType`/`NodeData`（src/types/node.ts:42,2366）为封闭联合 → store 边界放宽为 `NodeType | (string & {})`，保留已知类型收窄。
10. **report 爆炸半径**：`NodeExecutionReport` 加 `plugin_data` 触及 ~7 个构造点 + 5 个测试文件（report_structure/report_storage/ws_debug_flow/node_report_fields/flow_report_fields）；`FlowExecutionReport` 加 `plugin_streams` 2 个构造点。批量改，一个 PR，编译错误即清单。

### E 组：估算（评审修正后）

| 阶段 | 内容 | 估算 |
|---|---|---|
| **P0 核心** | M0 脚手架 + M1 传输核心（2-3 周）+ M2a 发现 + M2b 生命周期 + M2c 注册 + M3 包装/注入/报告扩展 + M4 验证器+前端 | **6-7 周** |
| **P0 附加** | M5 SDK（宏去 frontend_port）+ Python/Node 模板 + 一致性套件 + 基准 + e2e | +1.5-2 周 |
| **P1** | execute_stream / cancel RPC / heartbeat / iframe 桥接（CSP+加载合一）/ 连接池 SDK / 市场 / 热加载 / 流式 viewer | 另计 |

> 原 v2 的"MVP 约 6-9 周"在流式严格排 P1、且以本修正记录为准的前提下仍成立（偏紧）。
