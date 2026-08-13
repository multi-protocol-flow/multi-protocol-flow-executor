const en_US_nodes = {
  add_node_title: 'Add Node',
  category: {
    all: 'All',
    http: 'HTTP',
    websocket: 'WS',
    sse: 'SSE',
    graphql: 'GraphQL',
    grpc: 'gRPC',
    tools: 'Tool',
    script: 'Script',
    assertion: 'Assertion',
    database: 'DB',
    tcp: 'TCP',
    udp: 'UDP'
  },
  superCategory: {
    network: 'Network',
    services: 'Services',
    messaging: 'Messaging',
    tools: 'Tools',
    other: 'Other',
  },
  no_nodes_available: 'No nodes available',
  entry: 'Entry',
  http: 'HTTP Request',
  conditional: 'Conditional',
  script: 'Script',
  assertion: 'Assertion',
  end: 'End',
  end_config_hint: 'This node requires no configuration. It marks the end of the flow.',
  ws_connect: 'WS-Connect',
  ws_send_collect: 'WS-Send/Collect',
  ws_close: 'WS-Close',
  grpc_connect: 'gRPC-Connect',
  grpc_call: 'gRPC-Call',
  grpc_close: 'gRPC-Close',
  tcp_connect: 'TCP Connect',
  tcp_send: 'TCP Send',
  tcp_close: 'TCP Close',
  udp_bind: 'UDP Bind',
  udp_send_to: 'UDP Send',
  udp_recv_from: 'UDP Receive',
  udp_close: 'UDP Close',
  tcp_receive: 'TCP Receive',
  sse_connect: 'SSE Connect',
  sse_listen: 'SSE Listen',
  sse_disconnect: 'SSE Disconnect',
  graphql_connect: 'GraphQL Connect',
  graphql_query: 'GraphQL Query',
  graphql_subscribe: 'GraphQL Subscribe',
  graphql_introspect: 'GraphQL Introspect',
  graphql_disconnect: 'GraphQL Disconnect',
  variable_extractor: 'Variable Extractor',
  proxy: 'Proxy Settings',
  /** Assertion node mode labels */
  assertion_mode: {
    all_pass: 'All Pass',
    any_pass: 'Any Pass',
  },
  /** Assertion node on-failure labels */
  assertion_on_failure: {
    route: 'Route',
    throw: 'Throw',
  },
  /** Assertion node on-error labels (execution crash/timeout) */
  assertion_on_error: {
    route_to_false: 'Route',
    abort_flow: 'Abort',
  },
  /** Proxy node mode labels */
  proxy_mode: {
    clear: 'Clear',
  },
};

export default en_US_nodes;
