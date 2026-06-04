/**
 * 节点帮助内容注册表
 * 为每种节点类型提供帮助文档内容
 */
export interface NodeHelpParam {
  name: string;
  type: string;
  required: boolean;
  description: string;
  example: string;
}

export interface NodeHelpUseCase {
  title: string;
  description: string;
  flowExample?: string;
}

export interface NodeHelpFAQ {
  question: string;
  answer: string;
}

export interface NodeHelpContent {
  typeId: string;
  summary: string;
  params: NodeHelpParam[];
  useCases: NodeHelpUseCase[];
  faq: NodeHelpFAQ[];
}

const nodeHelpRegistry = new Map<string, NodeHelpContent>();

// Register HTTP help
nodeHelpRegistry.set('http', {
  typeId: 'http',
  summary: '发送 HTTP 请求，支持 GET/POST/PUT/DELETE/PATCH 等方法。支持 JSON、表单、文件等多种请求体格式。',
  params: [
    { name: 'url', type: 'string', required: true, description: '请求目标 URL', example: 'https://api.example.com/users' },
    { name: 'method', type: 'enum', required: true, description: 'HTTP 请求方法', example: 'GET' },
    { name: 'headers', type: 'object', required: false, description: '自定义请求头', example: '{ "Authorization": "Bearer token" }' },
    { name: 'timeout_ms', type: 'number', required: false, description: '请求超时时间（毫秒）', example: '30000' },
  ],
  useCases: [
    { title: 'REST API 调用', description: '调用 RESTful API 获取或提交数据，支持各种认证方式。' },
    { title: 'Webhook 触发', description: '作为流程中的 Webhook 调用节点，触发外部系统操作。' },
  ],
  faq: [
    { question: '支持 HTTPS 吗？', answer: '支持，TLS 配置中可选择跳过证书验证用于开发环境。' },
    { question: '如何引用上一个节点的响应？', answer: '使用 {{body.field}} 引用响应体，{{status}} 引用状态码。' },
  ],
});

// Register entry help
nodeHelpRegistry.set('entry', {
  typeId: 'entry',
  summary: '流程的起点节点。每个流程必须有且仅有一个入口节点。',
  params: [
    { name: 'initial_variables', type: 'object', required: false, description: '流程初始变量', example: '{ "api_base": "https://api.example.com" }' },
  ],
  useCases: [
    { title: '定义初始变量', description: '在入口节点设置流程的初始变量，供后续节点引用。' },
  ],
  faq: [
    { question: '一个流程能有多个入口吗？', answer: '不可以，每个流程必须有且仅有一个入口节点。' },
  ],
});

// Register conditional help
nodeHelpRegistry.set('conditional', {
  typeId: 'conditional',
  summary: '根据条件选择执行路径，支持简单比较和高级表达式。',
  params: [
    { name: 'condition_type', type: 'enum', required: true, description: '条件类型：simple 或 advanced', example: 'simple' },
    { name: 'field_path', type: 'string', required: true, description: '要检查的字段路径', example: 'status' },
    { name: 'operator', type: 'enum', required: true, description: '比较操作符', example: 'equal' },
    { name: 'expected_value', type: 'any', required: true, description: '期望值', example: '200' },
  ],
  useCases: [
    { title: 'HTTP 状态码判断', description: '根据 HTTP 响应的状态码选择不同执行路径。' },
    { title: '数据字段校验', description: '根据上游节点输出中的字段值进行分支选择。' },
  ],
  faq: [
    { question: 'true 和 false 端口有什么区别？', answer: '条件成立时走 true 端口，不成立时走 false 端口。' },
  ],
});

// Register script help
nodeHelpRegistry.set('script', {
  typeId: 'script',
  summary: '执行 QuickJS JavaScript 代码，用于数据转换、变量操作和复杂逻辑。',
  params: [
    { name: 'code', type: 'string', required: true, description: 'JavaScript 脚本代码', example: 'fn.variables.set("result", body.data)' },
    { name: 'timeout_ms', type: 'number', required: false, description: '脚本执行超时', example: '5000' },
  ],
  useCases: [
    { title: '数据转换', description: '将上游节点的输出数据转换为下游需要的格式。' },
    { title: '变量操作', description: '读取、设置或删除流程变量，实现复杂状态管理。' },
  ],
  faq: [
    { question: '支持 async/await 吗？', answer: '支持 ES2020 语法，包括 async/await、Promise 等。' },
    { question: '脚本中能访问网络吗？', answer: '不能，脚本在沙箱环境中运行，禁止网络请求和文件系统访问。' },
  ],
});

// Add help for all other protocol types with at least basic info
const PROTOCOL_TYPES = ['ws_connect', 'ws_send_collect', 'ws_close', 'db_connect', 'db_query', 'db_begin', 'db_commit', 'db_rollback', 'db_close', 'grpc_connect', 'grpc_call', 'grpc_close', 'mcp_connect', 'mcp_call_tool', 'mcp_read_resource', 'mcp_get_prompt', 'mcp_close', 'tcp_connect', 'tcp_send', 'tcp_close', 'udp_bind', 'udp_send_to', 'udp_recv_from', 'udp_close', 'mqtt_connect', 'mqtt_publish', 'mqtt_subscribe', 'mqtt_unsubscribe', 'mqtt_disconnect', 'mqtt_wait_message', 'mqtt_request', 'amqp_connect', 'amqp_publish', 'amqp_consume', 'graphql_introspect', 'graphql_query', 'graphql_subscribe', 'graphql_unsubscribe', 'imap_connect', 'imap_fetch', 'imap_search', 'imap_folder', 'imap_idle', 'kafka_produce', 'kafka_consume', 'kafka_batch_consume', 'kafka_admin', 'smtp_connect', 'smtp_send', 'smtp_close', 'sse_connect', 'sse_listen', 'sse_close', 'assertion', 'end'];

const PROTOCOL_TYPE_META: Record<string, { summary: string; params: NodeHelpParam[] }> = {
  ws_connect: { summary: '建立 WebSocket 连接，可选在连接后收集消息。', params: [{ name: 'url', type: 'string', required: true, description: 'WebSocket 服务端 URL', example: 'wss://echo.websocket.org' }, { name: 'connect_timeout_ms', type: 'number', required: false, description: '连接超时', example: '10000' }] },
  ws_send_collect: { summary: '通过 WebSocket 发送消息，可选收集响应。', params: [{ name: 'message', type: 'string', required: true, description: '要发送的消息内容', example: '{"action":"ping"}' }] },
  ws_close: { summary: '关闭 WebSocket 连接。', params: [{ name: 'close_code', type: 'number', required: false, description: '关闭状态码', example: '1000' }] },
  db_connect: { summary: '建立数据库连接，支持 PostgreSQL/MySQL/SQLite/Redis。', params: [{ name: 'driver', type: 'enum', required: true, description: '数据库类型', example: 'postgresql' }] },
  db_query: { summary: '执行 SQL 查询语句，支持参数化查询。', params: [{ name: 'sql', type: 'string', required: true, description: 'SQL 查询语句', example: 'SELECT * FROM users WHERE id = ?' }] },
  db_begin: { summary: '开始数据库事务，可指定隔离级别。', params: [{ name: 'isolation_level', type: 'enum', required: false, description: '事务隔离级别', example: 'ReadCommitted' }] },
  db_commit: { summary: '提交当前数据库事务。', params: [] },
  db_rollback: { summary: '回滚当前数据库事务。', params: [] },
  db_close: { summary: '关闭数据库连接，释放连接池资源。', params: [] },
  grpc_connect: { summary: '建立 gRPC 连接，支持 Server Reflection。', params: [{ name: 'url', type: 'string', required: true, description: 'gRPC 服务端地址', example: 'http://localhost:50051' }] },
  grpc_call: { summary: '调用 gRPC 服务方法，支持 Unary/SS/CS/Bidi。', params: [{ name: 'service_name', type: 'string', required: true, description: '服务名称', example: 'example.ExampleService' }] },
  grpc_close: { summary: '关闭 gRPC 连接。', params: [] },
  mcp_connect: { summary: '建立 MCP 协议连接，支持 Streamable HTTP 和 Stdio 模式。', params: [] },
  mcp_call_tool: { summary: '调用 MCP 服务器的工具，传入参数并获取结果。', params: [{ name: 'tool_name', type: 'string', required: true, description: '工具名称', example: 'search' }] },
  mcp_read_resource: { summary: '读取 MCP 服务器上的资源。', params: [{ name: 'resource_uri', type: 'string', required: true, description: '资源 URI', example: 'file:///config.json' }] },
  mcp_get_prompt: { summary: '从 MCP 服务器获取提示词模板。', params: [{ name: 'prompt_name', type: 'string', required: true, description: '提示词名称', example: 'greeting' }] },
  mcp_close: { summary: '关闭 MCP 连接。', params: [] },
  tcp_connect: { summary: '建立 TCP 连接，支持 TLS 加密。', params: [{ name: 'host', type: 'string', required: true, description: '主机地址', example: '127.0.0.1' }, { name: 'port', type: 'number', required: true, description: '端口号', example: '8080' }] },
  tcp_send: { summary: '通过已建立的 TCP 连接发送数据。', params: [{ name: 'data', type: 'string', required: true, description: '要发送的数据', example: 'Hello World' }] },
  tcp_close: { summary: '关闭 TCP 连接。', params: [] },
  udp_bind: { summary: '绑定本地 UDP 端口，支持广播和多播。', params: [{ name: 'local_port', type: 'number', required: true, description: '本地端口', example: '41234' }] },
  udp_send_to: { summary: '通过 UDP 发送数据到指定地址。', params: [{ name: 'target_addr', type: 'string', required: true, description: '目标地址:端口', example: '127.0.0.1:41234' }] },
  udp_recv_from: { summary: '从 UDP 端口接收数据。', params: [{ name: 'buffer_size', type: 'number', required: false, description: '接收缓冲区大小', example: '65535' }] },
  udp_close: { summary: '关闭 UDP 端口。', params: [] },
  mqtt_connect: { summary: '建立 MQTT Broker 连接，支持 5.0 和 3.1.1 协议。', params: [{ name: 'url', type: 'string', required: true, description: 'Broker 地址', example: 'mqtt://broker.emqx.io:1883' }] },
  mqtt_publish: { summary: '向 MQTT Broker 发布消息到指定主题。', params: [{ name: 'topic', type: 'string', required: true, description: '发布主题', example: 'test/topic' }] },
  mqtt_subscribe: { summary: '订阅 MQTT 主题并接收消息。', params: [{ name: 'topic', type: 'string', required: true, description: '订阅主题', example: 'test/#' }] },
  mqtt_unsubscribe: { summary: '取消订阅 MQTT 主题。', params: [{ name: 'topic', type: 'string', required: true, description: '取消订阅的主题', example: 'test/#' }] },
  mqtt_disconnect: { summary: '断开 MQTT Broker 连接。', params: [] },
  mqtt_wait_message: { summary: '等待接收 MQTT 消息。', params: [{ name: 'timeout_ms', type: 'number', required: false, description: '等待超时', example: '30000' }] },
  mqtt_request: { summary: 'MQTT 请求/响应模式，发送请求并等待响应。', params: [{ name: 'request_topic', type: 'string', required: true, description: '请求主题', example: 'request/topic' }, { name: 'response_topic', type: 'string', required: true, description: '响应主题', example: 'response/topic' }] },
  amqp_connect: { summary: '建立 AMQP 连接，支持 0-9-1 和 1.0 协议。', params: [{ name: 'url', type: 'string', required: true, description: 'AMQP 服务端地址', example: 'amqp://guest:guest@localhost:5672' }] },
  amqp_publish: { summary: '发布消息到 AMQP 交换器或队列。', params: [{ name: 'exchange', type: 'string', required: false, description: '交换器名称', example: 'amq.direct' }] },
  amqp_consume: { summary: '消费 AMQP 队列中的消息。', params: [{ name: 'queue', type: 'string', required: true, description: '队列名称', example: 'my_queue' }] },
  graphql_introspect: { summary: '对 GraphQL 端点执行内省查询，获取 Schema。', params: [{ name: 'endpoint', type: 'string', required: true, description: 'GraphQL 端点 URL', example: 'https://api.example.com/graphql' }] },
  graphql_query: { summary: '执行 GraphQL 查询或变更操作。', params: [{ name: 'query', type: 'string', required: true, description: 'GraphQL 查询语句', example: 'query { users { id name } }' }] },
  graphql_subscribe: { summary: '订阅 GraphQL 实时更新。', params: [{ name: 'subscription', type: 'string', required: true, description: 'GraphQL 订阅语句', example: 'subscription { onMessage { text } }' }] },
  graphql_unsubscribe: { summary: '取消 GraphQL 订阅。', params: [] },
  imap_connect: { summary: '建立 IMAP 连接，支持 SSL/TLS。', params: [{ name: 'host', type: 'string', required: true, description: 'IMAP 服务器地址', example: 'imap.example.com' }] },
  imap_fetch: { summary: '获取指定邮件的完整内容。', params: [{ name: 'mailbox', type: 'string', required: false, description: '邮箱文件夹', example: 'INBOX' }] },
  imap_search: { summary: '搜索符合条件的邮件。', params: [{ name: 'criteria', type: 'string', required: true, description: '搜索条件', example: 'ALL' }] },
  imap_folder: { summary: '管理 IMAP 文件夹（创建/删除/重命名）。', params: [{ name: 'action', type: 'enum', required: true, description: '文件夹操作', example: 'create' }] },
  imap_idle: { summary: '进入 IMAP IDLE 模式等待新邮件通知。', params: [{ name: 'timeout_ms', type: 'number', required: false, description: '等待超时', example: '30000' }] },
  kafka_produce: { summary: '向 Kafka 主题生产消息。', params: [{ name: 'topic', type: 'string', required: true, description: 'Kafka 主题', example: 'my-topic' }] },
  kafka_consume: { summary: '从 Kafka 主题消费消息。', params: [{ name: 'topic', type: 'string', required: true, description: 'Kafka 主题', example: 'my-topic' }] },
  kafka_batch_consume: { summary: '批量消费 Kafka 消息。', params: [{ name: 'topic', type: 'string', required: true, description: 'Kafka 主题', example: 'my-topic' }, { name: 'max_messages', type: 'number', required: false, description: '最大消息数', example: '100' }] },
  kafka_admin: { summary: '执行 Kafka 管理操作（创建/删除主题等）。', params: [{ name: 'operation', type: 'enum', required: true, description: '管理操作类型', example: 'create_topic' }] },
  smtp_connect: { summary: '建立 SMTP 连接，支持 SSL/TLS/STARTTLS。', params: [{ name: 'host', type: 'string', required: true, description: 'SMTP 服务器地址', example: 'smtp.example.com' }] },
  smtp_send: { summary: '发送电子邮件，支持 HTML 内容和附件。', params: [{ name: 'from', type: 'string', required: true, description: '发件人地址', example: 'sender@example.com' }] },
  smtp_close: { summary: '关闭 SMTP 连接。', params: [] },
  sse_connect: { summary: '建立 SSE 连接，监听服务端事件。', params: [{ name: 'url', type: 'string', required: true, description: 'SSE 端点 URL', example: 'https://api.example.com/events' }] },
  sse_listen: { summary: '监听 SSE 事件流。', params: [{ name: 'event_type', type: 'string', required: false, description: '要监听的事件类型', example: 'message' }] },
  sse_close: { summary: '关闭 SSE 连接。', params: [] },
  assertion: { summary: '对上游节点的输出进行断言验证，支持 15 种操作符。', params: [{ name: 'operator', type: 'enum', required: true, description: '断言操作符', example: 'eq' }] },
  end: { summary: '流程的终点节点，标记流程结束。', params: [] },
};

for (const typeId of PROTOCOL_TYPES) {
  const meta = PROTOCOL_TYPE_META[typeId];
  if (meta) {
    nodeHelpRegistry.set(typeId, {
      typeId,
      summary: meta.summary,
      params: meta.params,
      useCases: [{ title: '标准使用', description: '按参数说明配置即可。' }],
      faq: [],
    });
  }
}

export function getNodeHelp(typeId: string): NodeHelpContent | undefined {
  return nodeHelpRegistry.get(typeId);
}

export function getAllNodeHelpTypes(): string[] {
  return Array.from(nodeHelpRegistry.keys());
}
