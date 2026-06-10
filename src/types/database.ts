/**
 * 数据库类型定义
 * 与后端 src-tauri/crates/flow-engine-core/src/db_types.rs 中的结构体对齐
 */

/**
 * 数据库驱动类型
 */
export type DbDriver = 'postgres' | 'mysql' | 'sqlite';

/**
 * 查询获取模式
 */
export type DbFetchMode = 'all' | 'one' | 'scalar';

/**
 * 事务隔离级别
 */
export type DbIsolationLevel =
  | 'default'
  | 'read_uncommitted'
  | 'read_committed'
  | 'repeatable_read'
  | 'serializable';
