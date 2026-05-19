/**
 * 内置流程模板
 */
import type { Flow } from '@/types/flow';
import { getLL } from '@/i18n/i18n-lazy';

/**
 * HTTP 节点示例流程
 */
/** 内置模板版本号 - 修改模板后递增此数字 */
export const BUILTIN_FLOW_VERSION = 5;


export const DEMO_FLOW: Flow = {
  uuid: 'demo-http-testing',
  name: 'HTTP节点示例流程',
  initial_variables: {},
  nodes: [
    // 节点 1: Entry 节点
    {
      uuid: 'entry-001',
      type: 'entry',
      name: '入口',
      position: { x: 0, y: 0 },
      data: { type: 'entry' }
    },
    // 节点 2: HTTP GET 节点
    {
      uuid: 'http-get-001',
      type: 'http',
      name: 'HTTP GET 示例',
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
      name: '检查状态码',
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
      name: 'HTTP POST 示例',
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
  name: 'WebSocket节点示例流程',
  initial_variables: {},
  nodes: [
    // 节点 1: Entry 节点
    {
      uuid: 'entry-ws-001',
      type: 'entry',
      name: '入口',
      position: { x: 0, y: 0 },
      data: { type: 'entry' }
    },
    // 节点 2: WS Connect 节点
    {
      uuid: 'ws-connect-001',
      type: 'ws_connect',
      name: '连接WebSocket',
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
      name: '发送消息并收集响应',
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
      name: '关闭连接',
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
  name: 'gRPC节点示例流程',
  initial_variables: {},
  nodes: [
    // 节点 1: Entry 节点
    {
      uuid: 'entry-grpc-001',
      type: 'entry',
      name: '入口',
      position: { x: 0, y: 0 },
      data: { type: 'entry' }
    },
    // 节点 2: gRPC Connect 节点
    {
      uuid: 'grpc-connect-001',
      type: 'grpc_connect',
      name: '连接gRPC服务',
      position: { x: 250, y: 0 },
      data: {
        type: 'grpc_connect',
        url: 'http://localhost:50051',
        use_tls: false,
        enable_reflection: true,
        proto_files: [
          {
            path: 'example.proto',
            content: '// 请替换为你的 Proto 文件内容\nsyntax = "proto3";\n\npackage example;\n\nservice ExampleService {\n  rpc GetData (GetDataRequest) returns (GetDataResponse);\n}\n\nmessage GetDataRequest {\n  string id = 1;\n}\n\nmessage GetDataResponse {\n  string result = 1;\n}'
          }
        ],
        connect_timeout_ms: 10000
      }
    },
    // 节点 3: gRPC Call 节点
    {
      uuid: 'grpc-call-001',
      type: 'grpc_call',
      name: '调用gRPC方法',
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
      name: '关闭连接',
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
  name: 'MQTT 基础连接测试',
  description: getLL().flow.mqtt_basic_desc(),
  initial_variables: { "timestamp": "" },
  nodes: [
    // 节点 1: Entry 节点
    {
      uuid: 'entry-mqtt-basic-001',
      type: 'entry',
      name: '入口',
      position: { x: 0, y: 0 },
      data: { type: 'entry' }
    },
    // 节点 2: MQTT Connect 节点
    {
      uuid: 'mqtt-connect-basic-001',
      type: 'mqtt_connect',
      name: '连接 Broker',
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
      name: '订阅主题',
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
      name: '发布测试消息',
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
      name: '断开连接',
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
  name: 'MQTT 请求/响应模式',
  description: getLL().flow.mqtt_request_response_desc(),
  initial_variables: {},
  nodes: [
    // 节点 1: Entry 节点
    {
      uuid: 'entry-mqtt-rpc-001',
      type: 'entry',
      name: '入口',
      position: { x: 0, y: 0 },
      data: { type: 'entry' }
    },
    // 节点 2: MQTT Connect 节点
    {
      uuid: 'mqtt-connect-rpc-001',
      type: 'mqtt_connect',
      name: '连接 Broker',
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
      name: '发送请求',
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
      name: '断开连接',
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
