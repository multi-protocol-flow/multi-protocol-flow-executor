/**
 * 内置流程模板
 */
import type { Flow } from '@/types/flow';

// ============================================================================
// 模板系统类型定义
// ============================================================================

/** 模板分类 */
export type TemplateCategory =
  | 'http' | 'websocket' | 'database'
  | 'graphql'
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
      uuid: 'http-post-request-001', type: 'http:request', name: 'POST Data', position: { x: 250, y: 0 },
      data: { type: 'http:request', url: 'https://httpbin.org/post', method: 'post', body_type: 'body', content_type: 'json', body: '{"title":"foo","body":"bar","userId":1}' }
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
      uuid: 'db-connect-001', type: 'db:connect', name: 'Connect Database', position: { x: 250, y: 0 },
      data: { type: 'db:connect', connection_uuid: '', driver: 'sqlite', connection_string: ':memory:' }
    },
    {
      uuid: 'db-query-001', type: 'db:query', name: 'Execute Query', position: { x: 500, y: 0 },
      data: { type: 'db:query', connection_uuid: 'db-connect-001', sql: 'SELECT 1 as result;' }
    },
    {
      uuid: 'db-close-001', type: 'db:close', name: 'Close Connection', position: { x: 750, y: 0 },
      data: { type: 'db:close', connection_uuid: 'db-connect-001' }
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
      uuid: 'tcp-connect-001', type: 'tcp:connect', name: 'TCP Connect', position: { x: 250, y: 0 },
      data: { type: 'tcp:connect', host: 'example.com', port: 80, connect_timeout_ms: 5000 }
    },
    {
      uuid: 'tcp-send-001', type: 'tcp:send', name: 'Send Data', position: { x: 500, y: 0 },
      data: { type: 'tcp:send', connection_id: 'tcp-connect-001', data: 'Hello TCP Server', send_timeout_ms: 5000 }
    },
    {
      uuid: 'tcp-close-001', type: 'tcp:close', name: 'Close Connection', position: { x: 750, y: 0 },
      data: { type: 'tcp:close', connection_id: 'tcp-connect-001' }
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
      uuid: 'udp-bind-001', type: 'udp:bind', name: 'Bind Port', position: { x: 250, y: 0 },
      data: { type: 'udp:bind', local_port: 0 }
    },
    {
      uuid: 'udp-send-001', type: 'udp:send_to', name: 'Send Data', position: { x: 500, y: 0 },
      data: { type: 'udp:send_to', connection_id: 'udp-bind-001', data: 'Hello UDP', target_addr: '127.0.0.1', target_port: 9999 }
    },
    {
      uuid: 'udp-close-001', type: 'udp:close', name: 'Close', position: { x: 750, y: 0 },
      data: { type: 'udp:close', connection_id: 'udp-bind-001' }
    },
  ],
  connections: [
    { id: 'conn-udp-1', source_node_uuid: 'entry-udp-001', target_node_uuid: 'udp-bind-001', source_port_id: 'out', target_port_id: 'in' },
    { id: 'conn-udp-2', source_node_uuid: 'udp-bind-001', target_node_uuid: 'udp-send-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-udp-3', source_node_uuid: 'udp-send-001', target_node_uuid: 'udp-close-001', source_port_id: 'true', target_port_id: 'in' },
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
      uuid: 'gql-connect-001', type: 'graphql:connect', name: 'Connect GraphQL', position: { x: 250, y: 0 },
      data: { type: 'graphql:connect', endpoint: 'https://countries.trevorblades.com/graphql' }
    },
    {
      uuid: 'gql-query-001', type: 'graphql:query', name: 'Query Data', position: { x: 500, y: 0 },
      data: { type: 'graphql:query', connection_id: 'gql-connect-001', query: '{ countries { name code } }', timeout_ms: 10000 }
    },
    {
      uuid: 'gql-disconnect-001', type: 'graphql:disconnect', name: 'Disconnect', position: { x: 750, y: 0 },
      data: { type: 'graphql:disconnect', connection_id: 'gql-connect-001' }
    },
  ],
  connections: [
    { id: 'conn-gql-1', source_node_uuid: 'entry-gql-001', target_node_uuid: 'gql-connect-001', source_port_id: 'out', target_port_id: 'in' },
    { id: 'conn-gql-2', source_node_uuid: 'gql-connect-001', target_node_uuid: 'gql-query-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-gql-3', source_node_uuid: 'gql-query-001', target_node_uuid: 'gql-disconnect-001', source_port_id: 'true', target_port_id: 'in' },
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
      uuid: 'sse-connect-001', type: 'sse:connect', name: 'Connect SSE', position: { x: 250, y: 0 },
      data: { type: 'sse:connect', url: 'https://example.com/sse', timeout_ms: 10000, auto_reconnect: true }
    },
    {
      uuid: 'sse-listen-001', type: 'sse:listen', name: 'Listen for Events', position: { x: 500, y: 0 },
      data: { type: 'sse:listen', connection_id: 'sse-connect-001', max_events: 5, timeout_ms: 30000 }
    },
    {
      uuid: 'sse-close-001', type: 'sse:disconnect', name: 'Disconnect', position: { x: 750, y: 0 },
      data: { type: 'sse:disconnect', connection_id: 'sse-connect-001' }
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

/** MCP Connect/CallTool（插件节点：`mcp:*` 命名空间，由 plugins/mcp 提供） */
const MCP_FLOW: Flow = {
  uuid: 'template-mcp',
  name: 'MCP Call Tool',
  initial_variables: {},
  nodes: [
    { uuid: 'entry-mcp-001', type: 'entry', name: 'Entry', position: { x: 0, y: 0 }, data: { type: 'entry' } },
    {
      uuid: 'mcp-connect-001', type: 'mcp:connect', name: 'Connect MCP', position: { x: 250, y: 0 },
      data: { type: 'mcp:connect', transport: 'stdio', command: 'npx', args: ['-y', '@modelcontextprotocol/server-everything'], env: {}, protocol_version: '2025-11-25' }
    },
    {
      uuid: 'mcp-call-001', type: 'mcp:call_tool', name: 'Call Tool', position: { x: 500, y: 0 },
      data: { type: 'mcp:call_tool', connection_id: 'mcp-connect-001', tool_name: 'echo', arguments: { message: 'Hello MCP' } }
    },
    {
      uuid: 'mcp-close-001', type: 'mcp:close', name: 'Close Connection', position: { x: 750, y: 0 },
      data: { type: 'mcp:close', connection_id: 'mcp-connect-001' }
    },
  ],
  connections: [
    { id: 'conn-mcp-1', source_node_uuid: 'entry-mcp-001', target_node_uuid: 'mcp-connect-001', source_port_id: 'out', target_port_id: 'in' },
    { id: 'conn-mcp-2', source_node_uuid: 'mcp-connect-001', target_node_uuid: 'mcp-call-001', source_port_id: 'true', target_port_id: 'in' },
    { id: 'conn-mcp-3', source_node_uuid: 'mcp-call-001', target_node_uuid: 'mcp-close-001', source_port_id: 'true', target_port_id: 'in' },
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
      type: 'http:request',
      name: 'HTTP GET Example',
      position: { x: 250, y: 0 },
      data: {
        type: 'http:request',
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
      type: 'http:request',
      name: 'HTTP POST Example',
      position: { x: 750, y: 0 },
      data: {
        type: 'http:request',
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
      type: 'ws:connect',
      name: 'Connect WebSocket',
      position: { x: 250, y: 0 },
      data: {
        type: 'ws:connect',
        url: 'wss://echo.websocket.org',
        connect_timeout_ms: 10000,
        collect_message: false,
        collect_timeout_ms: 5000
      }
    },
    // 节点 3: WS Send+Collect 节点
    {
      uuid: 'ws-send-collect-001',
      type: 'ws:send_collect',
      name: 'Send and Collect Response',
      position: { x: 500, y: 0 },
      data: {
        type: 'ws:send_collect',
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
      type: 'ws:close',
      name: 'Close Connection',
      position: { x: 750, y: 0 },
      data: {
        type: 'ws:close',
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
  // ── SMTP 模板 ──
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
];

