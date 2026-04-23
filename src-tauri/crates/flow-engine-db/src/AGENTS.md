# flow-engine-db

**Layer 1** | 16 文件 | ~8,400 行 | 依赖 core

## 概述

数据库连接池 + 6 种数据库节点执行器 + NodePlugin 实现。支持 PostgreSQL、MySQL、SQLite、Redis。

## 结构

```
flow-engine-db/src/
├── lib.rs                     # 模块导出
├── plugin.rs                  # NodePlugin 实现 (555 行)
├── db_connect_executor.rs     # 连接执行器 (~1,027 行)
├── db_query_executor.rs       # 查询执行器 (~1,012 行)
├── db_begin_executor.rs       # 事务开始
├── db_commit_executor.rs      # 事务提交
├── db_rollback_executor.rs    # 事务回滚
├── db_close_executor.rs       # 关闭连接
└── db/                        # 数据库子系统
    ├── mod.rs
    ├── traits.rs              # DatabasePool trait
    ├── types.rs               # IsolationLevel, TransactionState, QueryResult
    ├── pool.rs                # 连接池管理 (820 行)
    ├── postgres.rs            # PostgreSQL 实现 (699 行)
    ├── mysql.rs               # MySQL 实现 (894 行)
    ├── sqlite.rs              # SQLite 实现 (620 行)
    └── redis.rs               # Redis 实现 (835 行)
```

## 快速定位

| 任务 | 文件 |
|------|------|
| NodePlugin 注册 | `plugin.rs` |
| 数据库连接 | `db_connect_executor.rs` |
| SQL/Redis 查询 | `db_query_executor.rs` |
| 连接池管理 | `db/pool.rs` |
| PostgreSQL | `db/postgres.rs` |
| MySQL | `db/mysql.rs` |
| SQLite | `db/sqlite.rs` |
| Redis | `db/redis.rs` |
| 事务隔离级别 | `db/types.rs` |

## 节点类型 (6 种)

| 类型 | 执行器 | 端口 | 用途 |
|------|--------|------|------|
| DbConnect | `db_connect_executor.rs` | true/false | 建立连接 |
| DbQuery | `db_query_executor.rs` | true/false | SQL/Redis 命令 |
| DbBegin | `db_begin_executor.rs` | true/false | 开始事务 |
| DbCommit | `db_commit_executor.rs` | true/false | 提交事务 |
| DbRollback | `db_rollback_executor.rs` | true/false | 回滚事务 |
| DbClose | `db_close_executor.rs` | out | 关闭连接 |

## 关键依赖

- **sqlx**: PostgreSQL/MySQL/SQLite 异步驱动
- **redis**: Redis 异步客户端
- **DatabasePool trait**: 抽象连接池接口，支持不同数据库后端

## 编译影响

修改此 crate → `flow-engine-executor` → `flow-engine` → `app` (~3 crate)

```bash
cd src-tauri
cargo test -p flow-engine-db            # 仅测 DB
cargo clippy -p flow-engine-db          # 仅 Lint DB
```
