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
/// gRPC 连接节点
pub const GRPC_CONNECT: &str = "grpc_connect";
/// gRPC 调用节点
pub const GRPC_CALL: &str = "grpc_call";
/// gRPC 关闭节点
pub const GRPC_CLOSE: &str = "grpc_close";
/// MCP 连接节点
pub const MCP_CONNECT: &str = "mcp_connect";
/// MCP 调用工具节点
pub const MCP_CALL_TOOL: &str = "mcp_call_tool";
/// MCP 读取资源节点
pub const MCP_READ_RESOURCE: &str = "mcp_read_resource";
/// MCP 关闭连接节点
pub const MCP_CLOSE: &str = "mcp_close";
/// MCP 获取提示词节点
pub const MCP_GET_PROMPT: &str = "mcp_get_prompt";
/// TCP 连接节点
pub const TCP_CONNECT: &str = "tcp_connect";
/// TCP 发送节点
pub const TCP_SEND: &str = "tcp_send";
/// TCP 关闭节点
pub const TCP_CLOSE: &str = "tcp_close";
/// UDP 绑定节点
pub const UDP_BIND: &str = "udp_bind";
/// UDP 发送节点
pub const UDP_SEND_TO: &str = "udp_send_to";
/// UDP 接收节点
pub const UDP_RECV_FROM: &str = "udp_recv_from";
/// UDP 关闭节点
pub const UDP_CLOSE: &str = "udp_close";
/// MQTT 连接节点
pub const MQTT_CONNECT: &str = "mqtt_connect";
/// MQTT 发布节点
pub const MQTT_PUBLISH: &str = "mqtt_publish";
/// MQTT 订阅节点
pub const MQTT_SUBSCRIBE: &str = "mqtt_subscribe";
/// MQTT 取消订阅节点
pub const MQTT_UNSUBSCRIBE: &str = "mqtt_unsubscribe";
/// MQTT 断开连接节点
pub const MQTT_DISCONNECT: &str = "mqtt_disconnect";
/// MQTT 等待消息节点
pub const MQTT_WAIT_MESSAGE: &str = "mqtt_wait_message";
/// MQTT 请求-响应节点
pub const MQTT_REQUEST: &str = "mqtt_request";
// SSE 节点类型
/// SSE 连接节点
pub const SSE_CONNECT: &str = "sse_connect";
/// SSE 监听事件节点
pub const SSE_LISTEN: &str = "sse_listen";
/// SSE 断开连接节点
pub const SSE_DISCONNECT: &str = "sse_disconnect";
// GraphQL 节点类型
/// GraphQL 连接节点
pub const GRAPHQL_CONNECT: &str = "graphql_connect";
/// GraphQL 查询节点
pub const GRAPHQL_QUERY: &str = "graphql_query";
/// GraphQL 订阅节点
pub const GRAPHQL_SUBSCRIBE: &str = "graphql_subscribe";
/// GraphQL 取消订阅节点
pub const GRAPHQL_UNSUBSCRIBE: &str = "graphql_unsubscribe";
/// GraphQL Schema 探索节点
pub const GRAPHQL_INTROSPECT: &str = "graphql_introspect";
/// GraphQL 断开连接节点
pub const GRAPHQL_DISCONNECT: &str = "graphql_disconnect";
// AMQP 0-9-1 节点类型
/// AMQP 连接节点
pub const AMQP_CONNECT: &str = "amqp_connect";
/// AMQP 关闭连接节点
pub const AMQP_CLOSE: &str = "amqp_close";
/// AMQP 声明交换器节点
pub const AMQP_EXCHANGE_DECLARE: &str = "amqp_exchange_declare";
/// AMQP 删除交换器节点
pub const AMQP_EXCHANGE_DELETE: &str = "amqp_exchange_delete";
/// AMQP 声明队列节点
pub const AMQP_QUEUE_DECLARE: &str = "amqp_queue_declare";
/// AMQP 删除队列节点
pub const AMQP_QUEUE_DELETE: &str = "amqp_queue_delete";
/// AMQP 绑定队列节点
pub const AMQP_QUEUE_BIND: &str = "amqp_queue_bind";
/// AMQP 发布消息节点
pub const AMQP_PUBLISH: &str = "amqp_publish";
/// AMQP 消费消息节点
pub const AMQP_CONSUME: &str = "amqp_consume";
/// AMQP 获取消息节点
pub const AMQP_GET: &str = "amqp_get";
/// AMQP 确认消息节点
pub const AMQP_ACK: &str = "amqp_ack";
/// AMQP 管理节点
pub const AMQP_MANAGEMENT: &str = "amqp_management";
// AMQP 1.0 节点类型
/// AMQP 1.0 连接节点
pub const AMQP10_CONNECT: &str = "amqp10_connect";
/// AMQP 1.0 发送消息节点
pub const AMQP10_SEND: &str = "amqp10_send";
/// AMQP 1.0 接收消息节点
pub const AMQP10_RECEIVE: &str = "amqp10_receive";
/// AMQP 1.0 确认消息节点
pub const AMQP10_ACCEPT: &str = "amqp10_accept";
/// AMQP 1.0 拒绝消息节点
pub const AMQP10_REJECT: &str = "amqp10_reject";
/// AMQP 1.0 断开连接节点
pub const AMQP10_DISCONNECT: &str = "amqp10_disconnect";
/// 变量提取节点
pub const VARIABLE_EXTRACTOR: &str = "variable_extractor";
