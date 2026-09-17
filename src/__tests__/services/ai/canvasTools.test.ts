import { describe, it, expect, beforeEach } from 'vitest';
import { useFlowStore } from '@/stores/flow';
import {
  resolveDefaultPorts,
  buildDefaultNodeData,
  normalizeJsonPath,
  canvasToolHandlers,
} from '@/services/ai/tools/canvasTools';
import { buildSystemPrompt } from '@/services/ai/contextBuilder';
import type { Flow } from '@/types';

describe('resolveDefaultPorts', () => {
  it('entry -> http: 应该将源端口解析为 out，目标端口解析为 in', () => {
    const { sourcePort, targetPort } = resolveDefaultPorts('entry', 'http:request');
    expect(sourcePort).toBe('out');
    expect(targetPort).toBe('in');
  });

  it('http -> assertion: 默认源端口应为 true，目标端口应为 in', () => {
    const { sourcePort, targetPort } = resolveDefaultPorts('http:request', 'assertion');
    expect(sourcePort).toBe('true');
    expect(targetPort).toBe('in');
  });

  it('智能纠偏：如果源节点是 http 且模型传入 out，应自动纠偏为 true', () => {
    const { sourcePort, targetPort } = resolveDefaultPorts('http:request', 'script', 'out', 'default');
    expect(sourcePort).toBe('true');
    expect(targetPort).toBe('in');
  });

  it('智能纠偏：如果传入 default 端口，自动转换为正确端口', () => {
    const { sourcePort, targetPort } = resolveDefaultPorts('script', 'assertion', 'default', 'default');
    expect(sourcePort).toBe('true');
    expect(targetPort).toBe('in');
  });

  it('保留显式指定的 false 失败/异常分支端口', () => {
    const { sourcePort, targetPort } = resolveDefaultPorts('conditional', 'http:request', 'false', 'in');
    expect(sourcePort).toBe('false');
    expect(targetPort).toBe('in');
  });

  it('variable:extractor -> end: 源端口为 out，目标端口为 in', () => {
    const { sourcePort, targetPort } = resolveDefaultPorts('variable:extractor', 'end');
    expect(sourcePort).toBe('out');
    expect(targetPort).toBe('in');
  });
  it('flow:sub -> http: 源端口默认应为 out，目标端口为 in', () => {
    const { sourcePort, targetPort } = resolveDefaultPorts('flow:sub', 'http:request');
    expect(sourcePort).toBe('out');
    expect(targetPort).toBe('in');
  });

  it('智能纠偏：如果模型给 flow:sub 传入 true 端口，应自动纠偏为 out', () => {
    const { sourcePort, targetPort } = resolveDefaultPorts('flow:sub', 'script', 'true', 'in');
    expect(sourcePort).toBe('out');
    expect(targetPort).toBe('in');
  });
});

describe('buildDefaultNodeData 格式规范化与容错', () => {
  it('HTTP 节点：方法大写自动转小写，自动推断 body_type', () => {
    const data = buildDefaultNodeData('http:request', {
      method: 'POST',
      url: 'https://example.com/api',
      body: '{"username":"alice"}',
    }) as unknown as Record<string, unknown>;

    expect(data.type).toBe('http:request');
    expect(data.method).toBe('post');
    expect(data.body_type).toBe('body');
    expect(data.url).toBe('https://example.com/api');
  });

  it('Assertion 节点：on_failure abort 自动纠正为 route_false，rules 归一化为 assertions', () => {
    const data = buildDefaultNodeData('assertion', {
      on_failure: 'abort',
      rules: [
        { target: 'status_code', operator: 'equal', expected: 200 },
      ],
    }) as unknown as Record<string, unknown>;

    expect(data.type).toBe('assertion');
    expect(data.on_failure).toBe('route_false');
    const assertions = data.assertions as Array<{ target: string; operator: string; expected: unknown }>;
    expect(assertions).toHaveLength(1);
    expect(assertions[0].target).toBe('status_code');
    expect(assertions[0].expected).toBe(200);
  });

  it('Conditional 节点：自动支持高级表达式或 conditions 容错', () => {
    const advData = buildDefaultNodeData('conditional', {
      advanced_expression: '{{status}} == 200',
    }) as unknown as Record<string, unknown>;
    expect(advData.condition_type).toBe('advanced');
    expect(advData.advanced_expression).toBe('{{status}} == 200');

    const simpleData = buildDefaultNodeData('conditional', {
      conditions: [{ field: 'status', operator: 'equal', value: 200 }],
    }) as unknown as Record<string, unknown>;
    expect(simpleData.condition_type).toBe('simple');
    expect(simpleData.field_path).toBe('status');
    expect(simpleData.expected_value).toBe(200);
  });

  it('VariableExtractor 节点：extractions 自动纠偏为 output_mappings 并补齐 $. 前缀', () => {
    const data = buildDefaultNodeData('variable:extractor', {
      extractions: [
        { field: 'body.data.token', name: 'auth_token' },
        { field: '$.status', name: 'http_status' },
      ],
    }) as unknown as Record<string, unknown>;

    expect(data.type).toBe('variable:extractor');
    const mappings = data.output_mappings as Array<{ source: string; target: string }>;
    expect(mappings).toHaveLength(2);
    expect(mappings[0].source).toBe('$.body.data.token');
    expect(mappings[0].target).toBe('auth_token');
    expect(mappings[1].source).toBe('$.status');
    expect(mappings[1].target).toBe('http_status');
  });
  it('flow:sub 节点：配置标准化与默认继承配置', () => {
    const data = buildDefaultNodeData('flow:sub', {
      target_flow_path: './subflows/auth.json',
    }) as unknown as Record<string, unknown>;

    expect(data.type).toBe('flow:sub');
    expect(data.target_flow_path).toBe('./subflows/auth.json');
    expect(data.inherit_all_variables).toBe(true);
    expect(data.export_all_variables).toBe(true);
  });

  it('normalizeJsonPath: 正确为省略 $. 前缀的路径补齐前缀', () => {
    expect(normalizeJsonPath('')).toBe('');
    expect(normalizeJsonPath('$.body.data')).toBe('$.body.data');
    expect(normalizeJsonPath('body.data.provider.id')).toBe('$.body.data.provider.id');
    expect(normalizeJsonPath('status')).toBe('$.status');
    expect(normalizeJsonPath('[0].id')).toBe('$[0].id');
    expect(normalizeJsonPath('.body.id')).toBe('$.body.id');
  });
});

describe('canvasToolHandlers 集成测试', () => {
  beforeEach(() => {
    useFlowStore.getState().reset();
    const testFlow: Flow = {
      uuid: 'flow-test-1',
      name: 'Test Flow',
      nodes: [
        {
          uuid: 'entry-1',
          type: 'entry',
          name: 'Entry',
          position: { x: 50, y: 100 },
          data: { type: 'entry' } as unknown as Flow['nodes'][0]['data'],
        },
      ],
      connections: [],
    };
    useFlowStore.getState().setFlow(testFlow);
  });

  it('connect_nodes: 自动推断并在 entry 和 http 间建立有效连线', async () => {
    const httpRes = await canvasToolHandlers.add_node({
      type: 'http:request',
      name: 'Get User',
      config: { url: 'https://api.example.com/user' },
    });

    const connRes = await canvasToolHandlers.connect_nodes({
      source_node_uuid: 'entry-1',
      target_node_uuid: httpRes.node_uuid,
    });

    expect(connRes.success).toBe(true);
    const flow = useFlowStore.getState().flow;
    expect(flow?.connections).toHaveLength(1);
    expect(flow?.connections[0].source_port_id).toBe('out');
    expect(flow?.connections[0].target_port_id).toBe('in');
  });

  it('batch_create_pipeline: 一键创建完整的链式工作流并自动完成端口串联', async () => {
    const result = await canvasToolHandlers.batch_create_pipeline({
      nodes: [
        {
          type: 'http:request',
          name: 'Fetch Data',
          config: { method: 'get', url: 'https://httpbin.org/json' },
        },
        {
          type: 'assertion',
          name: 'Assert 200',
          config: {
            assertions: [{ target: 'status_code', operator: 'equal', expected: 200 }],
          },
        },
        {
          type: 'variable:extractor',
          name: 'Extract Token',
          config: {
            output_mappings: [{ source: 'body.token', target: 'token' }],
          },
        },
        {
          type: 'end',
          name: 'End Step',
        },
      ],
    });

    expect(result.success).toBe(true);
    expect(result.created_count).toBe(4);

    const flow = useFlowStore.getState().flow;
    // 包含初始 entry 节点 + 4 个新节点 = 5 个节点
    expect(flow?.nodes).toHaveLength(5);
    // 应该自动建立 4 条连线：entry -> http -> assertion -> extractor -> end
    expect(flow?.connections).toHaveLength(4);

    const [c1, c2, c3, c4] = flow!.connections;
    // entry -> http
    expect(c1.source_port_id).toBe('out');
    expect(c1.target_port_id).toBe('in');

    // http -> assertion
    expect(c2.source_port_id).toBe('true');
    expect(c2.target_port_id).toBe('in');

    // assertion -> extractor
    expect(c3.source_port_id).toBe('true');
    expect(c3.target_port_id).toBe('in');

    // extractor -> end
    expect(c4.source_port_id).toBe('out');
    expect(c4.target_port_id).toBe('in');
  });

  it('get_node_detail: 获取指定节点的完整配置、连接和执行报告', async () => {
    const httpRes = await canvasToolHandlers.add_node({
      type: 'http:request',
      name: 'Get User',
      config: { url: 'https://api.example.com/user', method: 'get' },
    });

    await canvasToolHandlers.connect_nodes({
      source_node_uuid: 'entry-1',
      target_node_uuid: httpRes.node_uuid,
    });

    const res = await canvasToolHandlers.get_node_detail({ node_uuid: httpRes.node_uuid });
    expect(res.success).toBe(true);
    expect(res.node.uuid).toBe(httpRes.node_uuid);
    expect(res.node.name).toBe('Get User');
    expect(res.node.config).toBeDefined();
    expect(res.connections.incoming).toHaveLength(1);
    expect(res.connections.incoming[0].source_node_uuid).toBe('entry-1');
  });

  it('get_node_type_schema: 获取内置节点与未知节点的 Schema', async () => {
    const httpSchema = await canvasToolHandlers.get_node_type_schema({ node_type: 'http:request' });
    expect(httpSchema.success).toBe(true);
    expect(httpSchema.builtin_schema).toBeDefined();
    expect(httpSchema.default_config).toBeDefined();

    const customSchema = await canvasToolHandlers.get_node_type_schema({ node_type: 'redis:command' });
    expect(customSchema.success).toBe(true);
    expect(customSchema.hint).toBeDefined();

    const subflowSchema = await canvasToolHandlers.get_node_type_schema({ node_type: 'flow:sub' });
    expect(subflowSchema.success).toBe(true);
    expect(subflowSchema.builtin_schema).toBeDefined();
    const schema = subflowSchema.builtin_schema;
    if (schema && typeof schema === 'object' && 'display_name' in schema) {
      expect(schema.display_name).toBe('SubFlow');
    } else {
      expect.fail('flow:sub builtin_schema missing display_name');
    }
  });
  it('get_flow_nodes_config: 批量获取节点完整配置', async () => {
    const allConfigs = await canvasToolHandlers.get_flow_nodes_config();
    expect(allConfigs.success).toBe(true);
    expect(allConfigs.nodes.length).toBeGreaterThan(0);
  });

  it('auto_layout: 支持 horizontal 和 adaptive 两种排版模式', async () => {
    const horizRes = await canvasToolHandlers.auto_layout({ mode: 'horizontal' });
    expect(horizRes.success).toBe(true);
    expect(horizRes.mode).toBe('horizontal');

    const adaptRes = await canvasToolHandlers.auto_layout({ mode: 'adaptive', viewport_width: 1000 });
    expect(adaptRes.success).toBe(true);
    expect(adaptRes.mode).toBe('adaptive');
  });
});

describe('buildSystemPrompt', () => {
  it('系统提示词包含精准的 DAG 路由规则，明确 target 为 in，source 为 true/out', () => {
    const prompt = buildSystemPrompt();
    expect(prompt).toContain("Target Port");
    expect(prompt).toContain("'in'");
    expect(prompt).toContain("'true'");
    expect(prompt).toContain("'false'");
    expect(prompt).toContain("'out'");
    expect(prompt).toContain("NEVER use 'default' as a port name!");
    expect(prompt).toContain("batch_create_pipeline");
    expect(prompt).toContain("Node Configuration Schemas");
  });
});
