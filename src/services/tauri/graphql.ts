/**
 * GraphQL 服务层 — GraphQL 协议相关的 Tauri IPC 调用
 */

import { invoke } from '@tauri-apps/api/core';

// ============================================================================
// 类型定义
// ============================================================================

/** GraphQL 内省请求参数 */
export interface GraphqlIntrospectInput {
  /** GraphQL 端点 URL */
  endpoint: string;
  /** 请求头 */
  headers?: Record<string, string>;
}

/** GraphQL 内省响应 */
export interface GraphqlIntrospectOutput {
  /** 是否成功 */
  success: boolean;
  /** Schema JSON 数据 */
  schema: Record<string, unknown> | null;
  /** 错误信息 */
  error: string | null;
}

// ============================================================================
// IPC 调用
// ============================================================================

/**
 * GraphQL Schema 内省
 *
 * 向 GraphQL 端点发送内省查询，返回完整的 Schema 信息。
 *
 * @param input - 内省请求参数
 * @returns 内省结果
 */
export async function introspectSchema(
  input: GraphqlIntrospectInput
): Promise<GraphqlIntrospectOutput> {
  return invoke<GraphqlIntrospectOutput>('graphql_introspect_schema', { input });
}
