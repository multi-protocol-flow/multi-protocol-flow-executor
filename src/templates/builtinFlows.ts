/**
 * 内置流程模板
 */
import type { Flow } from '@/types/flow';

/**
 * HTTP 节点示例流程
 */
/** 内置模板版本号 - 修改模板后递增此数字 */
export const BUILTIN_FLOW_VERSION = 4;


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
    // HTTP GET → 条件
    {
      id: 'conn-2',
      source_node_uuid: 'http-get-001',
      target_node_uuid: 'conditional-001',
      source_port_id: 'out',
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
        wait_for_message: false,
        message_timeout_ms: 5000
      }
    },
    // 节点 3: WS Send+Wait 节点
    {
      uuid: 'ws-send-wait-001',
      type: 'ws_send_wait',
      name: '发送消息并等待响应',
      position: { x: 500, y: 0 },
      data: {
        type: 'ws_send_wait',
        connection_id: 'ws-connect-001',
        message: '{"action":"ping","data":"Hello WebSocket!"}',
        message_type: 'text',
        send_timeout_ms: 5000,
        wait_for_response: true,
        wait_mode: 'first_message',
        message_timeout_ms: 5000
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
        wait_close_timeout_ms: 5000
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
    // WS Connect → WS Send+Wait
    {
      id: 'conn-ws-2',
      source_node_uuid: 'ws-connect-001',
      target_node_uuid: 'ws-send-wait-001',
      source_port_id: 'true',
      target_port_id: 'in'
    },
    // WS Send+Wait → WS Close
    {
      id: 'conn-ws-3',
      source_node_uuid: 'ws-send-wait-001',
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
