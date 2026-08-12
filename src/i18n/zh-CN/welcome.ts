import type { BaseTranslation } from '../i18n-types';

const zh_CN_welcome: BaseTranslation = {
  // Template categories
  template_category_http: 'HTTP',
  template_category_websocket: 'WebSocket', 
  template_category_database: '数据库',
  template_category_grpc: 'gRPC',
  template_category_graphql: 'GraphQL',
  template_category_sse: 'SSE',
  template_category_tcp: 'TCP',
  template_category_udp: 'UDP',
  template_category_mcp: 'MCP',
  template_category_script: '脚本',
  template_category_combination: '组合流程',

  // Template names
  template_http_demo: 'HTTP API 请求',
  template_http_demo_desc: '演示 GET/POST 请求和条件判断',
  template_ws_echo: 'WebSocket Echo 测试',
  template_ws_echo_desc: '连接 WebSocket Echo 服务并收发消息',
  template_grpc_demo: 'gRPC 服务调用',
  template_grpc_demo_desc: '连接 gRPC 服务并调用方法',
  template_mcp_demo: 'MCP 工具调用',
  template_mcp_demo_desc: '连接 MCP 服务器并调用工具',
  template_tcp_send_recv: 'TCP 发送接收',
  template_tcp_send_recv_desc: '建立 TCP 连接并收发数据',
  template_udp_communication: 'UDP 通信',
  template_udp_communication_desc: 'UDP 绑定、发送和接收',
  template_graphql_query: 'GraphQL 查询',
  template_graphql_query_desc: 'GraphQL 查询操作',
  template_sse_events: 'SSE 事件监听',
  template_sse_events_desc: 'SSE 服务端事件监听',
  template_script_transform: '脚本数据转换',
  template_script_transform_desc: '使用脚本转换数据',

  // Welcome wizard
  welcome_step1_title: '欢迎使用 MPE',
  welcome_step1_desc: '多协议流程编排与执行引擎，支持 HTTP、WebSocket、数据库、gRPC、MQTT 等 15+ 协议，轻松构建复杂的自动化流程。',
  welcome_step2_title: '创建你的第一个流程',
  welcome_step2_desc: '点击左侧面板的「新建流程」按钮，然后从节点库拖拽节点到画布上，开始编排你的第一个流程。',
  welcome_step3_title: '配置并执行',
  welcome_step3_desc: '选中节点，在右侧面板配置参数，然后点击工具栏的「执行」按钮运行流程。',
  welcome_step1_action: '开始使用',
  welcome_step2_action: '我知道了',
  welcome_step3_action: '开始编排',
  welcome_skip: '跳过引导',

  // Help drawer
  help_title: '帮助',
  help_summary: '概述',
  help_params: '参数说明',
  help_use_cases: '使用场景',
  help_faq: '常见问题',
  help_no_content: '暂无帮助内容',

  // Empty canvas
  empty_title: '还没有流程',
  empty_desc: '创建新流程或选择已有流程开始编排',
  empty_new_flow: '新建空白流程',
  empty_new_flow_desc: '从零开始搭建你的流程',
  empty_from_template: '从模板创建',
  empty_from_template_desc: '选择预置模板快速开始',
  empty_import_flow: '导入流程文件',
  empty_import_flow_desc: '拖入或选择 .mpf 文件',
  empty_quick_start: '快速开始',
  empty_right_click_key: '右键',
  empty_right_click_desc: '画布打开节点选择器',
  empty_global_search: '全局搜索',

  // Tutorial dialogs
  new_flow_from_template: '从模板创建',
  template_tab_title: '模板',
  blank_tab_title: '空白',
  template_category_all: '全部',
  template_difficulty_beginner: '入门',
  template_difficulty_intermediate: '进阶',
  template_difficulty_advanced: '高级',
  template_use: '使用此模板',
  template_search_placeholder: '搜索模板...',
  template_no_matches: '没有找到匹配的模板',
  template_creating: '创建中...',
};

export default zh_CN_welcome;
