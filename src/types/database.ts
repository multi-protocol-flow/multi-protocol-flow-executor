/**
 * 数据库连接类型定义
 * 与后端 src-tauri/crates/flow-engine/src/db/types.rs 中的结构体对齐
 */

/**
 * 数据库驱动类型
 */
export type DbDriver = 'postgres' | 'mysql' | 'sqlite' | 'redis';

/**
 * 数据库连接配置
 */
export interface DbConnection {
  /** 连接 UUID */
  id: string;
  /** 连接名称（显示用） */
  name: string;
  /** 数据库驱动 */
  driver: DbDriver;

  // 连接参数
  /** 主机地址 */
  host?: string;
  /** 端口号 */
  port?: number;
  /** 数据库名 */
  database?: string;
  /** 用户名 */
  username?: string;
  /** 密码（加密存储） */
  password?: string;
  /** 环境变量名（用于获取密码） */
  password_from_env?: string;
  /** 连接串（可选） */
  connection_string?: string;

  // 连接池配置
  /** 最小连接数（默认 1） */
  min_connections: number;
  /** 最大连接数（默认 10） */
  max_connections: number;
  /** 连接超时时间（毫秒，默认 10000） */
  connect_timeout_ms: number;
  /** 空闲超时时间（毫秒，默认 300000） */
  idle_timeout_ms: number;

  // 运行时状态
  /** 连接状态 */
  status: 'connected' | 'disconnected' | 'error' | 'testing';
  /** 最后错误信息 */
  last_error?: string;
  /** 最后测试时间（时间戳） */
  last_tested_at?: number;
}

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
