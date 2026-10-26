/**
 * AI Canvas & Workflow Manipulation Tools
 */

import { useFlowStore } from '@/stores/flow';
import { useExecutionStore } from '@/stores/execution';
import { executionToolHandlers } from './executionTools';
import { getCachedDefaultConfig } from '@/lib/NodeRegistry';
import { getLayoutedPositions, getAdaptiveLayoutedPositions } from '@/components/canvas/layout';
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
 *    - 'entry'、'variable:extractor'、'proxy'、'sse:disconnect'、'graphql:disconnect' 的输出端口为 'out'。
 *    - 'http:request'、'script'、'assertion'、'conditional'、'ws_*'、'tcp_*'、'udp_*'、'sse_*'、'graphql_*' 以及插件节点的输出端口为 'true' (主干成功流) 和 'false' (异常/失败分支)。
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
    'flow:sub',
    'variable:extractor',
    'proxy',
    'sse:disconnect',
    'graphql:disconnect',
  ].includes(sourceType);

  let sourcePort = requestedSourcePort;
  if (!sourcePort || sourcePort === 'default') {
    sourcePort = isOutPortSource ? 'out' : 'true';
  } else if (sourcePort === 'out' && !isOutPortSource) {
    // 保护纠偏：协议/条件节点没有 'out' 端口，模型传 'out' 时自动映射到 'true'
    sourcePort = 'true';
  } else if (sourcePort === 'true' && sourceType === 'flow:sub') {
    // 保护纠偏：flow:sub 成功端口为 'out'，模型传 'true' 时自动映射到 'out'
    sourcePort = 'out';
  }
  return { sourcePort, targetPort };
}

/**
 * 规范化 JSONPath 路径
 * 如果未包含 `$` 前缀（如 `body.data.token` 或 `status`），自动补齐 `$.` 前缀
 */
export function normalizeJsonPath(path: string): string {
  const trimmed = path.trim();
  if (!trimmed) return trimmed;
  if (trimmed.startsWith('$')) return trimmed;
  if (trimmed.startsWith('[')) return `$${trimmed}`;
  if (trimmed.startsWith('.')) return `$${trimmed}`;
  return `$.${trimmed}`;
}

/**
 * 判断节点类型是否为数据生产者（即执行后会输出网络响应、数据查询或脚本返回的节点）
 * 用于 variable:extractor 智能数据源推导与回溯
 */
const NON_PRODUCER_NODE_TYPES: Record<string, true> = {
  entry: true,
  end: true,
  'variable:extractor': true,
  assertion: true,
  conditional: true,
  proxy: true,
};

export function isProducerNode(type: string): boolean {
  return !NON_PRODUCER_NODE_TYPES[type];
}

export function buildDefaultNodeData(type: string, config?: Record<string, unknown>): NodeData {
  const defaults = createDefaultNodeData(type as AnyNodeType);
  const rawConfig: Record<string, unknown> = {
    ...defaults,
    ...(config || {}),
    type,
  };
  switch (type) {
    case 'http:request': {
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
        type: 'http:request',
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
        ...(Array.isArray(rawConfig.output_mappings)
          ? {
              output_mappings: (rawConfig.output_mappings as Array<Record<string, unknown>>).map((item) => ({
                source: normalizeJsonPath(String(item.source || item.field || item.path || '')),
                target: String(item.target || item.name || item.var_name || ''),
                description: item.description ? String(item.description) : undefined,
              })),
            }
          : {}),
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

    case 'variable:extractor': {
      let rawMappings = rawConfig.output_mappings;
      if (!Array.isArray(rawMappings) || rawMappings.length === 0) {
        if (Array.isArray(rawConfig.extractions) && rawConfig.extractions.length > 0) rawMappings = rawConfig.extractions;
        else if (Array.isArray(rawConfig.variables) && rawConfig.variables.length > 0) rawMappings = rawConfig.variables;
        else if (Array.isArray(rawConfig.mappings) && rawConfig.mappings.length > 0) rawMappings = rawConfig.mappings;
        else if (!Array.isArray(rawMappings)) rawMappings = [];
      }

      const outputMappings = (rawMappings as Array<Record<string, unknown>>).map((item) => ({
        source: normalizeJsonPath(String(item.source || item.field || item.path || '')),
        target: String(item.target || item.name || item.var_name || ''),
        description: item.description ? String(item.description) : undefined,
      }));

      const sourceNodeId = rawConfig.source_node_id
        ? String(rawConfig.source_node_id).trim()
        : (rawConfig.source_node
            ? String(rawConfig.source_node).trim()
            : (rawConfig.data_source ? String(rawConfig.data_source).trim() : undefined));

      return {
        ...rawConfig,
        type: 'variable:extractor',
        output_mappings: outputMappings,
        source_node_id: sourceNodeId || undefined,
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
    case 'flow:sub': {
      const variableMappings = Array.isArray(rawConfig.variable_mappings)
        ? rawConfig.variable_mappings
        : [];
      const outputMappings = Array.isArray(rawConfig.output_mappings)
        ? rawConfig.output_mappings
        : [];
      return {
        ...rawConfig,
        type: 'flow:sub',
        target_flow_id: rawConfig.target_flow_id ? String(rawConfig.target_flow_id) : undefined,
        target_flow_path: rawConfig.target_flow_path ? String(rawConfig.target_flow_path) : undefined,
        inherit_all_variables: rawConfig.inherit_all_variables ?? true,
        variable_mappings: variableMappings,
        export_all_variables: rawConfig.export_all_variables ?? true,
        output_mappings: outputMappings,
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
        'Create an entire multi-node pipeline or workflow branch in a single atomic step. Creates all nodes with their configs, connects them sequentially or via explicit declarative edges, and applies auto-layout. PREFER THIS tool when creating 2 or more connected workflow steps to avoid multi-turn latency and connection errors.',
      parameters: {
        type: 'object',
        properties: {
          nodes: {
            type: 'array',
            description: 'Ordered list of nodes to create in sequence.',
            items: {
              type: 'object',
              properties: {
                id: {
                  type: 'string',
                  description:
                    'Optional string alias/identifier for this node (e.g. "login", "getUser", "assertSuccess"). Can be referenced in edges.',
                },
                type: {
                  type: 'string',
                  description:
                    "Node type: 'http:request', 'script', 'assertion', 'conditional', 'flow:sub', 'variable:extractor', 'end', 'ws:connect', 'tcp:connect', etc.",
                },
                name: {
                  type: 'string',
                  description: 'Human-readable name for this node.',
                },
                config: {
                  type: 'object',
                  description:
                    "Configuration specific to the node type. For 'variable:extractor', ALWAYS specify 'source_node_id' to point to the upstream data producer node (e.g. {source_node_id: 'login', output_mappings: [...]}) to prevent extraction failure from intermediate nodes.",
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
          edges: {
            type: 'array',
            description:
              'Optional explicit topological edges between nodes. If provided, overrides default sequential linking. Allows creating branches, joins, and error handling paths.',
            items: {
              type: 'object',
              properties: {
                source: {
                  type: 'string',
                  description: 'Source node alias id, node name, or existing node UUID.',
                },
                target: {
                  type: 'string',
                  description: 'Target node alias id, node name, or existing node UUID.',
                },
                source_port: {
                  type: 'string',
                  description:
                    "Outgoing port: 'true' (success/main branch), 'false' (error/condition false branch), or 'out' (entry/extractor/proxy). Defaults to auto-resolved port.",
                },
                target_port: {
                  type: 'string',
                  description: "Incoming port on target node. Defaults to 'in'.",
                },
              },
              required: ['source', 'target'],
            },
          },
          auto_run: {
            type: 'boolean',
            description:
              'Whether to immediately execute and verify the workflow after creation and layout. Defaults to true. When user asks to write or create a workflow, it automatically executes to verify live results. Set to false only if explicitly requested not to run.',
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
            description: 'The node type (e.g. http:request, script, conditional, assertion, flow:sub, entry, end, ws:connect, etc.).',
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
              "Node configuration parameters specific to the node type (e.g. for http:request: {method: 'get'|'post', url, headers, body}, for 'flow:sub': {target_flow_id, target_flow_path, inherit_all_variables}, for assertion: {assertions: [...]}, for conditional: {condition_type, field_path, operator, expected_value}).",
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
        "Connect two nodes in the workflow canvas. Automatically determines correct ports ('out' for entry/flow:sub/extractor/proxy, 'true' for http:request/script/assertion/conditional/plugins, 'in' for target).",
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
      description: 'Automatically organize and arrange all nodes and connections on the canvas into an adaptive wrap layout (or horizontal DAG layout).',
      parameters: {
        type: 'object',
        properties: {
          direction: {
            type: 'string',
            enum: ['LR'],
            description: 'Layout direction (horizontal LR). Defaults to LR.',
          },
          mode: {
            type: 'string',
            enum: ['adaptive', 'horizontal'],
            description: 'Layout mode: "adaptive" (adaptive wrap layout based on viewport width to prevent infinite horizontal scrolling, default and recommended) or "horizontal" (single line LR). Defaults to adaptive.',
          },
          viewport_width: {
            type: 'number',
            description: 'Optional canvas viewport width in pixels for adaptive layout mode. Defaults to 1200.',
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
        'Query the complete configuration schema, allowable parameter names, data types, descriptions, default values, and usage examples for any node type (including built-in protocols and plugins like http, flow:sub, assertion, conditional, script, variable:extractor, ws_*, tcp_*, sse_*, graphql_*, redis:*, db:*, kafka:*, etc.).',
      parameters: {
        type: 'object',
        properties: {
          node_type: {
            type: 'string',
            description:
              "The node type identifier, e.g. 'http:request', 'flow:sub', 'assertion', 'conditional', 'script', 'variable:extractor', 'ws:connect', 'tcp:connect', 'redis:command', 'db:query'.",
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
  'http:request': {
    display_name: 'HTTP Request',
    category: 'http',
    description: 'Send HTTP/REST requests (GET, POST, PUT, DELETE, PATCH, etc.)',
    ports: {
      input: ['in'],
      output: ['true (success)', 'false (failure/error)'],
    },
    fields: {
      url: { type: 'string', required: true, description: 'Target URL, supports variable placeholders by bare key ({{API_URL}}, {{USER.id}}) and built-in dynamic variables like {{$guid}}, {{$timestamp}}' },
      method: { type: 'string', enum: ['get', 'post', 'put', 'delete', 'patch', 'head', 'options'], default: 'get', required: true },
      headers: { type: 'object', description: 'Key-value map of HTTP headers, e.g. {"Authorization": "Bearer {{TOKEN}}", "X-Request-Id": "{{$guid}}"}' },
      query_params: { type: 'object', description: 'Key-value map of URL query parameters' },
      body_type: { type: 'string', enum: ['none', 'body', 'form-data', 'x-www-form-urlencoded', 'binary'], default: 'none' },
      body: { type: 'string', description: 'Raw request body (JSON string, text, etc.), supports variable placeholders by bare key ({{API_URL}}) and dynamic variables like {{$guid}}, {{$timestamp}}, {{$randomEmail}}, {{$randomInt}}' },
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
      body: '{"username": "{{USERNAME}}", "password": "{{PASSWORD}}"}',
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
  'flow:sub': {
    display_name: 'SubFlow',
    category: 'tools',
    description: 'Execute another workflow as a modular, reusable subflow with variable inheritance and mapping',
    ports: {
      input: ['in'],
      output: ['out (subflow succeeded)', 'false (subflow failed)'],
    },
    fields: {
      target_flow_id: { type: 'string', description: 'Target flow UUID (for saved workflow in workspace)' },
      target_flow_path: { type: 'string', description: 'Target flow file path (relative or absolute .mpf / .yaml / .json path)' },
      inherit_all_variables: { type: 'boolean', default: true, description: 'Whether child flow inherits all variables from parent' },
      variable_mappings: {
        type: 'array',
        description: 'Explicit input variable mappings from parent to child: [{ source, target }]',
        items: {
          source: { type: 'string', description: 'Variable name or expression in parent flow' },
          target: { type: 'string', description: 'Variable name in child flow' },
        },
      },
      export_all_variables: { type: 'boolean', default: true, description: 'Whether to export all variables from child back to parent' },
      output_mappings: {
        type: 'array',
        description: 'Specific variable mappings exported from child to parent when export_all_variables is false',
        items: {
          source: { type: 'string', description: 'Variable name in child flow' },
          target: { type: 'string', description: 'Variable name in parent flow' },
        },
      },
    },
    example: {
      target_flow_id: 'flow-uuid-12345',
      target_flow_path: './subflows/login.json',
      inherit_all_variables: true,
      export_all_variables: true,
    },
  },
  'variable:extractor': {
    display_name: 'Variable Extractor',
    category: 'tools',
    description: 'Extract values from preceding node response using JSONPath and save into workflow variable pool',
    ports: {
      input: ['in'],
      output: ['out'],
    },
    fields: {
      source_node_id: {
        type: 'string',
        required: false,
        description: 'UUID, alias ID, or name of the upstream data producer node whose output will be extracted. Highly recommended to set explicitly to prevent extraction failures across intermediate nodes.',
      },
      output_mappings: {
        type: 'array',
        items: {
          source: { type: 'string', required: true, description: 'RFC 9535 JSONPath starting with $., e.g. $.body.data.token, $.status' },
          target: { type: 'string', required: true, description: 'Target variable name, e.g. auth_token' },
          description: { type: 'string' },
          enabled: { type: 'boolean', default: true },
        },
      },
    },
    example: {
      source_node_id: 'login',
      output_mappings: [
        { source: '$.body.data.token', target: 'auth_token', description: 'User session token' },
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
  'ws:connect': {
    display_name: 'WebSocket Connect',
    category: 'websocket',
    description: 'Establish WebSocket connection',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      url: { type: 'string', required: true, description: 'WebSocket URL, e.g. wss://echo.websocket.org' },
      headers: { type: 'object' },
    },
  },
  'ws:send_collect': {
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
  'ws:close': {
    display_name: 'WebSocket Close',
    category: 'websocket',
    description: 'Close WebSocket connection',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {},
  },
  'tcp:connect': {
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
  'tcp:send': {
    display_name: 'TCP Send',
    category: 'tcp',
    description: 'Send raw data over TCP socket',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      data: { type: 'string', required: true },
    },
  },
  'tcp:receive': {
    display_name: 'TCP Receive',
    category: 'tcp',
    description: 'Receive data from TCP socket',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      max_bytes: { type: 'number', default: 4096 },
      timeout_ms: { type: 'number', default: 10000 },
    },
  },
  'tcp:close': {
    display_name: 'TCP Close',
    category: 'tcp',
    description: 'Close TCP socket connection',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {},
  },
  'udp:bind': {
    display_name: 'UDP Bind',
    category: 'udp',
    description: 'Bind local UDP socket',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      local_port: { type: 'number', required: true },
    },
  },
  'udp:send_to': {
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
  'udp:recv_from': {
    display_name: 'UDP Receive From',
    category: 'udp',
    description: 'Receive UDP datagram',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      timeout_ms: { type: 'number', default: 5000 },
    },
  },
  'udp:close': {
    display_name: 'UDP Close',
    category: 'udp',
    description: 'Close UDP socket',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {},
  },
  'sse:connect': {
    display_name: 'SSE Connect',
    category: 'sse',
    description: 'Connect to Server-Sent Events stream',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      url: { type: 'string', required: true },
      headers: { type: 'object' },
    },
  },
  'sse:listen': {
    display_name: 'SSE Listen',
    category: 'sse',
    description: 'Listen for SSE event',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      event: { type: 'string', default: 'message' },
      timeout_ms: { type: 'number', default: 15000 },
    },
  },
  'sse:disconnect': {
    display_name: 'SSE Disconnect',
    category: 'sse',
    description: 'Disconnect SSE stream',
    ports: { input: ['in'], output: ['out'] },
    fields: {},
  },
  'graphql:connect': {
    display_name: 'GraphQL Connect',
    category: 'graphql',
    description: 'Initialize GraphQL endpoint connection',
    ports: { input: ['in'], output: ['true', 'false'] },
    fields: {
      endpoint: { type: 'string', required: true },
      headers: { type: 'object' },
    },
  },
  'graphql:query': {
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
      id?: string;
      type: string;
      name?: string;
      config?: Record<string, unknown>;
      on_error?: ExecutionErrorStrategy;
    }>;
    edges?: Array<{
      source: string;
      target: string;
      source_port?: string;
      target_port?: string;
    }>;
    auto_run?: boolean;
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

    // 建立节点引用映射表 (alias id, name, index, uuid -> { uuid, type })
    const nodeIdMap = new Map<string, { uuid: string; type: string }>();
    for (const existingNode of currentFlow.nodes) {
      nodeIdMap.set(existingNode.uuid, { uuid: existingNode.uuid, type: existingNode.type });
      nodeIdMap.set(existingNode.name, { uuid: existingNode.uuid, type: existingNode.type });
    }

    // 1. 判断起始连接节点 (未显式提供 edges，或 edges 中未显式包含起始前驱连线时自动连入首个新建节点)
    let initialPredecessorUuid: string | undefined = args.connect_from_node_uuid;
    let initialPredecessorType: string | undefined;

    if (!initialPredecessorUuid && currentFlow.nodes.length === 1 && currentFlow.nodes[0].type === 'entry') {
      const entryNode = currentFlow.nodes[0];
      const hasOutConn = currentFlow.connections.some((c) => c.source_node_uuid === entryNode.uuid);
      if (!hasOutConn) {
        initialPredecessorUuid = entryNode.uuid;
        initialPredecessorType = 'entry';
      }
    } else if (initialPredecessorUuid) {
      const found = currentFlow.nodes.find((n) => n.uuid === initialPredecessorUuid);
      if (found) {
        initialPredecessorType = found.type;
      }
    }

    let prevNodeUuid: string | undefined = initialPredecessorUuid;
    let prevNodeType: string | undefined = initialPredecessorType;
    const createdNodes: Array<{ uuid: string; name: string; type: string; id?: string }> = [];
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
      createdNodes.push({ uuid, name, type: nodeSpec.type, ...(nodeSpec.id ? { id: nodeSpec.id } : {}) });

      nodeIdMap.set(uuid, { uuid, type: nodeSpec.type });
      nodeIdMap.set(name, { uuid, type: nodeSpec.type });
      nodeIdMap.set(String(i), { uuid, type: nodeSpec.type });
      if (nodeSpec.id) {
        nodeIdMap.set(nodeSpec.id, { uuid, type: nodeSpec.type });
      }

      // 如果未指定显式 edges 且有前驱节点，自动建立线性连线
      if ((!args.edges || args.edges.length === 0) && prevNodeUuid && prevNodeType) {
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

    // 建立起始前驱节点与管道首个节点的连线（若 edges 未显式包含以起始前驱为源的连线）
    const hasPredecessorInEdges = args.edges?.some(
      (e) =>
        e.source === initialPredecessorUuid ||
        e.source === 'entry' ||
        e.source === 'Entry' ||
        (initialPredecessorType && e.source === initialPredecessorType),
    );

    if (
      initialPredecessorUuid &&
      initialPredecessorType &&
      args.edges &&
      args.edges.length > 0 &&
      !hasPredecessorInEdges &&
      createdNodes.length > 0
    ) {
      const firstNodeSpec = args.nodes[0];
      const firstCreatedNode = createdNodes[0];
      const { sourcePort, targetPort } = resolveDefaultPorts(
        initialPredecessorType,
        firstNodeSpec.type,
        args.connect_from_port,
        'in',
      );
      const connId = `conn_${initialPredecessorUuid}_${sourcePort}_to_${firstCreatedNode.uuid}_${targetPort}`;
      const connection: Connection = {
        id: connId,
        source_node_uuid: initialPredecessorUuid,
        source_port_id: sourcePort,
        target_node_uuid: firstCreatedNode.uuid,
        target_port_id: targetPort,
      };
      flowState.addConnection(connection);
      createdConnections.push({
        id: connId,
        source: `${initialPredecessorUuid}:${sourcePort}`,
        target: `${firstCreatedNode.uuid}:${targetPort}`,
      });
    }

    // 如果指定了显式 edges，按 edges 构建拓扑连线
    if (args.edges && args.edges.length > 0) {
      const latestFlow = useFlowStore.getState().flow;
      const allNodes = latestFlow ? latestFlow.nodes : currentFlow.nodes;

      for (const edge of args.edges) {
        const sourceInfo =
          nodeIdMap.get(edge.source) ||
          allNodes.find((n) => n.uuid === edge.source || n.name === edge.source);
        const targetInfo =
          nodeIdMap.get(edge.target) ||
          allNodes.find((n) => n.uuid === edge.target || n.name === edge.target);

        if (!sourceInfo) {
          throw new Error(
            `Cannot resolve edge source "${edge.source}". Valid aliases or names are: ${Array.from(nodeIdMap.keys()).join(', ')}`,
          );
        }
        if (!targetInfo) {
          throw new Error(
            `Cannot resolve edge target "${edge.target}". Valid aliases or names are: ${Array.from(nodeIdMap.keys()).join(', ')}`,
          );
        }

        const { sourcePort, targetPort } = resolveDefaultPorts(
          sourceInfo.type,
          targetInfo.type,
          edge.source_port,
          edge.target_port,
        );

        const connId = `conn_${sourceInfo.uuid}_${sourcePort}_to_${targetInfo.uuid}_${targetPort}`;
        const connection: Connection = {
          id: connId,
          source_node_uuid: sourceInfo.uuid,
          source_port_id: sourcePort,
          target_node_uuid: targetInfo.uuid,
          target_port_id: targetPort,
        };

        flowState.addConnection(connection);
        createdConnections.push({
          id: connId,
          source: `${sourceInfo.uuid}:${sourcePort}`,
          target: `${targetInfo.uuid}:${targetPort}`,
        });
      }
    }
    // 自动解析与补全 variable:extractor 的 source_node_id（数据源）
    const flowBeforeLayout = useFlowStore.getState().flow;
    const currentNodes = flowBeforeLayout?.nodes || [];
    const currentConnections = flowBeforeLayout?.connections || [];

    for (const createdNode of createdNodes) {
      if (createdNode.type !== 'variable:extractor') continue;

      const flowNode = currentNodes.find((n) => n.uuid === createdNode.uuid);
      if (!flowNode) continue;

      const extractorConfig = flowNode.data as unknown as { source_node_id?: string };
      let resolvedSourceUuid: string | undefined;

      // 1. 如果指定了 source_node_id，先解析别名/名称/UUID
      if (extractorConfig.source_node_id) {
        const rawSource = extractorConfig.source_node_id.trim();
        const match =
          nodeIdMap.get(rawSource)?.uuid ||
          currentNodes.find((n) => n.uuid === rawSource || n.name === rawSource)?.uuid;
        if (match) {
          resolvedSourceUuid = match;
        }
      }

      // 2. 如果未指定或未能解析，智能回溯查找最近的数据生产者节点
      if (!resolvedSourceUuid) {
        // 2a. 沿连线拓扑向上回溯 (BFS 查找最近的上游 producer)
        const queue: string[] = [];
        const visited = new Set<string>([createdNode.uuid]);

        for (const conn of currentConnections) {
          if (conn.target_node_uuid === createdNode.uuid) {
            queue.push(conn.source_node_uuid);
          }
        }

        while (queue.length > 0) {
          const currId = queue.shift()!;
          if (visited.has(currId)) continue;
          visited.add(currId);

          const srcNode = currentNodes.find((n) => n.uuid === currId);
          if (!srcNode) continue;

          if (isProducerNode(srcNode.type)) {
            resolvedSourceUuid = srcNode.uuid;
            break;
          }

          for (const conn of currentConnections) {
            if (conn.target_node_uuid === currId && !visited.has(conn.source_node_uuid)) {
              queue.push(conn.source_node_uuid);
            }
          }
        }

        // 2b. 若拓扑回溯未命中，按 args.nodes 顺序向前查找最近的 producer
        if (!resolvedSourceUuid) {
          const nodeIdx = createdNodes.findIndex((n) => n.uuid === createdNode.uuid);
          if (nodeIdx > 0) {
            for (let j = nodeIdx - 1; j >= 0; j--) {
              if (isProducerNode(args.nodes[j].type)) {
                const prevCreated = createdNodes[j];
                if (prevCreated) {
                  resolvedSourceUuid = prevCreated.uuid;
                  break;
                }
              }
            }
          }
        }

        // 2c. 若仍未命中，从已有画布节点从前驱向前找最近的 producer
        if (!resolvedSourceUuid) {
          const currentIdx = currentNodes.findIndex((n) => n.uuid === createdNode.uuid);
          if (currentIdx > 0) {
            for (let j = currentIdx - 1; j >= 0; j--) {
              if (isProducerNode(currentNodes[j].type)) {
                resolvedSourceUuid = currentNodes[j].uuid;
                break;
              }
            }
          }
        }
      }

      if (resolvedSourceUuid && extractorConfig.source_node_id !== resolvedSourceUuid) {
        flowState.updateNode(createdNode.uuid, {
          data: {
            ...flowNode.data,
            source_node_id: resolvedSourceUuid,
          } as unknown as NodeData,
        });
      }
    }

    // 自动重排布局：采用自适应折行布局 (adaptive wrap layout)
    const refreshedFlow = useFlowStore.getState().flow;
    if (refreshedFlow && refreshedFlow.nodes.length > 0) {
      const positions = getAdaptiveLayoutedPositions(refreshedFlow.nodes, refreshedFlow.connections, {
        viewportWidth: 1200,
      });
      flowState.batchUpdateNodePositions(positions);
    }
    if (createdNodes.length > 0) {
      flowState.selectNode(createdNodes[0].uuid);
      flowState.centerOnNode(createdNodes[0].uuid);
    }

    let executionResult: {
      status?: string;
      duration_ms?: number;
      executed_count?: number;
      flow_error?: string | null;
      failed_nodes?: string[];
      diagnostic_summary?: string;
      error?: string;
    } | undefined;

    const shouldAutoRun = args.auto_run !== false; // 默认执行：用户要求写流程默认写完执行
    if (shouldAutoRun) {
      try {
        const runRes = await executionToolHandlers.run_flow();
        const execReport = useExecutionStore.getState().report;
        const executedCount = execReport?.executed_count ?? execReport?.node_reports?.length;
        let flowErrorStr: string | null = null;
        if ('flow_error' in runRes && runRes.flow_error?.message) {
          flowErrorStr = runRes.flow_error.message;
        } else if (runRes.error) {
          flowErrorStr = runRes.error;
        }
        executionResult = {
          status: 'status' in runRes && runRes.status ? runRes.status : (runRes.success ? 'success' : 'failed'),
          duration_ms: 'execution_time_ms' in runRes ? runRes.execution_time_ms : undefined,
          executed_count: executedCount,
          flow_error: flowErrorStr,
          failed_nodes: 'failed_nodes' in runRes ? runRes.failed_nodes : [],
          diagnostic_summary: 'diagnostic_summary' in runRes ? runRes.diagnostic_summary : undefined,
          error: runRes.error,
        };
      } catch (err: unknown) {
        executionResult = {
          status: 'failed',
          flow_error: err instanceof Error ? err.message : String(err),
          failed_nodes: [],
        };
      }
    }

    return {
      success: true,
      created_count: createdNodes.length,
      created_nodes: createdNodes,
      created_connections: createdConnections,
      ...(executionResult ? { execution: executionResult } : {}),
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
    if (args.type === 'variable:extractor') {
      const extData = data as unknown as { source_node_id?: string };
      if (extData.source_node_id) {
        const rawSource = extData.source_node_id.trim();
        const match = currentFlow.nodes.find((n) => n.uuid === rawSource || n.name === rawSource);
        if (match) {
          extData.source_node_id = match.uuid;
        }
      } else {
        const producer = [...currentFlow.nodes].reverse().find((n) => isProducerNode(n.type));
        if (producer) {
          extData.source_node_id = producer.uuid;
        }
      }
    }

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
      const mergedConfig = { ...(node.data as unknown as Record<string, unknown>), ...args.config };
      const defaultData = buildDefaultNodeData(node.type, mergedConfig);
      if (node.type === 'variable:extractor') {
        const extData = defaultData as unknown as { source_node_id?: string };
        if (extData.source_node_id) {
          const rawSource = extData.source_node_id.trim();
          const match = currentFlow.nodes.find((n) => n.uuid === rawSource || n.name === rawSource);
          if (match) {
            extData.source_node_id = match.uuid;
          }
        }
      }
      updates.data = defaultData;
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

  async auto_layout(args?: { direction?: 'LR'; mode?: 'horizontal' | 'adaptive'; viewport_width?: number }) {
    const flowState = useFlowStore.getState();
    const currentFlow = flowState.flow;
    if (!currentFlow || currentFlow.nodes.length === 0) {
      return { success: true, node_count: 0 };
    }

    const mode = args?.mode || 'adaptive';
    const direction = args?.direction || 'LR';

    let positions: Map<string, { x: number; y: number }>;
    if (mode === 'adaptive') {
      positions = getAdaptiveLayoutedPositions(currentFlow.nodes, currentFlow.connections, {
        viewportWidth: args?.viewport_width ?? 1200,
      });
    } else {
      positions = getLayoutedPositions(currentFlow.nodes, currentFlow.connections, direction);
    }
    flowState.batchUpdateNodePositions(positions);

    return {
      success: true,
      node_count: currentFlow.nodes.length,
      direction,
      mode,
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
