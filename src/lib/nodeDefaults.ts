/**
 * nodeDefaults - 智能默认值推断
 * 根据上下文（上游节点类型、已有配置）自动推断新节点的合理默认值
 */
import type { NodeType } from '@/types';
import type { Node } from 'reactflow';

/**
 * 从 URL 中提取 base URL（协议 + 主机 + 端口）
 */
function extractBaseUrl(url: string): string | null {
  try {
    // 尝试解析为完整 URL
    const parsed = new URL(url);
    return `${parsed.protocol}//${parsed.host}`;
  } catch {
    // 如果是变量引用或相对路径，尝试截取到第一个 /
    const match = url.match(/^(https?:\/\/[^/]+)/i);
    return match?.[1] ?? null;
  }
}

/**
 * 推断新节点的默认值
 * @param upstreamNode 上游节点（连接的来源节点）
 * @param targetType 新添加的节点类型
 * @returns 需要覆盖的默认值
 */
export function inferDefaults(
  upstreamNode: Pick<Node, 'id' | 'type'> & { data: Record<string, unknown> } | null,
  targetType: NodeType
): Record<string, unknown> {
  if (!upstreamNode) return {};

  const upType = upstreamNode.type;

  switch (targetType) {
    // ========== HTTP ==========
    case 'http':
      if (upType === 'http') {
        const baseUrl = extractBaseUrl(upstreamNode.data.url as string);
        if (baseUrl) return { url: baseUrl };
      }
      // 如果上游是 entry 且有 base_url 变量引用，则使用默认 URL
      return {};

    // ========== Kafka ==========
    case 'kafka_produce':
    case 'kafka_consume':
    case 'kafka_batch_consume':
      if (upType === 'kafka_connect') {
        return {
          brokers: upstreamNode.data.brokers,
        };
      }
      break;

    // ========== MQTT ==========
    case 'mqtt_publish':
    case 'mqtt_subscribe':
    case 'mqtt_unsubscribe':
      if (upType === 'mqtt_connect') {
        return {
          host: upstreamNode.data.host,
          port: upstreamNode.data.port,
        };
      }
      break;

    // ========== 数据库 ==========
    case 'db_query':
    case 'db_begin':
    case 'db_commit':
    case 'db_rollback':
      if (upType === 'db_connect') {
        return {
          connection_id: upstreamNode.data.connection_id,
        };
      }
      break;

    // ========== gRPC ==========
    case 'grpc_call':
      if (upType === 'grpc_connect') {
        return {
          proto_file: upstreamNode.data.proto_file,
          service_name: upstreamNode.data.service_name,
          connection_id: upstreamNode.data.connection_id,
        };
      }
      break;

    // ========== MCP ==========
    case 'mcp_call_tool':
    case 'mcp_read_resource':
    case 'mcp_get_prompt':
      if (upType === 'mcp_connect') {
        return {
          server_url: upstreamNode.data.server_url,
        };
      }
      break;

    // ========== AMQP ==========
    case 'amqp_publish':
    case 'amqp_consume':
      if (upType === 'amqp_connect') {
        return {
          connection_id: upstreamNode.data.connection_id,
        };
      }
      break;

    // ========== TCP ==========
    case 'tcp_send':
      if (upType === 'tcp_connect') {
        return {
          host: upstreamNode.data.host,
          port: upstreamNode.data.port,
        };
      }
      break;

    // ========== SMTP ==========
    case 'smtp_send':
      if (upType === 'smtp_connect') {
        return {
          host: upstreamNode.data.host,
          port: upstreamNode.data.port,
        };
      }
      break;

    // ========== IMAP ==========
    case 'imap_fetch':
    case 'imap_search':
    case 'imap_folder':
    case 'imap_idle':
      if (upType === 'imap_connect') {
        return {
          host: upstreamNode.data.host,
          port: upstreamNode.data.port,
        };
      }
      break;

    // ========== GraphQL ==========
    case 'graphql_query':
    case 'graphql_subscribe':
      if (upType === 'graphql_introspect') {
        return {
          url: upstreamNode.data.url,
        };
      }
      break;

    // ========== SSE ==========
    case 'sse_listen':
      if (upType === 'sse_connect') {
        return {
          url: upstreamNode.data.url,
        };
      }
      break;

    // ========== WebSocket ==========
    case 'ws_send_collect':
      if (upType === 'ws_connect') {
        return {
          url: upstreamNode.data.url,
        };
      }
      break;
  }

  return {};
}
