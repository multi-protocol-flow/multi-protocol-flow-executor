# 后端代码审查修复计划

> 基于 2026-07-05 后端深度审查汇总，20 crates, ~141K LOC, branch `dev`

## 发现总览

| 严重度 | 数量 | 类别 |
|--------|------|------|
| 🔴 Bug | 2 | 功能性错误 |
| 🟡 健壮性 | 3 | 可能崩溃 |
| 🟠 维护性 | 7 | 代码债务 |
| ⚪ 建议 | 4 | 值得讨论 |
| ~~安全~~ | ~~7~~ | ~~已废弃（本地客户端无需考虑）~~ |

---

## Phase 1 — Bug Fixes（最短时间修复，逐个 1h 以内）

### P1-1: `fn.response.time` 始终返回 0

**文件**: `src-tauri/crates/flow-engine-script/src/script_executor.rs:447`

```rust
time: 0, // TODO: 从 output 提取实际时间
```

**现象**: 用户在脚本中调用 `fn.response.time`，期望获取前一个节点的执行耗时（毫秒），实际永远拿到 `0`。

**修复思路**:
- 执行引擎在 `executor.rs` 中已有 `duration_ms` 追踪（如 `executor.rs:440` 附近的 `node_duration`）
- 需要把这个值透传到脚本 API 的 `extract_response_data()` 方法中
- 修改 `ExtractResponseDataParams` 或类似结构，增加 `duration_ms` 字段
- `extract_response_data()` 中把该值赋值给返回 JSON 的 `time` 字段

**涉及改动量**: 小（2-3 个函数签名调整 + 1 行赋值）

---

### P1-2: `with_error_code(String)` 静默吞掉错误码拼写错误

**文件**: `src-tauri/crates/flow-engine-core/src/error.rs:67-73`

```rust
fn with_error_code(self, code: String) -> ErrorDetail {
    let error_code = serde_json::from_str(&format!("\"{}\"", code))
        .unwrap_or(ErrorCode::E_EXECUTION_ERROR);
    self.with_error_code_typed(error_code)
}
```

**现象**: `with_error_code("E_TYPO")` 不会编译报错，而是运行时静默降级为 `E_EXECUTION_ERROR`，掩盖了真正的错误意图。

**修复思路**:
- 方法已标记「backward compat，待移除」
- `git grep "\.with_error_code("` 找出所有调用点
- 全部替换为 `with_error_code_typed(ErrorCode::XXX)`
- 移除 `with_error_code(String)` 方法

**涉及改动量**: 小（纯机械替换，grep + replace）

---

## Phase 2 — 崩溃风险修复（预防性，每项 1h 以内）

### P2-1: 锁中毒导致全应用崩溃

**文件**: `src-tauri/src/tauri_commands.rs:77,83,90,94`

```rust
// GrpcCancelSessionManager 中每把锁都用 expect()
let mut guard = self.grpc_manager.lock().expect("lock should not fail");
```

**风险**: `std::sync::Mutex` 在持有线程 panic 后会中毒。如果 gRPC 执行中某处 panic，之后所有 `cancel_stream` / `channelz_info` / `clear` 操作都会触发 panic → 应用崩溃。

**修复思路**:
```rust
// 改前
let mut guard = self.grpc_manager.lock().expect("lock should not fail");

// 改后
let mut guard = self.grpc_manager.lock().map_err(|e| format!("lock poisoned: {}", e))?;
```

**同时检查**:
- `ExecutionSessionManager` 中也用了 `std::sync::Mutex`
- `DatasetExecutionSessionManager` 同上
- `cli/config.rs` 中的 `ENV_TEST_MUTEX.lock().unwrap()`

**涉及改动量**: 小（4-5 处替换）

---

### P2-2: `SystemTime::duration_since(UNIX_EPOCH).unwrap()` 集群

**文件**: `flow-engine-executor/src/executor.rs`（约 10 处）

**风险**: 系统时间异常（如 NTP 同步临时跳变到 1970 年之前）会导致 panic。虽然概率极低，但违背项目自身"无 unwrap 生产代码"规范。

**修复思路**:
- 添加一个工具函数：
```rust
fn unix_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
```
- 所有 `duration_since(UNIX_EPOCH).unwrap()` 替换为此函数
- 放在 `flow-engine-core/src/utils.rs` 中，按需引入

**涉及改动量**: 中（~10 处替换，需扫全项目）

---

### P2-3: `log_sender.rs` 中 `unwrap()` 风险

**文件**: `flow-engine-core/src/traits/log_sender.rs:282`

```rust
tx.send(event).unwrap();
```

**风险**: 如果通道接收端（前端）断开，发送可能失败。虽然 `mpsc::Sender::send()` 只会在接收端 drop 后才失败，但仍有理论风险。

**修复思路**: `tx.send(event).map_err(|e| ...)?` 或 `if tx.send(event).is_err() { break; }`

**涉及改动量**: 极小（1-2 行）

---

## Phase 3 — 代码债务清理（按性价比排序）

### P3-1: 拆分 `tauri_commands.rs`（4396 行）

**文件**: `src-tauri/src/tauri_commands.rs`

**现状**: 单文件包含约 72 个 `#[tauri::command]` 函数 + 5 个 SessionManager + 10+ 输入输出类型定义 + 测试。

**拆分方案**:
```
src-tauri/src/
├── tauri_commands.rs          # 保留公共 re-export + 少量通用命令
├── commands/
│   ├── mod.rs                 # pub use 各领域模块
│   ├── flow.rs                # create_flow, execute_flow, cancel_flow, validate_flow
│   ├── reports.rs             # save/load/list/delete flow report
│   ├── stress.rs              # stress test 相关
│   ├── debug.rs               # debug session 相关
│   ├── dataset.rs             # 数据集执行相关
│   ├── flow_files.rs          # .mpf 文件 CRUD
│   ├── folders.rs             # 文件夹管理
│   ├── grpc.rs                # gRPC 8 个命令（cfg-gated）
│   ├── email.rs               # SMTP/IMAP/OAuth2（cfg-gated）
│   ├── mqtt_kafka.rs          # MQTT + Kafka（cfg-gated）
│   └── misc.rs                # graphql:introspect, tcp_templates 等（test_redis 已随 redis 插件化移除）
```

**收益**: 合并冲突大幅减少，新命令可独立文件添加，Review diff 更聚焦。

**耗时**: 初步拆分约 1-2 天（纯机械移动，几乎零风险）

---

### P3-2: `executor.rs` 解耦

**文件**: `flow-engine-executor/src/executor.rs`（2361 行）

**问题**:
1. 条件节点路由硬编码字符串比较 `node.node_type == node_types::CONDITIONAL`（545/617 行）
2. 错误处理（RouteToFalse/Ignore/AbortFlow）与主执行循环交织
3. 报告构建逻辑散落在执行代码中

**拆分方案**:
```
flow-engine-executor/src/
├── executor.rs                 # 主执行循环（精简到核心逻辑）
├── executor_error_handler.rs   # on_error 策略分发
├── executor_report_builder.rs  # FlowExecutionReport 构建
```

**关键注意点**:
- 条件节点硬编码 → 应当移到 `ConditionalPlugin.execute()` 内部。插件需要一个 `FlowTermination` 或类似信号来告知执行器「此分支正常结束」
- 改动前先写回归测试

**耗时**: 中等（3-5 天），建议与 P3-1 分开安排

---

### P3-3: `Node::node_type` 从 `String` 改为 `NodeType`

**文件**: `flow-engine-core/src/node.rs:343`

```rust
pub struct Node {
    pub node_type: String,        // 当前
    // pub node_type: NodeType,   // 期望
```

**影响**: 这是 IPC 边界核心类型。前端发来的 JSON 中 `type: "http:request"` 当前被反序列化为 `String`，改枚举后需要：
1. 确认前端 `src/types/node.ts` 也使用枚举而非自由字符串
2. `NodeData::type_name()` 返回 `String` 需调整为返回 `NodeType` 或保持兼容

**风险**: 如果前端用自由字符串（如用户自定义节点类型），改枚举会破坏扩展性。

**建议**: **先讨论再动手**（见 ⚪ 讨论区）

---

### P3-4: `validate_flow` 返回类型化结构体

**文件**: `src-tauri/src/tauri_commands.rs:324-346`

```rust
pub fn validate_flow(flow: Flow) -> serde_json::Value {
    // 手工拼 JSON
    serde_json::json!({ "valid": ..., "errors": ..., "warnings": ... })
}
```

**修复**: 定义 `ValidationResult` 结构体：

```rust
#[derive(Serialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

#[derive(Serialize)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
}
```

**涉及改动量**: 极小（新建 struct + 替换 json! 宏）

---

### P3-5: NodeType `rename_all` 清理

**文件**: `flow-engine-core/src/node.rs:7`

**当前**: `#[serde(rename_all = "lowercase")]` + 大量 `#[serde(rename = "ws:connect")]` 覆盖

**问题**: 多单词变体（`WsConnect` → `"wsconnect"` 错误）导致需要 90% 的变体加显式 rename

**修复**: 改为 `#[serde(rename_all = "snake_case")]`，自动生成 `ws:connect`、`db_query` 等正确名称，消除绝大多数显式 rename

**风险**: 需确认前端也使用 `snake_case` 字符串（从已有显式 rename 看，前端确实用 `snake_case`），纯向后兼容改动。

**涉及改动量**: 小（1 行枚举属性 + 删除冗余注解）

---

### P3-6: 逐步清理 clippy 警告

**现状**: 741 个警告，主要分布：

| 类型 | 数量 | 说明 |
|------|------|------|
| `doc_markdown` | 419 | 注释中的标识符缺少反引号，多数为中文注释 |
| `map_or` | 159 | `.map_or(false, ...)` 可简化为 `.map_or(false, ...)` 等形式 |
| `large_enum_variant` | 17 | `NodeData` 等枚举的变体大小差异大 |
| `manual_range_contains` | 24 | 可替换为 `.contains()` |
| 其他 | 122 | is_some, expect_used, redundant_closure 等 |

**建议**: 分批处理，先修 `large_enum_variant`（性能相关），再修 `manual_*`，`doc_markdown` 优先级最低。

---

### P3-7: `mpe-cli` 增加测试

**文件**: `src-tauri/crates/mpe-cli/`

**现状**: 仅 6 个单元测试。

**建议**: 至少为 `parse_vars()`、输出格式化、各子命令的 clap 参数解析添加测试。

---

## ⚪ 待讨论项

### D-1: `NodeType` 枚举 vs 字符串

- **枚举派**: 92 变体 `NodeType` 已存在，应作为 `Node.node_type` 的类型，获得编译期匹配检查
- **字符串派**: 前端可能用自由字符串，枚举限制了可扩展性。当前 `NodeRegistry` 也是按字符串索引
- **建议统一结论**: 你们团队内部对齐一下，确认前端 `node.type` 是否只取枚举值

### D-2: 生产代码中的 unwrap/expect

- 测试代码中的 unwrap 没问题
- 生产路径上的需要清理，但全项目 2029 个手动修量太大
- **建议**: 先添加 clippy `#[deny(clippy::unwrap_used)]` 到 `[workspace.lints.clippy]`，新代码不会再引入；已有的用 `cargo clippy --fix` 自动修一批

### D-3: 拆文件 vs 不拆

- `tauri_commands.rs` 4396 行 — 你们团队习惯大文件还是小文件？
- 如果拆，需要统一命名和 `mod.rs` 导出风格

### D-4: CI 集成 clippy

- 当前 `cargo clippy` 有 741 个警告，PR 中不可能现在强制执行
- 建议建立一个底线：新代码不引入新 warning（可考虑逐步降低容忍值）

---

## 分批执行建议

| 批次 | 内容 | 预估耗时 | 并行度 |
|------|------|---------|--------|
| 🥇 Batch 1 | P1-1 + P1-2 + P3-4 | 半天 | 可并行 |
| 🥈 Batch 2 | P2-1 + P2-2 + P2-3 | 半天 | 可并行 |
| 🥉 Batch 3 | D-2（加 clippy deny）+ 清理一批 warning | 1 天 | 单人 |
| 🏅 Batch 4 | P3-1 拆分 tauri_commands.rs | 1-2 天 | 单人 |
| 🏅 Batch 5 | P3-2 executor.rs 解耦 | 3-5 天 | 需规划 |
| 🏅 Batch 6 | P3-3 + P3-5 NodeType 改动 | 1 天 | 与前端配合 |
| 🎯 持续 | P3-6 clippy 警告逐步清理 | 碎片时间 | 可并行 |

---

## 附录：废弃的安全发现（本地客户端无需处理）

| 原发现 | 原评级 | 废弃理由 |
|--------|--------|---------|
| MCP Stdio 命令注入 | CRITICAL | Stdio 传输模式本质就是执行用户配置的 MCP 服务端二进制 |
| SSRF 防护 | CRITICAL | 流程引擎核心能力就是连接任意协议/任意目标 |
| Proto 路径遍历写入 | HIGH | 用户通过文件选择器选择文件，路径由用户控制 |
| gRPC 文件读取无校验 | HIGH | Tauri dialog 已限制文件选择范围 |
| HTTP 重定向 SSRF | HIGH | 用户自行创建 flow，重定向目标是 flow 行为的一部分 |
| CSV/Flow 文件 DoS | MEDIUM | 用户处理自己的 CSV 和 flow 文件 |
| 凭据泄露到日志 | MEDIUM | 桌面应用的日志仅用户自己可访问 |
