//! Flow Engine DB - 数据库模块子 crate
//!
//! 包含数据库连接池管理和 6 个数据库节点执行器。

/// 数据库连接池管理、驱动实现及通用类型定义
pub mod db;
/// 数据库连接节点执行器，建立与数据库的连接并存入连接池
pub mod db_connect_executor;
/// 数据库查询节点执行器，执行 SQL 语句或 Redis 命令并返回结果
pub mod db_query_executor;
/// 事务开始节点执行器，在指定连接上开启事务并设置隔离级别
pub mod db_begin_executor;
/// 事务提交节点执行器，提交当前活跃事务
pub mod db_commit_executor;
/// 事务回滚节点执行器，回滚当前活跃事务
pub mod db_rollback_executor;
/// 连接关闭节点执行器，从连接池中移除并关闭数据库连接
pub mod db_close_executor;
