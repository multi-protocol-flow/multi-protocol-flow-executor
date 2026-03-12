# 数据库节点功能需求文档

**版本**: 1.0  
**创建日期**: 2026-03-12  
**状态**: 需求确认完成

---

## 一、概述

### 1.1 背景

当前系统支持 HTTP 和 WebSocket 协议节点，但缺少对数据库操作的支持。数据库操作是企业级流程编排的核心能力，需要支持主流关系型和非关系型数据库。

### 1.2 目标

- 新增数据库节点支持
- 支持主流数据库：PostgreSQL、MySQL/MariaDB、SQLite、Redis
- 支持连接池复用，避免频繁建立连接
- 支持事务操作（BEGIN/COMMIT/ROLLBACK）
- 支持所有 SQL 语句类型（SELECT/INSERT/UPDATE/DELETE/DDL）
- 支持结果提取和变量映射
- 提供独立的数据库连接管理界面
- 集成 Monaco Editor 作为 SQL 编辑器
- 提供查询结果预览功能

### 1.3 核心设计原则

| 原则 | 说明 |
|------|------|
| 连接池复用 | 同一流程内共享连接池，避免频繁建连 |
| 连接集中管理 | 全局连接配置中心，节点通过 UUID 引用 |
| 流程级生命周期 | 连接池随流程创建和销毁，流程结束自动清理 |
| 统一节点接口 | 所有数据库类型使用统一的节点类型，通过 driver 字段区分 |
| 事务用户负责 | 事务超时由用户自行处理，程序不做自动限制 |
| 错误走 false 分支 | 数据库错误记录错误信息，走 false 端口 |

---

## 二、节点设计

### 2.1 节点类型总览

| 节点类型 | 功能 | 输入端口 | 输出端口 |
|----------|------|----------|----------|
| DB Connect | 建立数据库连接（创建连接池） | 1 | 2 (true/false) |
| DB Query | 执行数据库查询/命令 | 1 | 2 (true/false) |
| DB Begin | 开启事务 | 1 | 2 (true/false) |
| DB Commit | 提交事务 | 1 | 2 (true/false) |
| DB Rollback | 回滚事务 | 1 | 2 (true/false) |
| DB Close | 关闭数据库连接（销毁连接池） | 1 | 2 (true/false) |

### 2.2 节点关系图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              全局连接配置中心                                 │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  prod_mysql    │  dev_postgres    │  cache_redis    │  local_sqlite │   │
│  │  (UUID: conn1) │  (UUID: conn2)   │  (UUID: conn3)  │  (UUID: conn4)│   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ 通过 connection_uuid 引用
                                    ▼
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  Entry   │────▶│DB Connect│────▶│ DB Query │────▶│ DB Close │     │          │
└──────────┘     │ (conn1)  │     │ (conn1)  │     │ (conn1)  │     │          │
                 └──────────┘     └──────────┘     └──────────┘     │          │
                                      │                            │          │
                                      │ true/false                 │          │
                                      ▼                            │          │
                 ┌──────────┐     ┌──────────┐     ┌──────────┐     │          │
                 │ DB Begin │────▶│ DB Query │────▶│DB Commit │────▶│          │
                 │ (conn1)  │     │ (conn1)  │     │ (conn1)  │     │          │
                 └──────────┘     └──────────┘     └──────────┘     └──────────┘
                                      │
                                      │ false (事务失败)
                                      ▼
                 ┌──────────────┐
                 │ DB Rollback  │
                 │   (conn1)    │
                 └──────────────┘
```

### 2.3 连接配置中心

独立的数据库连接配置管理界面，位于左侧面板。

**连接配置项**：

| 字段 | 说明 | 必填 |
|------|------|------|
| 连接名称 | 连接标识，节点引用时显示 | 是 |
| 数据库类型 | postgres / mysql / sqlite / redis | 是 |
| 主机地址 | 数据库服务器地址 | 是（sqlite 除外） |
| 端口 | 数据库端口 | 是（sqlite 除外） |
| 数据库名 | 数据库名称 | 是 |
| 用户名 | 连接用户名 | 是（sqlite 除外） |
| 密码 | 连接密码（支持环境变量引用） | 否 |
| 最小连接数 | 连接池最小连接数，默认 1 | 否 |
| 最大连接数 | 连接池最大连接数，默认 10 | 否 |
| 连接超时 | 连接超时时间，默认 10000ms | 否 |
| 空闲超时 | 连接空闲超时，默认 300000ms | 否 |

**密码存储策略**：
- 直接输入：明文存储在配置中（适合开发/测试环境）
- 环境变量：存储环境变量名（如 `DB_PASSWORD`），运行时从环境变量读取

---

## 三、节点详细设计

### 3.1 DB Connect 节点

#### 3.1.1 功能描述

建立数据库连接并创建连接池。连接池在流程级别共享，后续 DB Query 节点通过 connection_uuid 引用。

#### 3.1.2 配置项

```
┌─────────────────────────────────────────────────────────────┐
│  基本配置                                                    │
├─────────────────────────────────────────────────────────────┤
│  连接: prod_mysql  （下拉选择已配置的连接）                    │
│                                                              │
│  显示连接信息（只读）：                                        │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ 类型: MySQL                                          │    │
│  │ 地址: 192.168.1.100:3306                             │    │
│  │ 数据库: production                                    │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  连接超时: 10000 ms  （可选，覆盖全局配置）                    │
├─────────────────────────────────────────────────────────────┤
│  连接后测试（可选）                                           │
│  ☑ 启用                                                      │
│                                                              │
│  测试查询: SELECT 1                                          │
│  （用于验证连接是否成功）                                      │
└─────────────────────────────────────────────────────────────┘
```

#### 3.1.3 输出端口

| 端口 | 条件 | 输出数据 |
|------|------|----------|
| true | 连接成功 | 连接信息 |
| false | 连接失败 | 错误信息 |

#### 3.1.4 输出数据结构

```json
{
  "connection": {
    "uuid": "conn-550e8400",
    "name": "prod_mysql",
    "driver": "mysql",
    "status": "connected"
  },
  "timing": {
    "connect_ms": 25,
    "total_ms": 30
  }
}
```

---

### 3.2 DB Query 节点

#### 3.2.1 功能描述

执行数据库查询或命令。支持所有 SQL 语句类型，也支持 Redis 命令。

#### 3.2.2 配置项

**关系型数据库（PostgreSQL/MySQL/SQLite）**：

```
┌─────────────────────────────────────────────────────────────┐
│  基本配置                                                    │
├─────────────────────────────────────────────────────────────┤
│  连接: prod_mysql  （下拉选择已建立的连接）                    │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  SQL 编辑器 (Monaco Editor)                          │    │
│  │                                                      │    │
│  │  SELECT * FROM users                                 │    │
│  │  WHERE status = {{input.status}}                     │    │
│  │  AND created_at > {{input.start_date}}               │    │
│  │                                                      │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  参数化查询（可选，防注入）：                                   │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ 参数1: {{input.status}}                              │    │
│  │ 参数2: {{input.start_date}}                          │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  获取模式: ○ 全部  ○ 单行  ○ 单值                             │
│                                                              │
│  查询超时: 30000 ms  （默认 30000ms，范围 1000-300000ms）      │
├─────────────────────────────────────────────────────────────┤
│  结果处理                                                    │
│                                                              │
│  整体结果存入变量: db_result  （可选）                         │
│                                                              │
│  字段映射:                                                    │
│  ┌─────────────────────────┬───────────────────────┐        │
│  │ JSONPath                │ 目标变量名             │        │
│  ├─────────────────────────┼───────────────────────┤        │
│  │ $.rows[0].id            │ user_id               │        │
│  │ $.rows[0].name          │ user_name             │        │
│  └─────────────────────────┴───────────────────────┘        │
├─────────────────────────────────────────────────────────────┤
│  危险操作检测                                                 │
│  ⚠️ 此操作将修改数据：DELETE FROM users                       │
│  （自动检测 INSERT/UPDATE/DELETE/DROP/TRUNCATE 等语句）        │
└─────────────────────────────────────────────────────────────┘
```

**Redis 数据库**：

```
┌─────────────────────────────────────────────────────────────┐
│  基本配置                                                    │
├─────────────────────────────────────────────────────────────┤
│  连接: cache_redis  （下拉选择已建立的连接）                   │
│                                                              │
│  命令: GET  （下拉选择）                                       │
│  ├─ 字符串: GET, SET, DEL, INCR, DECR                        │
│  ├─ 哈希: HGET, HSET, HDEL, HGETALL                          │
│  ├─ 列表: LPUSH, RPUSH, LPOP, RPOP, LRANGE                   │
│  ├─ 集合: SADD, SREM, SMEMBERS, SISMEMBER                    │
│  └─ 有序集合: ZADD, ZREM, ZRANGE, ZSCORE                     │
│                                                              │
│  Key: user:{{input.user_id}}                                │
│  Value: {{input.user_data}}  （SET/HSET 等需要）              │
│  TTL (秒): 3600  （可选）                                     │
│                                                              │
│  查询超时: 30000 ms                                           │
├─────────────────────────────────────────────────────────────┤
│  结果处理                                                    │
│                                                              │
│  整体结果存入变量: redis_result                               │
└─────────────────────────────────────────────────────────────┘
```

#### 3.2.3 输出端口

| 端口 | 条件 | 输出数据 |
|------|------|----------|
| true | 查询成功 | 查询结果 |
| false | 查询失败 / 超时 / 语法错误 | 错误信息 |

#### 3.2.4 输出数据结构

**SELECT 查询**：

```json
{
  "success": true,
  "rows": [
    {"id": 1, "name": "Alice", "email": "alice@example.com"},
    {"id": 2, "name": "Bob", "email": "bob@example.com"}
  ],
  "row_count": 2,
  "affected_rows": 0,
  "columns": ["id", "name", "email"],
  "request": {
    "sql": "SELECT * FROM users WHERE status = ?",
    "params": ["active"]
  },
  "timing": {
    "execute_ms": 15,
    "fetch_ms": 5,
    "total_ms": 20
  }
}
```

**INSERT/UPDATE/DELETE**：

```json
{
  "success": true,
  "rows": [],
  "row_count": 0,
  "affected_rows": 5,
  "last_insert_id": 123,
  "request": {
    "sql": "INSERT INTO users (name, email) VALUES (?, ?)",
    "params": ["Charlie", "charlie@example.com"]
  },
  "timing": {
    "execute_ms": 10,
    "total_ms": 10
  }
}
```

**Redis 命令**：

```json
{
  "success": true,
  "result": "cached_value_here",
  "result_type": "string",
  "request": {
    "command": "GET",
    "key": "user:123"
  },
  "timing": {
    "execute_ms": 2,
    "total_ms": 2
  }
}
```

#### 3.2.5 获取模式说明

| 模式 | 说明 | 适用场景 |
|------|------|----------|
| all | 返回所有行 | SELECT 查询，列表数据 |
| one | 返回单行 | 查询单条记录 |
| scalar | 返回单个值 | COUNT(*)、SUM() 等聚合函数 |

---

### 3.3 DB Begin 节点

#### 3.3.1 功能描述

开启数据库事务。事务开启后，后续的 DB Query 节点将在同一事务中执行。

#### 3.3.2 配置项

```
┌─────────────────────────────────────────────────────────────┐
│  基本配置                                                    │
├─────────────────────────────────────────────────────────────┤
│  连接: prod_mysql  （下拉选择已建立的连接）                    │
│                                                              │
│  事务隔离级别（可选）:                                         │
│  ○ 默认（使用数据库默认）                                      │
│  ○ READ UNCOMMITTED                                         │
│  ○ READ COMMITTED                                           │
│  ○ REPEATABLE READ                                          │
│  ○ SERIALIZABLE                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 3.3.3 输出端口

| 端口 | 条件 |
|------|------|
| true | 事务开启成功 |
| false | 事务开启失败（数据库不支持事务等） |

---

### 3.4 DB Commit 节点

#### 3.4.1 功能描述

提交当前事务。

#### 3.4.2 配置项

```
┌─────────────────────────────────────────────────────────────┐
│  基本配置                                                    │
├─────────────────────────────────────────────────────────────┤
│  连接: prod_mysql  （下拉选择有活跃事务的连接）                 │
└─────────────────────────────────────────────────────────────┘
```

#### 3.4.3 输出端口

| 端口 | 条件 |
|------|------|
| true | 提交成功 |
| false | 提交失败（无活跃事务等） |

---

### 3.5 DB Rollback 节点

#### 3.5.1 功能描述

回滚当前事务。

#### 3.5.2 配置项

```
┌─────────────────────────────────────────────────────────────┐
│  基本配置                                                    │
├─────────────────────────────────────────────────────────────┤
│  连接: prod_mysql  （下拉选择有活跃事务的连接）                 │
└─────────────────────────────────────────────────────────────┘
```

#### 3.5.3 输出端口

| 端口 | 条件 |
|------|------|
| true | 回滚成功 |
| false | 回滚失败（无活跃事务等） |

---

### 3.6 DB Close 节点

#### 3.6.1 功能描述

关闭数据库连接并销毁连接池。

#### 3.6.2 配置项

```
┌─────────────────────────────────────────────────────────────┐
│  基本配置                                                    │
├─────────────────────────────────────────────────────────────┤
│  连接: prod_mysql  （下拉选择要关闭的连接）                    │
└─────────────────────────────────────────────────────────────┘
```

#### 3.6.3 输出端口

| 端口 | 条件 |
|------|------|
| true | 关闭成功 |
| false | 关闭失败 |

#### 3.6.4 自动清理

流程执行结束时，自动关闭所有未关闭的连接池，防止资源泄漏。

---

## 四、危险操作处理

### 4.1 SQL 语句分级

| 级别 | 操作类型 | 示例 |
|------|----------|------|
| safe | 只读查询 | SELECT, EXPLAIN |
| warning | 数据修改 | INSERT, UPDATE, DELETE |
| danger | 结构变更/不可逆 | DROP, TRUNCATE, ALTER |

### 4.2 处理策略

1. **实时提示**：SQL 编辑器下方显示风险等级标识
2. **执行确认**：危险操作执行前弹窗确认
3. **全局策略**：用户可选择 warn（警告）/ block（禁止）/ allow（直接执行）

### 4.3 警告示例

```
┌─────────────────────────────────────────────────────────────┐
│  ⚠️ 危险操作                                                 │
├─────────────────────────────────────────────────────────────┤
│  此操作可能不可逆地修改或删除数据：                             │
│                                                              │
│  DROP TABLE users                                           │
│                                                              │
│  建议先备份数据库再执行此操作。                                 │
│                                                              │
│                              [取消]  [确认执行]               │
└─────────────────────────────────────────────────────────────┘
```

---

## 五、调试模式支持

### 5.1 事务在调试模式下的处理

- 调试暂停时，数据库连接保持活跃
- 事务状态保存在连接池管理器中
- 继续执行时，事务连接仍然有效
- 终止调试时，回滚所有未提交事务并关闭连接

### 5.2 调试事件扩展

| 事件 | 说明 |
|------|------|
| DbConnectionCreated | 连接创建成功 |
| DbTransactionStarted | 事务开启 |
| DbQueryExecuted | 查询执行完成 |
| DbTransactionCommitted | 事务提交 |
| DbTransactionRolledBack | 事务回滚 |
| DbConnectionClosed | 连接关闭 |

---

## 六、数据结构定义

### 6.1 连接配置（全局）

```typescript
// src/types/database.ts

export type DbDriver = 'postgres' | 'mysql' | 'sqlite' | 'redis';

export interface DbConnection {
  id: string;                      // 连接 UUID
  name: string;                    // 连接名称（显示用）
  driver: DbDriver;
  
  // 连接参数
  host?: string;
  port?: number;
  database?: string;
  username?: string;
  password?: string;               // 加密存储
  password_from_env?: string;      // 环境变量名
  connection_string?: string;      // 连接串（可选）
  
  // 连接池配置
  min_connections: number;         // 默认 1
  max_connections: number;         // 默认 10
  connect_timeout_ms: number;      // 默认 10000
  idle_timeout_ms: number;         // 默认 300000
  
  // 运行时状态
  status: 'connected' | 'disconnected' | 'error' | 'testing';
  last_error?: string;
  last_tested_at?: number;
}
```

### 6.2 节点配置

```typescript
// src/types/node.ts

// DB Connect 配置
export interface DbConnectConfig {
  type: 'db_connect';
  connection_uuid: string;         // 引用全局连接配置
  test_query?: string;             // 连接后测试查询
  connect_timeout_ms?: number;     // 覆盖全局超时
}

// DB Query 配置
export interface DbQueryConfig {
  type: 'db_query';
  connection_uuid: string;         // 引用已建立的连接
  
  // SQL（关系型）
  sql?: string;
  params?: string[];               // 参数化查询
  
  // Redis
  redis_command?: string;
  redis_key?: string;
  redis_value?: string;
  redis_ttl?: number;
  
  // 结果处理
  fetch_mode?: 'all' | 'one' | 'scalar';
  result_to?: string;              // 整体结果存入变量
  output_mappings?: OutputMapping[];
  timeout_ms?: number;
}

// DB Begin 配置
export interface DbBeginConfig {
  type: 'db_begin';
  connection_uuid: string;
  isolation_level?: 'default' | 'read_uncommitted' | 'read_committed' | 'repeatable_read' | 'serializable';
}

// DB Commit 配置
export interface DbCommitConfig {
  type: 'db_commit';
  connection_uuid: string;
}

// DB Rollback 配置
export interface DbRollbackConfig {
  type: 'db_rollback';
  connection_uuid: string;
}

// DB Close 配置
export interface DbCloseConfig {
  type: 'db_close';
  connection_uuid: string;
}
```

### 6.3 Rust 后端结构

```rust
// node.rs
pub enum NodeType {
    // ... 现有类型
    #[serde(rename = "db_connect")]
    DbConnect,
    #[serde(rename = "db_query")]
    DbQuery,
    #[serde(rename = "db_begin")]
    DbBegin,
    #[serde(rename = "db_commit")]
    DbCommit,
    #[serde(rename = "db_rollback")]
    DbRollback,
    #[serde(rename = "db_close")]
    DbClose,
}

// node_config.rs
pub enum NodeData {
    // ... 现有类型
    DbConnect(DbConnectConfig),
    DbQuery(DbQueryConfig),
    DbBegin(DbBeginConfig),
    DbCommit(DbCommitConfig),
    DbRollback(DbRollbackConfig),
    DbClose(DbCloseConfig),
}

// db/pool.rs
pub struct DatabaseConnectionManager {
    pools: RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>,
    transactions: RwLock<HashMap<String, TransactionState>>,
}

pub struct TransactionState {
    pub connection_uuid: String,
    pub started_at: u64,
    pub started_by_node: String,
    pub is_active: bool,
}

// context.rs 扩展
pub struct ExecutionContext {
    // ... 现有字段
    pub db_manager: Option<Arc<DatabaseConnectionManager>>,
}
```

---

## 七、前端组件

### 7.1 新增文件

```
src/
├── types/
│   └── database.ts              # 数据库类型定义
├── stores/
│   └── connections.ts           # 连接管理状态
├── components/
│   ├── sidebar/
│   │   └── ConnectionsPanel.tsx # 连接列表面板
│   ├── panels/
│   │   ├── DbConnectConfig.tsx  # 连接节点配置
│   │   ├── DbQueryConfig.tsx    # 查询节点配置
│   │   ├── DbBeginConfig.tsx    # 事务开始配置
│   │   ├── DbCommitConfig.tsx   # 事务提交配置
│   │   ├── DbRollbackConfig.tsx # 事务回滚配置
│   │   └── DbCloseConfig.tsx    # 关闭连接配置
│   ├── canvas/nodes/
│   │   ├── DbConnectNode.tsx    # 连接节点画布组件
│   │   ├── DbQueryNode.tsx      # 查询节点画布组件
│   │   ├── DbBeginNode.tsx      # 事务开始节点
│   │   ├── DbCommitNode.tsx     # 事务提交节点
│   │   ├── DbRollbackNode.tsx   # 事务回滚节点
│   │   └── DbCloseNode.tsx      # 关闭连接节点
│   ├── dialogs/
│   │   └── ConnectionDialog.tsx # 新建/编辑连接对话框
│   └── ui/
│       ├── SqlEditor.tsx        # SQL 编辑器（Monaco）
│       └── DbResultPreview.tsx  # 结果预览组件
```

### 7.2 连接管理面板

位置：左侧边栏，新增折叠面板

```
┌─────────────────────────────────┐
│  流程列表                        │
│  ─────────────────────────────  │
│  节点库                          │
│  ─────────────────────────────  │
│  变量                            │
│  ─────────────────────────────  │
│  数据库连接                  [+] │
│  ┌───────────────────────────┐  │
│  │ ● prod_mysql              │  │
│  │   MySQL · 192.168.1.100   │  │
│  ├───────────────────────────┤  │
│  │ ○ dev_postgres            │  │
│  │   PostgreSQL · localhost  │  │
│  ├───────────────────────────┤  │
│  │ ○ cache_redis             │  │
│  │   Redis · 127.0.0.1       │  │
│  └───────────────────────────┘  │
└─────────────────────────────────┘
```

---

## 八、后端实现

### 8.1 新增文件

```
src-tauri/crates/flow-engine/src/
├── db/
│   ├── mod.rs                   # 模块导出
│   ├── pool.rs                  # 连接池管理器
│   ├── traits.rs                # 数据库抽象 trait
│   ├── postgres.rs              # PostgreSQL 实现
│   ├── mysql.rs                 # MySQL 实现
│   ├── sqlite.rs                # SQLite 实现
│   └── redis.rs                 # Redis 实现
├── db_connect_executor.rs       # 连接执行器
├── db_query_executor.rs         # 查询执行器
├── db_begin_executor.rs         # 事务开始执行器
├── db_commit_executor.rs        # 事务提交执行器
├── db_rollback_executor.rs      # 事务回滚执行器
└── db_close_executor.rs         # 关闭连接执行器
```

### 8.2 依赖

```toml
# src-tauri/Cargo.toml
[dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "mysql", "sqlite"] }
redis = { version = "0.27", features = ["tokio-comp", "connection-manager"] }
```

```json
// package.json
{
  "dependencies": {
    "@monaco-editor/react": "^4.6.0"
  }
}
```

---

## 九、验证规则

### 9.1 保存时验证

| 规则 | 错误级别 | 说明 |
|------|----------|------|
| DB Query 必须引用存在的连接 | 错误 | connection_uuid 必须对应一个 DB Connect 节点 |
| DB Query 必须在连接之后 | 错误 | 不能在 DB Connect 之前使用连接 |
| DB Close 必须在连接之后 | 错误 | 不能在 DB Connect 之前关闭连接 |
| 事务节点必须引用同一连接 | 警告 | Begin/Commit/Rollback 应使用同一连接 |

### 9.2 执行时验证

| 规则 | 处理方式 |
|------|----------|
| 引用的连接不存在 | 走 false 端口 |
| 连接已关闭 | 走 false 端口 |
| 无活跃事务时 Commit/Rollback | 走 false 端口 |
| SQL 语法错误 | 走 false 端口 |
| 查询超时 | 走 false 端口 |

---

## 十、实施计划

### 10.1 阶段划分

| 阶段 | 内容 | 预估时间 |
|------|------|----------|
| P0 | 类型定义 + 后端骨架 | 0.5 天 |
| P0 | 连接池管理器 + PostgreSQL/MySQL 实现 | 2 天 |
| P0 | DB Connect / DB Query / DB Close 节点 | 1.5 天 |
| P1 | 前端配置面板 + 节点组件（6个） | 2 天 |
| P1 | Monaco Editor + 结果预览 | 1 天 |
| P1 | 连接管理面板 + 对话框 | 1 天 |
| P2 | 事务支持（Begin/Commit/Rollback） | 1 天 |
| P2 | 调试模式集成 | 0.5 天 |
| P2 | Redis 支持 | 1 天 |
| P3 | SQLite 支持 | 0.5 天 |
| P3 | 单元测试 + 集成测试 | 1.5 天 |

**总计**：约 12-13 天

### 10.2 依赖关系

```
P0 (类型定义) 
  ├─→ P0 (连接池管理器) ─→ P0 (核心三节点)
  │                              ↓
  └─→ P1 (前端组件) ←────────────┘
                ↓
          P1 (Monaco/预览)
                ↓
          P1 (连接管理面板)
                ↓
          P2 (事务支持)
                ↓
          P2 (调试集成)
                ↓
          P2/P3 (Redis/SQLite)
                ↓
          P3 (测试)
```

---

## 十一、注意事项

1. **连接池生命周期**：流程级，流程结束自动清理
2. **大结果集**：当前全量返回，后续迭代考虑限制
3. **批量操作**：不支持，用户自行通过条件节点实现循环
4. **事务超时**：用户自行负责，程序不做自动限制
5. **错误处理**：所有数据库错误走 false 端口，不自动重试
6. **敏感信息**：支持环境变量引用，避免明文存储密码

---

**文档结束**
