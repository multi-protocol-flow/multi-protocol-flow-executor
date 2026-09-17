//! 节点类型常量模块
//! 提供所有已注册节点类型的字符串常量，用于替代 NodeType enum 匹配

/// 入口节点
pub const ENTRY: &str = "entry";
/// HTTP 请求节点
pub const HTTP: &str = "http:request";
/// 条件分支节点
pub const CONDITIONAL: &str = "conditional";
/// 结束节点
pub const END: &str = "end";
/// WebSocket 连接节点
pub const WS_CONNECT: &str = "ws:connect";
/// WebSocket 发送采集节点
pub const WS_SEND_COLLECT: &str = "ws:send_collect";
/// WebSocket 关闭节点
pub const WS_CLOSE: &str = "ws:close";
/// 断言节点
pub const ASSERTION: &str = "assertion";
/// 脚本节点
pub const SCRIPT: &str = "script";
/// gRPC 连接节点
/// gRPC 调用节点
/// gRPC 关闭节点
/// TCP 连接节点
pub const TCP_CONNECT: &str = "tcp:connect";
/// TCP 发送节点
pub const TCP_SEND: &str = "tcp:send";
/// TCP 接收节点
pub const TCP_RECEIVE: &str = "tcp:receive";
/// TCP 关闭节点
pub const TCP_CLOSE: &str = "tcp:close";
/// UDP 绑定节点
pub const UDP_BIND: &str = "udp:bind";
/// UDP 发送节点
pub const UDP_SEND_TO: &str = "udp:send_to";
/// UDP 接收节点
pub const UDP_RECV_FROM: &str = "udp:recv_from";
/// UDP 关闭节点
pub const UDP_CLOSE: &str = "udp:close";
// SSE 节点类型
/// SSE 连接节点
pub const SSE_CONNECT: &str = "sse:connect";
/// SSE 监听事件节点
pub const SSE_LISTEN: &str = "sse:listen";
/// SSE 断开连接节点
pub const SSE_DISCONNECT: &str = "sse:disconnect";
// GraphQL 节点类型
/// GraphQL 连接节点
pub const GRAPHQL_CONNECT: &str = "graphql:connect";
/// GraphQL 查询节点
pub const GRAPHQL_QUERY: &str = "graphql:query";
/// GraphQL 订阅节点
pub const GRAPHQL_SUBSCRIBE: &str = "graphql:subscribe";
/// GraphQL Schema 探索节点
pub const GRAPHQL_INTROSPECT: &str = "graphql:introspect";
/// GraphQL 断开连接节点
pub const GRAPHQL_DISCONNECT: &str = "graphql:disconnect";
/// 子流程节点
pub const SUB_FLOW: &str = "flow:sub";
/// HTTP Ingress 监听节点（Mock 服务入口）
pub const HTTP_INGRESS: &str = "http:ingress";
/// HTTP Respond 响应节点（Mock 服务出口）
pub const HTTP_RESPOND: &str = "http:respond";

/// 判断节点类型是否可作为流程执行入口
///
/// `entry` 是流程的标准入口；`http:ingress` 作为 Mock 服务的请求入口，
/// 在没有 `entry` 时同样允许作为执行起点（校验器、执行器、调试器共用此判定，
/// 避免出现“校验通过但无法执行/无法调试”的口径分裂）。
pub fn is_entry_node_type(node_type: &str) -> bool {
    node_type == ENTRY || node_type == HTTP_INGRESS
}
