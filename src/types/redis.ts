/**
 * Redis 节点配置类型
 * 与后端 flow-engine-core/src/node_config.rs 中的 Redis*Config 结构体对齐
 */

/** Redis 连接节点配置 */
export interface RedisConnectConfig {
  type: 'redis_connect';
  /** 服务器地址 */
  host: string;
  /** 端口（支持 {{variable}} 引用） */
  port: string;
  /** 密码 */
  password?: string;
  /** 数据库索引 */
  db_index: number;
  /** 连接超时（毫秒） */
  connect_timeout_ms: number;
}

/** Redis 命令节点配置 */
export interface RedisCommandConfig {
  type: 'redis_command';
  /** 连接 ID */
  connection_id: string;
  /** Redis 命令 */
  command: string;
  /** 命令参数 */
  args: string[];
}

/** Redis 关闭连接节点配置 */
export interface RedisCloseConfig {
  type: 'redis_close';
  /** 连接 ID */
  connection_id: string;
}
