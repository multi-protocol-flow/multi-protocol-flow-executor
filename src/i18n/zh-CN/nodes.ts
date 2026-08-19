const zh_CN_nodes = {
  add_node_title: '添加节点',
  category: {
    all: '全部',
    http: 'HTTP',
    websocket: 'WS',
    sse: 'SSE',
    graphql: 'GraphQL',
    tools: 'Tool',
    script: '脚本',
    assertion: '断言',
    database: 'DB',
    tcp: 'TCP',
    udp: 'UDP'
  },
  superCategory: {
    network: '网络协议',
    services: '服务',
    messaging: '消息队列',
    tools: '工具',
    other: '其他',
  },
  no_nodes_available: '暂无可用节点',
  entry: '入口',
  http: 'HTTP 请求',
  conditional: '条件判断',
  script: '脚本',
  assertion: '断言',
  end: '终止',
  end_config_hint: '此节点无需配置，仅用于标记流程终点。',
  ws_connect: 'WS-连接',
  ws_send_collect: 'WS-发送采集',
  ws_close: 'WS-关闭',
  tcp_connect: 'TCP 连接',
  tcp_send: 'TCP 发送',
  tcp_close: 'TCP 关闭',
  udp_bind: 'UDP 绑定',
  udp_send_to: 'UDP 发送',
  udp_recv_from: 'UDP 接收',
  udp_close: 'UDP 关闭',
  tcp_receive: 'TCP 接收',
  sse_connect: 'SSE 连接',
  sse_listen: 'SSE 监听',
  sse_disconnect: 'SSE 断开',
  graphql_connect: 'GraphQL 连接',
  graphql_query: 'GraphQL 查询',
  graphql_subscribe: 'GraphQL 订阅',
  graphql_introspect: 'GraphQL 内省',
  graphql_disconnect: 'GraphQL 断开',
  variable_extractor: '变量提取',
  proxy: '代理设置',
  /** 断言节点模式标签 */
  assertion_mode: {
    all_pass: '全部通过',
    any_pass: '任一通过',
  },
  /** 断言节点失败处理标签 */
  assertion_on_failure: {
    route: '路由',
    throw: '抛出',
  },
  /** 断言节点执行错误处理标签 */
  assertion_on_error: {
    route_to_false: '路由',
    abort_flow: '终止',
  },
  /** 代理节点模式标签 */
  proxy_mode: {
    clear: '清除',
  },
};

export default zh_CN_nodes;
