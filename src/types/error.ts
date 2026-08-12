/**
 * 错误类型定义
 * 与后端 src-tauri/src/error.rs 中的结构体对齐
 */

// ============================================================================
// 错误类型枚举
// ============================================================================

/**
 * 错误类型枚举（决定错误处理策略，同时标识具体错误类型）
 *
 * 错误分为两类：
 * - **致命错误**：配置错误和条件语法错误，需要立即终止流程
 * - **可恢复错误**：执行错误、验证错误、解析错误等，在容错模式下可继续执行
 */
export type ErrorKind =
  // ===== 配置错误（立即终止流程） =====
  | 'configuration' // 通用配置错误
  | 'condition_syntax' // 条件表达式语法错误
  // ===== 执行错误（容错模式，继续执行） =====
  | 'execution' // 通用执行错误
  | 'connection' // 网络连接失败
  | 'timeout' // 操作超时
  | 'condition_evaluation' // 条件评估运行时错误
  // ===== 验证错误（容错模式，继续执行） =====
  | 'validation' // 参数验证失败
  // ===== 解析错误（容错模式，继续执行） =====
  | 'parse' // 数据解析失败
  // ===== WebSocket 错误（容错模式，继续执行） =====
  | 'web_socket_connection' // WebSocket 连接失败
  | 'web_socket_timeout' // WebSocket 超时
  | 'web_socket_message' // WebSocket 消息错误
  | 'web_socket_close' // WebSocket 关闭错误
  | 'web_socket_protocol' // WebSocket 协议错误
  // ===== gRPC 错误（容错模式，继续执行） =====
  | 'grpc_connection' // gRPC 连接失败
  | 'grpc_timeout' // gRPC 超时
  | 'grpc_message' // gRPC 消息错误
  | 'grpc_status' // gRPC 状态错误
  | 'grpc_close' // gRPC 关闭错误
  // ===== 断言错误（容错模式，继续执行） =====
  | 'assertion' // 断言失败
  // ===== 脚本错误（根据 on_error 配置决定是否致命） =====
  | 'script_syntax' // 脚本语法错误
  | 'script_runtime' // 脚本运行时错误
  | 'script_timeout' // 脚本执行超时
  | 'script_assertion' // 脚本断言失败
  | 'script_abort'; // 用户调用 fn.flow.abort()

/**
 * 错误代码枚举（用于 i18n 翻译和程序化处理）
 * 与后端 flow-engine-core/src/error.rs 的 ErrorCode 枚举对齐
 */
export type ErrorCode =
  // General
  | 'E_CONFIG_ERROR'
  | 'E_CONDITION_SYNTAX'
  | 'E_EXECUTION_ERROR'
  | 'E_TIMEOUT'
  | 'E_CONDITION_EVALUATION'
  | 'E_UNKNOWN'
  | 'E_CANCELLED'
  | 'E_PARSE'
  | 'E_ASSERTION'
  | 'E_SCRIPT_ABORT'
  // Connection
  | 'E_CONN_TIMEOUT'
  | 'E_CONN_REFUSED'
  | 'E_CONN_CLOSED'
  // Validation
  | 'E_VAL_PORT_DUPLICATE'
  | 'E_VAL_CYCLE_DETECTED'
  | 'E_VAL_MISSING_ENTRY'
  | 'E_VAL_DUPLICATE_NODE_ID'
  | 'E_VAL_CONN_NODE_NOT_FOUND'
  | 'E_VAL_PORT_NOT_FOUND'
  | 'E_VAL_NODE_UNREACHABLE'
  | 'E_VAL_UNKNOWN_FIELD'
  | 'E_VAL_MISSING_REQUIRED_FIELD'
  | 'E_VAL_TYPE_MISMATCH'
  | 'E_VAL_UNKNOWN_NODE_TYPE'
  // HTTP
  | 'E_HTTP_REQUEST_FAILED'
  | 'E_HTTP_STATUS_ERROR'
  | 'E_HTTP_INVALID_URL'
  // WebSocket
  | 'E_WS_CONNECTION' | 'E_WS_TIMEOUT' | 'E_WS_MESSAGE' | 'E_WS_CLOSE' | 'E_WS_PROTOCOL'
  // Script
  | 'E_SCRIPT_SYNTAX' | 'E_SCRIPT_RUNTIME' | 'E_SCRIPT_TIMEOUT' | 'E_SCRIPT_ASSERTION'
  // gRPC
  | 'E_GRPC_CONNECTION' | 'E_GRPC_TIMEOUT' | 'E_GRPC_MESSAGE' | 'E_GRPC_STATUS' | 'E_GRPC_CLOSE'
  // MCP
  | 'E_MCP_CONNECTION' | 'E_MCP_TOOL_CALL' | 'E_MCP_RESOURCE_READ' | 'E_MCP_NOT_CONNECTED'
  // TCP
  | 'E_TCP_CONNECTION' | 'E_TCP_TIMEOUT' | 'E_TCP_SEND' | 'E_TCP_RECEIVE' | 'E_TCP_CLOSE'
  // UDP
  | 'E_UDP_BIND' | 'E_UDP_SEND' | 'E_UDP_RECEIVE' | 'E_UDP_CLOSE'
  // SSE
  | 'E_SSE_CONNECTION' | 'E_SSE_TIMEOUT' | 'E_SSE_EVENT'
  // GraphQL
  | 'E_GRAPHQL_CONNECTION' | 'E_GRAPHQL_QUERY' | 'E_GRAPHQL_SUBSCRIPTION'
  | 'E_GRAPHQL_INTROSPECTION' | 'E_GRAPHQL_CLOSE'
  // Non-protocol
  | 'E_VARIABLE_EXTRACTOR_FAILED'
  | 'E_PROXY_FAILED'
  // Plugin
  | 'E_PLUGIN_CRASHED';

// ============================================================================
// 错误详情结构体
// ============================================================================

/**
 * 错误详情结构（用于执行调试）
 *
 * 提供结构化的错误信息，包括错误类型、错误代码、消息、堆栈跟踪和上下文快照。
 * 这些信息用于调试和错误分析。
 */
export interface ErrorDetail {
  /** 错误类型（决定错误处理策略，同时标识具体错误类型） */
  error_kind: ErrorKind;
  /** 错误代码（可选，用于程序化处理） */
  error_code?: ErrorCode;
  /** i18n 翻译参数（可选，用于前端显示） */
  i18n_args?: Record<string, string>;
  /** 错误消息（人类可读） */
  message: string;
  /** 堆栈跟踪（可选） */
  stack_trace?: string;
  /** 错误发生时的上下文快照（可选） */
  context?: unknown;
}

// ============================================================================
// 辅助函数
// ============================================================================

/**
 * 致命错误类型列表
 */
const FATAL_ERROR_KINDS: ErrorKind[] = ['configuration', 'condition_syntax'];

/**
 * 判断是否为致命错误（需要立即终止流程）
 * @param errorKind 错误类型
 * @returns 如果是致命错误则返回 true
 */
export function isFatalError(errorKind: ErrorKind): boolean {
  return FATAL_ERROR_KINDS.includes(errorKind);
}
