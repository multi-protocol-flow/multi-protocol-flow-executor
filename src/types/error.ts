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
  error_code?: string;
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
