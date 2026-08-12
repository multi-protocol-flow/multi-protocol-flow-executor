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



  // Non-protocol
  E_VARIABLE_EXTRACTOR_FAILED: 'Variable extraction failed: {path}',
  E_PROXY_FAILED: 'Proxy failed: {host}',

  // Plugin
  E_PLUGIN_CRASHED: 'Plugin process crashed: {plugin_name} - {message}',

  // UI
  clipboard_save_failed: 'Failed to save node configuration to clipboard',
};

export default en_US_errors;
