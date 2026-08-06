# flow-engine-db

**Generated:** 2026-06-12 | **Branch:** dev | **Layer:** L1 (Protocol)
**Dependency:** flow-engine-core | **Files:** 16 | **LOC:** ~8,400

## OVERVIEW

Database protocol crate. Provides 6 DB node types via NodePlugin trait + NodeExecutor impls. Backed by a pool abstraction over sqlx for PostgreSQL, MySQL, and SQLite.

## ARCHITECTURE

```
plugin.rs          → DbPluginGroup::register_all (6 NodePlugin impls, delegates to executors)
db_*_executor.rs   → 6 executor files (DbConnect/DbQuery/DbBegin/DbCommit/DbRollback/DbClose)
db/pool.rs         → DatabaseConnectionManager — per-execution pool registry (ActivePool HashMap, 每次执行新建，无全局长驻)
db/{postgres,mysql,sqlite}.rs → sqlx pool wrappers per driver
db/traits.rs       → DatabasePool / IsolationLevel / QueryResult (re-exported from core)
db/types.rs        → DbConnection / DbDriver / DbConnectionStatus
```

## NODE TYPES

| Type | Executor | Ports | Purpose |
|------|----------|-------|---------|
| db_connect | db_connect_executor | success/failure | Create pool + test query |
| db_query | db_query_executor | success/failure | Run SQL with params, configurable fetch |
| db_begin | db_begin_executor | success/failure | Start transaction, isolation level |
| db_commit | db_commit_executor | success/failure | Commit active transaction |
| db_rollback | db_rollback_executor | success/failure | Rollback active transaction |
| db_close | db_close_executor | out only | Remove pool (idempotent, single port) |

## WHERE TO LOOK

| Task | File |
|------|------|
| DB node plugin definition | plugin.rs |
| Connection logic (create pool, test query, driver dispatch) | db_connect_executor.rs |
| Query execution (fetch modes, param binding, timing) | db_query_executor.rs |
| Transaction begin/commit/rollback | db_begin_executor.rs / db_commit_executor.rs / db_rollback_executor.rs |
| Pool close (remove from registry) | db_close_executor.rs |
| Pool registry (ActivePool HashMap) | db/pool.rs |
| Postgres driver (sqlx) | db/postgres.rs |
| MySQL driver (sqlx) | db/mysql.rs |
| SQLite driver (sqlx) | db/sqlite.rs |
| Pool trait, IsolationLevel, QueryResult | db/traits.rs |
| DbConnection, DbDriver enum | db/types.rs |
