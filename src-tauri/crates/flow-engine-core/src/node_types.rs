//! 节点类型常量模块
//! 提供所有已注册节点类型的字符串常量，用于替代 NodeType enum 匹配

/// 入口节点
pub const ENTRY: &str = "entry";
/// HTTP 请求节点
pub const HTTP: &str = "http";
/// 条件分支节点
pub const CONDITIONAL: &str = "conditional";
/// 结束节点
pub const END: &str = "end";
/// WebSocket 连接节点
pub const WS_CONNECT: &str = "ws_connect";
/// WebSocket 发送采集节点
pub const WS_SEND_COLLECT: &str = "ws_send_collect";
/// WebSocket 关闭节点
pub const WS_CLOSE: &str = "ws_close";
/// 断言节点
pub const ASSERTION: &str = "assertion";
/// 脚本节点
pub const SCRIPT: &str = "script";
/// gRPC 连接节点
/// gRPC 调用节点
/// gRPC 关闭节点
/// TCP 连接节点
pub const TCP_CONNECT: &str = "tcp_connect";
/// TCP 发送节点
pub const TCP_SEND: &str = "tcp_send";
/// TCP 接收节点
pub const TCP_RECEIVE: &str = "tcp_receive";
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
/// GraphQL Schema 探索节点
pub const GRAPHQL_INTROSPECT: &str = "graphql_introspect";
/// GraphQL 断开连接节点
pub const GRAPHQL_DISCONNECT: &str = "graphql_disconnect";
/// 子流程节点
pub const SUB_FLOW: &str = "sub_flow";
