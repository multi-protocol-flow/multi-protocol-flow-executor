/**
 * 内置流程模板
 */
import type { Flow } from '@/types/flow';
import { getLL } from '@/i18n/i18n-lazy';

// ============================================================================
// 模板系统类型定义
// ============================================================================

/** 模板分类 */
export type TemplateCategory =
  | 'http' | 'websocket' | 'database' | 'grpc' | 'mqtt'
  | 'kafka' | 'amqp' | 'graphql' | 'imap' | 'smtp'
  | 'sse' | 'tcp' | 'udp' | 'mcp' | 'script' | 'assertion' | 'combination';

/** 模板难度等级 */
export type TemplateDifficulty = 'beginner' | 'intermediate' | 'advanced';

/** 流程模板结构 */
export interface FlowTemplate {
  id: string;
  name: string;
  description: string;
  category: TemplateCategory;
  icon: string;
  tags: string[];
  difficulty: TemplateDifficulty;
  flow: Flow;
}

// ============================================================================
// 新模板流程定义
// ============================================================================

/** HTTP POST with JSON Body */
const HTTP_POST_FLOW: Flow = {
  uuid: 'template-http-post',
  name: 'HTTP POST Request',
  initial_variables: {},
  nodes: [
    { uuid: 'entry-http-post-001', type: 'entry', name: 'Entry', position: { x: 0, y: 0 }, data: { type: 'entry' } },
    {
      uuid: 'http-post-request-001', type: 'http', name: 'POST Data', position: { x: 250, y: 0 },
      data: { type: 'http', url: 'https://httpbin.org/post', method: 'post', body_type: 'body', content_type: 'json', body: '{"title":"foo","body":"bar","userId":1}' }
    },
  ],
  connections: [
    { id: 'conn-http-post-1', source_node_uuid: 'entry-http-post-001', target_node_uuid: 'http-post-request-001', source_port_id: 'out', target_port_id: 'in' },
  ],
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

/** Database SQLite Query */
const DB_SQLITE_FLOW: Flow = {
  uuid: 'template-db-sqlite',
  name: 'SQLite Database Query',
  initial_variables: {},
  nodes: [
    { uuid: 'entry-db-001', type: 'entry', name: 'Entry', position: { x: 0, y: 0 }, data: { type: 'entry' } },
    {
      uuid: 'db-connect-001', type: 'db_connect', name: 'Connect Database', position: { x: 250, y: 0 },
      data: { type: 'db_connect', driver: 'sqlite', connection_string: ':memory:' }
    },
    {
      uuid: 'db-query-001', type: 'db_query', name: 'Execute Query', position: { x: 500, y: 0 },
      data: { type: 'db_query', connection_uuid: 'db-connect-001', sql: 'SELECT 1 as result;' }
    },
    {
      uuid: 'db-close-001', type: 'db_close', name: 'Close Connection', position: { x: 750, y: 0 },
      data: { type: 'db_close', connection_uuid: 'db-connect-001' }
    },
  ],
  connections: [
    { id: 'conn-db-1', source_node_uuid: 'entry-db-001', target_node_uuid: 'db-connect-001', source_port_id: 'out', target_port_id: 'in' },
    { id: 'conn-db-2', source_node_uuid: 'db-connect-001', target_node_uuid: 'db-query-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-db-3', source_node_uuid: 'db-query-001', target_node_uuid: 'db-close-001', source_port_id: 'true', target_port_id: 'in' },
  ],
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

/** TCP Connect/Send/Close */
const TCP_SEND_FLOW: Flow = {
  uuid: 'template-tcp-send',
  name: 'TCP Send Message',
  initial_variables: {},
  nodes: [
    { uuid: 'entry-tcp-001', type: 'entry', name: 'Entry', position: { x: 0, y: 0 }, data: { type: 'entry' } },
    {
      uuid: 'tcp-connect-001', type: 'tcp_connect', name: 'TCP Connect', position: { x: 250, y: 0 },
      data: { type: 'tcp_connect', host: 'example.com', port: 80, connect_timeout_ms: 5000 }
    },
    {
      uuid: 'tcp-send-001', type: 'tcp_send', name: 'Send Data', position: { x: 500, y: 0 },
      data: { type: 'tcp_send', connection_id: 'tcp-connect-001', data: 'Hello TCP Server', send_timeout_ms: 5000 }
    },
    {
      uuid: 'tcp-close-001', type: 'tcp_close', name: 'Close Connection', position: { x: 750, y: 0 },
      data: { type: 'tcp_close', connection_id: 'tcp-connect-001' }
    },
  ],
  connections: [
    { id: 'conn-tcp-1', source_node_uuid: 'entry-tcp-001', target_node_uuid: 'tcp-connect-001', source_port_id: 'out', target_port_id: 'in' },
    { id: 'conn-tcp-2', source_node_uuid: 'tcp-connect-001', target_node_uuid: 'tcp-send-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-tcp-3', source_node_uuid: 'tcp-send-001', target_node_uuid: 'tcp-close-001', source_port_id: 'true', target_port_id: 'in' },
  ],
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

/** UDP Bind/Send/Close */
const UDP_SEND_FLOW: Flow = {
  uuid: 'template-udp-send',
  name: 'UDP Send Message',
  initial_variables: {},
  nodes: [
    { uuid: 'entry-udp-001', type: 'entry', name: 'Entry', position: { x: 0, y: 0 }, data: { type: 'entry' } },
    {
      uuid: 'udp-bind-001', type: 'udp_bind', name: 'Bind Port', position: { x: 250, y: 0 },
      data: { type: 'udp_bind', local_port: 0 }
    },
    {
      uuid: 'udp-send-001', type: 'udp_send_to', name: 'Send Data', position: { x: 500, y: 0 },
      data: { type: 'udp_send_to', connection_id: 'udp-bind-001', data: 'Hello UDP', target_addr: '127.0.0.1', target_port: 9999 }
    },
    {
      uuid: 'udp-close-001', type: 'udp_close', name: 'Close', position: { x: 750, y: 0 },
      data: { type: 'udp_close', connection_id: 'udp-bind-001' }
    },
  ],
  connections: [
    { id: 'conn-udp-1', source_node_uuid: 'entry-udp-001', target_node_uuid: 'udp-bind-001', source_port_id: 'out', target_port_id: 'in' },
    { id: 'conn-udp-2', source_node_uuid: 'udp-bind-001', target_node_uuid: 'udp-send-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-udp-3', source_node_uuid: 'udp-send-001', target_node_uuid: 'udp-close-001', source_port_id: 'true', target_port_id: 'in' },
  ],
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

/** Kafka Produce Message */
const KAFKA_PRODUCE_FLOW: Flow = {
  uuid: 'template-kafka-produce',
  name: 'Kafka Produce Message',
  initial_variables: {},
  nodes: [
    { uuid: 'entry-kafka-001', type: 'entry', name: 'Entry', position: { x: 0, y: 0 }, data: { type: 'entry' } },
    {
      uuid: 'kafka-connect-001', type: 'kafka_connect', name: 'Connect Kafka', position: { x: 250, y: 0 },
      data: { type: 'kafka_connect', bootstrap_servers: 'localhost:9092', client_id: 'mpe-client' }
    },
    {
      uuid: 'kafka-produce-001', type: 'kafka_produce', name: 'Send Message', position: { x: 500, y: 0 },
      data: { type: 'kafka_produce', connection_id: 'kafka-connect-001', topic: 'test-topic', key: 'key-1', value: '{"message":"Hello Kafka"}' }
    },
    {
      uuid: 'kafka-close-001', type: 'kafka_close', name: 'Close Connection', position: { x: 750, y: 0 },
      data: { type: 'kafka_close', connection_id: 'kafka-connect-001' }
    },
  ],
  connections: [
    { id: 'conn-kafka-1', source_node_uuid: 'entry-kafka-001', target_node_uuid: 'kafka-connect-001', source_port_id: 'out', target_port_id: 'in' },
    { id: 'conn-kafka-2', source_node_uuid: 'kafka-connect-001', target_node_uuid: 'kafka-produce-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-kafka-3', source_node_uuid: 'kafka-produce-001', target_node_uuid: 'kafka-close-001', source_port_id: 'true', target_port_id: 'in' },
  ],
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

/** AMQP Publish/Consume */
const AMQP_FLOW: Flow = {
  uuid: 'template-amqp',
  name: 'AMQP Publish/Consume',
  initial_variables: {},
  nodes: [
    { uuid: 'entry-amqp-001', type: 'entry', name: 'Entry', position: { x: 0, y: 0 }, data: { type: 'entry' } },
    {
      uuid: 'amqp-connect-001', type: 'amqp_connect', name: 'Connect Broker', position: { x: 250, y: 0 },
      data: { type: 'amqp_connect', url: 'amqp://guest:guest@localhost:5672/%2f' }
    },
    {
      uuid: 'amqp-declare-001', type: 'amqp_exchange_declare', name: 'Declare Exchange', position: { x: 500, y: 0 },
      data: { type: 'amqp_exchange_declare', connection_id: 'amqp-connect-001', exchange: 'test.exchange', exchange_type: 'direct', durable: false, auto_delete: true }
    },
    {
      uuid: 'amqp-publish-001', type: 'amqp_publish', name: 'Publish Message', position: { x: 750, y: 0 },
      data: { type: 'amqp_publish', connection_id: 'amqp-connect-001', exchange: 'test.exchange', routing_key: 'test.key', body: '{"msg":"Hello AMQP"}', body_type: 'json' }
    },
    {
      uuid: 'amqp-consume-001', type: 'amqp_consume', name: 'Consume Message', position: { x: 1000, y: 0 },
      data: { type: 'amqp_consume', connection_id: 'amqp-connect-001', queue: 'test.queue', ack_mode: 'auto', max_messages: 1, timeout_ms: 5000 }
    },
    {
      uuid: 'amqp-close-001', type: 'amqp_close', name: 'Close Connection', position: { x: 1250, y: 0 },
      data: { type: 'amqp_close', connection_id: 'amqp-connect-001' }
    },
  ],
  connections: [
    { id: 'conn-amqp-1', source_node_uuid: 'entry-amqp-001', target_node_uuid: 'amqp-connect-001', source_port_id: 'out', target_port_id: 'in' },
    { id: 'conn-amqp-2', source_node_uuid: 'amqp-connect-001', target_node_uuid: 'amqp-declare-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-amqp-3', source_node_uuid: 'amqp-declare-001', target_node_uuid: 'amqp-publish-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-amqp-4', source_node_uuid: 'amqp-publish-001', target_node_uuid: 'amqp-consume-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-amqp-5', source_node_uuid: 'amqp-consume-001', target_node_uuid: 'amqp-close-001', source_port_id: 'true', target_port_id: 'in' },
  ],
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

/** GraphQL Query */
const GRAPHQL_QUERY_FLOW: Flow = {
  uuid: 'template-graphql',
  name: 'GraphQL Query',
  initial_variables: {},
  nodes: [
    { uuid: 'entry-gql-001', type: 'entry', name: 'Entry', position: { x: 0, y: 0 }, data: { type: 'entry' } },
    {
      uuid: 'gql-connect-001', type: 'graphql_connect', name: 'Connect GraphQL', position: { x: 250, y: 0 },
      data: { type: 'graphql_connect', endpoint: 'https://countries.trevorblades.com/graphql' }
    },
    {
      uuid: 'gql-query-001', type: 'graphql_query', name: 'Query Data', position: { x: 500, y: 0 },
      data: { type: 'graphql_query', connection_id: 'gql-connect-001', query: '{ countries { name code } }', timeout_ms: 10000 }
    },
    {
      uuid: 'gql-disconnect-001', type: 'graphql_disconnect', name: 'Disconnect', position: { x: 750, y: 0 },
      data: { type: 'graphql_disconnect', connection_id: 'gql-connect-001' }
    },
  ],
  connections: [
    { id: 'conn-gql-1', source_node_uuid: 'entry-gql-001', target_node_uuid: 'gql-connect-001', source_port_id: 'out', target_port_id: 'in' },
    { id: 'conn-gql-2', source_node_uuid: 'gql-connect-001', target_node_uuid: 'gql-query-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-gql-3', source_node_uuid: 'gql-query-001', target_node_uuid: 'gql-disconnect-001', source_port_id: 'true', target_port_id: 'in' },
  ],
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

/** SMTP Send Email */
const SMTP_SEND_FLOW: Flow = {
  uuid: 'template-smtp',
  name: 'SMTP Send Email',
  initial_variables: {},
  nodes: [
    { uuid: 'entry-smtp-001', type: 'entry', name: 'Entry', position: { x: 0, y: 0 }, data: { type: 'entry' } },
    {
      uuid: 'smtp-connect-001', type: 'smtp_connect', name: 'Connect SMTP', position: { x: 250, y: 0 },
      data: { type: 'smtp_connect', host: 'smtp.example.com', port: 587, tls_mode: 'starttls', auth: { auth_type: 'plain', username: 'user@example.com', password: '' } }
    },
    {
      uuid: 'smtp-send-001', type: 'smtp_send', name: 'Send Email', position: { x: 500, y: 0 },
      data: { type: 'smtp_send', connection_id: 'smtp-connect-001', from: 'user@example.com', to: ['recipient@example.com'], subject: 'Test Email', body: 'Hello from MPE!', body_type: 'text' }
    },
    {
      uuid: 'smtp-close-001', type: 'smtp_disconnect', name: 'Disconnect', position: { x: 750, y: 0 },
      data: { type: 'smtp_disconnect', connection_id: 'smtp-connect-001' }
    },
  ],
  connections: [
    { id: 'conn-smtp-1', source_node_uuid: 'entry-smtp-001', target_node_uuid: 'smtp-connect-001', source_port_id: 'out', target_port_id: 'in' },
    { id: 'conn-smtp-2', source_node_uuid: 'smtp-connect-001', target_node_uuid: 'smtp-send-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-smtp-3', source_node_uuid: 'smtp-send-001', target_node_uuid: 'smtp-close-001', source_port_id: 'true', target_port_id: 'in' },
  ],
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

/** SSE Connect/Listen */
const SSE_FLOW: Flow = {
  uuid: 'template-sse',
  name: 'SSE Connect/Listen',
  initial_variables: {},
  nodes: [
    { uuid: 'entry-sse-001', type: 'entry', name: 'Entry', position: { x: 0, y: 0 }, data: { type: 'entry' } },
    {
      uuid: 'sse-connect-001', type: 'sse_connect', name: 'Connect SSE', position: { x: 250, y: 0 },
      data: { type: 'sse_connect', url: 'https://example.com/sse', timeout_ms: 10000, auto_reconnect: true }
    },
    {
      uuid: 'sse-listen-001', type: 'sse_listen', name: 'Listen for Events', position: { x: 500, y: 0 },
      data: { type: 'sse_listen', connection_id: 'sse-connect-001', max_events: 5, timeout_ms: 30000 }
    },
    {
      uuid: 'sse-close-001', type: 'sse_disconnect', name: 'Disconnect', position: { x: 750, y: 0 },
      data: { type: 'sse_disconnect', connection_id: 'sse-connect-001' }
    },
  ],
  connections: [
    { id: 'conn-sse-1', source_node_uuid: 'entry-sse-001', target_node_uuid: 'sse-connect-001', source_port_id: 'out', target_port_id: 'in' },
    { id: 'conn-sse-2', source_node_uuid: 'sse-connect-001', target_node_uuid: 'sse-listen-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-sse-3', source_node_uuid: 'sse-listen-001', target_node_uuid: 'sse-close-001', source_port_id: 'true', target_port_id: 'in' },
  ],
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

/** Script Data Transform */
const SCRIPT_TRANSFORM_FLOW: Flow = {
  uuid: 'template-script',
  name: 'Script Data Transform',
  initial_variables: { input_data: '{"name":"Alice","age":30}' },
  nodes: [
    { uuid: 'entry-script-001', type: 'entry', name: 'Entry', position: { x: 0, y: 0 }, data: { type: 'entry' } },
    {
      uuid: 'script-process-001', type: 'script', name: 'Process Data', position: { x: 250, y: 0 },
      data: { type: 'script', script: 'const data = JSON.parse(fn.variables.get("input_data"));\ndata.greeting = `Hello, ${data.name}!`;\ndata.age_next_year = data.age + 1;\nfn.variables.set("output_data", JSON.stringify(data));\nfn.console.log("Processing complete:", data);' }
    },
    { uuid: 'end-script-001', type: 'end', name: 'End', position: { x: 500, y: 0 }, data: { type: 'end' } },
  ],
  connections: [
    { id: 'conn-script-1', source_node_uuid: 'entry-script-001', target_node_uuid: 'script-process-001', source_port_id: 'out', target_port_id: 'in' },
    { id: 'conn-script-2', source_node_uuid: 'script-process-001', target_node_uuid: 'end-script-001', source_port_id: 'true', target_port_id: 'in' },
  ],
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

/** MCP Connect/CallTool */
const MCP_FLOW: Flow = {
  uuid: 'template-mcp',
  name: 'MCP Call Tool',
  initial_variables: {},
  nodes: [
    { uuid: 'entry-mcp-001', type: 'entry', name: 'Entry', position: { x: 0, y: 0 }, data: { type: 'entry' } },
    {
      uuid: 'mcp-connect-001', type: 'mcp_connect', name: 'Connect MCP', position: { x: 250, y: 0 },
      data: { type: 'mcp_connect', server_command: 'npx', server_args: ['-y', '@modelcontextprotocol/server-everything'], transport_type: 'stdio' }
    },
    {
      uuid: 'mcp-call-001', type: 'mcp_call_tool', name: 'Call Tool', position: { x: 500, y: 0 },
      data: { type: 'mcp_call_tool', connection_id: 'mcp-connect-001', tool_name: 'echo', arguments: { message: 'Hello MCP' } }
    },
    {
      uuid: 'mcp-close-001', type: 'mcp_close', name: 'Close Connection', position: { x: 750, y: 0 },
      data: { type: 'mcp_close', connection_id: 'mcp-connect-001' }
    },
  ],
  connections: [
    { id: 'conn-mcp-1', source_node_uuid: 'entry-mcp-001', target_node_uuid: 'mcp-connect-001', source_port_id: 'out', target_port_id: 'in' },
    { id: 'conn-mcp-2', source_node_uuid: 'mcp-connect-001', target_node_uuid: 'mcp-call-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-mcp-3', source_node_uuid: 'mcp-call-001', target_node_uuid: 'mcp-close-001', source_port_id: 'true', target_port_id: 'in' },
  ],
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

/** IMAP Read Email */
const IMAP_READ_FLOW: Flow = {
  uuid: 'template-imap',
  name: 'IMAP Read Email',
  initial_variables: {},
  nodes: [
    { uuid: 'entry-imap-001', type: 'entry', name: 'Entry', position: { x: 0, y: 0 }, data: { type: 'entry' } },
    {
      uuid: 'imap-connect-001', type: 'imap_connect', name: 'Connect IMAP', position: { x: 250, y: 0 },
      data: { type: 'imap_connect', host: 'imap.example.com', port: 993, tls: true, auth: { auth_type: 'plain', username: 'user@example.com', password: '' } }
    },
    {
      uuid: 'imap-fetch-001', type: 'imap_fetch', name: 'Fetch Emails', position: { x: 500, y: 0 },
      data: { type: 'imap_fetch', connection_id: 'imap-connect-001', mailbox: 'INBOX', message_count: 5, fetch_attachments: false }
    },
    {
      uuid: 'imap-close-001', type: 'imap_close', name: 'Close Connection', position: { x: 750, y: 0 },
      data: { type: 'imap_close', connection_id: 'imap-connect-001' }
    },
  ],
  connections: [
    { id: 'conn-imap-1', source_node_uuid: 'entry-imap-001', target_node_uuid: 'imap-connect-001', source_port_id: 'out', target_port_id: 'in' },
    { id: 'conn-imap-2', source_node_uuid: 'imap-connect-001', target_node_uuid: 'imap-fetch-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-imap-3', source_node_uuid: 'imap-fetch-001', target_node_uuid: 'imap-close-001', source_port_id: 'true', target_port_id: 'in' },
  ],
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

/** Combination: HTTP → Conditional → SMTP Email */
const HTTP_CONDITIONAL_EMAIL_FLOW: Flow = {
  uuid: 'template-http-conditional-email',
  name: 'HTTP → Conditional → Email',
  initial_variables: {},
  nodes: [
    { uuid: 'entry-hce-001', type: 'entry', name: 'Entry', position: { x: 0, y: 0 }, data: { type: 'entry' } },
    {
      uuid: 'hce-http-001', type: 'http', name: 'Check Service Status', position: { x: 250, y: 0 },
      data: { type: 'http', url: 'https://httpbin.org/status/200', method: 'get' }
    },
    {
      uuid: 'hce-cond-001', type: 'conditional', name: 'Status = 200?', position: { x: 500, y: 0 },
      data: { type: 'conditional', condition_type: 'simple', field_path: 'status', operator: 'equal', expected_value: 200 }
    },
    {
      uuid: 'hce-smtp-001', type: 'smtp_connect', name: 'Connect SMTP', position: { x: 750, y: 0 },
      data: { type: 'smtp_connect', host: 'smtp.example.com', port: 587, tls_mode: 'starttls', auth: { auth_type: 'plain', username: 'alert@example.com', password: '' } }
    },
    {
      uuid: 'hce-send-001', type: 'smtp_send', name: 'Send Alert', position: { x: 1000, y: 0 },
      data: { type: 'smtp_send', connection_id: 'hce-smtp-001', from: 'alert@example.com', to: ['admin@example.com'], subject: 'Service Status Notification', body: 'Service check complete, status code: {{status}}', body_type: 'text' }
    },
    {
      uuid: 'hce-close-001', type: 'smtp_disconnect', name: 'Disconnect SMTP', position: { x: 1250, y: 0 },
      data: { type: 'smtp_disconnect', connection_id: 'hce-smtp-001' }
    },
  ],
  connections: [
    { id: 'conn-hce-1', source_node_uuid: 'entry-hce-001', target_node_uuid: 'hce-http-001', source_port_id: 'out', target_port_id: 'in' },
    { id: 'conn-hce-2', source_node_uuid: 'hce-http-001', target_node_uuid: 'hce-cond-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-hce-3', source_node_uuid: 'hce-cond-001', target_node_uuid: 'hce-smtp-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-hce-4', source_node_uuid: 'hce-smtp-001', target_node_uuid: 'hce-send-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-hce-5', source_node_uuid: 'hce-send-001', target_node_uuid: 'hce-close-001', source_port_id: 'true', target_port_id: 'in' },
  ],
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

/** Combination: Kafka → Script → DB */
const KAFKA_SCRIPT_DB_FLOW: Flow = {
  uuid: 'template-kafka-script-db',
  name: 'Kafka → Script → Database',
  initial_variables: {},
  nodes: [
    { uuid: 'entry-ksdb-001', type: 'entry', name: 'Entry', position: { x: 0, y: 0 }, data: { type: 'entry' } },
    {
      uuid: 'ksdb-kafka-001', type: 'kafka_connect', name: 'Connect Kafka', position: { x: 250, y: 0 },
      data: { type: 'kafka_connect', bootstrap_servers: 'localhost:9092', client_id: 'mpe-consumer' }
    },
    {
      uuid: 'ksdb-consume-001', type: 'kafka_consume', name: 'Consume Message', position: { x: 500, y: 0 },
      data: { type: 'kafka_consume', connection_id: 'ksdb-kafka-001', topic: 'input-topic', consumer_group: 'mpe-group', max_messages: 1, timeout_ms: 10000 }
    },
    {
      uuid: 'ksdb-script-001', type: 'script', name: 'Transform Data', position: { x: 750, y: 0 },
      data: { type: 'script', script: 'const raw = JSON.parse(fn.variables.get("last_node_output") || "{}");\nconst transformed = { id: raw.id, value: raw.data, processed_at: new Date().toISOString() };\nfn.variables.set("transformed", JSON.stringify(transformed));\nfn.console.log("Transform result:", transformed);' }
    },
    {
      uuid: 'ksdb-db-001', type: 'db_connect', name: 'Connect Database', position: { x: 1000, y: 0 },
      data: { type: 'db_connect', driver: 'sqlite', connection_string: ':memory:' }
    },
    {
      uuid: 'ksdb-query-001', type: 'db_query', name: 'Insert Data', position: { x: 1250, y: 0 },
      data: { type: 'db_query', connection_uuid: 'ksdb-db-001', sql: 'SELECT 1 as inserted;' }
    },
    {
      uuid: 'ksdb-close-001', type: 'db_close', name: 'Close Connection', position: { x: 1500, y: 0 },
      data: { type: 'db_close', connection_uuid: 'ksdb-db-001' }
    },
  ],
  connections: [
    { id: 'conn-ksdb-1', source_node_uuid: 'entry-ksdb-001', target_node_uuid: 'ksdb-kafka-001', source_port_id: 'out', target_port_id: 'in' },
    { id: 'conn-ksdb-2', source_node_uuid: 'ksdb-kafka-001', target_node_uuid: 'ksdb-consume-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-ksdb-3', source_node_uuid: 'ksdb-consume-001', target_node_uuid: 'ksdb-script-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-ksdb-4', source_node_uuid: 'ksdb-script-001', target_node_uuid: 'ksdb-db-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-ksdb-5', source_node_uuid: 'ksdb-db-001', target_node_uuid: 'ksdb-query-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-ksdb-6', source_node_uuid: 'ksdb-query-001', target_node_uuid: 'ksdb-close-001', source_port_id: 'true', target_port_id: 'in' },
  ],
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

/** Combination: MQTT → HTTP Webhook */
const MQTT_HTTP_WEBHOOK_FLOW: Flow = {
  uuid: 'template-mqtt-http-webhook',
  name: 'MQTT → HTTP Webhook',
  initial_variables: {},
  nodes: [
    { uuid: 'entry-mh-001', type: 'entry', name: 'Entry', position: { x: 0, y: 0 }, data: { type: 'entry' } },
    {
      uuid: 'mh-mqtt-001', type: 'mqtt_connect', name: 'Connect MQTT', position: { x: 250, y: 0 },
      data: { type: 'mqtt_connect', url: 'mqtt://broker.emqx.io:1883', mqtt_version: '5.0', clean_session: true, keep_alive_secs: 60 }
    },
    {
      uuid: 'mh-sub-001', type: 'mqtt_subscribe', name: 'Subscribe Topic', position: { x: 500, y: 0 },
      data: { type: 'mqtt_subscribe', connection_id: 'mh-mqtt-001', topic: 'test/webhook/#', qos: 0, max_messages: 1, collect_timeout_ms: 10000, output_mode: 'first' }
    },
    {
      uuid: 'mh-http-001', type: 'http', name: 'Forward Webhook', position: { x: 750, y: 0 },
      data: { type: 'http', url: 'https://httpbin.org/post', method: 'post', body_type: 'body', content_type: 'json', body: '{"source":"mqtt","payload":{{body}}}' }
    },
    {
      uuid: 'mh-close-001', type: 'mqtt_disconnect', name: 'Disconnect MQTT', position: { x: 1000, y: 0 },
      data: { type: 'mqtt_disconnect', connection_id: 'mh-mqtt-001' }
    },
  ],
  connections: [
    { id: 'conn-mh-1', source_node_uuid: 'entry-mh-001', target_node_uuid: 'mh-mqtt-001', source_port_id: 'out', target_port_id: 'in' },
    { id: 'conn-mh-2', source_node_uuid: 'mh-mqtt-001', target_node_uuid: 'mh-sub-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-mh-3', source_node_uuid: 'mh-sub-001', target_node_uuid: 'mh-http-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-mh-4', source_node_uuid: 'mh-http-001', target_node_uuid: 'mh-close-001', source_port_id: 'true', target_port_id: 'in' },
  ],
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

/**
 * HTTP 节点示例流程
 */
/** 内置模板版本号 - 修改模板后递增此数字 */
export const BUILTIN_FLOW_VERSION = 5;


export const DEMO_FLOW: Flow = {
  uuid: 'demo-http-testing',
  name: 'HTTP Node Example Flow',
  initial_variables: {},
  nodes: [
    // 节点 1: Entry 节点
    {
      uuid: 'entry-001',
      type: 'entry',
      name: 'Entry',
      position: { x: 0, y: 0 },
      data: { type: 'entry' }
    },
    // 节点 2: HTTP GET 节点
    {
      uuid: 'http-get-001',
      type: 'http',
      name: 'HTTP GET Example',
      position: { x: 250, y: 0 },
      data: {
        type: 'http',
        url: 'https://httpbin.org/get',
        method: 'get'
      }
    },
    // 节点 3: 条件节点
    {
      uuid: 'conditional-001',
      type: 'conditional',
      name: 'Check Status Code',
      position: { x: 500, y: 0 },
      data: {
        type: 'conditional',
        condition_type: 'simple',
        field_path: 'status',
        operator: 'equal',
        expected_value: 200
      }
    },
    // 节点 4: HTTP POST 节点（成功分支）
    {
      uuid: 'http-post-001',
      type: 'http',
      name: 'HTTP POST Example',
      position: { x: 750, y: 0 },
      data: {
        type: 'http',
        url: 'https://httpbin.org/post',
        method: 'post',
        body_type: 'body',
        content_type: 'json',
        body: '{"success":true}'
      }
    }
  ],
  connections: [
    // Entry → HTTP GET
    {
      id: 'conn-1',
      source_node_uuid: 'entry-001',
      target_node_uuid: 'http-get-001',
      source_port_id: 'out',
      target_port_id: 'in'
    },
    // HTTP GET → 条件（使用 true 端口，表示成功时继续）
    {
      id: 'conn-2',
      source_node_uuid: 'http-get-001',
      target_node_uuid: 'conditional-001',
      source_port_id: 'true',
      target_port_id: 'in'
    },
    // 条件 → HTTP POST（成功分支）
    {
      id: 'conn-3',
      source_node_uuid: 'conditional-001',
      target_node_uuid: 'http-post-001',
      source_port_id: 'true',
      target_port_id: 'in'
    }
  ],
  // 内置模板元数据（用于自动更新检测）
  _meta: {
    version: BUILTIN_FLOW_VERSION,
    isBuiltin: true,
  },
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

/**
 * WebSocket 节点示例流程
 * 使用 wss://echo.websocket.org (Echo 服务器，会原样返回发送的消息)
 */
export const WS_DEMO_FLOW: Flow = {
  uuid: 'demo-ws-testing',
  name: 'WebSocket Node Example Flow',
  initial_variables: {},
  nodes: [
    // 节点 1: Entry 节点
    {
      uuid: 'entry-ws-001',
      type: 'entry',
      name: 'Entry',
      position: { x: 0, y: 0 },
      data: { type: 'entry' }
    },
    // 节点 2: WS Connect 节点
    {
      uuid: 'ws-connect-001',
      type: 'ws_connect',
      name: 'Connect WebSocket',
      position: { x: 250, y: 0 },
      data: {
        type: 'ws_connect',
        url: 'wss://echo.websocket.org',
        connect_timeout_ms: 10000,
        collect_message: false,
        collect_timeout_ms: 5000
      }
    },
    // 节点 3: WS Send+Collect 节点
    {
      uuid: 'ws-send-collect-001',
      type: 'ws_send_collect',
      name: 'Send and Collect Response',
      position: { x: 500, y: 0 },
      data: {
        type: 'ws_send_collect',
        connection_id: 'ws-connect-001',
        message: '{"action":"ping","data":"Hello WebSocket!"}',
        message_type: 'text',
        send_timeout_ms: 5000,
        collect_response: true,
        collect_mode: 'first_match',
        collect_timeout_ms: 5000
      }
    },
    // 节点 4: WS Close 节点
    {
      uuid: 'ws-close-001',
      type: 'ws_close',
      name: 'Close Connection',
      position: { x: 750, y: 0 },
      data: {
        type: 'ws_close',
        connection_id: 'ws-connect-001',
        close_code: 1000,
        close_reason: 'Normal closure',
        close_timeout_ms: 5000
      }
    }
  ],
  connections: [
    // Entry → WS Connect
    {
      id: 'conn-ws-1',
      source_node_uuid: 'entry-ws-001',
      target_node_uuid: 'ws-connect-001',
      source_port_id: 'out',
      target_port_id: 'in'
    },
    // WS Connect → WS Send+Collect
    {
      id: 'conn-ws-2',
      source_node_uuid: 'ws-connect-001',
      target_node_uuid: 'ws-send-collect-001',
      source_port_id: 'true',
      target_port_id: 'in'
    },
    // WS Send+Collect → WS Close
    {
      id: 'conn-ws-3',
      source_node_uuid: 'ws-send-collect-001',
      target_node_uuid: 'ws-close-001',
      source_port_id: 'true',
      target_port_id: 'in'
    }
  ],
  _meta: {
    version: BUILTIN_FLOW_VERSION,
    isBuiltin: true,
  },
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

/**
 * gRPC 节点示例流程
 * 注意：请将 proto 文件内容和服务器地址替换为你自己的 gRPC 服务信息
 */
export const GRPC_DEMO_FLOW: Flow = {
  uuid: 'demo-grpc-testing',
  name: 'gRPC Node Example Flow',
  initial_variables: {},
  nodes: [
    // 节点 1: Entry 节点
    {
      uuid: 'entry-grpc-001',
      type: 'entry',
      name: 'Entry',
      position: { x: 0, y: 0 },
      data: { type: 'entry' }
    },
    // 节点 2: gRPC Connect 节点
    {
      uuid: 'grpc-connect-001',
      type: 'grpc_connect',
      name: 'Connect gRPC Service',
      position: { x: 250, y: 0 },
      data: {
        type: 'grpc_connect',
        url: 'http://localhost:50051',
        use_tls: false,
        enable_reflection: true,
        proto_files: [
          {
            path: 'example.proto',
            content: '// Replace with your Proto file content\nsyntax = "proto3";\n\npackage example;\n\nservice ExampleService {\n  rpc GetData (GetDataRequest) returns (GetDataResponse);\n}\n\nmessage GetDataRequest {\n  string id = 1;\n}\n\nmessage GetDataResponse {\n  string result = 1;\n}'
          }
        ],
        connect_timeout_ms: 10000
      }
    },
    // 节点 3: gRPC Call 节点
    {
      uuid: 'grpc-call-001',
      type: 'grpc_call',
      name: 'Call gRPC Method',
      position: { x: 500, y: 0 },
      data: {
        type: 'grpc_call',
        connection_id: 'grpc-connect-001',
        service_name: 'example.ExampleService',
        method_name: 'GetData',
        request_json: '{"id": "1"}',
        timeout_ms: 5000
      }
    },
    // 节点 4: gRPC Close 节点
    {
      uuid: 'grpc-close-001',
      type: 'grpc_close',
      name: 'Close Connection',
      position: { x: 750, y: 0 },
      data: {
        type: 'grpc_close',
        connection_id: 'grpc-connect-001'
      }
    }
  ],
  connections: [
    // Entry → gRPC Connect
    {
      id: 'conn-grpc-1',
      source_node_uuid: 'entry-grpc-001',
      target_node_uuid: 'grpc-connect-001',
      source_port_id: 'out',
      target_port_id: 'in'
    },
    // gRPC Connect → gRPC Call
    {
      id: 'conn-grpc-2',
      source_node_uuid: 'grpc-connect-001',
      target_node_uuid: 'grpc-call-001',
      source_port_id: 'true',
      target_port_id: 'in'
    },
    // gRPC Call → gRPC Close
    {
      id: 'conn-grpc-3',
      source_node_uuid: 'grpc-call-001',
      target_node_uuid: 'grpc-close-001',
      source_port_id: 'true',
      target_port_id: 'in'
    }
  ],
  _meta: {
    version: BUILTIN_FLOW_VERSION,
    isBuiltin: true,
  },
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

/**
 * MQTT 基础连接测试流程
 * 演示 MQTT 连接、订阅、发布和断开的基本工作流
 */
export const MQTT_BASIC_FLOW: Flow = {
  uuid: 'demo-mqtt-basic',
  name: 'MQTT Basic Connection Test',
  description: getLL().flow.mqtt_basic_desc(),
  initial_variables: { "timestamp": "" },
  nodes: [
    // 节点 1: Entry 节点
    {
      uuid: 'entry-mqtt-basic-001',
      type: 'entry',
      name: 'Entry',
      position: { x: 0, y: 0 },
      data: { type: 'entry' }
    },
    // 节点 2: MQTT Connect 节点
    {
      uuid: 'mqtt-connect-basic-001',
      type: 'mqtt_connect',
      name: 'Connect Broker',
      position: { x: 250, y: 0 },
      data: {
        type: 'mqtt_connect',
        url: 'mqtt://broker.emqx.io:1883',
        mqtt_version: '5.0',
        clean_session: true,
        keep_alive_secs: 60
      }
    },
    // 节点 3: MQTT Subscribe 节点
    {
      uuid: 'mqtt-subscribe-basic-001',
      type: 'mqtt_subscribe',
      name: 'Subscribe to Topic',
      position: { x: 500, y: 0 },
      data: {
        type: 'mqtt_subscribe',
        connection_id: 'mqtt-connect-basic-001',
        topic: 'test/multi-protocolflow/#',
        qos: 0,
        max_messages: 1,
        collect_timeout_ms: 5000,
        output_mode: 'first'
      }
    },
    // 节点 4: MQTT Publish 节点
    {
      uuid: 'mqtt-publish-basic-001',
      type: 'mqtt_publish',
      name: 'Publish Test Message',
      position: { x: 750, y: 0 },
      data: {
        type: 'mqtt_publish',
        connection_id: 'mqtt-connect-basic-001',
        topic: 'test/multi-protocolflow/hello',
        payload: '{"message":"Hello from Multi-Protocol Flow Executor","timestamp":"{{initial_variables.timestamp}}"}',
        qos: 0,
        payload_format: 'json'
      }
    },
    // 节点 5: MQTT Disconnect 节点
    {
      uuid: 'mqtt-disconnect-basic-001',
      type: 'mqtt_disconnect',
      name: 'Disconnect',
      position: { x: 1000, y: 0 },
      data: {
        type: 'mqtt_disconnect',
        connection_id: 'mqtt-connect-basic-001'
      }
    }
  ],
  connections: [
    // Entry → MQTT Connect
    {
      id: 'conn-mqtt-basic-1',
      source_node_uuid: 'entry-mqtt-basic-001',
      target_node_uuid: 'mqtt-connect-basic-001',
      source_port_id: 'out',
      target_port_id: 'in'
    },
    // MQTT Connect → MQTT Subscribe
    {
      id: 'conn-mqtt-basic-2',
      source_node_uuid: 'mqtt-connect-basic-001',
      target_node_uuid: 'mqtt-subscribe-basic-001',
      source_port_id: 'true',
      target_port_id: 'in'
    },
    // MQTT Subscribe → MQTT Publish
    {
      id: 'conn-mqtt-basic-3',
      source_node_uuid: 'mqtt-subscribe-basic-001',
      target_node_uuid: 'mqtt-publish-basic-001',
      source_port_id: 'true',
      target_port_id: 'in'
    },
    // MQTT Publish → MQTT Disconnect
    {
      id: 'conn-mqtt-basic-4',
      source_node_uuid: 'mqtt-publish-basic-001',
      target_node_uuid: 'mqtt-disconnect-basic-001',
      source_port_id: 'true',
      target_port_id: 'in'
    }
  ],
  _meta: {
    version: BUILTIN_FLOW_VERSION,
    isBuiltin: true,
  },
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

/**
 * MQTT 请求/响应模式流程
 * 演示 MQTT 5.0 请求/响应模式，使用 correlation_data 自动匹配
 */
export const MQTT_REQUEST_RESPONSE_FLOW: Flow = {
  uuid: 'demo-mqtt-request-response',
  name: 'MQTT Request/Response Pattern',
  description: getLL().flow.mqtt_request_response_desc(),
  initial_variables: {},
  nodes: [
    // 节点 1: Entry 节点
    {
      uuid: 'entry-mqtt-rpc-001',
      type: 'entry',
      name: 'Entry',
      position: { x: 0, y: 0 },
      data: { type: 'entry' }
    },
    // 节点 2: MQTT Connect 节点
    {
      uuid: 'mqtt-connect-rpc-001',
      type: 'mqtt_connect',
      name: 'Connect Broker',
      position: { x: 250, y: 0 },
      data: {
        type: 'mqtt_connect',
        url: 'mqtt://broker.emqx.io:1883',
        mqtt_version: '5.0'
      }
    },
    // 节点 3: MQTT Request 节点
    {
      uuid: 'mqtt-request-rpc-001',
      type: 'mqtt_request',
      name: 'Send Request',
      position: { x: 500, y: 0 },
      data: {
        type: 'mqtt_request',
        connection_id: 'mqtt-connect-rpc-001',
        request_topic: 'test/rpc/request',
        response_topic: 'test/rpc/response',
        payload: '{"action":"ping","id":"1"}',
        qos: 1,
        timeout_ms: 10000
      }
    },
    // 节点 4: MQTT Disconnect 节点
    {
      uuid: 'mqtt-disconnect-rpc-001',
      type: 'mqtt_disconnect',
      name: 'Disconnect',
      position: { x: 750, y: 0 },
      data: {
        type: 'mqtt_disconnect',
        connection_id: 'mqtt-connect-rpc-001'
      }
    }
  ],
  connections: [
    // Entry → MQTT Connect
    {
      id: 'conn-mqtt-rpc-1',
      source_node_uuid: 'entry-mqtt-rpc-001',
      target_node_uuid: 'mqtt-connect-rpc-001',
      source_port_id: 'out',
      target_port_id: 'in'
    },
    // MQTT Connect → MQTT Request
    {
      id: 'conn-mqtt-rpc-2',
      source_node_uuid: 'mqtt-connect-rpc-001',
      target_node_uuid: 'mqtt-request-rpc-001',
      source_port_id: 'true',
      target_port_id: 'in'
    },
    // MQTT Request → MQTT Disconnect
    {
      id: 'conn-mqtt-rpc-3',
      source_node_uuid: 'mqtt-request-rpc-001',
      target_node_uuid: 'mqtt-disconnect-rpc-001',
      source_port_id: 'true',
      target_port_id: 'in'
    }
  ],
  _meta: {
    version: BUILTIN_FLOW_VERSION,
    isBuiltin: true,
  },
} as Flow & { _meta?: { version: number; isBuiltin: boolean } };

// ============================================================================
// 模板注册表
// ============================================================================

export const FLOW_TEMPLATES: FlowTemplate[] = [
  // ── HTTP 模板 ──
  {
    id: 'template-http-get',
    name: 'HTTP API Request',
    description: 'Demo GET request with conditional logic to check response status',
    category: 'http',
    icon: 'Globe',
    tags: ['HTTP', 'Getting Started'],
    difficulty: 'beginner',
    flow: DEMO_FLOW,
  },
  {
    id: 'template-http-post',
    name: 'HTTP POST Submit',
    description: 'Send POST request to an API with JSON body',
    category: 'http',
    icon: 'Globe',
    tags: ['HTTP', 'POST', 'JSON'],
    difficulty: 'beginner',
    flow: HTTP_POST_FLOW,
  },
  // ── WebSocket 模板 ──
  {
    id: 'template-ws-echo',
    name: 'WebSocket Echo Test',
    description: 'Connect to WebSocket echo server, send and receive messages',
    category: 'websocket',
    icon: 'Wifi',
    tags: ['WebSocket', 'Getting Started'],
    difficulty: 'beginner',
    flow: WS_DEMO_FLOW,
  },
  // ── 数据库模板 ──
  {
    id: 'template-db-sqlite',
    name: 'SQLite Database Query',
    description: 'Connect to SQLite in-memory database, run a query, then close',
    category: 'database',
    icon: 'Database',
    tags: ['Database', 'SQLite', 'Query'],
    difficulty: 'beginner',
    flow: DB_SQLITE_FLOW,
  },
  // ── gRPC 模板 ──
  {
    id: 'template-grpc-call',
    name: 'gRPC Service Call',
    description: 'Connect to gRPC server and call a remote method',
    category: 'grpc',
    icon: 'Server',
    tags: ['gRPC', 'RPC'],
    difficulty: 'intermediate',
    flow: GRPC_DEMO_FLOW,
  },
  // ── MQTT 模板 ──
  {
    id: 'template-mqtt-basic',
    name: 'MQTT Publish/Subscribe',
    description: 'Connect to MQTT broker, subscribe to topic, and publish test message',
    category: 'mqtt',
    icon: 'Zap',
    tags: ['MQTT', 'IoT', 'Getting Started'],
    difficulty: 'beginner',
    flow: MQTT_BASIC_FLOW,
  },
  {
    id: 'template-mqtt-rpc',
    name: 'MQTT Request/Response',
    description: 'Use MQTT 5.0 request/response pattern for RPC communication',
    category: 'mqtt',
    icon: 'MessageSquare',
    tags: ['MQTT', 'RPC', 'Advanced'],
    difficulty: 'advanced',
    flow: MQTT_REQUEST_RESPONSE_FLOW,
  },
  // ── Kafka 模板 ──
  {
    id: 'template-kafka-produce',
    name: 'Kafka Produce Message',
    description: 'Connect to Kafka server and send messages to a topic',
    category: 'kafka',
    icon: 'GitBranch',
    tags: ['Kafka', 'Message Queue'],
    difficulty: 'intermediate',
    flow: KAFKA_PRODUCE_FLOW,
  },
  // ── AMQP 模板 ──
  {
    id: 'template-amqp',
  name: 'AMQP Publish/Consume',
    description: 'Connect to AMQP broker, declare exchange, publish and consume messages',
    category: 'amqp',
    icon: 'Server',
    tags: ['AMQP', 'RabbitMQ'],
    difficulty: 'intermediate',
    flow: AMQP_FLOW,
  },
  // ── GraphQL 模板 ──
  {
    id: 'template-graphql',
    name: 'GraphQL Data Query',
    description: 'Connect to GraphQL endpoint and execute a query',
    category: 'graphql',
    icon: 'Code',
    tags: ['GraphQL', 'API'],
    difficulty: 'intermediate',
    flow: GRAPHQL_QUERY_FLOW,
  },
  // ── IMAP 模板 ──
  {
    id: 'template-imap',
    name: 'IMAP Read Email',
    description: 'Connect to IMAP server and read emails from inbox',
    category: 'imap',
    icon: 'Mail',
    tags: ['IMAP', 'Email'],
    difficulty: 'intermediate',
    flow: IMAP_READ_FLOW,
  },
  // ── SMTP 模板 ──
  {
    id: 'template-smtp',
    name: 'SMTP Send Email',
    description: 'Send emails via SMTP server',
    category: 'smtp',
    icon: 'Send',
    tags: ['SMTP', 'Email'],
    difficulty: 'intermediate',
    flow: SMTP_SEND_FLOW,
  },
  // ── SSE 模板 ──
  {
    id: 'template-sse',
    name: 'SSE Event Listener',
    description: 'Connect to SSE endpoint and listen for server-sent events',
    category: 'sse',
    icon: 'Radio',
    tags: ['SSE', 'Push'],
    difficulty: 'intermediate',
    flow: SSE_FLOW,
  },
  // ── TCP 模板 ──
  {
    id: 'template-tcp',
    name: 'TCP Client Communication',
    description: 'Connect to server via TCP protocol and send data',
    category: 'tcp',
    icon: 'Network',
    tags: ['TCP', 'Networking'],
    difficulty: 'intermediate',
    flow: TCP_SEND_FLOW,
  },
  // ── UDP 模板 ──
  {
    id: 'template-udp',
    name: 'UDP Datagram Send',
    description: 'Bind UDP port and send datagrams to target address',
    category: 'udp',
    icon: 'Radio',
    tags: ['UDP', 'Networking'],
    difficulty: 'intermediate',
    flow: UDP_SEND_FLOW,
  },
  // ── MCP 模板 ──
  {
    id: 'template-mcp',
    name: 'MCP Call Tool',
    description: 'Connect to MCP server and call tools for AI integration',
    category: 'mcp',
    icon: 'Cpu',
    tags: ['MCP', 'AI'],
    difficulty: 'advanced',
    flow: MCP_FLOW,
  },
  // ── 脚本模板 ──
  {
    id: 'template-script',
    name: 'Script Data Transform',
    description: 'Use JavaScript scripts to process and transform data',
    category: 'script',
    icon: 'FileCode',
    tags: ['Script', 'Transform'],
    difficulty: 'intermediate',
    flow: SCRIPT_TRANSFORM_FLOW,
  },
  // ── 组合模板 ──
  {
    id: 'template-http-conditional-email',
    name: 'HTTP → Condition → Email',
    description: 'Comprehensive demo: Check HTTP service status, send email notification based on result',
    category: 'combination',
    icon: 'GitMerge',
    tags: ['HTTP', 'Condition', 'Email', 'Combination'],
    difficulty: 'advanced',
    flow: HTTP_CONDITIONAL_EMAIL_FLOW,
  },
  {
    id: 'template-kafka-script-db',
    name: 'Kafka → Script → Database',
    description: 'Comprehensive demo: Consume Kafka messages, transform with script, store in DB',
    category: 'combination',
    icon: 'GitMerge',
    tags: ['Kafka', 'Script', 'Database', 'ETL'],
    difficulty: 'advanced',
    flow: KAFKA_SCRIPT_DB_FLOW,
  },
  {
    id: 'template-mqtt-http-webhook',
    name: 'MQTT → HTTP Webhook',
    description: 'Comprehensive demo: Subscribe to MQTT topic, forward messages via HTTP webhook',
    category: 'combination',
    icon: 'GitMerge',
    tags: ['MQTT', 'HTTP', 'Webhook', 'Combination'],
    difficulty: 'advanced',
    flow: MQTT_HTTP_WEBHOOK_FLOW,
  },
];

/** 热门模板（前 4 个） */
export const POPULAR_TEMPLATES = FLOW_TEMPLATES.slice(0, 4);
