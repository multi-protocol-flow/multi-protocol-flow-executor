import type { BaseTranslation } from '../i18n-types';

const zh_CN_errors: BaseTranslation = {
  // General
  E_CONFIG_ERROR: '配置错误',
  E_CONDITION_SYNTAX: '条件语法错误',
  E_EXECUTION_ERROR: '执行错误: {reason}',
  E_TIMEOUT: '操作 "{operation}" 超时 ({ms}ms)',
  E_CONDITION_EVALUATION: '条件评估错误: {expression}',
  E_UNKNOWN: '未知错误',
  E_CANCELLED: '已取消',
  E_PARSE: '解析错误: {value}',
  E_ASSERTION: '断言失败',
  E_SCRIPT_ABORT: '脚本已中止',

  // Connection
  E_CONN_TIMEOUT: '连接 {host} 超时 ({ms}ms)',
  E_CONN_REFUSED: '连接 {host}:{port} 被拒绝',
  E_CONN_CLOSED: '与 {host} 的连接已关闭',

  // Validation
  E_VAL_PORT_DUPLICATE: '端口连接重复',
  E_VAL_CYCLE_DETECTED: '检测到流程中存在循环依赖',
  E_VAL_MISSING_ENTRY: '缺少入口节点',

  // HTTP
  E_HTTP_REQUEST_FAILED: 'HTTP 请求 {url} 失败，状态码: {status}',
  E_HTTP_STATUS_ERROR: 'HTTP 错误 {url}: {status} - {body}',
  E_HTTP_INVALID_URL: '无效的 URL: {url}',

  // WebSocket
  E_WS_CONNECTION: 'WebSocket 连接失败: {url}',
  E_WS_TIMEOUT: 'WebSocket 超时: {url}',
  E_WS_MESSAGE: 'WebSocket 消息错误: {url}',
  E_WS_CLOSE: 'WebSocket 连接已关闭: {url}',
  E_WS_PROTOCOL: 'WebSocket 协议错误: {url}',

  // Database
  E_DB_CONNECTION: '数据库连接失败: {host}',
  E_DB_QUERY: '数据库查询错误: {query}',
  E_DB_TIMEOUT: '数据库超时: {host}',

  // Script
  E_SCRIPT_SYNTAX: '脚本语法错误: {detail}',
  E_SCRIPT_RUNTIME: '脚本运行时错误: {detail}',
  E_SCRIPT_TIMEOUT: '脚本执行超时 ({ms}ms)',
  E_SCRIPT_ASSERTION: '脚本断言失败: {detail}',

  // gRPC
  E_GRPC_CONNECTION: 'gRPC 连接失败: {target}',
  E_GRPC_TIMEOUT: 'gRPC 超时: {target}',
  E_GRPC_MESSAGE: 'gRPC 消息错误: {target}',
  E_GRPC_STATUS: 'gRPC 错误 {target}: 状态码 {code}',
  E_GRPC_CLOSE: 'gRPC 连接已关闭: {target}',

  // MCP
  E_MCP_CONNECTION: 'MCP 连接失败: {server}',
  E_MCP_TOOL_CALL: 'MCP 工具调用失败: {tool}',
  E_MCP_RESOURCE_READ: 'MCP 资源读取失败: {uri}',
  E_MCP_NOT_CONNECTED: '未连接到 MCP 服务器: {server}',

  // TCP
  E_TCP_CONNECTION: 'TCP 连接失败: {addr}',
  E_TCP_TIMEOUT: 'TCP 超时: {addr}',
  E_TCP_SEND: 'TCP 发送错误: {addr}',
  E_TCP_RECEIVE: 'TCP 接收错误: {addr}',
  E_TCP_CLOSE: 'TCP 连接已关闭: {addr}',

  // UDP
  E_UDP_BIND: 'UDP 绑定失败: {addr}',
  E_UDP_SEND: 'UDP 发送错误: {addr}',
  E_UDP_RECEIVE: 'UDP 接收错误: {addr}',
  E_UDP_CLOSE: 'UDP 套接字已关闭: {addr}',

  // SSE
  E_SSE_CONNECTION: 'SSE 连接失败: {url}',
  E_SSE_TIMEOUT: 'SSE 超时: {url}',
  E_SSE_EVENT: 'SSE 事件错误: {url}',

  // GraphQL
  E_GRAPHQL_CONNECTION: 'GraphQL 连接失败: {url}',
  E_GRAPHQL_QUERY: 'GraphQL 查询错误: {url}',
  E_GRAPHQL_SUBSCRIPTION: 'GraphQL 订阅错误: {url}',
  E_GRAPHQL_INTROSPECTION: 'GraphQL 内省失败: {url}',
  E_GRAPHQL_CLOSE: 'GraphQL 连接已关闭: {url}',

  // AMQP
  E_AMQP_CONNECTION: 'AMQP 连接失败: {addr}',
  E_AMQP_CHANNEL: 'AMQP 通道错误: {addr}',
  E_AMQP_PUBLISH: 'AMQP 发布错误: {addr}',
  E_AMQP_CONSUME: 'AMQP 消费错误: {addr}',
  E_AMQP_QUEUE: 'AMQP 队列错误: {addr}',
  E_AMQP_EXCHANGE: 'AMQP 交换机错误: {addr}',
  E_AMQP_ACK: 'AMQP 确认错误: {addr}',
  E_AMQP_MANAGEMENT: 'AMQP 管理错误: {addr}',
  E_AMQP_PROTOCOL: 'AMQP 协议错误: {addr}',

  // Redis
  E_REDIS_CONNECTION: 'Redis 连接失败: {host}',
  E_REDIS_COMMAND: 'Redis 命令错误: {cmd}',
  E_REDIS_TIMEOUT: 'Redis 超时: {host}',
  E_REDIS_AUTH: 'Redis 认证失败: {host}',

  // SMTP
  E_SMTP_CONNECTION: 'SMTP 连接失败: {host}',
  E_SMTP_SEND: 'SMTP 发送错误: {host}',
  E_SMTP_AUTH: 'SMTP 认证失败: {host}',
  E_SMTP_TIMEOUT: 'SMTP 超时: {host}',

  // IMAP
  E_IMAP_CONNECTION: 'IMAP 连接失败: {host}',
  E_IMAP_FETCH: 'IMAP 获取错误: {host}',
  E_IMAP_SEARCH: 'IMAP 搜索错误: {host}',
  E_IMAP_AUTH: 'IMAP 认证失败: {host}',
  E_IMAP_STORE: 'IMAP 存储错误: {host}',
  E_IMAP_TIMEOUT: 'IMAP 超时: {host}',

  // Non-protocol
  E_VARIABLE_EXTRACTOR_FAILED: '变量提取失败: {path}',
  E_PROXY_FAILED: '代理失败: {host}',

  // Plugin
  E_PLUGIN_CRASHED: '插件进程崩溃: {plugin_name} - {message}',

  // UI
  clipboard_save_failed: '节点配置保存到剪贴板失败',
};

export default zh_CN_errors;
