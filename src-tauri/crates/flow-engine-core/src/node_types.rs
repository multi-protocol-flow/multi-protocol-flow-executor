//! 节点类型常量模块
//! 提供所有已注册节点类型的字符串常量，用于替代 NodeType enum 匹配

/// 入口节点
pub const ENTRY: &str = "entry";
/// HTTP 请求节点
pub const HTTP: &str = "http";
/// 条件分支节点
pub const CONDITIONAL: &str = "conditional";
/// WebSocket 连接节点
pub const WS_CONNECT: &str = "ws_connect";
/// WebSocket 发送采集节点
pub const WS_SEND_COLLECT: &str = "ws_send_collect";
/// WebSocket 发送等待节点
pub const WS_SEND_WAIT: &str = "ws_send_wait";
/// WebSocket 关闭节点
pub const WS_CLOSE: &str = "ws_close";
/// 断言节点
pub const ASSERTION: &str = "assertion";
/// 脚本节点
pub const SCRIPT: &str = "script";
/// 数据库连接节点
pub const DB_CONNECT: &str = "db_connect";
/// 数据库查询节点
pub const DB_QUERY: &str = "db_query";
/// 数据库事务开始节点
pub const DB_BEGIN: &str = "db_begin";
/// 数据库事务提交节点
pub const DB_COMMIT: &str = "db_commit";
/// 数据库事务回滚节点
pub const DB_ROLLBACK: &str = "db_rollback";
/// 数据库关闭节点
pub const DB_CLOSE: &str = "db_close";
