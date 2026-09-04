/**
 * AI Canvas & Workflow Manipulation Tools
 */

import { useFlowStore } from '@/stores/flow';
import { useExecutionStore } from '@/stores/execution';
import { getCachedDefaultConfig } from '@/lib/NodeRegistry';
import { getLayoutedPositions } from '@/components/canvas/layout';
import { createDefaultNodeData } from '@/components/canvas/utils';
import type { Node, Connection, NodeType, NodeData, AnyNodeType, ExecutionErrorStrategy } from '@/types';
import type { AIToolDefinition } from '@/types/ai';

/**
 * 智能解析源端口和目标端口，纠偏 LLM 常犯的端口命名错误
 *
 * 规则：
 * 1. 目标端口（Target Port）：
 *    - 在 MPE 架构中，所有接收输入的节点输入端口一律为 'in'。
 *    - 无论模型传入 'default'、空还是其他别名，均纠偏为 'in'。
 * 2. 源端口（Source Port）：
 *    - 'entry'、'variable_extractor'、'proxy'、'sse_disconnect'、'graphql_disconnect' 的输出端口为 'out'。
 *    - 'http'、'script'、'assertion'、'conditional'、'ws_*'、'tcp_*'、'udp_*'、'sse_*'、'graphql_*' 以及插件节点的输出端口为 'true' (主干成功流) 和 'false' (异常/失败分支)。
 *    - 如果模型传了 'default' 或遗漏：按源节点类型自动推断为 'out' 或 'true'。
 *    - 如果模型误给协议节点传了 'out'：自动重定向为 'true'，防止流程执行静默中断。
 */
export function resolveDefaultPorts(
  sourceType: string,
  _targetType: string,
  requestedSourcePort?: string,
  requestedTargetPort?: string,
): { sourcePort: string; targetPort: string } {
  let targetPort = 'in';
  if (requestedTargetPort && requestedTargetPort !== 'default') {
    targetPort = requestedTargetPort;
  }

  const isOutPortSource = [
    'entry',
    'variable_extractor',
    'proxy',
    'sse_disconnect',
    'graphql_disconnect',
  ].includes(sourceType);

  let sourcePort = requestedSourcePort;
  if (!sourcePort || sourcePort === 'default') {
    sourcePort = isOutPortSource ? 'out' : 'true';
  } else if (sourcePort === 'out' && !isOutPortSource) {
    // 保护纠偏：协议/条件节点没有 'out' 端口，模型传 'out' 时自动映射到 'true'
    sourcePort = 'true';
  }

  return { sourcePort, targetPort };
}

/**
 * 构造并规范化 NodeData，兼容并纠正大模型生成的参数格式
 */
export function buildDefaultNodeData(type: string, config?: Record<string, unknown>): NodeData {
  const defaults = createDefaultNodeData(type as AnyNodeType);
  const rawConfig: Record<string, unknown> = {
    ...defaults,
    ...(config || {}),
    type,
  };

  switch (type) {
    case 'http': {
      let method = String(rawConfig.method || 'get').toLowerCase();
      const validMethods = ['get', 'post', 'put', 'delete', 'patch', 'head', 'options'];
      if (!validMethods.includes(method)) {
        method = 'get';
      }
      const body = typeof rawConfig.body === 'string'
        ? rawConfig.body
        : (rawConfig.body ? JSON.stringify(rawConfig.body) : '');
      let bodyType = rawConfig.body_type as string | undefined;
      if (!bodyType || bodyType === 'none') {
        bodyType = body ? 'body' : 'none';
      }
      return {
        ...rawConfig,
        type: 'http',
        method,
        url: typeof rawConfig.url === 'string' ? rawConfig.url : 'https://httpbin.org/get',
        headers: (rawConfig.headers && typeof rawConfig.headers === 'object' && !Array.isArray(rawConfig.headers))
          ? rawConfig.headers
          : {},
        query_params: (rawConfig.query_params && typeof rawConfig.query_params === 'object' && !Array.isArray(rawConfig.query_params))
          ? rawConfig.query_params
          : {},
        body_type: bodyType,
        body,
        timeout_ms: typeof rawConfig.timeout_ms === 'number' ? rawConfig.timeout_ms : 30000,
      } as unknown as NodeData;
    }

    case 'conditional': {
      if (rawConfig.advanced_expression && typeof rawConfig.advanced_expression === 'string') {
        return {
          ...rawConfig,
          type: 'conditional',
          condition_type: 'advanced',
          advanced_expression: rawConfig.advanced_expression,
          field_path: '',
          operator: 'equal',
          expected_value: '',
        } as unknown as NodeData;
      }

      let fieldPath = typeof rawConfig.field_path === 'string' ? rawConfig.field_path : '';
      let operator = typeof rawConfig.operator === 'string' ? rawConfig.operator : 'equal';
      let expectedValue = rawConfig.expected_value !== undefined ? rawConfig.expected_value : '';

      // 容错：处理模型可能传的 conditions 数组
      if (!fieldPath && Array.isArray(rawConfig.conditions) && rawConfig.conditions.length > 0) {
        const first = rawConfig.conditions[0] as Record<string, unknown>;
        fieldPath = String(first.field || first.field_path || first.path || '');
        operator = String(first.operator || 'equal');
        expectedValue = first.value !== undefined
          ? first.value
          : (first.expected_value !== undefined ? first.expected_value : '');
      }

      return {
        ...rawConfig,
        type: 'conditional',
        condition_type: rawConfig.condition_type || 'simple',
        field_path: fieldPath,
        operator,
        expected_value: expectedValue,
      } as unknown as NodeData;
    }

    case 'assertion': {
      let onFailure = rawConfig.on_failure;
      if (onFailure === 'abort' || !onFailure) {
        onFailure = 'route_false';
      }

      let rawAssertions = rawConfig.assertions;
      if (!Array.isArray(rawAssertions) || rawAssertions.length === 0) {
        if (Array.isArray(rawConfig.rules) && rawConfig.rules.length > 0) rawAssertions = rawConfig.rules;
        else if (Array.isArray(rawConfig.items) && rawConfig.items.length > 0) rawAssertions = rawConfig.items;
        else if (!Array.isArray(rawAssertions)) rawAssertions = [];
      }

      const assertions = (rawAssertions as Array<Record<string, unknown>>).map((item, idx) => ({
        id: item.id ? String(item.id) : `assert_${idx + 1}_${Date.now()}`,
        description: item.description ? String(item.description) : undefined,
        target: String(item.target || item.field || item.path || 'status_code'),
        operator: String(item.operator || 'equal'),
        expected: item.expected !== undefined
          ? item.expected
          : (item.value !== undefined ? item.value : 200),
        enabled: item.enabled !== false,
      }));

      return {
        ...rawConfig,
        type: 'assertion',
        mode: rawConfig.mode === 'any' ? 'any' : 'all',
        on_failure: onFailure,
        continue_on_failure: Boolean(rawConfig.continue_on_failure),
        assertions,
      } as unknown as NodeData;
    }

    case 'variable_extractor': {
      let rawMappings = rawConfig.output_mappings;
      if (!Array.isArray(rawMappings) || rawMappings.length === 0) {
        if (Array.isArray(rawConfig.extractions) && rawConfig.extractions.length > 0) rawMappings = rawConfig.extractions;
        else if (Array.isArray(rawConfig.variables) && rawConfig.variables.length > 0) rawMappings = rawConfig.variables;
        else if (Array.isArray(rawConfig.mappings) && rawConfig.mappings.length > 0) rawMappings = rawConfig.mappings;
        else if (!Array.isArray(rawMappings)) rawMappings = [];
      }

      const outputMappings = (rawMappings as Array<Record<string, unknown>>).map((item) => ({
        source: String(item.source || item.field || item.path || ''),
        target: String(item.target || item.name || item.var_name || ''),
        description: item.description ? String(item.description) : undefined,
      }));

      return {
        ...rawConfig,
        type: 'variable_extractor',
        output_mappings: outputMappings,
        source_node_id: rawConfig.source_node_id ? String(rawConfig.source_node_id) : undefined,
      } as unknown as NodeData;
    }

    case 'script': {
      return {
        ...rawConfig,
        type: 'script',
        script: typeof rawConfig.script === 'string' && rawConfig.script.trim()
          ? rawConfig.script
          : '// JavaScript Execution Sandbox\nconsole.log("Running script step");\n',
      } as unknown as NodeData;
    }

    case 'entry':
      return { type: 'entry' } as unknown as NodeData;

    case 'end':
      return { type: 'end' } as unknown as NodeData;

    default:
      return rawConfig as unknown as NodeData;
  }
}

/**
 * Canvas 工具定义
 */
export const canvasToolDefinitions: AIToolDefinition[] = [
  {
    type: 'function',
    function: {
      name: 'batch_create_pipeline',
      description:
        'Create an entire multi-node pipeline or workflow branch in a single atomic step. Creates all nodes with their configs, connects them sequentially (source -> target), and applies auto-layout. PREFER THIS tool when creating 2 or more connected workflow steps to avoid multi-turn latency and connection errors.',
      parameters: {
        type: 'object',
        properties: {
          nodes: {
            type: 'array',
            description: 'Ordered list of nodes to create in sequence.',
            items: {
              type: 'object',
              properties: {
                type: {
                  type: 'string',
                  description:
                    "Node type: 'http', 'script', 'assertion', 'conditional', 'variable_extractor', 'end', 'ws_connect', 'tcp_connect', etc.",
                },
                name: {
                  type: 'string',
                  description: 'Human-readable name for this node.',
                },
                config: {
                  type: 'object',
                  description:
                    "Configuration specific to the node type (e.g. for http: {method: 'get', url: '...'}, for assertion: {assertions: [...]}, for extractor: {output_mappings: [...]}).",
                },
                on_error: {
                  type: 'string',
                  enum: ['route_to_false', 'ignore', 'abort_flow'],
                  description: 'Error routing policy if execution fails. Defaults to route_to_false.',
                },
              },
              required: ['type'],
            },
          },
          connect_from_node_uuid: {
            type: 'string',
            description:
              'Optional UUID of an existing node (e.g. entry node) to connect the first new node from. If omitted and canvas only has an unconnected entry node, automatically connects from that entry node.',
          },
          connect_from_port: {
            type: 'string',
            description:
              "Outgoing port on connect_from_node_uuid ('out', 'true', or 'false'). Automatically determined if omitted.",
          },
        },
        required: ['nodes'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'add_node',
      description: 'Add a single new node to the current workflow canvas.',
      parameters: {
        type: 'object',
        properties: {
          type: {
            type: 'string',
            description: 'The node type (e.g. http, script, conditional, assertion, entry, end, ws_connect, etc.).',
          },
          name: {
            type: 'string',
            description: 'Human-readable name for the node.',
          },
          position: {
            type: 'object',
            description: 'Position on the canvas {x, y}. If omitted, automatically placed next to existing nodes.',
            properties: {
              x: { type: 'number' },
              y: { type: 'number' },
            },
          },
          config: {
            type: 'object',
            description:
              "Node configuration parameters specific to the node type (e.g. for http: {method: 'get'|'post', url, headers, body}, for assertion: {assertions: [...]}, for conditional: {condition_type, field_path, operator, expected_value}).",
          },
          on_error: {
            type: 'string',
            enum: ['route_to_false', 'ignore', 'abort_flow'],
            description: 'Error routing policy if execution fails. Defaults to route_to_false.',
          },
        },
        required: ['type'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'update_node_config',
      description: 'Update the properties or configuration of an existing node in the workflow.',
      parameters: {
        type: 'object',
        properties: {
          node_uuid: {
            type: 'string',
            description: 'The UUID of the node to update.',
          },
          name: {
            type: 'string',
            description: 'Updated node display name.',
          },
          config: {
            type: 'object',
            description: 'Updated configuration fields to merge into node.data.',
          },
          on_error: {
            type: 'string',
            enum: ['route_to_false', 'ignore', 'abort_flow'],
            description: 'Updated error routing policy.',
          },
        },
        required: ['node_uuid'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'connect_nodes',
      description:
        "Connect two nodes in the workflow canvas. Automatically determines correct ports ('out' for entry/extractor/proxy, 'true' for http/script/assertion/conditional/plugins, 'in' for target).",
      parameters: {
        type: 'object',
        properties: {
          source_node_uuid: {
            type: 'string',
            description: 'UUID of the source node.',
          },
          target_node_uuid: {
            type: 'string',
            description: 'UUID of the target node.',
          },
          source_port_id: {
            type: 'string',
            description:
              "Source port ID ('true', 'false', or 'out'). Use 'true' for normal success flow, 'false' for failure/error branch, 'out' for entry/extractor/proxy. Defaults intelligently if omitted.",
          },
          target_port_id: {
            type: 'string',
            description: "Target port ID. Almost always 'in' in MPE. Defaults to 'in' if omitted.",
          },
        },
        required: ['source_node_uuid', 'target_node_uuid'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'disconnect_nodes',
      description: 'Remove a connection between nodes.',
      parameters: {
        type: 'object',
        properties: {
          connection_id: {
            type: 'string',
            description: 'ID of the connection to remove (if known).',
          },
          source_node_uuid: {
            type: 'string',
            description: 'Source node UUID (used if connection_id not provided).',
          },
          target_node_uuid: {
            type: 'string',
            description: 'Target node UUID (used if connection_id not provided).',
          },
        },
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'auto_layout',
      description: 'Automatically organize and arrange all nodes and connections on the canvas into a clean DAG layout.',
      parameters: {
        type: 'object',
        properties: {
          direction: {
            type: 'string',
            enum: ['LR'],
            description: 'Layout direction (horizontal LR). Defaults to LR.',
          },
        },
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'delete_nodes',
      description: 'Delete one or more nodes and their attached connections from the workflow. [HIGH RISK]',
      parameters: {
        type: 'object',
        properties: {
          node_uuids: {
            type: 'array',
            items: { type: 'string' },
            description: 'List of node UUIDs to delete.',
          },
        },
        required: ['node_uuids'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'clear_canvas',
      description: 'Delete all nodes and connections from the current workflow canvas. [HIGH RISK]',
      parameters: {
        type: 'object',
        properties: {},
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'get_node_detail',
      description:
        'Get complete detailed configuration, port definitions, incoming/outgoing connections, and latest execution error/report for a specific node in the current workflow. Use this whenever you need to inspect, understand, or debug a node.',
      parameters: {
        type: 'object',
        properties: {
          node_uuid: {
            type: 'string',
            description:
              'UUID of the node to inspect. If omitted, checks node_name or falls back to the currently selected node on canvas.',
          },
          node_name: {
            type: 'string',
            description: 'Human-readable name of the node to inspect.',
          },
        },
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'get_node_type_schema',
      description:
        'Query the complete configuration schema, allowable parameter names, data types, descriptions, default values, and usage examples for any node type (including built-in protocols and plugins like http, assertion, conditional, script, variable_extractor, ws_*, tcp_*, sse_*, graphql_*, redis:*, db:*, kafka:*, etc.).',
      parameters: {
        type: 'object',
        properties: {
          node_type: {
            type: 'string',
            description:
              "The node type identifier, e.g. 'http', 'assertion', 'conditional', 'script', 'variable_extractor', 'ws_connect', 'tcp_connect', 'redis:command', 'db:query'.",
          },
        },
        required: ['node_type'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'get_flow_nodes_config',
      description:
        'Get complete configuration dictionaries for all nodes (or a specified subset) in the current flow. Useful for inspecting variable dependencies, mapping chains, and cross-node consistency.',
      parameters: {
        type: 'object',
        properties: {
          node_uuids: {
            type: 'array',
            items: { type: 'string' },
            description: 'Optional list of node UUIDs to filter. If omitted, returns all nodes.',
          },
        },
      },
    },
  },
];

/**
 * 内置节点类型标准配置规范定义，用于向 LLM 呈现权威完整的字段与参数说明
 */
export const BUILTIN_SCHEMAS: Record<string, unknown> = {
  http: {
    display_name: 'HTTP Request',
    category: 'http',
    description: 'Send HTTP/REST requests (GET, POST, PUT, DELETE, PATCH, etc.)',
    ports: {
      input: ['in'],
      output: ['true (success)', 'false (failure/error)'],
    },
    fields: {
      url: { type: 'string', required: true, description: 'Target URL, supports {{variables.xxx}} or {{env.XXX}}' },
      method: { type: 'string', enum: ['get', 'post', 'put', 'delete', 'patch', 'head', 'options'], default: 'get', required: true },
      headers: { type: 'object', description: 'Key-value map of HTTP headers, e.g. {"Authorization": "Bearer {{variables.token}}"}' },
      query_params: { type: 'object', description: 'Key-value map of URL query parameters' },
      body_type: { type: 'string', enum: ['none', 'body', 'form-data', 'x-www-form-urlencoded', 'binary'], default: 'none' },
      body: { type: 'string', description: 'Raw request body (JSON string, text, etc.)' },
      timeout_ms: { type: 'number', default: 30000, description: 'Request timeout in milliseconds' },
      auth: {
        type: 'object',
        properties: {
          auth_type: { type: 'string', enum: ['none', 'bearer', 'basic', 'api_key_header', 'api_key_query'] },
          token: { type: 'string' },
          username: { type: 'string' },
          password: { type: 'string' },
        },
      },
      follow_redirects: { type: 'boolean', default: true },
      tls_skip_verify: { type: 'boolean', default: false },
      retry_count: { type: 'number', default: 0 },
      retry_delay_ms: { type: 'number', default: 1000 },
    },
    example: {
      method: 'post',
      url: 'https://api.example.com/v1/auth/login',
      headers: { 'Content-Type': 'application/json' },
      body_type: 'body',
      body: '{"username": "{{env.USERNAME}}", "password": "{{env.PASSWORD}}"}',
      timeout_ms: 30000,
    },
  },
  assertion: {
    display_name: 'Assertion',
    category: 'tools',
    description: 'Verify and assert conditions on upstream response, variables, or status codes',
    ports: {
      input: ['in'],
      output: ['true (assertions passed)', 'false (assertions failed)'],
    },
    fields: {
      mode: { type: 'string', enum: ['all', 'any'], default: 'all', description: 'Whether all or any assertions must pass' },
      on_failure: { type: 'string', enum: ['route_false', 'throw'], default: 'route_false' },
      continue_on_failure: { type: 'boolean', default: false },
      assertions: {
        type: 'array',
        items: {
          id: { type: 'string', required: true },
          target: { type: 'string', required: true, description: 'e.g. status_code, response_time_ms, body.code, headers.content-type' },
          operator: { type: 'string', enum: ['equal', 'not_equal', 'greater_than', 'less_than', 'greater_equal', 'less_equal', 'contains', 'not_contains', 'is_null', 'is_not_null', 'regex_match'], required: true },
          expected: { type: 'any', required: true },
          enabled: { type: 'boolean', default: true },
        },
      },
    },
    example: {
      mode: 'all',
      on_failure: 'route_false',
      assertions: [
        { id: 'assert_1', target: 'status_code', operator: 'equal', expected: 200, enabled: true },
        { id: 'assert_2', target: 'body.success', operator: 'equal', expected: true, enabled: true },
      ],
    },
  },
  conditional: {
    display_name: 'Conditional Branch',
    category: 'tools',
    description: 'Branch execution path based on simple condition or JavaScript-like expression',
    ports: {
      input: ['in'],
      output: ['true (condition matched)', 'false (condition not matched)'],
    },
    fields: {
      condition_type: { type: 'string', enum: ['simple', 'advanced'], default: 'simple' },
      field_path: { type: 'string', description: 'Used in simple mode, e.g. status, body.code' },
      operator: { type: 'string', enum: ['equal', 'not_equal', 'greater_than', 'less_than', 'contains'], default: 'equal' },
      expected_value: { type: 'any', description: 'Used in simple mode' },
      advanced_expression: { type: 'string', description: 'Used in advanced mode, e.g. {{context.steps.node1.response.status}} == 200' },
    },
    example: {
      condition_type: 'simple',
      field_path: 'status',
      operator: 'equal',
      expected_value: 200,
    },
  },
  variable_extractor: {
    display_name: 'Variable Extractor',
    category: 'tools',
    description: 'Extract values from preceding node response using JSONPath and save into workflow variable pool',
    ports: {
      input: ['in'],
      output: ['out'],
    },
    fields: {
      output_mappings: {
        type: 'array',
        items: {
          source: { type: 'string', required: true, description: 'JSONPath from preceding response, e.g. body.data.token' },
          target: { type: 'string', required: true, description: 'Target variable name, e.g. auth_token' },
          description: { type: 'string' },
          enabled: { type: 'boolean', default: true },
        },
      },
    },
    example: {
      output_mappings: [
        { source: 'body.data.token', target: 'auth_token', description: 'User session token' },
      ],
    },
  },
  script: {
    display_name: 'Script',
    category: 'script',
    description: 'Execute JavaScript code in a sandboxed runtime to transform data, set variables, or assert',
    ports: {
      input: ['in'],
      output: ['true (script succeeded)', 'false (script threw error or abort)'],
    },
    fields: {
      script: { type: 'string', required: true, description: 'JavaScript source code' },
    },
    example: {
      script: 'const res = context.steps.prev_step.response.body;\ncontext.variables.set("totalCount", res.items.length);\n',
    },
  },
  ws_connect: {
    display_name: 'WebSocket Connect',
    category: 'websocket',
    description: 'Establish WebSocket connection',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      url: { type: 'string', required: true, description: 'WebSocket URL, e.g. wss://echo.websocket.org' },
      headers: { type: 'object' },
    },
  },
  ws_send_collect: {
    display_name: 'WebSocket Send & Collect',
    category: 'websocket',
    description: 'Send message over WebSocket and collect responses',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      message: { type: 'string', required: true },
      collect_response: { type: 'boolean', default: true },
      timeout_ms: { type: 'number', default: 10000 },
    },
  },
  ws_close: {
    display_name: 'WebSocket Close',
    category: 'websocket',
    description: 'Close WebSocket connection',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {},
  },
  tcp_connect: {
    display_name: 'TCP Connect',
    category: 'tcp',
    description: 'Establish TCP socket connection',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      host: { type: 'string', required: true, default: '127.0.0.1' },
      port: { type: 'number', required: true },
      timeout_ms: { type: 'number', default: 5000 },
    },
  },
  tcp_send: {
    display_name: 'TCP Send',
    category: 'tcp',
    description: 'Send raw data over TCP socket',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      data: { type: 'string', required: true },
    },
  },
  tcp_receive: {
    display_name: 'TCP Receive',
    category: 'tcp',
    description: 'Receive data from TCP socket',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      max_bytes: { type: 'number', default: 4096 },
      timeout_ms: { type: 'number', default: 10000 },
    },
  },
  tcp_close: {
    display_name: 'TCP Close',
    category: 'tcp',
    description: 'Close TCP socket connection',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {},
  },
  udp_bind: {
    display_name: 'UDP Bind',
    category: 'udp',
    description: 'Bind local UDP socket',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      local_port: { type: 'number', required: true },
    },
  },
  udp_send_to: {
    display_name: 'UDP Send To',
    category: 'udp',
    description: 'Send UDP datagram to target address',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      target_addr: { type: 'string', required: true },
      target_port: { type: 'number', required: true },
      data: { type: 'string', required: true },
    },
  },
  udp_recv_from: {
    display_name: 'UDP Receive From',
    category: 'udp',
    description: 'Receive UDP datagram',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      timeout_ms: { type: 'number', default: 5000 },
    },
  },
  udp_close: {
    display_name: 'UDP Close',
    category: 'udp',
    description: 'Close UDP socket',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {},
  },
  sse_connect: {
    display_name: 'SSE Connect',
    category: 'sse',
    description: 'Connect to Server-Sent Events stream',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      url: { type: 'string', required: true },
      headers: { type: 'object' },
    },
  },
  sse_listen: {
    display_name: 'SSE Listen',
    category: 'sse',
    description: 'Listen for SSE event',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      event: { type: 'string', default: 'message' },
      timeout_ms: { type: 'number', default: 15000 },
    },
  },
  sse_disconnect: {
    display_name: 'SSE Disconnect',
    category: 'sse',
    description: 'Disconnect SSE stream',
    ports: { input: ['in'], output: ['out'] },
    fields: {},
  },
  graphql_connect: {
    display_name: 'GraphQL Connect',
    category: 'graphql',
    description: 'Initialize GraphQL endpoint connection',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      endpoint: { type: 'string', required: true },
      headers: { type: 'object' },
    },
  },
  graphql_query: {
    display_name: 'GraphQL Query / Mutation',
    category: 'graphql',
    description: 'Execute GraphQL query or mutation',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      query: { type: 'string', required: true },
      variables: { type: 'object' },
    },
  },
  proxy: {
    display_name: 'Proxy',
    category: 'tools',
    description: 'Configure HTTP/SOCKS5 proxy settings for downstream network calls',
    ports: { input: ['in'], output: ['out'] },
    fields: {
      mode: { type: 'string', enum: ['http', 'socks5', 'clear'], default: 'http' },
      http_proxy: { type: 'string' },
      socks_proxy: { type: 'string' },
    },
  },
};

/**
 * Canvas 工具执行实现
 */
export const canvasToolHandlers = {
  async batch_create_pipeline(args: {
    nodes: Array<{
      type: string;
      name?: string;
      config?: Record<string, unknown>;
      on_error?: ExecutionErrorStrategy;
    }>;
    connect_from_node_uuid?: string;
    connect_from_port?: string;
  }) {
    const flowState = useFlowStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow) {
      throw new Error('No active flow to add pipeline to.');
    }
    if (!args.nodes || args.nodes.length === 0) {
      throw new Error('Pipeline nodes array cannot be empty.');
    }

    // 1. 判断起始连接节点
    let prevNodeUuid: string | undefined = args.connect_from_node_uuid;
    let prevNodeType: string | undefined;

    if (!prevNodeUuid && currentFlow.nodes.length === 1 && currentFlow.nodes[0].type === 'entry') {
      const entryNode = currentFlow.nodes[0];
      const hasOutConn = currentFlow.connections.some((c) => c.source_node_uuid === entryNode.uuid);
      if (!hasOutConn) {
        prevNodeUuid = entryNode.uuid;
        prevNodeType = 'entry';
      }
    } else if (prevNodeUuid) {
      const found = currentFlow.nodes.find((n) => n.uuid === prevNodeUuid);
      if (found) {
        prevNodeType = found.type;
      }
    }

    const createdNodes: Array<{ uuid: string; name: string; type: string }> = [];
    const createdConnections: Array<{ id: string; source: string; target: string }> = [];

    let startX = 100;
    let startY = 150;
    if (currentFlow.nodes.length > 0) {
      const lastNode = currentFlow.nodes[currentFlow.nodes.length - 1];
      startX = lastNode.position.x + 220;
      startY = lastNode.position.y;
    }

    for (let i = 0; i < args.nodes.length; i++) {
      const nodeSpec = args.nodes[i];
      const uuid = crypto.randomUUID();
      const name = nodeSpec.name || `${nodeSpec.type.toUpperCase()}_${currentFlow.nodes.length + i + 1}`;
      const data = buildDefaultNodeData(nodeSpec.type, nodeSpec.config);
      const position = { x: startX + i * 220, y: startY };

      const newNode: Node = {
        uuid,
        type: nodeSpec.type as NodeType,
        name,
        position,
        data,
        on_error: nodeSpec.on_error,
      };

      flowState.addNode(newNode);
      createdNodes.push({ uuid, name, type: nodeSpec.type });

      // 如果有前驱节点，自动建立连线
      if (prevNodeUuid && prevNodeType) {
        const requestedSourcePort = i === 0 && args.connect_from_port ? args.connect_from_port : undefined;
        const { sourcePort, targetPort } = resolveDefaultPorts(
          prevNodeType,
          nodeSpec.type,
          requestedSourcePort,
          'in',
        );

        const connId = `conn_${prevNodeUuid}_${sourcePort}_to_${uuid}_${targetPort}`;
        const connection: Connection = {
          id: connId,
          source_node_uuid: prevNodeUuid,
          source_port_id: sourcePort,
          target_node_uuid: uuid,
          target_port_id: targetPort,
        };

        flowState.addConnection(connection);
        createdConnections.push({
          id: connId,
          source: `${prevNodeUuid}:${sourcePort}`,
          target: `${uuid}:${targetPort}`,
        });
      }

      // 更新前驱为当前新创建的节点
      prevNodeUuid = uuid;
      prevNodeType = nodeSpec.type;
    }

    // 自动重排布局
    const refreshedFlow = useFlowStore.getState().flow;
    if (refreshedFlow && refreshedFlow.nodes.length > 0) {
      const positions = getLayoutedPositions(refreshedFlow.nodes, refreshedFlow.connections, 'LR');
      flowState.batchUpdateNodePositions(positions);
    }

    if (createdNodes.length > 0) {
      flowState.selectNode(createdNodes[0].uuid);
      flowState.centerOnNode(createdNodes[0].uuid);
    }

    return {
      success: true,
      created_count: createdNodes.length,
      created_nodes: createdNodes,
      created_connections: createdConnections,
    };
  },

  async add_node(args: {
    type: string;
    name?: string;
    position?: { x: number; y: number };
    config?: Record<string, unknown>;
    on_error?: ExecutionErrorStrategy;
  }) {
    const flowState = useFlowStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow) {
      throw new Error('No active flow to add node to.');
    }

    const uuid = crypto.randomUUID();
    const name = args.name || `${args.type.toUpperCase()}_${currentFlow.nodes.length + 1}`;

    // 计算合理初始位置
    let position = args.position;
    if (!position) {
      if (currentFlow.nodes.length === 0) {
        position = { x: 100, y: 150 };
      } else {
        const lastNode = currentFlow.nodes[currentFlow.nodes.length - 1];
        position = { x: lastNode.position.x + 220, y: lastNode.position.y };
      }
    }

    const data = buildDefaultNodeData(args.type, args.config);

    const newNode: Node = {
      uuid,
      type: args.type as NodeType,
      name,
      position,
      data,
      on_error: args.on_error,
    };

    flowState.addNode(newNode);
    flowState.selectNode(uuid);
    flowState.centerOnNode(uuid);

    return {
      success: true,
      node_uuid: uuid,
      name,
      type: args.type,
      position,
    };
  },

  async update_node_config(args: {
    node_uuid: string;
    name?: string;
    config?: Record<string, unknown>;
    on_error?: ExecutionErrorStrategy;
  }) {
    const flowState = useFlowStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow) {
      throw new Error('No active flow.');
    }

    const node = currentFlow.nodes.find((n) => n.uuid === args.node_uuid);
    if (!node) {
      throw new Error(`Node with UUID "${args.node_uuid}" not found.`);
    }

    const updates: Partial<Node> = {};
    if (args.name !== undefined) {
      updates.name = args.name;
    }
    if (args.on_error !== undefined) {
      updates.on_error = args.on_error;
    }
    if (args.config !== undefined) {
      updates.data = buildDefaultNodeData(node.type, {
        ...node.data,
        ...args.config,
      });
    }

    flowState.updateNode(args.node_uuid, updates);

    return {
      success: true,
      node_uuid: args.node_uuid,
      updated_fields: Object.keys(updates),
    };
  },

  async connect_nodes(args: {
    source_node_uuid: string;
    target_node_uuid: string;
    source_port_id?: string;
    target_port_id?: string;
  }) {
    const flowState = useFlowStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow) {
      throw new Error('No active flow.');
    }

    const sourceNode = currentFlow.nodes.find((n) => n.uuid === args.source_node_uuid);
    if (!sourceNode) {
      throw new Error(`Source node "${args.source_node_uuid}" not found.`);
    }
    const targetNode = currentFlow.nodes.find((n) => n.uuid === args.target_node_uuid);
    if (!targetNode) {
      throw new Error(`Target node "${args.target_node_uuid}" not found.`);
    }

    const { sourcePort, targetPort } = resolveDefaultPorts(
      sourceNode.type,
      targetNode.type,
      args.source_port_id,
      args.target_port_id,
    );
    const connId = `conn_${args.source_node_uuid}_${sourcePort}_to_${args.target_node_uuid}_${targetPort}`;

    const connection: Connection = {
      id: connId,
      source_node_uuid: args.source_node_uuid,
      source_port_id: sourcePort,
      target_node_uuid: args.target_node_uuid,
      target_port_id: targetPort,
    };

    flowState.addConnection(connection);

    return {
      success: true,
      connection_id: connId,
      source: `${args.source_node_uuid}:${sourcePort}`,
      target: `${args.target_node_uuid}:${targetPort}`,
    };
  },

  async disconnect_nodes(args: {
    connection_id?: string;
    source_node_uuid?: string;
    target_node_uuid?: string;
  }) {
    const flowState = useFlowStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow) {
      throw new Error('No active flow.');
    }

    if (args.connection_id) {
      flowState.removeConnection(args.connection_id);
      return { success: true, removed_connection_id: args.connection_id };
    }

    if (args.source_node_uuid && args.target_node_uuid) {
      const match = currentFlow.connections.find(
        (c) =>
          c.source_node_uuid === args.source_node_uuid &&
          c.target_node_uuid === args.target_node_uuid,
      );
      if (match) {
        flowState.removeConnection(match.id);
        return { success: true, removed_connection_id: match.id };
      }
      throw new Error('No matching connection found between specified nodes.');
    }

    throw new Error('Either connection_id or both source and target UUIDs must be provided.');
  },

  async auto_layout(args?: { direction?: 'LR' }) {
    const flowState = useFlowStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow || currentFlow.nodes.length === 0) {
      return { success: true, node_count: 0 };
    }

    const direction = args?.direction || 'LR';
    const positions = getLayoutedPositions(currentFlow.nodes, currentFlow.connections, direction);
    flowState.batchUpdateNodePositions(positions);

    return {
      success: true,
      node_count: currentFlow.nodes.length,
      direction,
    };
  },

  async delete_nodes(args: { node_uuids: string[] }) {
    const flowState = useFlowStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow) {
      throw new Error('No active flow.');
    }

    flowState.removeNodes(args.node_uuids);
    return {
      success: true,
      deleted_count: args.node_uuids.length,
    };
  },

  async clear_canvas() {
    const flowState = useFlowStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow) {
      return { success: true, cleared_nodes: 0 };
    }

    const nodeCount = currentFlow.nodes.length;
    const allUuids = currentFlow.nodes.map((n) => n.uuid);
    flowState.removeNodes(allUuids);

    return {
      success: true,
      cleared_nodes: nodeCount,
    };
  },

  async get_node_detail(args?: { node_uuid?: string; node_name?: string }) {
    const flowState = useFlowStore.getState();
    const execStore = useExecutionStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow) {
      throw new Error('No active flow.');
    }

    let targetNode = null;
    if (args?.node_uuid) {
      targetNode = currentFlow.nodes.find((n) => n.uuid === args.node_uuid);
    } else if (args?.node_name) {
      targetNode = currentFlow.nodes.find(
        (n) => n.name.toLowerCase() === args.node_name!.toLowerCase(),
      );
    } else if (flowState.selectedNodeId) {
      targetNode = currentFlow.nodes.find((n) => n.uuid === flowState.selectedNodeId);
    }

    if (!targetNode) {
      throw new Error(
        `Node not found. Available nodes: ${currentFlow.nodes.map((n) => `"${n.name}" (${n.uuid})`).join(', ')}`,
      );
    }

    // 上游连接 (连入本节点)
    const incomingConnections = currentFlow.connections
      .filter((c) => c.target_node_uuid === targetNode.uuid)
      .map((c) => {
        const src = currentFlow.nodes.find((n) => n.uuid === c.source_node_uuid);
        return {
          source_node_uuid: c.source_node_uuid,
          source_node_name: src?.name,
          source_port: c.source_port_id,
          target_port: c.target_port_id,
        };
      });

    // 下游连接 (由本节点连出)
    const outgoingConnections = currentFlow.connections
      .filter((c) => c.source_node_uuid === targetNode.uuid)
      .map((c) => {
        const tgt = currentFlow.nodes.find((n) => n.uuid === c.target_node_uuid);
        return {
          target_node_uuid: c.target_node_uuid,
          target_node_name: tgt?.name,
          source_port: c.source_port_id,
          target_port: c.target_port_id,
        };
      });

    // 最近一次执行报告
    const nodeReport =
      execStore.report?.node_reports?.find((nr) => nr.node_uuid === targetNode.uuid) ||
      execStore.nodeReportsMap[targetNode.uuid];

    return {
      success: true,
      node: {
        uuid: targetNode.uuid,
        name: targetNode.name,
        type: targetNode.type,
        on_error: targetNode.on_error ?? 'route_to_false',
        position: targetNode.position,
        config: targetNode.data,
      },
      connections: {
        incoming: incomingConnections,
        outgoing: outgoingConnections,
      },
      last_execution: nodeReport
        ? {
            status: nodeReport.status,
            duration_ms: nodeReport.duration_ms,
            error: nodeReport.error,
            resolved_config: nodeReport.resolved_config,
            output_data: nodeReport.output_data,
            input_data: nodeReport.input_data,
          }
        : null,
    };
  },

  async get_node_type_schema(args: { node_type: string }) {
    const nodeType = args.node_type;
    const builtin = BUILTIN_SCHEMAS[nodeType];
    const defaultConfig =
      getCachedDefaultConfig(nodeType) || createDefaultNodeData(nodeType as AnyNodeType);

    return {
      success: true,
      node_type: nodeType,
      builtin_schema: builtin || null,
      default_config: defaultConfig,
      hint: builtin
        ? undefined
        : 'This may be a plugin or custom node type. Inspect default_config for available fields.',
    };
  },

  async get_flow_nodes_config(args?: { node_uuids?: string[] }) {
    const flowState = useFlowStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow) {
      throw new Error('No active flow.');
    }

    let nodes = currentFlow.nodes;
    if (args?.node_uuids && args.node_uuids.length > 0) {
      nodes = nodes.filter((n) => args.node_uuids!.includes(n.uuid));
    }

    return {
      success: true,
      total_flow_nodes: currentFlow.nodes.length,
      matched_nodes_count: nodes.length,
      nodes: nodes.map((n) => ({
        uuid: n.uuid,
        name: n.name,
        type: n.type,
        on_error: n.on_error ?? 'route_to_false',
        data: n.data,
      })),
    };
  },
};
