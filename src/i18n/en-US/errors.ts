const en_US_errors = {
  // General
  E_CONFIG_ERROR: 'Configuration error',
  E_CONDITION_SYNTAX: 'Condition syntax error',
  E_EXECUTION_ERROR: 'Execution error: {reason}',
  E_TIMEOUT: 'Operation "{operation}" timed out after {ms}ms',
  E_CONDITION_EVALUATION: 'Condition evaluation error: {expression}',
  E_UNKNOWN: 'Unknown error',
  E_CANCELLED: 'Cancelled',
  E_PARSE: 'Parse error: {value}',
  E_ASSERTION: 'Assertion failed',
  E_SCRIPT_ABORT: 'Script aborted',

  // Connection
  E_CONN_TIMEOUT: 'Connection to {host} timed out after {ms}ms',
  E_CONN_REFUSED: 'Connection to {host}:{port} refused',
  E_CONN_CLOSED: 'Connection to {host} closed',

  // Validation
  E_VAL_PORT_DUPLICATE: 'Duplicate port connection',
  E_VAL_CYCLE_DETECTED: 'Cycle detected in flow',
  E_VAL_MISSING_ENTRY: 'Missing entry node',

  // HTTP
  E_HTTP_REQUEST_FAILED: 'HTTP request to {url} failed with status {status}',
  E_HTTP_STATUS_ERROR: 'HTTP error from {url}: {status} - {body}',
  E_HTTP_INVALID_URL: 'Invalid URL: {url}',

  // WebSocket
  E_WS_CONNECTION: 'WebSocket connection failed: {url}',
  E_WS_TIMEOUT: 'WebSocket timeout: {url}',
  E_WS_MESSAGE: 'WebSocket message error: {url}',
  E_WS_CLOSE: 'WebSocket closed: {url}',
  E_WS_PROTOCOL: 'WebSocket protocol error: {url}',

  // Database
  E_DB_CONNECTION: 'Database connection failed: {host}',
  E_DB_QUERY: 'Database query error: {query}',
  E_DB_TIMEOUT: 'Database timeout: {host}',

  // Script
  E_SCRIPT_SYNTAX: 'Script syntax error: {detail}',
  E_SCRIPT_RUNTIME: 'Script runtime error: {detail}',
  E_SCRIPT_TIMEOUT: 'Script timed out after {ms}ms',
  E_SCRIPT_ASSERTION: 'Script assertion failed: {detail}',

  // gRPC
  E_GRPC_CONNECTION: 'gRPC connection failed: {target}',
  E_GRPC_TIMEOUT: 'gRPC timeout: {target}',
  E_GRPC_MESSAGE: 'gRPC message error: {target}',
  E_GRPC_STATUS: 'gRPC error from {target}: status {code}',
  E_GRPC_CLOSE: 'gRPC connection closed: {target}',

  // MCP
  E_MCP_CONNECTION: 'MCP connection failed: {server}',
  E_MCP_TOOL_CALL: 'MCP tool call failed: {tool}',
  E_MCP_RESOURCE_READ: 'MCP resource read failed: {uri}',
  E_MCP_NOT_CONNECTED: 'Not connected to MCP server: {server}',

  // TCP
  E_TCP_CONNECTION: 'TCP connection failed: {addr}',
  E_TCP_TIMEOUT: 'TCP timeout: {addr}',
  E_TCP_SEND: 'TCP send error: {addr}',
  E_TCP_RECEIVE: 'TCP receive error: {addr}',
  E_TCP_CLOSE: 'TCP connection closed: {addr}',

  // UDP
  E_UDP_BIND: 'UDP bind failed: {addr}',
  E_UDP_SEND: 'UDP send error: {addr}',
  E_UDP_RECEIVE: 'UDP receive error: {addr}',
  E_UDP_CLOSE: 'UDP socket closed: {addr}',

  // SSE
  E_SSE_CONNECTION: 'SSE connection failed: {url}',
  E_SSE_TIMEOUT: 'SSE timeout: {url}',
  E_SSE_EVENT: 'SSE event error: {url}',

  // GraphQL
  E_GRAPHQL_CONNECTION: 'GraphQL connection failed: {url}',
  E_GRAPHQL_QUERY: 'GraphQL query error: {url}',
  E_GRAPHQL_SUBSCRIPTION: 'GraphQL subscription error: {url}',
  E_GRAPHQL_INTROSPECTION: 'GraphQL introspection failed: {url}',
  E_GRAPHQL_CLOSE: 'GraphQL connection closed: {url}',

  // AMQP
  E_AMQP_CONNECTION: 'AMQP connection failed: {addr}',
  E_AMQP_CHANNEL: 'AMQP channel error: {addr}',
  E_AMQP_PUBLISH: 'AMQP publish error: {addr}',
  E_AMQP_CONSUME: 'AMQP consume error: {addr}',
  E_AMQP_QUEUE: 'AMQP queue error: {addr}',
  E_AMQP_EXCHANGE: 'AMQP exchange error: {addr}',
  E_AMQP_ACK: 'AMQP ack error: {addr}',
  E_AMQP_MANAGEMENT: 'AMQP management error: {addr}',
  E_AMQP_PROTOCOL: 'AMQP protocol error: {addr}',

  // Redis
  E_REDIS_CONNECTION: 'Redis connection failed: {host}',
  E_REDIS_COMMAND: 'Redis command error: {cmd}',
  E_REDIS_TIMEOUT: 'Redis timeout: {host}',
  E_REDIS_AUTH: 'Redis authentication failed: {host}',

  // SMTP
  E_SMTP_CONNECTION: 'SMTP connection failed: {host}',
  E_SMTP_SEND: 'SMTP send error: {host}',
  E_SMTP_AUTH: 'SMTP authentication failed: {host}',
  E_SMTP_TIMEOUT: 'SMTP timeout: {host}',

  // IMAP
  E_IMAP_CONNECTION: 'IMAP connection failed: {host}',
  E_IMAP_FETCH: 'IMAP fetch error: {host}',
  E_IMAP_SEARCH: 'IMAP search error: {host}',
  E_IMAP_AUTH: 'IMAP authentication failed: {host}',
  E_IMAP_STORE: 'IMAP store error: {host}',
  E_IMAP_TIMEOUT: 'IMAP timeout: {host}',

  // Non-protocol
  E_VARIABLE_EXTRACTOR_FAILED: 'Variable extraction failed: {path}',
  E_PROXY_FAILED: 'Proxy failed: {host}',

  // Plugin
  E_PLUGIN_CRASHED: 'Plugin process crashed: {plugin_name} - {message}',

  // UI
  clipboard_save_failed: 'Failed to save node configuration to clipboard',
};

export default en_US_errors;
