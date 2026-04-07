# flow-engine Crate 拆分：编译优化重构

## TL;DR

> **Quick Summary**: 将 `flow-engine` 单体 crate（74 源文件、41 依赖）拆分为 8 个子 crate，实现增量编译优化。通过 Trait Object 解耦 ExecutionContext，用注册式分发替代巨型 match。
> 
> **Deliverables**:
> - 8 个子 crate：core / http / db / ws / script / assertion / crypto / executor（facade）
> - 3 个新 trait：WsProvider、DbProvider、DebugEventSender（解耦 ExecutionContext）
> - 共享工具函数迁移：extract_value_by_path、replace_variables
> - 增量编译验证：改 http 代码不重编译 db/ws/script
> 
> **Estimated Effort**: Large
> **Parallel Execution**: YES - 5 waves
> **Critical Path**: Infrastructure → Core → ExecutionContext解耦 → Protocol Crates → Executor Facade

---

## Context

### Original Request
用户反馈项目编译很慢很卡，希望优化编译速度。讨论后确定将 `flow-engine` 拆成多个子 crate。

### Interview Summary
**Key Discussions**:
- 删掉了未使用的 `reqwest` 依赖（纯冗余）
- 选择中粒度 8 crate 方案：core / http / db / ws / script / assertion / crypto / executor
- ExecutionContext 用 Trait Object 方案解耦
- WS 消息流方法留在 ExecutionContext 上（用 trait object 包装 WsConnectionStream）
- 测试策略：拆完统一跑一次 cargo test + clippy

**Research Findings**:
- 完整依赖矩阵：74 源文件全部扫描，3 个主要循环依赖环
- `extract_value_by_path` 被 3 个文件引用：http_executor.rs（定义 pub(crate)）、ws_send_wait_executor.rs:783（调用）、db_query_executor.rs:507（私有副本）
- `replace_variables` 仅被 ws_send_wait_executor.rs 使用（3 处调用），定义在 executor.rs
- `storage.rs` → `stress_test` 单向依赖（storage 引用 StressTestReport 类型，stress_test 不引用 storage）
- `FlowExecutionReport.ws_streams: Vec<WsConnectionStream>` 直接嵌入 WS 类型
- debug/executor.rs 是 executor.rs 的近镜像，Script 节点返回 Err("尚未实现")
- encryption.rs 和 script_api.rs 完全独立，零跨模块依赖
- `ws_send_wait_executor.rs:783` 调用 `crate::http_executor::extract_value_by_path`，形成 WS→HTTP 跨协议依赖（需 Task 4 将函数迁入 core 后在 Task 10 消解）

### Metis Review
**Identified Gaps** (addressed):
- extract_value_by_path 使用范围：3 个文件引用（http_executor 定义、ws_send_wait_executor 调用、db_query 私有副本），需统一到 core
- replace_variables 仅被 ws_send_wait_executor 使用（非 4 个执行器），但仍需迁入 core 以解耦
- storage → stress_test 单向依赖（非双向循环），两者同放 executor crate 即可
- FlowExecutionReport 直接嵌入 Vec<WsConnectionStream>，影响 crate 划分
- debug executor 的 Script 节点行为与主 executor 不同
- NodeExecutor trait 在 executor.rs 而非 traits/ 目录
- 没有基线编译时间测量

---

## Work Objectives

### Core Objective
将 flow-engine 拆分为 8 个独立编译的子 crate，使得修改单个协议模块（如 HTTP）不会触发其他重量级模块（如 sqlx、rquickjs）的重编译。

### Concrete Deliverables
- `crates/flow-engine-core/` — 核心类型 + trait + 共享工具
- `crates/flow-engine-http/` — HTTP 客户端 + 执行器（hyper 1.x）
- `crates/flow-engine-db/` — 数据库连接池 + 执行器（sqlx + redis）
- `crates/flow-engine-ws/` — WebSocket 管理 + 执行器（tokio-tungstenite）
- `crates/flow-engine-script/` — 脚本节点（rquickjs）
- `crates/flow-engine-assertion/` — 断言模块
- `crates/flow-engine-crypto/` — 加密模块（AES-256-GCM）
- `crates/flow-engine-executor/` — Facade crate（re-export + FlowExecutor + DebugExecutor）
- 原 `flow-engine` crate 保留为兼容层 facade

### Definition of Done
- [ ] `cd src-tauri && cargo test && cargo clippy` 全部通过
- [ ] `cd src-tauri && cargo build --bin cli` 编译成功
- [ ] 修改 `http_executor.rs` 后 `cargo build --timings` 显示 db/ws/script crate 不重编译
- [ ] `src-tauri/tests/` 外部测试文件零改动（通过 facade 正常工作）
- [ ] `src-tauri/src/lib.rs` 和 `bin/cli.rs` 改动最小化（仅 import path 调整）

### Must Have
- 8 个子 crate 全部编译通过
- ExecutionContext 通过 trait object 解耦（WsProvider, DbProvider, DebugEventSender）
- extract_value_by_path 和 replace_variables 迁移到 core
- ws_stream 纯数据类型迁移到 core（WsConnectionStream 等 5 个类型）
- NodeExecutor trait 迁移到 core 的 traits 模块
- workspace.dependencies 统一管理共享依赖版本
- 编译基线测量 + 拆分后对比

### Must NOT Have (Guardrails)
- 不改变 FlowExecutionReport 的序列化格式（ws_streams 字段保持不变）
- 不把 executor match 分发改成 dyn trait object（保持 match，闭类型集零开销）
- 不添加 enum_dispatch / inventory 等新依赖
- 不重新设计错误处理系统（ErrorKind/ErrorDetail 保持原样）
- 不改动前端代码（src/ 目录）
- 不在拆分过程中做额外重构（如合并重复代码、改命名等）
- 不添加 cargo-hakari（后续优化）
- 不改变任何 pub 可见性（仅新增跨 crate 的 pub）
- debug executor 的 Script 节点行为保持返回 Err("尚未实现")

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed.

### Test Decision
- **Infrastructure exists**: YES
- **Automated tests**: Tests-after（拆完统一测）
- **Framework**: cargo test + cargo clippy

### QA Policy
每个 task 包含 agent-executed QA scenarios。
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Rust Crate**: Use Bash — `cargo test`, `cargo clippy`, `cargo build --timings`
- **CLI Binary**: Use Bash — `cargo build --bin cli`

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 0 (Start Immediately — 基线 + 基础设施):
├── Task 1: 捕获编译基线 (cargo build --timings) [quick]
└── Task 2: 创建 workspace 基础设施 (8 个空 crate skeleton + workspace.dependencies) [quick]

Wave 1 (After Wave 0 — Core 提取):
├── Task 3: 提取 flow-engine-core：纯类型 (node, flow, connection, error, variable, log_event) [deep]
├── Task 4: 提取 flow-engine-core：共享工具 (extract_value_by_path + replace_variables + ws_stream 类型) [unspecified-high]
└── Task 5: 提取 flow-engine-core：trait 定义 (NodeExecutor + WsProvider + DbProvider + DebugEventSender) [deep]

Wave 2 (After Wave 1 — ExecutionContext 解耦):
└── Task 6: 重构 ExecutionContext：5 个外部字段改为 trait object + 迁移相关方法 [deep]

Wave 3 (After Wave 2 — 协议 Crate 并行提取, MAX PARALLEL):
├── Task 7: 提取 flow-engine-crypto (encryption.rs) [quick]
├── Task 8: 提取 flow-engine-http (http/ + http_executor.rs) [unspecified-high]
├── Task 9: 提取 flow-engine-db (db/ + db_*_executor.rs) [deep]
├── Task 10: 提取 flow-engine-ws (websocket_manager + ws_*_executor.rs) [unspecified-high]
├── Task 11: 提取 flow-engine-script (script_api + script_executor) [unspecified-high]
└── Task 12: 提取 flow-engine-assertion (assertion/ + assertion_executor.rs) [quick]

Wave 4 (After Wave 3 — 编排层):
├── Task 13: 创建 flow-engine-executor (FlowExecutor + match dispatch + entry_executor + conditional_executor) [deep]
├── Task 14: 迁移 debug/executor.rs 到 flow-engine-executor（处理 Script 节点差异） [unspecified-high]
├── Task 15: 迁移 stress_test, commands, report, storage, validator 到 flow-engine-executor [deep]
├── Task 16: 设置 flow-engine facade crate (re-export 全部) + 更新 app/CLI 引用 [unspecified-high]
└── Task 17: 全量测试 + 编译验证 + 基线对比 [deep]

Wave FINAL (After ALL tasks — 4 parallel reviews):
├── Task F1: Plan compliance audit (oracle)
├── Task F2: Code quality review (unspecified-high)
├── Task F3: Real manual QA (unspecified-high)
└── Task F4: Scope fidelity check (deep)
-> Present results -> Get explicit user okay

Critical Path: Task 1 → Task 2 → Task 3-5 → Task 6 → Task 7-12 → Task 13-17 → F1-F4
Parallel Speedup: ~60% faster than sequential
Max Concurrent: 6 (Wave 3)
```

### Dependency Matrix

| Task | Depends On | Blocks | Wave |
|------|-----------|--------|------|
| 1 | — | 17 | 0 |
| 2 | — | 3-5 | 0 |
| 3 | 2 | 6 | 1 |
| 4 | 3 | 6 | 1 |
| 5 | 3 | 6 | 1 |
| 6 | 3,4,5 | 7-12 | 2 |
| 7 | 6 | 13 | 3 |
| 8 | 6 | 13 | 3 |
| 9 | 6 | 13 | 3 |
| 10 | 6 | 13 | 3 |
| 11 | 6 | 13 | 3 |
| 12 | 6 | 13 | 3 |
| 13 | 7-12 | 14,15 | 4 |
| 14 | 13 | 16 | 4 |
| 15 | 13 | 16 | 4 |
| 16 | 14,15 | 17 | 4 |
| 17 | 1,16 | F1-F4 | 4 |

### Agent Dispatch Summary

- **Wave 0**: 2 — T1 → `quick`, T2 → `quick`
- **Wave 1**: 3 — T3 → `deep`, T4 → `unspecified-high`, T5 → `deep`
- **Wave 2**: 1 — T6 → `deep`
- **Wave 3**: 6 — T7 → `quick`, T8 → `unspecified-high`, T9 → `deep`, T10 → `unspecified-high`, T11 → `unspecified-high`, T12 → `quick`
- **Wave 4**: 5 — T13 → `deep`, T14 → `unspecified-high`, T15 → `deep`, T16 → `unspecified-high`, T17 → `deep`
- **FINAL**: 4 — F1 → `oracle`, F2 → `unspecified-high`, F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [x] 1. 捕获编译基线

  **What to do**:
  - 运行 `cargo clean` 清空缓存
  - 运行 `cargo build --timings 2>&1` 并保存完整输出到 `.sisyphus/evidence/task-1-baseline-timings.txt`
  - 记录 wall-clock 时间（`Measure-Command { cargo build }`）
  - 记录 `target/debug/` 目录大小
  - 保存 `Cargo.lock` 副本作为基线参考

  **Must NOT do**:
  - 不修改任何源文件
  - 不安装新工具

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 0 (with Task 2)
  - **Blocks**: Task 17
  - **Blocked By**: None

  **References**:
  - `src-tauri/Cargo.toml` — 当前 workspace 配置，确认只有一个 member
  - `src-tauri/crates/flow-engine/Cargo.toml` — 当前单体 crate 的 41 个依赖

  **WHY Each Reference Matters**:
  - Cargo.toml workspace 配置确认当前编译范围
  - flow-engine Cargo.toml 用于理解当前编译单元大小

  **Acceptance Criteria**:
  - [ ] 文件 `.sisyphus/evidence/task-1-baseline-timings.txt` 存在且包含 timing 数据
  - [ ] 记录了 wall-clock 时间和 target 目录大小

  **QA Scenarios (agent-executed)**:

  ```
  Scenario: 基线数据完整性
    Tool: Bash
    Steps:
      1. 检查 `.sisyphus/evidence/task-1-baseline-timings.txt` 文件存在
      2. 验证文件内容包含 "Compiling flow-engine" 或 "flow-engine" timing 数据
      3. 验证文件大小 > 0 bytes
    Expected Result: 文件存在，内容有效
    Evidence: .sisyphus/evidence/task-1-qa-baseline.txt
  ```

  **Commit**: YES
  - Message: `chore(flow-engine): capture baseline compilation metrics`
  - Files: `.sisyphus/evidence/task-1-baseline-timings.txt`

- [x] 2. 创建 workspace 基础设施

  **What to do**:
  - 在 `src-tauri/Cargo.toml` `[workspace]` 下添加 8 个新 crate 到 members
  - 添加 `[workspace.dependencies]` 统一管理共享依赖版本（serde, serde_json, tokio, async-trait, uuid, chrono, log, regex, thiserror, anyhow）
  - 在 `src-tauri/crates/` 下为每个子 crate 创建目录骨架：
    - `flow-engine-core/` → `Cargo.toml` + `src/lib.rs`（空 `pub mod` 占位）
    - `flow-engine-http/` → 同上
    - `flow-engine-db/` → 同上
    - `flow-engine-ws/` → 同上
    - `flow-engine-script/` → 同上
    - `flow-engine-assertion/` → 同上
    - `flow-engine-crypto/` → 同上
    - `flow-engine-executor/` → 同上
  - 每个子 crate 的 `Cargo.toml` 使用 `workspace = true` 继承共享依赖
  - 原 `flow-engine` crate 保留不动（后续改为 facade）
  - 验证 `cargo check --workspace` 通过（空 crate 不影响现有编译）

  **Must NOT do**:
  - 不移动任何源文件（仅创建空骨架）
  - 不改变原 flow-engine 的任何代码
  - 不添加 enum_dispatch/inventory 等新依赖

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 0 (with Task 1)
  - **Blocks**: Tasks 3-5
  - **Blocked By**: None

  **References**:
  - `src-tauri/Cargo.toml` — 当前 workspace 配置，需要添加 members
  - `src-tauri/crates/flow-engine/Cargo.toml` — 参考依赖版本用于 workspace.dependencies
  - `src-tauri/crates/flow-engine/src/lib.rs` — 参考 module 声明模式

  **WHY Each Reference Matters**:
  - Cargo.toml 是需要修改的核心文件
  - flow-engine 的依赖版本用于确定 workspace.dependencies 的版本号
  - lib.rs 的 module 组织模式用于设计子 crate 的 lib.rs 结构

  **Acceptance Criteria**:
  - [ ] `cargo check --workspace` 通过（8 个空 crate + 原 flow-engine）
  - [ ] `cargo check -p flow-engine-core` 通过
  - [ ] workspace.dependencies 包含至少 10 个共享依赖

  **QA Scenarios (agent-executed)**:

  ```
  Scenario: Workspace 编译验证
    Tool: Bash
    Steps:
      1. `cargo check --workspace 2>&1`
      2. `cargo check -p flow-engine-core 2>&1`
      3. `cargo check -p flow-engine-http 2>&1`
    Expected Result: 全部输出 "Checking" + "Finished"，无 error
    Failure Indicators: "error" 或 "could not find"
    Evidence: .sisyphus/evidence/task-2-workspace-check.txt

  Scenario: 共享依赖验证
    Tool: Bash
    Steps:
      1. `cargo tree -p flow-engine-core --depth 0 2>&1`
      2. 验证输出包含 serde, tokio 等基础依赖
    Expected Result: 基础依赖正确解析
    Evidence: .sisyphus/evidence/task-2-deps-check.txt
  ```

  **Commit**: YES
  - Message: `chore(flow-engine): create workspace infrastructure for 8 sub-crates`
  - Files: `src-tauri/Cargo.toml`, `src-tauri/crates/flow-engine-*/Cargo.toml`, `src-tauri/crates/flow-engine-*/src/lib.rs`

---

- [x] 3. 提取 flow-engine-core：纯类型

  **What to do**:
  - 将以下纯数据类型文件从 `flow-engine/src/` 移动到 `flow-engine-core/src/`：
    - `node.rs` (Node, NodeType, NodePosition, Port)
    - `flow.rs` (Flow)
    - `connection.rs` (Connection)
    - `error.rs` (ErrorDetail, ErrorKind)
    - `variable.rs` (Variable, InitialVariable, OutputMapping)
    - `log_event.rs` (LogEvent, LogLevel)
    - `report.rs` (ExecutionResult, ExecutionStatus, ExecutionMode, NodeExecutionReport, FlowExecutionReport, VariableChange, SkippedNodeInfo 等)
    - `statistics.rs` (FlowStatistics)
    - `node_config.rs` (NodeData 枚举 + 14 种配置结构体)
    - `ws_stream.rs` (MessageDirection, MessageContent, WsMessage, WsStreamSegment, WsConnectionStream — 5 个纯数据类型，零外部依赖)
  - `report.rs` 依赖 `ws_stream` 的类型（`FlowExecutionReport.ws_streams` 字段 + `From` impl），两者同时移动到 core 即可解决
  - `node_config.rs` 引用 `variable::{InitialVariable, OutputMapping}`，两者同在 core 内部，无跨 crate 问题
  - 在 `flow-engine-core/src/lib.rs` 中声明并 re-export 这些模块
  - 在 `flow-engine/Cargo.toml` 中添加 `flow-engine-core = { path = "../flow-engine-core" }` 依赖
  - 在 `flow-engine/src/lib.rs` 中将原来的 `pub mod xxx` 改为 `pub use flow_engine_core::xxx` re-export

  **Must NOT do**:
  - 不改变任何序列化格式
  - 不改变 pub 可见性
  - 不做额外重构（如合并、重命名）

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO（Task 4,5 依赖此任务完成后的文件布局）
  - **Parallel Group**: Wave 1
  - **Blocks**: Tasks 4, 5, 6
  - **Blocked By**: Task 2

  **References**:
  - `src-tauri/crates/flow-engine/src/node.rs` — 核心类型定义，需要移动
  - `src-tauri/crates/flow-engine/src/flow.rs` — Flow 结构体
  - `src-tauri/crates/flow-engine/src/connection.rs` — Connection 结构体
  - `src-tauri/crates/flow-engine/src/error.rs` — 错误类型定义
  - `src-tauri/crates/flow-engine/src/variable.rs` — 变量相关类型
  - `src-tauri/crates/flow-engine/src/log_event.rs` — 日志事件
  - `src-tauri/crates/flow-engine/src/report.rs` — 报告类型（依赖 ws_stream，已合并移动）
  - `src-tauri/crates/flow-engine/src/statistics.rs` — 统计类型
  - `src-tauri/crates/flow-engine/src/node_config.rs` — 节点配置枚举（1476 行大文件）
  - `src-tauri/crates/flow-engine/src/ws_stream.rs` — WebSocket 消息流类型（5 个纯数据类型，零外部依赖）

  **WHY Each Reference Matters**:
  - 这些文件是需要移动的源文件
  - report.rs 依赖 ws_stream 类型（`FlowExecutionReport.ws_streams` 字段 + `From` impl），两者同时移动到 core 即可解决
  - node_config.rs 是最大的单文件，引用 variable 模块（同在 core 内部）
  - ws_stream.rs 是 5 个纯数据类型，零外部依赖，可安全移动

  **Acceptance Criteria**:
  - [ ] `cargo check -p flow-engine-core` 通过
  - [ ] `cargo check -p flow-engine` 通过（通过 re-export 保持兼容）
  - [ ] 所有 11 个文件成功移动到 flow-engine-core/src/（含 ws_stream.rs）

  **QA Scenarios (agent-executed)**:

  ```
  Scenario: Core crate 编译验证
    Tool: Bash
    Steps:
      1. `cargo check -p flow-engine-core 2>&1`
      2. 验证输出不含 error
    Expected Result: "Checking flow-engine-core" + "Finished" + 无 error
    Evidence: .sisyphus/evidence/task-3-core-check.txt

  Scenario: Facade 兼容性验证
    Tool: Bash
    Steps:
      1. `cargo check -p flow-engine 2>&1`
      2. 验证原 flow-engine 通过 re-export 仍可编译
    Expected Result: 无 import 错误
    Evidence: .sisyphus/evidence/task-3-facade-check.txt
  ```

  **Commit**: YES
  - Message: `refactor(flow-engine): extract pure types to flow-engine-core`
  - Files: `flow-engine-core/src/*.rs`, `flow-engine/src/lib.rs`, `flow-engine/Cargo.toml`

- [x] 4. 提取 flow-engine-core：共享工具函数

  **What to do**:
  - 从 `executor.rs` 提取 `replace_variables()` 函数到 `flow-engine-core/src/utils.rs`
    - 仅被 `ws_send_wait_executor.rs` 使用（3 处调用：connection_id、message 替换）
    - 注意：函数签名 `fn replace_variables(template: &str, context: &ExecutionContext)` 依赖 ExecutionContext，若 context 还未迁入 core 则先保留在原处，待 Task 6 完成后再迁移
  - 从 `http_executor.rs` 提取 `extract_value_by_path()` 函数到 `flow-engine-core/src/utils.rs`
    - 被以下模块引用：`ws_send_wait_executor.rs:783`（通过 `crate::http_executor::extract_value_by_path`）
    - db_query_executor.rs:507 有私有副本，需删除并改用 core 版本
  - 统一 `db_query_executor.rs` 中的 `extract_value_by_path` 私有副本为对 core 的引用（删除重复）
  - 在 `flow-engine-core/src/lib.rs` 中 re-export `utils` 模块
  - 在原 `flow-engine` 中添加 `pub use flow_engine_core::utils` re-export

  **Must NOT do**:
  - 不改变 `FlowExecutionReport` 的序列化格式
  - 不统一 extract_value_by_path 的行为差异（db_query 版本与 http 版本如有细微差异，暂保留行为不变，先做机械性移动）
  - 如果 replace_variables 依赖 ExecutionContext（尚未迁入 core），则暂时不迁移 replace_variables，仅迁移 extract_value_by_path

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 Task 5 并行，两者都只依赖 Task 3）
  - **Parallel Group**: Wave 1 (after Task 3)
  - **Blocks**: Task 6
  - **Blocked By**: Task 3

  **References**:
  - `src-tauri/crates/flow-engine/src/executor.rs:55-126` — `replace_variables()` 函数
  - `src-tauri/crates/flow-engine/src/http_executor.rs` — `extract_value_by_path()` pub(crate) 定义
  - `src-tauri/crates/flow-engine/src/db_query_executor.rs:507` — `extract_value_by_path` 私有副本（需删除）
  - `src-tauri/crates/flow-engine/src/ws_send_wait_executor.rs:783` — 调用 `crate::http_executor::extract_value_by_path`（形成 WS→HTTP 跨协议依赖）

  **WHY Each Reference Matters**:
  - replace_variables 仅被 ws_send_wait_executor 使用，但依赖 ExecutionContext，需评估迁移时机
  - extract_value_by_path 是 WS→HTTP 跨协议依赖的根源，迁入 core 后可在 Task 10 消解
  - db_query 的私有副本需要删除并改用 core 版本

  **Acceptance Criteria**:
  - [ ] `cargo check -p flow-engine-core` 通过
  - [ ] `cargo check -p flow-engine` 通过
  - [ ] extract_value_by_path 在 core/src/utils.rs 中
  - [ ] db_query_executor.rs 中不再有 extract_value_by_path 的私有定义

  **QA Scenarios (agent-executed)**:

  ```
  Scenario: 共享工具编译验证
    Tool: Bash
    Steps:
      1. `cargo check -p flow-engine-core 2>&1`
      2. `cargo check -p flow-engine 2>&1`
    Expected Result: 两个 crate 都编译通过
    Evidence: .sisyphus/evidence/task-4-utils-check.txt

  Scenario: 重复代码消除验证
    Tool: Bash (grep)
    Steps:
      1. 在 flow-engine/src/ 下搜索 "fn extract_value_by_path"
      2. 验证只剩 core/src/utils.rs 中的定义（db_query 私有副本已删除）
    Expected Result: 只有 1 处定义
    Evidence: .sisyphus/evidence/task-4-dedup-check.txt
  ```

  **Commit**: YES
  - Message: `refactor(flow-engine): extract shared utils to core`
  - Files: `flow-engine-core/src/utils.rs`, `flow-engine/src/lib.rs`

- [x] 5. 提取 flow-engine-core：trait 定义

  **What to do**:
  - 从 `executor.rs` 提取 `NodeExecutor` trait 到 `flow-engine-core/src/traits/node_executor.rs`
  - 创建 `flow-engine-core/src/traits/ws_provider.rs` — WsProvider trait（抽象 WebSocketConnectionManager 的公共接口）
  - 创建 `flow-engine-core/src/traits/db_provider.rs` — DbProvider trait（抽象 DatabaseConnectionManager 的公共接口）
  - 创建 `flow-engine-core/src/traits/debug_event_sender.rs` — DebugEventSender trait（抽象 mpsc::UnboundedSender<DebugEvent>）
  - 创建 `flow-engine-core/src/traits/mod.rs` — 统一 re-export
  - 从现有 `traits/` 模块移动 LogSender, AppDirProvider, DirectoryOpener 到 core
  - DebugEvent 类型定义也需要放入 core（因为 trait 引用它）

  **Must NOT do**:
  - 不改变 NodeExecutor trait 的签名
  - 不添加 enum_dispatch / inventory 等新依赖
  - 不把 executor match 分发改成 dyn trait object

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES（与 Task 4 并行，两者都只依赖 Task 3）
  - **Parallel Group**: Wave 1 (after Task 3)
  - **Blocks**: Task 6
  - **Blocked By**: Task 3

  **References**:
  - `src-tauri/crates/flow-engine/src/executor.rs:131-154` — NodeExecutor trait 定义
  - `src-tauri/crates/flow-engine/src/traits/` — 已有 trait 模块（LogSender 等）
  - `src-tauri/crates/flow-engine/src/websocket_manager.rs` — 需要抽象的公共方法
  - `src-tauri/crates/flow-engine/src/db/pool.rs` — DatabaseConnectionManager 的公共接口
  - `src-tauri/crates/flow-engine/src/debug/types.rs` — DebugEvent 类型定义

  **WHY Each Reference Matters**:
  - executor.rs 中的 NodeExecutor trait 是所有执行器的基础接口
  - websocket_manager 和 db/pool 需要被抽象成 trait 才能让 ExecutionContext 解耦
  - DebugEvent 需要 move 到 core 因为 trait 签名引用它

  **Acceptance Criteria**:
  - [ ] `cargo check -p flow-engine-core` 通过
  - [ ] NodeExecutor trait 在 `flow-engine-core/src/traits/` 中
  - [ ] WsProvider, DbProvider, DebugEventSender 三个新 trait 已定义
  - [ ] LogSender 等已有 trait 迁移到 core

  **QA Scenarios (agent-executed)**:

  ```
  Scenario: Trait 定义编译验证
    Tool: Bash
    Steps:
      1. `cargo check -p flow-engine-core 2>&1`
      2. 验证 trait 模块正确导出
    Expected Result: 编译通过
    Evidence: .sisyphus/evidence/task-5-traits-check.txt
  ```

  **Commit**: YES
  - Message: `refactor(flow-engine): extract trait definitions to flow-engine-core`
  - Files: `flow-engine-core/src/traits/*.rs`

- [x] 6. 重构 ExecutionContext：trait object 解耦

  **What to do**:
  - 将 ExecutionContext 的 5 个外部依赖字段改为 trait object：
    - `ws_manager: Option<Arc<WebSocketConnectionManager>>` → `Option<Arc<dyn WsProvider>>`
    - `ws_streams: Arc<RwLock<HashMap<String, WsConnectionStream>>>` → 保留（WsConnectionStream 已在 core）
    - `db_manager: Option<Arc<DatabaseConnectionManager>>` → `Option<Arc<dyn DbProvider>>`
    - `db_connections: HashMap<String, DbConnection>` → `HashMap<String, Box<dyn DbConnectionInfo>>` 或保持具体类型（DbConnection 是纯数据，可移到 core）
    - `debug_event_sender: Option<mpsc::UnboundedSender<DebugEvent>>` → `Option<Box<dyn DebugEventSender>>`
  - 为具体类型实现对应 trait：
    - `impl WsProvider for WebSocketConnectionManager`（在 flow-engine-ws 中）
    - `impl DbProvider for DatabaseConnectionManager`（在 flow-engine-db 中）
    - `impl DebugEventSender for mpsc::UnboundedSender<DebugEvent>`（在 flow-engine-core 中）
  - 保留所有现有方法签名不变（with_ws_manager, with_debug_event_sender 等）
  - 更新所有构造 ExecutionContext 的地方（executor.rs, debug/executor.rs, tests）

  **Must NOT do**:
  - 不改变序列化行为（所有外部字段已经是 `#[serde(skip)]`）
  - 不改变 FlowExecutionReport 格式
  - 不改变 pub 方法签名（with_* 方法保持兼容）
  - 不一次性改全部字段（增量：先 ws，确认后 db，最后 debug）

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO（这是关键路径，所有 Wave 3 任务依赖此）
  - **Parallel Group**: Wave 2
  - **Blocks**: Tasks 7-12
  - **Blocked By**: Tasks 3, 4, 5

  **References**:
  - `src-tauri/crates/flow-engine/src/context.rs` — ExecutionContext 定义（921 行）
  - `src-tauri/crates/flow-engine/src/executor.rs:426-476` — ExecutionContext 构造点
  - `src-tauri/crates/flow-engine/src/debug/executor.rs` — 第二个构造点
  - `src-tauri/crates/flow-engine/src/websocket_manager.rs` — WsProvider 需要抽象的方法
  - `src-tauri/crates/flow-engine/src/db/pool.rs` — DbProvider 需要抽象的方法

  **WHY Each Reference Matters**:
  - context.rs 是需要重构的核心文件
  - executor.rs 和 debug/executor.rs 是构造 ExecutionContext 的主要调用点
  - websocket_manager 和 db/pool 的公共方法决定了 trait 的方法签名

  **Acceptance Criteria**:
  - [ ] `cargo check -p flow-engine-core` 通过
  - [ ] `cargo check -p flow-engine` 通过
  - [ ] ExecutionContext 不再直接 import 任何 db/ws/debug 的具体类型
  - [ ] 所有 3 个 trait 都有对应的具体实现

  **QA Scenarios (agent-executed)**:

  ```
  Scenario: 解耦验证 — import 检查
    Tool: Bash (grep)
    Steps:
      1. 检查 `flow-engine-core/src/context.rs` 中是否还有 `use crate::db::` 或 `use crate::websocket_manager`
      2. 验证只有 trait import（dyn WsProvider, dyn DbProvider, dyn DebugEventSender）
    Expected Result: 零具体类型 import
    Failure Indicators: 仍包含具体类型路径
    Evidence: .sisyphus/evidence/task-6-decoupling-check.txt

  Scenario: 全量编译
    Tool: Bash
    Steps:
      1. `cargo check --workspace 2>&1`
    Expected Result: 零 error
    Evidence: .sisyphus/evidence/task-6-workspace-check.txt
  ```

  **Commit**: YES
  - Message: `refactor(flow-engine): decouple ExecutionContext with trait objects`
  - Files: `flow-engine-core/src/context.rs`, `flow-engine-core/src/traits/*.rs`

- [ ] 7. 提取 flow-engine-crypto

  **What to do**:
  - 移动 `encryption.rs` 到 `flow-engine-crypto/src/encryption.rs`
  - `flow-engine-crypto` 依赖：aes-gcm, keyring, zeroize, base64, urlencoding, rand, once_cell, sha2, md-5, hmac, hex, serde, log
  - 在 `flow-engine-crypto/src/lib.rs` 中 `pub mod encryption`
  - 更新 `flow-engine` facade 的 re-export
  - **此模块完全独立，零跨模块依赖，是最安全的提取**

  **Must NOT do**:
  - 不改变加密算法或 keyring 接口

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 8-12)
  - **Blocks**: Task 13
  - **Blocked By**: Task 6

  **References**:
  - `src-tauri/crates/flow-engine/src/encryption.rs` — 唯一源文件

  **Acceptance Criteria**:
  - [ ] `cargo check -p flow-engine-crypto` 通过
  - [ ] `cargo check -p flow-engine` 通过

  **QA Scenarios**:
  ```
  Scenario: 独立编译
    Tool: Bash
    Steps:
      1. `cargo check -p flow-engine-crypto 2>&1`
    Expected Result: 无 error
    Evidence: .sisyphus/evidence/task-7-crypto-check.txt
  ```

  **Commit**: YES
  - Message: `refactor(flow-engine): extract flow-engine-crypto sub-crate`

- [ ] 8. 提取 flow-engine-http

  **What to do**:
  - 移动 `http/` 目录到 `flow-engine-http/src/http/`
  - 移动 `http_executor.rs` 到 `flow-engine-http/src/http_executor.rs`
  - 依赖：hyper, hyper-util, http, http-body-util, tower-service, bytes, tokio-rustls, rustls, rustls-pki-types, webpki-roots, url, mime_guess, tempfile, serde, serde_json, flow-engine-core
  - `flow-engine-http` 实现 `NodeExecutor` trait（来自 flow-engine-core）用于 Http NodeType
  - 删除 `http_executor.rs` 中的 `extract_value_by_path` 定义（已迁移到 core utils）

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 13
  - **Blocked By**: Task 6

  **References**:
  - `src-tauri/crates/flow-engine/src/http/` — HTTP 客户端模块（7 个文件）
  - `src-tauri/crates/flow-engine/src/http_executor.rs` — HTTP 执行器

  **QA Scenarios**:
  ```
  Scenario: 独立编译
    Tool: Bash
    Steps:
      1. `cargo check -p flow-engine-http 2>&1`
    Expected Result: 无 error
    Evidence: .sisyphus/evidence/task-8-http-check.txt
  ```

  **Commit**: YES
  - Message: `refactor(flow-engine): extract flow-engine-http sub-crate`

- [ ] 9. 提取 flow-engine-db

  **What to do**:
  - 移动 `db/` 目录到 `flow-engine-db/src/db/`
  - 移动 `db_connect_executor.rs`, `db_query_executor.rs`, `db_begin_executor.rs`, `db_commit_executor.rs`, `db_rollback_executor.rs`, `db_close_executor.rs` 到 `flow-engine-db/src/`
  - 依赖：sqlx (runtime-tokio + sqlite + mysql + postgres + chrono + uuid), redis, serde, serde_json, flow-engine-core
  - 删除 `db_query_executor.rs` 中的 `extract_value_by_path` 私有副本（用 core utils 的版本）
  - `flow-engine-db` 实现 `NodeExecutor` trait 用于 6 种 DB NodeType

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 13
  - **Blocked By**: Task 6

  **References**:
  - `src-tauri/crates/flow-engine/src/db/` — 数据库模块（traits, types, pool, postgres, mysql, sqlite, redis）
  - `src-tauri/crates/flow-engine/src/db_query_executor.rs:507` — extract_value_by_path 私有副本

  **QA Scenarios**:
  ```
  Scenario: 独立编译
    Tool: Bash
    Steps:
      1. `cargo check -p flow-engine-db 2>&1`
    Expected Result: 无 error
    Evidence: .sisyphus/evidence/task-9-db-check.txt

  Scenario: 去重验证
    Tool: Bash (grep)
    Steps:
      1. 在 flow-engine-db 中搜索 extract_value_by_path 的重复定义
    Expected Result: 零重复定义（仅有对 core 的 use 引用）
    Evidence: .sisyphus/evidence/task-9-dedup-check.txt
  ```

  **Commit**: YES
  - Message: `refactor(flow-engine): extract flow-engine-db sub-crate`

- [ ] 10. 提取 flow-engine-ws

  **What to do**:
  - 移动 `websocket_manager.rs` 到 `flow-engine-ws/src/websocket_manager.rs`
  - 移动 `ws_connect_executor.rs`, `ws_send_wait_executor.rs`, `ws_close_executor.rs` 到 `flow-engine-ws/src/`
  - 依赖：tokio-tungstenite, futures-util, serde, serde_json, flow-engine-core
  - `websocket_manager.rs` 引用 `node_config::FormField, WsMatchType, WsWaitCondition` — 这些类型在 core 中
  - 删除 `ws_send_wait_executor.rs` 中对 `http_executor::extract_value_by_path` 的引用（已迁移到 core utils）

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 13
  - **Blocked By**: Task 6

  **References**:
  - `src-tauri/crates/flow-engine/src/websocket_manager.rs` — WS 连接管理器
  - `src-tauri/crates/flow-engine/src/ws_*_executor.rs` — 3 个 WS 执行器

  **QA Scenarios**:
  ```
  Scenario: 独立编译
    Tool: Bash
    Steps:
      1. `cargo check -p flow-engine-ws 2>&1`
    Expected Result: 无 error
    Evidence: .sisyphus/evidence/task-10-ws-check.txt
  ```

  **Commit**: YES
  - Message: `refactor(flow-engine): extract flow-engine-ws sub-crate`

- [ ] 11. 提取 flow-engine-script

  **What to do**:
  - 移动 `script_api.rs` 到 `flow-engine-script/src/script_api.rs`
  - 移动 `script_executor.rs` 到 `flow-engine-script/src/script_executor.rs`
  - 依赖：rquickjs (features: chrono, array-buffer), serde, serde_json, flow-engine-core
  - `script_executor` 实现 `NodeExecutor` trait 用于 Script NodeType

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 13
  - **Blocked By**: Task 6

  **References**:
  - `src-tauri/crates/flow-engine/src/script_api.rs` — QuickJS API（自包含）
  - `src-tauri/crates/flow-engine/src/script_executor.rs` — 脚本执行器

  **QA Scenarios**:
  ```
  Scenario: 独立编译
    Tool: Bash
    Steps:
      1. `cargo check -p flow-engine-script 2>&1`
    Expected Result: 无 error
    Evidence: .sisyphus/evidence/task-11-script-check.txt
  ```

  **Commit**: YES
  - Message: `refactor(flow-engine): extract flow-engine-script sub-crate`

- [ ] 12. 提取 flow-engine-assertion

  **What to do**:
  - 移动 `assertion/` 目录到 `flow-engine-assertion/src/assertion/`
  - 移动 `assertion_executor.rs` 到 `flow-engine-assertion/src/assertion_executor.rs`
  - 依赖：serde, serde_json, flow-engine-core
  - `assertion_executor` 实现 `NodeExecutor` trait 用于 Assertion NodeType

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 13
  - **Blocked By**: Task 6

  **References**:
  - `src-tauri/crates/flow-engine/src/assertion/` — 断言模块（operator, evaluator, preset）
  - `src-tauri/crates/flow-engine/src/assertion_executor.rs` — 断言执行器

  **QA Scenarios**:
  ```
  Scenario: 独立编译
    Tool: Bash
    Steps:
      1. `cargo check -p flow-engine-assertion 2>&1`
    Expected Result: 无 error
    Evidence: .sisyphus/evidence/task-12-assertion-check.txt
  ```

  **Commit**: YES
  - Message: `refactor(flow-engine): extract flow-engine-assertion sub-crate`

---

## Wave 4 Tasks (13-17)

- [ ] 13. 创建 flow-engine-executor（FlowExecutor + match dispatch）

  **What to do**:
  - 创建 `flow-engine-executor` crate，依赖所有 7 个子 crate（core, http, db, ws, script, assertion, crypto）
  - 移动 `executor.rs` 的核心逻辑（FlowExecutor, ExecutionResult, execute_flow, execute_node, build_input_data_from_node_config）
  - 移动 `entry_executor.rs` 和 `conditional_executor.rs`（这两个不属于任何协议 crate）
  - `execute_node()` 中的 match 分发保持闭类型 match，直接 import 各 crate 的执行器类型
  - 移动 `context.rs` 中与 ExecutionContext trait object 相关的工厂方法
  - 依赖 evalexpr（仅 conditional_executor 使用）

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4
  - **Blocks**: Tasks 14, 15
  - **Blocked By**: Tasks 7-12

  **References**:
  - `src-tauri/crates/flow-engine/src/executor.rs` — FlowExecutor + match dispatch（2154 行）
  - `src-tauri/crates/flow-engine/src/entry_executor.rs` — 入口执行器
  - `src-tauri/crates/flow-engine/src/conditional_executor.rs` — 条件执行器

  **QA Scenarios**:
  ```
  Scenario: 编排层编译
    Tool: Bash
    Steps:
      1. `cargo check -p flow-engine-executor 2>&1`
    Expected Result: 无 error
    Evidence: .sisyphus/evidence/task-13-executor-check.txt
  ```

  **Commit**: YES
  - Message: `refactor(flow-engine): create flow-engine-executor with match dispatch`

- [ ] 14. 迁移 debug/executor.rs 到 flow-engine-executor

  **What to do**:
  - 移动 `debug/executor.rs` 到 `flow-engine-executor/src/debug/executor.rs`
  - 移动 `debug/types.rs` 到 `flow-engine-executor/src/debug/types.rs`
  - **关键差异**：debug executor 的 Script 节点返回 `Err("Script 节点执行器尚未实现")`，而主 executor 调用 ScriptNodeExecutor
  - 处理方式：debug executor 的 match 中 Script 分支直接返回 Err，不导入 flow-engine-script
  - 保留 debug executor 独立的 match（不与主 FlowExecutor 共用，因为行为不同）

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4 (after Task 13)
  - **Blocks**: Task 16
  - **Blocked By**: Task 13

  **References**:
  - `src-tauri/crates/flow-engine/src/debug/executor.rs` — DebugFlowExecutor（镜像 executor.rs）
  - `src-tauri/crates/flow-engine/src/debug/types.rs` — DebugEvent, DebugCommand 等类型

  **QA Scenarios**:
  ```
  Scenario: Debug 编译
    Tool: Bash
    Steps:
      1. `cargo check -p flow-engine-executor 2>&1`
      2. 验证 debug executor 的 Script match 分支返回 Err
    Expected Result: 编译通过，Script 分支行为正确
    Evidence: .sisyphus/evidence/task-14-debug-check.txt
  ```

  **Commit**: YES
  - Message: `refactor(flow-engine): migrate debug executor with Script node divergence`

- [ ] 15. 迁移 stress_test, commands, report, storage, validator 到 flow-engine-executor

  **What to do**:
  - 移动 `stress_test/` 目录到 `flow-engine-executor/src/stress_test/`
  - 移动 `commands/` 目录到 `flow-engine-executor/src/commands/`
  - 移动 `validator.rs` 到 `flow-engine-executor/src/validator.rs`
  - 移动 `storage.rs` 到 `flow-engine-executor/src/storage.rs`
  - 移动 `report.rs` 到 `flow-engine-executor/src/report.rs`（如果未在 Task 3 移走）
  - 移动 `statistics.rs` 到 `flow-engine-executor/src/statistics.rs`（如果未在 Task 3 移走）
  - 移动 `command.rs` 到 `flow-engine-executor/src/command.rs`
  - **storage → stress_test 单向依赖**：storage 引用 stress_test::StressTestReport 类型，stress_test 不引用 storage。两者同放 flow-engine-executor 后自然消除此依赖）

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4 (after Task 13)
  - **Blocks**: Task 16
  - **Blocked By**: Task 13

  **References**:
  - `src-tauri/crates/flow-engine/src/stress_test/` — 压测模块（6 个文件）
  - `src-tauri/crates/flow-engine/src/commands/` — 命令模块（5 个文件）
  - `src-tauri/crates/flow-engine/src/storage.rs` — 文件存储
  - `src-tauri/crates/flow-engine/src/validator.rs` — 流程验证器

  **QA Scenarios**:
  ```
  Scenario: 全模块编译
    Tool: Bash
    Steps:
      1. `cargo check -p flow-engine-executor 2>&1`
    Expected Result: 无 error
    Evidence: .sisyphus/evidence/task-15-modules-check.txt
  ```

  **Commit**: YES
  - Message: `refactor(flow-engine): migrate upper layers to flow-engine-executor`

- [ ] 16. 设置 flow-engine facade crate + 更新外部引用

  **What to do**:
  - 将原 `flow-engine` crate 的 `src/lib.rs` 改为 facade：re-export 全部 8 个子 crate
  - 更新 `src-tauri/src/lib.rs`（Tauri app）的 import 路径（如果需要）
  - 更新 `src-tauri/bin/cli.rs` 的 import 路径
  - 验证 `src-tauri/tests/` 的外部测试文件零改动（通过 facade 的 re-export 自然兼容）
  - 运行 `cargo build --bin cli` 确认 CLI 编译

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4 (after Tasks 14, 15)
  - **Blocks**: Task 17
  - **Blocked By**: Tasks 14, 15

  **References**:
  - `src-tauri/crates/flow-engine/src/lib.rs` — 当前 facade 需要改造
  - `src-tauri/src/lib.rs` — Tauri app 入口
  - `src-tauri/bin/cli.rs` — CLI 入口

  **QA Scenarios**:
  ```
  Scenario: 外部引用兼容性
    Tool: Bash
    Steps:
      1. `cargo build --bin cli 2>&1`
      2. `cargo build --lib 2>&1`
      3. 验证 src-tauri/tests/ 中的文件无 diff
    Expected Result: 全部编译成功，测试文件零改动
    Evidence: .sisyphus/evidence/task-16-facade-check.txt

  Scenario: Re-export 完整性
    Tool: Bash (cargo doc)
    Steps:
      1. 验证 flow-engine crate re-export 的类型名称与原始一致
    Expected Result: pub type 名称不变
    Evidence: .sisyphus/evidence/task-16-reexports.txt
  ```

  **Commit**: YES
  - Message: `refactor(flow-engine): wire up facade and update external references`

- [ ] 17. 全量测试 + 编译验证 + 基线对比

  **What to do**:
  - 运行 `cargo clean && cargo build --timings` 保存编译时间数据
  - 运行 `cargo test --workspace` 全量测试
  - 运行 `cargo clippy --workspace` lint 检查
  - 增量编译验证：`touch flow-engine-http/src/http_executor.rs && cargo build --timings`
  - 对比 Task 1 的基线数据
  - 编写对比报告保存到 `.sisyphus/evidence/task-17-comparison-report.md`

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4 (final)
  - **Blocks**: F1-F4
  - **Blocked By**: Tasks 1, 16

  **References**:
  - `.sisyphus/evidence/task-1-baseline-timings.txt` — 基线数据

  **QA Scenarios**:
  ```
  Scenario: 增量编译验证
    Tool: Bash
    Steps:
      1. `cargo clean`
      2. `cargo build 2>&1`
      3. `echo "// test comment" >> src-tauri/crates/flow-engine-http/src/http_executor.rs`
      4. `cargo build --timings 2>&1 | findstr "flow-engine" > .sisyphus/evidence/task-17-incremental.txt`
      5. 验证输出中 flow-engine-db, flow-engine-ws, flow-engine-script 不出现
    Expected Result: 仅 flow-engine-http 和 flow-engine-executor 重编译
    Failure Indicators: flow-engine-db 或 flow-engine-script 出现在输出中
    Evidence: .sisyphus/evidence/task-17-incremental.txt

  Scenario: 全量测试
    Tool: Bash
    Steps:
      1. `cargo test --workspace 2>&1`
      2. `cargo clippy --workspace 2>&1`
    Expected Result: 零 test failure，零 clippy error
    Evidence: .sisyphus/evidence/task-17-full-test.txt

  Scenario: 基线对比
    Tool: Bash
    Steps:
      1. 对比 wall-clock 时间和 target 目录大小
      2. 生成对比报告
    Expected Result: 增量编译时间显著减少
    Evidence: .sisyphus/evidence/task-17-comparison-report.md
  ```

  **Commit**: YES
  - Message: `chore(flow-engine): verify compilation improvement with baseline comparison`

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE. Present consolidated results to user and get explicit "okay" before completing.
> Never mark F1-F4 as checked before getting user's okay.

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in .sisyphus/evidence/. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo test --workspace`, `cargo clippy --workspace`, `cargo build --timings` in src-tauri. Review all changed files for: broken imports, unused deps, missing re-exports. Check that workspace.dependencies are used. Verify no new `as any`/`@ts-ignore` equivalents in Rust.
  Output: `Build [PASS/FAIL] | Lint [PASS/FAIL] | Tests [N pass/N fail] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state (`cargo clean`). Run `cargo build --timings` and capture output. Verify incremental compilation: touch `http_executor.rs`, rebuild, confirm `flow-engine-db` and `flow-engine-script` NOT recompiled. Build CLI binary. Run all tests.
  Save to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Incremental [VERIFIED/FAILED] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff (git log/diff). Verify 1:1 — everything in spec was built, nothing beyond spec was built. Check "Must NOT Have" compliance. Verify FlowExecutionReport serialization format unchanged. Verify no frontend code changed. Verify debug executor Script node returns Err.
  Output: `Tasks [N/N compliant] | Scope Creep [CLEAN/N issues] | VERDICT`

---

## Commit Strategy

- **Task 1**: `chore(flow-engine): capture baseline compilation metrics`
- **Task 2**: `chore(flow-engine): create workspace infrastructure for 8 sub-crates`
- **Tasks 3-5**: `refactor(flow-engine): extract core types, shared utils, and trait definitions`
- **Task 6**: `refactor(flow-engine): decouple ExecutionContext with trait objects`
- **Tasks 7-12**: `refactor(flow-engine): extract {name} sub-crate` (one per crate)
- **Tasks 13-15**: `refactor(flow-engine): create executor facade with debug and upper layers`
- **Task 16**: `refactor(flow-engine): wire up facade and update external references`
- **Task 17**: `chore(flow-engine): verify compilation improvement with baseline comparison`

---

## Success Criteria

### Verification Commands
```bash
cd src-tauri && cargo test                    # Expected: all tests pass
cd src-tauri && cargo clippy                  # Expected: no new warnings
cd src-tauri && cargo build --bin cli         # Expected: success
cd src-tauri && cargo build --timings         # Expected: sub-crates compile independently
```

### Incremental Compilation Verification
```bash
# 触碰 HTTP 执行器后，只有 flow-engine-http 和 flow-engine-executor 重编译
touch src-tauri/crates/flow-engine-http/src/http_executor.rs
cd src-tauri && cargo build --timings 2>&1 | findstr "flow-engine"
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] cargo test + cargo clippy pass
- [ ] Incremental compilation verified
- [ ] CLI binary builds
- [ ] src-tauri/tests/ external tests unchanged
