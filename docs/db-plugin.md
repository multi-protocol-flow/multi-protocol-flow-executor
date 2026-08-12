# db 节点插件化实施技术文档（第三方视角试点）

> **Status:** 已实施（2026-08-11，含实施后修订）
> **范围:** 内置 db 节点（`flow-engine-db`）从宿主剥离，以第三方 sidecar 插件形式重新提供 6 个 db 节点
> **关联文档:** `docs/plugin-architecture.md`（方案）、`docs/plugin-protocol.md`（wire 规范）、`docs/plugin-sdk.md`（作者指南）
> **仓库位置:** `plugins/db/`（独立工程；**已被 `.gitignore` 的 `/plugins/` 规则忽略**，不随宿主仓库跟踪）

---

## 0. 实施后修订（2026-08-11，相对本设计的三处决策变更）

本文档其余部分为 2026-08-06 批准的设计；实际实施按以下决策执行，**以下列为准**：

1. **type_id 用命名空间 `db:connect` 等（非本文档 §1.3/§3.3 的裸 `db_connect`）**。连锁简化：
   - **§4.1 reserved 移出 → 不需要**：`db:connect` 不在 `RESERVED_TYPE_IDS`，插件注册天然无冲突、无冒充问题（reserved 中裸 db id 随内置移除清理）。
   - **§4.2 sidecar config 提取通用化 → 不需要**：`NodeData` 自定义反序列化对未知 type 兜底 `Plugin(Value)`，`db:connect` 直接走现有 `NodeData::Plugin` 提取路径。
   - **存量流程文件失效**：`type: "db_connect"` 不再映射显式变体；已迁移 `tests/cli/` 4 个夹具 + `builtinFlows.ts` 2 处模板为命名空间。
2. **NodeData/NodeType 的 db 变体彻底删除**（非 §1.3 路线 Y 死类型保留）：`node_config.rs`/`node.rs`/`node_types.rs` 的 db 6 变体、config 结构、构造器、反序列化分支全部删除。
3. **SchemaConfigPanel 增强**（非 §3.3 收敛最小字段集）：新增 `format:'password'`（掩码）与 `format:'sql'`（多行），db 插件 config_schema 承载完整字段集；`x-node-selector` 连接选择器（mongo 先例）解决 connection_uuid 手输问题。
4. **§4.8 宿主默认路由已实现**（wrapper `default_success_port` + sidecar 从 describe ports 推导）——插件无需显式 `next_ports`，端到端验证通过。
5. **CLI 三通道已接入插件 registry**（validate/run-node/debug），§9 风险 10 已解决。
6. **分发按 §11 的"独立工程不入仓库"路径**：无 CI 构建、无 installer seed；本地 `cargo build --manifest-path plugins/db/Cargo.toml` + 手工拷入 `MPE_PLUGIN_DIR`（与 redis/mongo 一致）。

**实施验证（全部通过）**：`cargo tree -i sqlx -p app` 0 命中（sqlx 完全退出宿主编译图，含 workspace-hack 再生成）；`get_registered_node_types` 无内置 db_*；`mpe run` 跑迁移后夹具端到端成功（含事务 begin→insert→count→commit→verify）；`mpe validate` 对 db:connect 流程 valid；前端 `npm run typecheck`/`vite build` 通过、测试无 db 回归。

---

## 1. 概述

### 1.1 背景

**现状核实**：内置 db 节点**不存在宿主级长驻连接池**。`provider_builder.rs:56-60` 每次 `with_default_providers()` 调用都新建 `DatabaseConnectionManager`；`executor.rs:250-268` 每次 `execute_flow` 都构建全新 `ExecutionContext`（从而得到全新的空 db provider）；全仓库 61 处 `DatabaseConnectionManager::new()` 均为单次执行或测试模块，**不存在任何全局/长驻 manager**。因此**跨执行撞池在当前代码中不可能发生**——早期草案"池 key = 节点 uuid、长驻宿主进程、换环境后第二次 `db_connect` 因连接池已存在失败"的前提不成立，予以废弃。

**真正动机（首要）**：**验证插件体系对有状态事务型协议的承载能力**（原目标 #4）。db 是典型的"建池 + 多节点共享同一池 + 事务状态机"有状态协议，作为插件化试点最能暴露 sidecar 边界问题：插件进程常驻宿主之外，其池注册表**跨执行存续**，必须自证能按执行隔离连接、流程结束后回收空闲池——这为后续拆 MQTT/Kafka 等有状态协议积累依据。

**顺带修复的两个内置缺陷**（真实存在，属于本次插件化正确落地的前提）：
1. **池 key 不对称**：`db_connect` 以**节点 uuid** 建池（`db_connect_executor.rs:64`），而 `db_query/db_begin/db_commit/db_rollback/db_close` 按 **`config.connection_uuid`** 取池（`db_query_executor.rs:109`）。两者依赖"配置里的 `connection_uuid` 恰好等于连接节点 uuid"才对齐；一旦用户显式填写了别的 `connection_uuid`，查询/事务将取不到池。存量流程碰巧一致，属**潜在不一致**（key 兜底规则见 §3.4）。
2. **`db_connect` 不校验空 `connection_uuid`**：连接节点配置为空时静默按节点 uuid 建池，与取池语义脱节，错误不易察觉。

本次将 db 节点插件化：插件内池 key = `连接标识（connection_uuid，空则兜底节点 uuid）+ execution_id（宿主 per-run 实例 id）`，插件常驻进程内的池注册表按执行隔离，流程结束后空闲回收。

### 1.2 目标

1. 内置 db 从宿主**彻底移除**（`protocol-db` feature 摘除；"sqlx 不进宿主编译图"需走完整移除路径，见 §4.5）
2. `plugins/db/` 以**第三方视角**（仅依赖 `mpe-plugin-sdk` + wire 契约）重新提供 6 个 db 节点
3. 连接池按执行实例隔离：插件常驻进程内不同执行/重复执行不串池、流程结束池清
4. 验证插件体系对**有状态事务型协议**的承载能力（为后续拆 MQTT/Kafka 等积累依据）

### 1.3 范围

**做**：宿主后端 7 项改造（§4）、宿主前端 db 移除（§5）、`plugins/db` 插件工程（§3）、`.gitignore`、验收（§8）。

**不做**：
- 不保留内置 db（无内置/插件共存）
- 不改 `flow-engine-core` 的 db 类型定义（路线 Y：死类型保留；db 节点 JSON 经 `node_config.rs:3840` 映射到显式 `NodeData::DbConnect` 等变体，**执行期由 §4.2 `other=>` 通用提取分支兜底**——该分支是承重墙，`Plugin(Value)` 变体不参与 db 兜底）
- 不改 `mpe-protocol` wire 契约的**既有方法**：`execution_id` 参数语义调整（per-run 实例 id，见 §4.3）；**唯一例外是新增 `flow_end` 通知**（§3.6 方案 (b)，复用既有 `Notification{method, params}` 机制，加 method 名即可，wire 侧 trivial），`plugin-protocol.md` / `plugin-sdk.md` 的示例已同步
- 共享文档（`plugin-protocol.md` / `plugin-sdk.md`）已随本设计**同步更新**（错误形状三方对齐见 §4.4、`execution_id` 示例见 §4.3）
- 不拆其他协议

---

## 2. 总体架构

```
┌─────────────────────────── 宿主（MPE） ───────────────────────────┐
│  flow-engine-executor                                            │
│    ├─ default_registry ── 无 db（protocol-db 移除）               │
│    └─ ExecutionContext ── + execution_instance_id（per-run）      │
│  flow-engine-plugin                                              │
│    ├─ reserved.rs      ── 移出 db 6 个 type_id                    │
│    ├─ sidecar.rs       ── config 提取通用化（任意 NodeData）       │
│    └─ wrapper.rs       ── errors 兼容字符串数组                   │
│          │  JSON-RPC 2.0 over stdio（describe/execute/…）        │
└──────────┼────────────────────────────────────────────────────────┘
           ▼
┌────────────────── 插件进程（mpe-plugin-db，Resident） ─────────────┐
│  pool.rs      — 池注册表（key = 连接标识 + execution_id，见 §3.4）  │
│  lifecycle.rs — 空闲回收（流程结束后释放）                         │
│  drivers/     — sqlx ×3（sqlite/mysql/postgres）                  │
│  nodes/       — 6 节点（connect/query/begin/commit/rollback/close）│
└───────────────────────────────────────────────────────────────────┘
```

**关键事实**（已验证）：
- `map_execute_response`（`wrapper.rs:92`）在插件 `success:false` 时返回 `Err(ErrorDetail)` → **on_error 三策略（RouteToFalse/Ignore/AbortFlow）对插件节点天然生效**（executor.rs:765-914 外层统一处理），无需补丁
- `NodeData` 序列化为 `#[serde(tag = "type")]`（`node_config.rs:3519`），config 字段平铺同层，`to_value` 后剥 `type` 即得前端 config 形状
- **`next_ports` 契约（§4.8 已落地）**：SDK（`mpe-plugin-sdk/src/types.rs:91-92`）声明 `next_ports` 为空时"宿主应用默认路由"；宿主 `wrapper.rs` `map_execute_response`（`wrapper.rs:92`）对空 `next_ports` 注入 describe 推导的 `default_success_port`（`wrapper.rs:33`：含 `"true"` → `"true"`，否则唯一非 `in` 端口如 `db:close` 的 `"out"`；显式 `next_ports` 仍优先）——最小插件（`ExecuteResult::ok(...)`）即可正确路由

---

## 3. 插件侧设计（`plugins/db/`）

### 3.1 工程结构

```
plugins/db/
├── Cargo.toml          # 独立 manifest，非 workspace member
├── plugin.json
└── src/
    ├── main.rs         # mpe_plugin_main!(DbPlugin)
    ├── nodes/          # connect.rs / query.rs / begin.rs / commit.rs / rollback.rs / close.rs
    ├── pool.rs         # 池注册表 + 事务状态机
    ├── drivers/        # sqlite.rs / mysql.rs / postgres.rs（+ mod.rs 定义 DatabasePool trait）
    └── lifecycle.rs    # per-execution key + 空闲回收任务
```

`Cargo.toml` 要点：
```toml
[dependencies]
mpe-plugin-sdk = { path = "../../src-tauri/crates/mpe-plugin-sdk" }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "mysql", "postgres", "json", "chrono"] }
tokio = { version = "1", features = ["time", "sync"] }
serde_json = "1"
log = "0.4"
```

> 禁止依赖 `flow-engine-core` / `flow-engine-plugin`（sidecar 边界）。所有 db 语义在插件内**重新实现**，不复用宿主类型。

> **"第三方视角"限定**：`Cargo.toml` 以 `path = "../../src-tauri/crates/mpe-plugin-sdk"` 依赖宿主仓库内的 SDK，属 **monorepo 内部插件**（仓库内路径依赖），并非真正脱离仓库的第三方开发者验证。本试点验证的是"sidecar 进程 + wire 契约 + 仅依赖 SDK"这一**边界**，而非"外部开发者可独立构建"；后者属后续独立工作项。

### 3.2 plugin.json

```json
{
  "name": "db",
  "version": "0.1.0",
  "description": "SQLite/MySQL/PostgreSQL sidecar plugin",
  "entry": { "command": "<二进制路径>/mpe-plugin-db" },
  "capabilities": { "streaming": true }
}
```

- `streaming: true` → **Resident 常驻**（`lifecycle.rs` 不起 reaper，进程活到宿主退出）——OnDemand 空闲 60s 回收会断连，db 必须常驻
- **`entry.command` 路径硬约束（Windows）**：`process.rs:82` 用 `Command::new(&entry.command)` 且**不做 `.exe` 追加**——Windows `CreateProcessW` 仅在模块名**不含路径分隔符**时自动补 `.exe`。本文档示例 `<二进制路径>/mpe-plugin-db`（含 `/` 且无扩展名）在 Windows 必然 `ERROR_FILE_NOT_FOUND`（尽管 `mpe-plugin-db.exe` 真实存在）。Windows 上 `entry.command` 必须是**绝对路径且带 `.exe`**（如 `C:\Program Files\MPE\plugins\db\mpe-plugin-db.exe`），或**裸命令名**（无分隔符，走 PATH + `.exe` 解析）
- **CWD 依赖**：`process.rs` 未设 `current_dir()`，相对路径按**宿主进程 CWD** 解析（与插件目录无关，plugin-sdk.md:262）。`entry.command` **一律绝对路径**，不依赖宿主 CWD（见 §9 风险 1）

### 3.3 节点规格

type_id **保持内置原名**（流程文件/前端连线零成本兼容），端口对齐内置（`flow-engine-db/src/plugin.rs:55-68`）：

| type_id | 端口 | 端口位置 | 功能 |
|---|---|---|---|
| `db_connect` | true/false | right/bottom | 建池 + 测试查询 |
| `db_query` | true/false | right/bottom | SQL + 参数 + fetch 模式 |
| `db_begin` | true/false | right/bottom | 事务开始（隔离级别） |
| `db_commit` | true/false | right/bottom | 提交 |
| `db_rollback` | true/false | right/bottom | 回滚 |
| `db_close` | out | right | 移除池（幂等，单端口） |

describe 每个节点提供：
- `default_config`：对齐内置 config 字段形状（如 `db_connect`：`connection_uuid/driver/connection_string/host/port/database/username/password/min_connections/max_connections/test_query/connect_timeout_ms`）
- `config_schema`：JSON Schema，经 §4.7 宿主前端管道下发后驱动前端 `SchemaConfigPanel` 动态表单（前端移除 db 面板后的配置入口；管道未落地前该字段不达前端、不驱动表单）

**config_schema 能力底线（已实施增强，§0.3 修订）**：`SchemaConfigPanel.tsx` 支持 string+enum→Select、string→Input（含 `format:'password'` 掩码、`format:'sql'` 多行 textarea、`format:'json'` 多行 JSON、`x-node-selector` 跨节点选择器）、array→JSON 数组编辑、integer/number、boolean→Switch、未知→text；**仍不支持** oneOf/anyOf/conditional/dependencies，必填仅表现为星号。故 db 插件 `config_schema` **承载完整字段集**（不收敛最小集）：
- `db:connect`：`connection_uuid`（留空触发 §3.4 兜底）、`driver`（enum: sqlite/mysql/postgres）、`connection_string` + `host/port/database/username/password`（connection_string 为空时拼 URL）、`min_connections`/`max_connections`/`test_query`/`connect_timeout_ms`
- `db:query`：`connection_uuid`（x-node-selector 选 db:connect 节点）、`sql`（format:'sql' 多行）、`params`（array JSON 编辑）、`fetch_mode`（enum）、`result_to`、`timeout_ms`

**output_data / variable_updates / 路由契约（宿主默认路由）**：
- **`next_ports` 无需插件返回**：6 个 db 节点均为二态或单端口节点，路由由**宿主默认推导**——success → `"true"`（db_connect/query/begin/commit/rollback）或 `"out"`（db_close），failure → 宿主 `on_error` 三策略处理（RouteToFalse → false 端口 / Ignore → true 继续 / AbortFlow → 终止，见 §2）。插件只需返回 `success` + `output_data` + `variable_updates`，用 SDK 惯用法 `ExecuteResult::ok(output)` 即可正确路由（依赖 §4.8 宿主默认路由落地）。
- 端口 id 统一为内置语义：`"true"` / `"false"` / `"out"`（与 `flow-engine-db/src/plugin.rs:55-68` 对齐）。注意 `plugins/db/tests/fixtures/db_sqlite_flow.json:121-123` 夹具曾用 `"output"` 端口 id，与内置/前端约定（`builtinFlows.ts:75` 的 `"true"`）不一致——插件侧一律以 `"true"/"false"/"out"` 为准。
- `db_query` 成功输出与内置一致（`db_query_executor.rs:413-419`）：fetch 模式 `output_data` 含 **rows + truncated 标记 + row_count**；失败路径 **`variable_changes = None`**（`executor.rs:822`，错误时不变更变量），成功路径按内置规则写入变量。

### 3.4 连接池注册表（核心设计）

```rust
/// 池 key：连接标识（connection_uuid；db_connect 空值兜底为连接节点自身 uuid）+ execution_id（宿主 per-run 实例 id）
type PoolKey = (String, String);

struct PoolEntry {
    pool: Arc<dyn DatabasePool>,
    execution_id: String,   // 所属执行（回收按执行隔离，§3.6）
    last_used_at: Instant,  // 最近使用时刻（喂空闲回收，§3.6）
}

struct DbPluginState {
    pools: RwLock<HashMap<PoolKey, Arc<PoolEntry>>>,
    transactions: RwLock<HashMap<PoolKey, TransactionInfo>>,
}
```

**池 key 兜底规则（空 `connection_uuid` 一律显式报错）**：

真实存量流程（`builtinFlows.ts:61-70`）中 `db_connect` 的 `connection_uuid` 为空（`DbConnectConfig.tsx:284-300` 从不写入该字段），而 `db_query` 等节点携带 `connection_uuid = 连接节点 uuid`。内置实现靠"connect 按 `node.uuid` 建池 + 其余按 `config.connection_uuid` 取池"的巧合对齐（`db_connect_executor.rs:64` / `db_query_executor.rs:109`）。插件必须**保持 connect 兜底语义**，否则全部存量流程文件失效：

| 节点 | 池 key 规则 | 空 `connection_uuid` 处理 |
|---|---|---|
| `db_connect` | `config.connection_uuid` 非空 → 用它；为空 → **兜底为连接节点自身 uuid**（node uuid） | 不报错，兜底建池（存量流程兼容的硬依赖） |
| `db_query` / `db_begin` / `db_commit` / `db_rollback` / `db_close` | **仅用 `config.connection_uuid`** | **一律 `ExecuteResult::fail("connection_uuid is required")` → 宿主 `on_error` 三策略处理** |

**有意偏离内置的语义（设计决策，非复刻）**：内置 `db_commit`/`db_rollback`/`db_close` 对空 uuid 是**静默 skip 并走成功端口**（`db_commit_executor.rs:55-86`、`db_close_executor.rs:55-88`）——这是内置缺陷：commit 空连接被"假装成功"继续走 true 分支，用户以为事务已提交实际没有（数据完整性风险）；前端新建节点默认 `connection_uuid: ''`（`src/components/canvas/utils.ts:211-219`），"配置不完整就运行"的路径恰好被静默吞掉。插件作为重实现**统一将空 uuid 视为配置错误 → 显式失败**，由 `on_error` 策略路由（默认 RouteToFalse → false 端口，报错可见）。

**兼容性影响（核实）**：存量流程 `builtinFlows.ts:61-70/409-418` 中 `db_query/db_close` 均携带 `connection_uuid = 连接节点 uuid`，commit/rollback/close 同理 → **空 uuid 改报错不影响任何存量流程**；唯一行为变化是"新建节点未配置连接即运行"从静默通过变为显式报错——这是修复而非回归。`db_connect` 空 uuid 兜底保持（存量流程依赖）。

**与内置的差异说明（报告层）**：内置 `db_query/db_begin` 空 uuid 在默认策略下返回 Ok + error_output 走 false 端口（非 Err）；插件用 `fail()` → 宿主 `Err(ErrorDetail)` → 走 `on_error`。在 RouteToFalse 下两者都路由到 false 端口，行为收敛；在 Ignore 下内置走 Err("Flow aborted")（`db_query_executor.rs:114-123`），插件 fail → Ignore 走 true 继续——**流级行为分叉**，需在 §8 验收中显式断言（见 §8.6）。

| 操作 | 语义（复刻内置 `pool.rs`） |
|---|---|
| `create_pool(key, entry)` | **幂等复用（已实施语义，§0 修订）**：同 `(execution_id, connection_uuid)` 且目标相同 → 复用既有池（`replaced=false`）；目标不同 → 告警替换旧池（`replaced=true`）；**无错误路径**（原设计"已存在 → `Err("连接池已存在")` 绝不覆盖"未采纳，§8.8(a) 断言已相应更新）。隔离由 execution_id 保证 |
| `get_pool(key)` | 取池 + 刷新 `last_used_at`（喂回收，§3.6） |
| `close_pool(key)` | 原子移除 + `pool.close()` + 清事务状态 |
| `begin/commit/rollback(key)` | 事务状态机（见 §3.5） |

`last_used_at` 由 §3.6 的 reaper 消费；`execution_id` 供回收与数量上限（§3.6）按执行隔离。

**execution_id 来源**：宿主 execute RPC params 的 `execution_id`（改造后 = per-run 实例 id，§4.3）。插件进程常驻、池注册表跨执行存续，`execution_id` 保证不同执行/重复执行不共用 key；`db_connect` 建池与 `db_query` 取池在同一执行内共享同 key。

### 3.5 事务状态机（复刻内置）

```
begin(key):  无活跃事务 && 池存在 → 真实 BEGIN → 记 TransactionState::Active
commit(key): 状态=Active → 真实 COMMIT → Committed
rollback(key): 状态=Active → 真实 ROLLBACK → RolledBack
```
违规（重复 begin / 无事务 commit）返回 Err → 插件 `ExecuteResult::fail(message)` → 宿主报告。

**事务内查询路由（关键缺口补齐）**：`begin` 为事务**独占一条专用连接**；事务 Active 期间 `db_query` **必须钉在该事务专用连接上执行**，否则查询会走池连接、**静默逃逸事务**（未提交隔离失效、或提交到事务外）。路由规则：
- `transactions[key] == Active` → `db_query` 复用**事务专用连接**（与 begin 同一连接）执行，随 commit/rollback 一并生效/回滚
- `transactions` 无该 key（非活跃事务）→ `db_query` 从池取连接（正常路径）
- 事务专用连接在 Active 期间**不参与池内借用**（begin 时独占、commit/rollback 后归还），避免并发借用串扰事务
- 同一执行内 begin 前后多次 `db_query` 必须命中同一连接——验收 §8.11 以服务端可观测副作用（如事务内 update 在提交前对外不可见）验证

### 3.6 生命周期与空闲回收

- **懒启动**：插件进程第一次 execute（db_connect）时才拉起（宿主 lifecycle 语义），非启动即常驻
- **空闲回收**：插件内后台任务（`tokio::spawn` 定时循环，间隔如 10s）扫描 `pools`，`last_used_at` 超过 `idle_timeout`（默认 30s）的 key → close + remove
- **防执行中误杀（执行边界信号）**：纯按 idle 时间回收会在**执行中途**误杀池——慢步骤（>30s 的长 HTTP/script/ws）横亘在 `db_connect` 与 `db_query` 之间时池被回收，随后 `db_query` 报"连接不存在"；更严重的是 `db_begin → 长步骤 → db_commit`，事务**专用连接被回收即触发服务端隐式回滚**，活跃事务被中途销毁。防护机制（**默认方案 (b)，方案 (a) 已证伪**）：
  - (a) **按执行刷新**：同一 `execution_id` 的**任意节点** execute（不限 `get_pool`）都刷新该执行内全部池的 `last_used_at`——实现最简单、**无需改 wire。⚠️ 已证伪（对抗评审）**：插件进程**只收到自身节点**的 execute RPC（sidecar.rs:179-185 每节点一次），`db_connect → 宿主慢节点(>30s) → db_query` 窗口内插件收不到任何信号，`last_used_at` 停在 T0，reaper（10s tick、idle 30s）在 T30 必杀池/活跃事务——§8.7(a)(d) 按此机制必失败；调大 idle_timeout 又矛盾于 §8.7(b) 的 30s 窗口。**不可用，仅作过渡垫片**
  - (b) **宿主 `flow_end` 通知**：流程结束由宿主显式告知插件回收该执行。**对抗评审核实：wire 侧 trivial**——`mpe-protocol` 的 `Notification{method, params}` 机制已存在（`notification.rs:16-43`，双向 fire-and-forget），加 `flow_end` method 名即可；真成本在宿主按 run 追踪 flow 结束并发送。**必选，纳入 §4 改造项**（原"标记待议"废弃）
  - 补充兜底：reaper 跳过含 **Active 事务**的 key + idle_timeout 可配置大默认 + §3.6 全局池总数上限（三重保险，防方案 (b) 未落地前的误杀）
- **原子回收（防取出即死竞态）**：reaper 与 `db_query` 并发取池必须原子——用 refcount/compare-remove（`remove` 后校验引用计数），**禁止 check-then-remove**（检查"在空闲"后移除，可能移除正在取用/刚取用的池）
- **数量上限（防无界积累，§9 风险 6）**：reaper 只限 **age** 不限 **count**。每次 run 生成新 `execution_id` → 新池 key，`contains_key` **跨执行永不触发**；快速重复执行（如 5s 节奏）可累积 6-8 代池 × `max_connections(10)` = 60-80 条活跃连接，分钟级耗尽 MySQL `max_connections=151`；压测 100 用户 × 5-10 iters/s × 30s 窗口 × `min_connections` 可耗尽 PG `max_connections=100`。**对抗评审结论：下述 3 个兜底选项均无法单独解决全局耗尽**（≤8 池限单执行内、压测 100 并发=100+ 池不触发；per-execution TTL 仍是 idle 判定，与方案 (a) 同病；跨执行复用违反目标 3 隔离且破坏 SQLite `:memory:` 语义）→ **强制全局池总数上限（如 ≤64，LRU 淘汰空闲池），对"有 Active 事务/正在执行"的池豁免**（§8.8(c) 须达明确数值目标，如峰值连接 ≤ 服务端 `max_connections` 的 50%）
- **流程结束 ≈ 空闲**：流程结束后该 execution_id 的池无访问 → 一个空闲窗口后释放
- **db_close**：按当前 execution 主动清池（同 key 幂等，对齐内置单 out 端口语义）
- **崩溃**：宿主 restart-once 重启新进程 → 插件内池全丢（已知权衡，§9 风险 2）

**测试用例（对应 §8.7）**：`db_connect → >30s 慢步骤 → db_query` 成功（池未被中途回收，方案 (b) `flow_end` 未触发前由 Active 事务豁免/idle_timeout 大默认保护）；`begin → 慢步骤 → commit` 长事务**不被中途销毁**。

### 3.7 错误语义

```rust
ExecuteResult::fail("SQL syntax error: ...")   // 携带真实错误消息
```
wire 层 `errors: ["..."]`（SDK `Vec<String>`）。宿主 `wrapper.rs` 改造后（§4.4）透传真实消息进报告。

### 3.8 秘密处理现状（存量全局问题，非本次引入）

**现状核实**：db 密码在 `NodeData` 中以**明文**存储（`node_config.rs:963-965`）；sidecar 经 stdio 传输**解析后**的 config（含密码）；报告把 `resolved_config`（`executor.rs:541-546`）与 `flow_snapshot`（`executor.rs:954`）**脱敏前落盘**（`storage.rs:581`）；全仓库 grep "redact" **0 命中**。

这是**存量全局问题**（所有节点类型同样泄漏，非 db 特有），插件化既不更糟也不更好——stdio 为**本机管道**，非远程泄漏向量。**秘密脱敏是独立全局工作项，超出本文档范围**；本文档不为本次改动新增任何秘密机制，仅如实记录基线（见 §9 风险 9）。

---

## 4. 宿主侧改造（7 项）

### 4.1 `reserved.rs` 移出 db 6 个 type_id

- **位置**：`flow-engine-plugin/src/reserved.rs`（`RESERVED_TYPE_IDS`，db_begin/db_close/db_commit/db_connect/db_query/db_rollback）
- **原因**：`decide_registration`（`registration.rs:74`）第一个检查 `is_reserved_type_id` 是硬编码列表，不移出插件注册 `db_connect` 会被 `Err(Reserved)` 拒绝
- **type_id 所有权（移出 reserved 后的空白）**：db 6 个 id 离开 `RESERVED_TYPE_IDS` 后，**任何插件都可注册 `db_connect`**。插件间冲突按 `fs::read_dir` **无序扫描序先到先得**——真 db 插件可能**丢自己的 type_id**，且任意插件可**冒充 db** 节点。`plugin-architecture.md` 的"内置 id 保留不可被遮蔽"策略对 db 从此失效。**补锁机制（本文档默认前者）**：
  - **manifest 声明所有权**：`plugin.json` 增加 `owned_type_ids` 字段；宿主 `scan_and_describe` 对同名 type_id 冲突做**确定性裁决**（固定排序/明确报错，而非扫描序先到先得）；db 插件声明 `["db_connect","db_query","db_begin","db_commit","db_rollback","db_close"]`。**⚠️ wire 契约例外**：`PluginManifest` schema 定义在 `mpe-protocol/src/manifest.rs`（宿主 + SDK 共享），加字段即改 `mpe-protocol`——与 §1.3"不改 mpe-protocol"需显式声明例外（§1.3 已同步）；**且该机制须列入 M0 完成标志**（否则 §8.3 验收悬空）。**已知局限**：纯 manifest 自声明无法判别"谁是真的 db 插件"（两个插件都声明时，确定性排序只保证确定性不保证真插件获胜）——如需强判别，改宿主侧硬编码所有权表（type_id → 插件名，零 wire 改动）
  - **显式槽位预留**：宿主维护 db 6 个 type_id 的专用槽位，仅允许声明对应 ownership 的插件占用
- **验证**：多插件遮蔽测试——同目录放置"真 db 插件"与"冒充 db 的冲突插件"（声明相同 type_id），断言真插件**确定性**获得全部 6 个 type_id、冒充者被确定性拒绝、`collision_reports()` 给出明确冲突原因（见 §8.3）

### 4.2 `sidecar.rs` config 提取通用化

- **位置**：`sidecar.rs:145-153`
- **现状**：只认 `NodeData::Plugin(Value)`，其余变体报 "non-plugin data"
- **改造**：
```rust
let raw_config = match &node.data {
    NodeData::Plugin(config) => config.clone(),
    other => {
        let mut json = serde_json::to_value(other).map_err(...)?;  // tag="type" 序列化
        if let Some(obj) = json.as_object_mut() { obj.remove("type"); }
        json
    }
};
```
- **⚠️ 承重墙地位**：db 节点 JSON 经 `node_config.rs:3840` 映射到**显式 `NodeData::DbConnect` 等变体**（非 `Plugin(Value)` 兜底，见 §1.3 修订），因此本 `other =>` 通用提取分支是"死类型 db 节点可被插件执行"的**唯一通道**——缺失它，所有存量 db 流程运行时全部报 "non-plugin data"。**必做，不可省略**
- **影响**：只影响"被插件接管的节点"（本次 = db 6 变体）；内置协议不经 SidecarPlugin、现有插件走 `Plugin` 变体，均零影响。**通用资产**：后续拆其他协议不再改此处

### 4.3 `ExecutionContext` 加 per-run 实例 id

- **位置**：`flow-engine-core/src/context.rs`（加字段）+ 所有 `ExecutionContext` 构造点 + `sidecar.rs:183`
- **现状**：`sidecar.rs` 用 `context.flow_uuid` 当 `execution_id`；但 `flow_uuid = flow.uuid`（流程文件固定 id，`executor.rs:250-251`），**不唯一** → 同流程重复执行会撞池
- **改造**：
  - `ExecutionContext` 增加 `execution_instance_id: String`（或 `Uuid`）
  - 构造点（已核实）：**`executor.rs:250-268`（主构造点——`execute_flow` 的 struct 字面量，唯一设置 `flow_uuid` 处，GUI/CLI/压测/数据集全经它，**必改**）**、`executor.rs:1258`、`flow-engine/src/lib.rs:336`、`debug/executor.rs:169`、各测试模块，每次 run 生成 `Uuid::new_v4()`（**注意：`executor.rs:355` 不是构造点**；`stress_test/` 与 `dataset_executor/` **不直接构造** `ExecutionContext`，它们委托 `execute_flow` → 每次 run/每行/每迭代各自构造 → 压测/数据集下**每次调用都产生新 execution_id → 新池 key**，见 §9 风险 6）
  - `context.rs:679` 的 serde 往返测试：该测试是**纯自往返**（serialize→deserialize 同一结构），新字段会被一并序列化、测试**不会破坏**——原"非 defaulted 会破坏该测试"的推理不成立；`#[serde(default)]` 仍建议加（防旧快照反序列化），但**需注意**：带 default 会掩盖构造点漏设——任一构造点漏设则 `execution_instance_id` 为空串 → 所有 run 共享空 key → **全量撞池**（比 flow_uuid 更糟）。建议改为 `#[serde(skip)]`（非持久化字段，无序列化价值）+ 构造点逐一设置，靠编译期强制而非 serde 容错
  - `sidecar.rs` 的 params 改用 `execution_instance_id`
- **验证**：同流程连续执行两次 → 两次 execute 的 `execution_id` 不同（**建议并入 M1 验收：第二次运行不撞池**，因宿主当前发 `flow_uuid`（sidecar.rs:183），§4.3 未落地前 M1 复跑即撞池）

### 4.4 `wrapper.rs` errors 兼容字符串数组

- **位置**：`wrapper.rs:122-137`
- **现状**：`errors.first().get("message")` 期望对象 `{message, code}`；SDK 序列化为**字符串数组** `["..."]` → `.get("message")` 返回 None → 真实错误消息丢失，落到通用文案 "plugin reported execution failure"
- **错误形状三方不一致（源由）**：共享文档（`plugin-sdk.md:151`、`plugin-protocol.md:227`）约定**对象数组** `[{code, message}]`，SDK 当前实际序列化为**字符串数组**（`mpe-plugin-sdk/src/types.rs:100`），宿主 `wrapper.rs` 期望对象——三处不一致。本改造的"先对象后字符串"是**兼容垫片**；**根源修复**是把 SDK 对齐到对象形状（代码改动，超出本文档范围，另立工作项）。字符串分支**无 code 概念**（仅 message），对象分支才有 `{code, message}`
- **改造**：先试对象形态，再试字符串形态：
```rust
let message = errors
    .and_then(|arr| arr.first())
    .map(|e| match e {
        Value::Object(m) => m.get("message").and_then(Value::as_str).map(str::to_string),
        Value::String(s) => Some(s.clone()),
        _ => None,
    })
    .flatten()
    .unwrap_or("plugin reported execution failure");
```
- **验证**：db 插件 SQL 错误 → 宿主报告显示真实错误消息

### 4.5 移除 `protocol-db`

| 文件 | 改动 |
|---|---|
| `src-tauri/Cargo.toml` | `all-protocols` 移除 `protocol-db` |
| `crates/flow-engine/Cargo.toml` | 同上 |
| `crates/flow-engine-executor/Cargo.toml` | 同上 + 移除 `protocol-db` feature 与 `flow-engine-db` 依赖 |
| `crates/mpe-cli/Cargo.toml` | **显式删除**第 41 行转发 `protocol-db = ["flow-engine/protocol-db"]`（草案原写"自动继承无需改动"是错的：mpe-cli 按 feature 逐项转发 `flow-engine`，不依赖 `all-protocols`） |
| `workspace-hack/Cargo.toml` | **若以"sqlx 完全不进宿主编译图"为目标，此步为必需**：将 `flow-engine-db` 移出 workspace members 后执行 `cargo hakari generate` 并提交产物。`workspace-hack/Cargo.toml:82-85 / 157-160` 将 `sqlx-core / sqlx-mysql / sqlx-postgres / sqlx-sqlite` 列为**无条件依赖**，而 `src-tauri/Cargo.toml:148`、`flow-engine/Cargo.toml:53`、`mpe-cli/Cargo.toml:32` 均无条件依赖 workspace-hack——只要 hack 包仍列出 sqlx，`cargo tree -i sqlx -p app` 就会经 `app → workspace-hack` 出现（build 统一，非代码路径） |

`default_registry.rs` 的 db 注册随 `cfg(feature = "protocol-db")` 自动消失。
**验证**：`cargo tree -p app` 无 `flow-engine-db`；`cargo tree -i sqlx -p app` **0 命中**（hakari 再生成后 sqlx 完全退出宿主编译图，见 §8.1）；`get_registered_node_types` 无内置 `db_*` type_id（见 §8.1）。

**移除清单连锁（测试/夹具，M5 同步项）**：
- `reserved.rs:185` 断言 `len()==88`（redis 移出时 91→88；本次移出 db 6 → 82，已同步更新）
- `default_registry.rs:245` 起 `test_plugin_count_matches_features` 的 protocol-db 分支（原 `expected += 6`）已随 feature 移除删除
- `tests/cli/` 下 db_mysql / db_sqlite / db_postgres / db_transaction 4 个 db 流程夹具——M0 后 cargo test 会失败，需处置策略（保留作 M1 兼容门，测试注入 `MPE_PLUGIN_DIR` 指向插件二进制；或标注 `#[ignore]` 直到插件可用）

### 4.6 可选小修

`SidecarPlugin::resolve_port` 对 `None` 返回 `"true"`（`sidecar.rs:133`），内置 db 返回 `"false"`（`flow-engine-db/src/plugin.rs:96`）——仅报告 `used_port_id` 展示差异，非路由问题。顺手对齐。

### 4.7 宿主前端 `config_schema` 管道

**现状（已实施，§0.3 修订）**：§3.3 依赖的"`config_schema` 驱动 `SchemaConfigPanel` 动态表单"管道已落地：
- 后端 `NodeMetadata`（`flow-engine-core/src/plugin.rs:64`）**已含 `config_schema` 字段**
- `all_metadata()`（`registry.rs:51`）透传 schema
- `get_registered_node_types`（`tauri_commands.rs:3514`）返回含 schema 的 metadata；`get_node_config_schema` IPC（`tauri_commands.rs:3523`）亦存在
- 前端 `cachedMetadata` 由该 IPC 填充（`NodeRegistry.ts:34`），`NodeConfigPanel.tsx:331` 经 `meta?.config_schema` 驱动 `SchemaConfigPanel` 动态表单（db 插件节点即经此渲染）

**必选管道（本文档选定方案 i）**：

> **方案 i**：`NodeMetadata` 增加 `config_schema: Option<serde_json::Value>` 字段，`all_metadata()` 透传各插件 `config_schema()` 返回值，随 `get_registered_node_types` **一次性下发**。前端 `cachedMetadata` 即含 schema，无额外 IPC、无加载态。
>
> 备选方案 ii：前端在 `meta.config_schema` 缺失时按 type_id 懒调用 `get_node_config_schema`——需新增 loading 态与缓存，mock 层需同步；本次**不采用**。

**验证（通过）**：`get_registered_node_types` 返回的 db 节点 metadata 带 `config_schema`；前端选中 db 节点后 `SchemaConfigPanel` 渲染非空动态表单（driver 下拉 + connection_string + SQL 多行 + params 数组）。

### 4.8 `wrapper.rs` 宿主默认路由（兑现 SDK 空 next_ports 契约）— 已实施

- **位置**：`flow-engine-plugin/src/wrapper.rs:33`（`default_success_port`）+ `wrapper.rs:92`（`map_execute_response`，含 `default_success_port` 参数）+ `sidecar.rs:225`（`SidecarPlugin::execute` 从 describe ports 推导传入）。
- **契约**：SDK（`mpe-plugin-sdk/src/types.rs:91-92`）声明 `next_ports` 为 "Explicitly chosen next ports; **empty lets the host apply its default routing**"——空数组 = 宿主应用默认路由。此前宿主对空 `next_ports` 得空数组 → `executor.rs` 对"空 next + 端口路由节点"判 `None` → **break 静默终止**（最小插件让流程在第一个成功节点断链）。
- **实现**：`default_success_port(ports)` 从 describe ports 推导：ports 含 `"true"` → `"true"`；否则 ports 中唯一的非 `"in"` 端口 → 该端口（如 `db:close` 的 `"out"`）；否则 `None`。success 且 `next_ports` 缺失/为空时，用默认端口替换空数组后再 `resolve_next_node_ids`。
- **语义**：插件显式返回 `next_ports` 仍覆盖默认路由（多分支节点）；二态/单端口节点零负担——下一跳归属宿主（`resolve_next_node_ids` 按 `connections` 解析，插件永不接触流程拓扑）。
- **验证（通过）**：`ExecuteResult::ok(...)` 最小插件 → db_connect 成功后流程继续（不 break）；插件显式 `next_ports` 仍生效；sidecar_tests 覆盖默认端口注入与显式覆盖。

---

## 5. 宿主前端移除清单

| 位置 | 移除内容 |
|---|---|
| `src/types/` | NodeData union 的 db 6 类型 + db config 类型 + 类型守卫（`isDbConnectConfig` 等） |
| `src/components/canvas/nodes/Db{Connect,Query,Begin,Commit,Rollback,Close}Node.tsx` | **画布层 6 个 db 节点组件**（评审补充遗漏）+ `FlowCanvas`/`NodePalette`/`utils.ts` 的 db 引用 |
| `src/lib/NodeRegistry.ts` | db 的 sortOrder（236-241 行）+ badge 配色（271-276 行） |
| `src/lib/configPanelRegistry.ts` | db 配置面板注册（db 节点改走 `SchemaConfigPanel`） |
| `src/lib/viewerRegistry.tsx` | db 查看器（**已核实：当前无 db 条目，db 早已走 `?? GenericViewer` 兜底 → 实际为空操作**，仅确认不误删其他条目） |
| `src/templates/builtinFlows.ts` | 2 处 db 内置模板（61-70、409-418 行） |
| `src/__tests__/types/node.test.ts` | db 类型守卫测试 |

**连锁**：
- `NodeType` union 不含 db 后清理所有引用，`npm run typecheck` 必须通过
- 后端 core 的 db 变体已随 §0.2 **彻底删除**（非路线 Y），前后端一致无 db 类型残留
- `SchemaConfigPanel` 动态表单**依赖 §4.7 config_schema 管道先行落地**（否则 `meta.config_schema` 恒为 `undefined`，面板渲染空白 "no fields" 占位）；管道未达标准前 db 面板**不得移除**（见 M3 决策门、§9 风险 3）
- **刻意验证点**：db 级复杂配置（driver 下拉/connection_string/SQL 多行）能否被 `SchemaConfigPanel` 承载

---

## 6. 关键执行链路

### 6.1 启动挂载

```
宿主启动 → plugins_dir()（MPE_PLUGIN_DIR 或 app_data_dir/plugins）
        → PluginManager::scan_and_describe()
        → db 插件 describe → 6 节点 → decide_registration（reserved 已移出 → 放行；**注：第二检查 `registry_has_entry && !claimed` 依赖 §4.5 移除内置 db 同步落地**——仅移 reserved 不移 protocol-db，registry 仍有内置 db 会拒绝插件注册）
        → SidecarPlugin 注册进共享 NodeRegistry
        → get_registered_node_types IPC → 前端节点库出现 6 节点
```

### 6.2 一次流程执行（db_connect → db_query）

```
db_connect execute:
  executor → registry.get("db_connect") → SidecarPlugin::execute
  → config 提取（通用化，剥 type）→ resolve_variables（{{var}} → 最终值）
  → params {config, execution_id=execution_instance_id, ...}
  → 插件：create_pool((conn_id, run_id)) → 幂等建池（同目标复用 / 异目标告警替换）→ test_query
    conn_id = config.connection_uuid 非空则用它，空则兜底连接节点 uuid（§3.4）
  → ExecuteResult::ok(output) → map_execute_response → Ok → true 端口

db_query execute（同 run_id）:
  → 插件 get_pool((connection_uuid, run_id)) → 执行 SQL → 结果 output_data / variable_updates
  → 失败 → ExecuteResult::fail(msg) → map_execute_response → Err(ErrorDetail)
  → executor 外层 on_error：RouteToFalse → false 分支 / Ignore → true 继续 / AbortFlow → 终止

流程结束:
  → (conn_id, run_id) 池无访问 → 插件空闲回收（~30s）→ close + remove
```

---

## 7. 实施顺序

| 阶段 | 内容 | 完成标志 | 决策门 |
|---|---|---|---|
| M0 | **宿主前置 + 桩插件**：§4.1 reserved 移出 db 6 个 type_id + §4.2 sidecar config 提取通用化 + §4.5 feature 裁剪（含 mpe-cli 转发删除、workspace-hack 再生成）+ **§4.8 宿主默认路由**（wrapper `default_success_port`，兑现 SDK 空 next_ports 契约）+ **describe-only 桩插件**（6 个 type_id / 端口 / default_config / config_schema，execute → `fail("not implemented")`） | `scan_and_describe` 成功注册 6 节点、`collision_reports()` 为空；桩插件 `ExecuteResult::ok(...)` 最小形态 → 流程不静默断链（§4.8 生效） | **注册成功才进 M1** |
| M1 | **插件功能**：插件内实现 SQLite connect/query，端到端可查询；**前端 db 面板仍在，不提前移除** | CLI/`run-node` 用**存量流程文件**端到端查询成功（注：`mpe run` 已接插件；`validate`/`run-node` 尚未注入插件 registry，见 §9 风险 10） | **通过才进 M2** |
| M2 | **宿主 only**：§4.3 `execution_instance_id` + §4.4 errors 兼容字符串数组 + **§4.7 config_schema 管道**（纯宿主改动） | 同流程连续两次执行 `execution_id` 不同；SQL 错误 → 报告显示**真实错误消息**；`get_registered_node_types` 返回 db 节点 metadata 带 `config_schema` | **通过才进 M3** |
| M3 | **前端收尾（最后）**：§5 前端 db 移除，gated on §9 风险 3（`SchemaConfigPanel` **非空渲染 + 可保存**才允许移除）；落地分发 + §8 完整验收 | §8 全部通过 | **面板不达标 → 回滚 §5** |

---

## 8. 验收标准

每项以**可执行动词**开头，均须**实际运行**验证（非代码审查）：

1. **验证**编译链路：`cargo check -p app` 通过；`cargo tree -p app` 无 `flow-engine-db`；`cargo tree -i sqlx -p app` **0 命中**（`workspace-hack` 经 `cargo hakari generate` 后 sqlx×4 无条件依赖移除，sqlx 完全退出宿主编译图——非"仅经 workspace-hack 出现"，那是改造前状态）；`get_registered_node_types` 返回中**无内置 `db_*` type_id**（db 节点仅以插件 describe 注册的 sidecar 形式出现，见 §6.1）
2. **编译**独立插件：`plugins/db` 独立编译出 `mpe-plugin-db` 二进制（不依赖宿主**内部类型**，仅依赖 `mpe-plugin-sdk` + wire 契约：`cargo build --manifest-path plugins/db/Cargo.toml`）
3. **扫描**并注册：宿主启动扫描 → 6 个 db 节点注册、`collision_reports()` 为空；GUI 打开 db 节点 → `SchemaConfigPanel` 渲染**非空**动态表单（driver 下拉 + connection_string + test_query）——验证 §4.7 config_schema 管道；另**执行**多插件遮蔽测试：同目录放置真 db 插件 + 声明相同 type_id 的冒充/冲突插件 → 真插件**确定性**获得全部 6 个 type_id、冒充者被拒绝、`collision_reports()` 给出明确冲突原因（§4.1 所有权机制）
4. **构建**前端：db 移除后 `npm run typecheck` + `npm run build` 通过
5. **核对**失败报告：SQL 错误 → 报告显示**真实错误消息**（对象错误形状 + §4.4 兼容垫片）
6. **校验**策略与字段：on_error 三策略（RouteToFalse/Ignore/AbortFlow）与内置行为一致；`mpe validate` 对插件节点做**同步结构校验**（config 为对象/空值）；插件字段级 async `validate` RPC 尚未接线——缺失 `connection_uuid` / 空 `sql` / 缺失 `driver` 目前在执行期报错而非 validate 期（已知缺口）。**空 `connection_uuid` 运行期语义（§3.4 修订）**：`db_connect` 空 uuid 兜底建池；其余 5 节点空 uuid → `fail("connection_uuid is required")` → 默认 RouteToFalse 走 false 端口、Ignore 走 true 继续、AbortFlow 终止——并在报告中给出**显式错误消息**（与内置 commit/rollback/close 的静默 skip、query/begin 的 Ok+false 输出均为**有意差异**）
7. **验证**生命周期：
   (a) `db_connect → 模拟 >30s 慢步骤 → db_query` 成功（池**未被中途回收**，§3.6 方案 (b) `flow_end` 通知 + Active 事务豁免生效）
   (b) 流程结束后，池在 `idle_timeout` 窗口内被回收（最坏 ≤ idle_timeout + reaper 周期，如 30s + 10s tick = 40s）
   (c) 同流程连续运行 N 次 → **无 contains_key 错误**，N 个池最终全部回收（execution_id 隔离 + §3.6 全局池总数上限）
   (d) `begin → 慢步骤 → commit` 长事务**不被中途销毁**（服务端无隐式回滚，§3.6 方案 (b) + Active 事务豁免）
8. **验证**并发：
   (a) 同执行内两个 connect 节点显式填**相同** `connection_uuid` → 第 2 个**幂等复用**（相同目标不替换、不同目标告警替换；无 Err 路径，§3.4 修订），后续查询共用同一池。**注**：原设计断言"第 2 个返回 Err"未采纳；存量流程两 connect 必为不同 node uuid，不触发
   (b) 两个不同 `execution_id` 流程并发执行 → **独立池、无数据串扰**
   (c) 压测 100 用户 × 迭代 → 连接数**有界**，不耗尽服务端 `max_connections`（**§3.6 全局池总数上限生效**，峰值连接 ≤ 服务端 `max_connections` 的 50%）
9. **验证**分发：全新 NSIS/DMG 安装 → `plugins` 目录含 `mpe-plugin-db`，Windows 上 6 节点可用（验证 `.exe` 启动路径，§3.2 / §11）；**seed 首启重写 `plugin.json` 绝对路径生效**（§11 第 3 步）。`plugins/` 是否入 git 跟踪属**仓库卫生说明**，与发布验收无关
10. **执行** SQLite 端到端（用户自行验证）：connect/query/begin/commit/rollback/close 全链路
11. **验证**事务路由：事务内 `db_query` 命中**事务专用连接**（服务端可观测副作用——事务内 update 提交前对外不可见 / 连接级审计，§3.5 路由规则）
12. **验证**宿主默认路由（§4.8）：最小插件形态（`ExecuteResult::ok(output)` 不返回 `next_ports`）执行 db_connect → db_query → db_close 全链路，流程**不**因空 `next_ports` 静默断链（executor 不落 "no matching port connection" 终止分支）；插件显式 `next_ports:["out"]` 仍覆盖默认路由

---

## 9. 风险与已知权衡

| # | 风险/权衡 | 说明 |
|---|---|---|
| 1 | `entry.command` 启动（CWD + Windows `.exe`） | `process.rs` 不设 `current_dir()`，相对路径按**宿主 CWD** 解析（plugin-sdk.md:262）；Windows `CreateProcessW` 仅对**无路径分隔符**的模块名自动补 `.exe`——含路径且无扩展名（如 `<二进制路径>/mpe-plugin-db`）必然 `ERROR_FILE_NOT_FOUND`。对策：`entry.command` **一律绝对路径（Windows 带 `.exe`）** 或裸命令名（见 §3.2） |
| 2 | Resident 崩溃丢池 | 连接可靠性绑定插件进程；崩溃重启后 `db_query` 报"连接不存在"（宿主 restart-once 限流 1s） |
| 3 | SchemaConfigPanel 能力底线 | **已实施增强（§0.3/§3.3 修订）**：面板现支持 `format:'password'` 掩码、`format:'sql'` 多行、`format:'json'`、`x-node-selector` 节点选择器、array JSON 编辑；仍无 oneOf/anyOf/conditional/dependencies。遗留：driver 切换**不改变字段可见性**（sqlite 用户也看到 host/port/username/password） |
| 4 | 前端/后端类型同步偏差 | 后端 core 保留 db 变体、前端删除——有意为之，注释说明 |
| 5 | 空闲回收延迟窗口 | 流程结束后池残留 ≤ idle_timeout（30s）；**方案 (a) 执行边界刷新已证伪**（对抗评审：插件进程只收到自身节点 execute RPC，`db_connect → 宿主慢节点(>30s) → db_query` 窗口内零刷新，reaper 在 T30 必杀池/活跃事务 → §8.7(a)(d) 自证失败）→ **必须落地方案 (b) `flow_end` 通知**（wire 侧 trivial：`mpe-protocol` 的 `Notification{method, params}` 机制已存在，加 method 名即可；真成本在宿主按 run 追踪），或 reaper 跳过含 Active 事务的 key + 可配置大默认 idle_timeout + 全局池上限 |
| 6 | 无界池积累 | 池只限 age 不限 count，每次 run 新 execution_id → 新池 key → 快速重复执行/压测可在分钟级耗尽 MySQL `max_connections=151` / PG `max_connections=100`。**§3.6 三选项无一解决全局耗尽**（≤8 池限单执行内、per-execution TTL 仍是 idle 判定、跨执行复用违反隔离目标且破坏 `:memory:` 语义）→ **强制全局池总数上限**（如 ≤64，LRU 淘汰空闲池），对"有 Active 事务/正在执行"的池豁免 |
| 7 | 执行中回收杀长事务 | 慢步骤（>30s）横亘 connect→query 或 begin→commit 时，纯 idle 回收误杀池/活跃事务（服务端隐式回滚）→ 同风险 5，须方案 (b) 或 Active 事务豁免 |
| 8 | 插件 id 遮蔽/冒充 | db 6 个 type_id 移出 reserved 后任意插件可注册/冒充 db，`fs::read_dir` 无序先到先得 → manifest 声明所有权 + 确定性冲突裁决（§4.1） |
| 9 | 秘密落盘现状 | 密码明文 + `resolved_config`/`flow_snapshot` 脱敏前落盘——存量全局问题，非本次引入，独立工作项（§3.8） |
| 10 | CLI 三通道（validate/debug/run-node）未接插件 | **已解决（§0.5）**：validate 用 `build_shared_registry`（validate.rs:26-27），debug 注入 registry（debug_cmd.rs），run-node 走插件 `single_node` capability——db:connect 流程 `mpe validate` 在插件可发现时 valid。遗留：validate 仅同步结构校验（插件字段级 async `validate` RPC 未接线，见 §8.6），且插件未安装时未知类型错误缺少发现提示（本次已补 hint） |
| 11 | 空 `connection_uuid` 语义有意变更 | 内置 commit/rollback/close 空 uuid 静默 skip 走成功端口（掩盖"事务未提交却成功"）；插件统一空 uuid → 显式报错（§3.4 修订）。存量流程 commit/rollback/close 均带 uuid，不受影响；仅"新建节点未配置即运行"由静默通过变显式报错。Ignore 下流级行为与内置分叉（内置 abort、插件继续走 true）需在 §8.6 断言 |

---

## 10. 参考

- `docs/plugin-sdk.md` — 插件作者指南（mpe_plugin_main! / describe / execute）
- `docs/plugin-protocol.md` — wire 规范（framing/超时/反压）
- `docs/plugin-architecture.md` — 架构决策（OD2 冲突策略、A10 变量契约）
- `crates/flow-engine-db/src/` — 被剥离的内置实现（语义参照：pool.rs / db_connect_executor.rs / plugin.rs）
- `crates/mpe-plugin-sdk/src/bin/demo_plugin.rs` — 最小插件骨架参考

---

## 11. 分发（distribution）

**现状缺口（已核实）**：目前**没有任何路径**让终端用户拿到 `mpe-plugin-db`：
- `src-tauri/tauri.conf.json` 的 bundle 仅配置 nsis/dmg targets，**无** `bundle.resources` / `externalBin` / plugins 相关项
- CI 构建/上传仅发布宿主安装包，**不构建、不随包上传**插件二进制
- `plugins_dir()`（`src-tauri/src/lib.rs:171-181`）返回 `PathBuf`，发现逻辑把**缺失目录当空目录**（非错误）——安装包不带插件时表现为"无 db 节点"
- `plugins/db` 源工程**已被 `.gitignore` 忽略**（根 `.gitignore:92` 的 `/plugins/` 规则，已核实；§1 头注同步）——独立工程不随仓库跟踪；CI 若需构建插件，应在 CI 侧单独 checkout/构建（见 §3.1"第三方视角"限定）

**本次分发方案**：

1. **打包进安装器**：`tauri.conf.json` bundle 增加 `resources`（或 `externalBin`），把插件构建产物 `mpe-plugin-db`（`.exe`/`.bin`）作为**插件种子**随安装器分发；保持目录结构 `plugins/db/mpe-plugin-db[.exe]` + `plugin.json`
2. **CI 构建并上传**：CI 在构建宿主的同时构建 `plugins/db` release 二进制，**作为发布资产随 GitHub Release 上传**（亦支持独立手工安装：解压拷入插件目录即可）
3. **首启种子**：宿主首次启动时，若 `plugins_dir()` 下无 db 插件，则从 bundled seed **创建插件目录并拷贝**一份 `mpe-plugin-db` + `plugin.json`（而非报错）；后续启动走常规 `scan_and_describe`。seed 与安装器版本强绑定（同一 release 构建产物）。**⚠️ Windows 路径硬约束**：seed 内静态 `plugin.json` 的 `entry.command` **无法预写绝对安装路径**（§3.2 要求 Windows 绝对路径带 `.exe`）——首启 seed 必须**按宿主可执行文件所在目录拼绝对路径并重写 `plugin.json` 写回磁盘**（或 process.rs spawn 时对 seed 布局做已知相对解析）；并处理 seed 失败（Program Files 只读 / `MPE_PLUGIN_DIR` 指向只读目录 → 回退报错而非静默）

**验收**：见 §8.9（全新 NSIS/DMG 安装 → 插件目录含 `mpe-plugin-db`，Windows 上 6 节点可用，验证 `.exe` 启动路径）。

**待议**：`bundle.resources` 与 `externalBin` 的选择（resources 保留目录结构、externalBin 附加启动语义）实施时按 Tauri 版本行为定；Windows 插件二进制签名（与安装器同签）亦待定。
