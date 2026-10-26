import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useFlowStore } from '@/stores/flow';
import { useExecutionStore } from '@/stores/execution';
import {
  resolveDefaultPorts,
  buildDefaultNodeData,
  normalizeJsonPath,
  canvasToolHandlers,
} from '@/services/ai/tools/canvasTools';
import { executionToolHandlers } from '@/services/ai/tools/executionTools';
import { buildSystemPrompt } from '@/services/ai/contextBuilder';
import type { Flow, Node } from '@/types';
import { isVariableExtractorConfig } from '@/types';

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
    expect(flow?.nodes).toHaveLength(5);
    expect(flow?.connections).toHaveLength(4);

    const [c1, c2, c3, c4] = flow!.connections;
    expect(c1.source_port_id).toBe('out');
    expect(c1.target_port_id).toBe('in');
    expect(c2.source_port_id).toBe('true');
    expect(c2.target_port_id).toBe('in');
    expect(c3.source_port_id).toBe('true');
    expect(c3.target_port_id).toBe('in');
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

  it('auto_layout: 默认采用 adaptive 自适应折行排版模式，并支持显式传入 horizontal/adaptive', async () => {
    const defaultRes = await canvasToolHandlers.auto_layout();
    expect(defaultRes.success).toBe(true);
    expect(defaultRes.mode).toBe('adaptive');

    const horizRes = await canvasToolHandlers.auto_layout({ mode: 'horizontal' });
    expect(horizRes.success).toBe(true);
    expect(horizRes.mode).toBe('horizontal');

    const adaptRes = await canvasToolHandlers.auto_layout({ mode: 'adaptive', viewport_width: 1000 });
    expect(adaptRes.success).toBe(true);
    expect(adaptRes.mode).toBe('adaptive');
  });
});

describe('canvasTools - batch_create_pipeline declarative topology & auto_run', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    useFlowStore.getState().reset();
    useExecutionStore.getState().reset();

    const initialFlow: Flow = {
      uuid: 'flow-test-1',
      name: 'Test Flow',
      nodes: [
        {
          uuid: 'entry-uuid-1',
          type: 'entry',
          name: 'Entry',
          position: { x: 50, y: 150 },
        } as Node,
      ],
      connections: [],
      initial_variables: {},
    };
    useFlowStore.getState().setFlow(initialFlow);
  });

  it('未提供 edges 时保持向下兼容：按顺序线性串联节点', async () => {
    const result = await canvasToolHandlers.batch_create_pipeline({
      nodes: [
        { type: 'http:request', name: 'HTTP_1' },
        { type: 'assertion', name: 'ASSERT_1' },
      ],
    });

    expect(result.success).toBe(true);
    expect(result.created_count).toBe(2);
    expect(result.created_nodes).toHaveLength(2);

    const flow = useFlowStore.getState().flow;
    expect(flow?.nodes).toHaveLength(3); // entry + 2 new nodes

    // 包含从 entry 到 HTTP_1，以及 HTTP_1 到 ASSERT_1 的线性连线
    expect(flow?.connections.length).toBeGreaterThanOrEqual(2);
    const conn1 = flow?.connections.find((c) => c.source_node_uuid === 'entry-uuid-1');
    expect(conn1).toBeDefined();
    expect(conn1?.source_port_id).toBe('out');
    expect(conn1?.target_port_id).toBe('in');
  });

  it('提供显式 edges 时，根据节点 id 别名精确构建多分支拓扑，并自动将画布起始 entry 节点连入首个新建节点', async () => {
    const result = await canvasToolHandlers.batch_create_pipeline({
      nodes: [
        {
          id: 'login',
          type: 'http:request',
          name: '用户登录',
          config: { url: 'https://api.test/login', method: 'post' },
        },
        {
          id: 'getUser',
          type: 'http:request',
          name: '获取用户信息',
          config: { url: 'https://api.test/user', method: 'get' },
        },
        {
          id: 'errorHandler',
          type: 'script',
          name: '登录失败告警',
          config: { script: 'console.error("Login failed");' },
        },
      ],
      edges: [
        { source: 'login', target: 'getUser', source_port: 'true' },
        { source: 'login', target: 'errorHandler', source_port: 'false' },
      ],
    });

    expect(result.success).toBe(true);
    expect(result.created_count).toBe(3);

    const flow = useFlowStore.getState().flow;
    const loginNode = flow?.nodes.find((n) => n.name === '用户登录');
    const getUserNode = flow?.nodes.find((n) => n.name === '获取用户信息');
    const errorNode = flow?.nodes.find((n) => n.name === '登录失败告警');

    expect(loginNode).toBeDefined();
    expect(getUserNode).toBeDefined();
    expect(errorNode).toBeDefined();

    // 验证起始 entry 节点自动连入首个新建节点 (entry:out -> login:in)
    const entryConn = flow?.connections.find(
      (c) =>
        c.source_node_uuid === 'entry-uuid-1' &&
        c.target_node_uuid === loginNode?.uuid &&
        c.source_port_id === 'out' &&
        c.target_port_id === 'in',
    );
    expect(entryConn).toBeDefined();

    // 校验成功分支连线 (login:true -> getUser:in)
    const trueConn = flow?.connections.find(
      (c) =>
        c.source_node_uuid === loginNode?.uuid &&
        c.target_node_uuid === getUserNode?.uuid &&
        c.source_port_id === 'true' &&
        c.target_port_id === 'in',
    );
    expect(trueConn).toBeDefined();

    // 校验失败异常路由分支 (login:false -> errorHandler:in)
    const falseConn = flow?.connections.find(
      (c) =>
        c.source_node_uuid === loginNode?.uuid &&
        c.target_node_uuid === errorNode?.uuid &&
        c.source_port_id === 'false' &&
        c.target_port_id === 'in',
    );
    expect(falseConn).toBeDefined();
  });

  it('当 edges 中的 source 或 target 无法解析时抛出明确异常', async () => {
    await expect(
      canvasToolHandlers.batch_create_pipeline({
        nodes: [{ id: 'valid_node', type: 'http:request' }],
        edges: [{ source: 'valid_node', target: 'non_existent_target' }],
      }),
    ).rejects.toThrow('Cannot resolve edge target "non_existent_target"');
  });

  it('指定 auto_run: true 时自动触发执行并挂载结构化测试结果', async () => {
    const runFlowSpy = vi.spyOn(executionToolHandlers, 'run_flow').mockResolvedValue({
      success: true,
      execution_time_ms: 120,
      status: 'success',
      failed_nodes: [],
      failed_node_reports: [],
      diagnostic_summary: 'All executed nodes succeeded.',
      flow_error: undefined,
      error: undefined,
    });

    const result = await canvasToolHandlers.batch_create_pipeline({
      nodes: [
        {
          id: 'n1',
          type: 'http:request',
          name: 'Test Endpoint',
        },
      ],
      auto_run: true,
    });

    expect(runFlowSpy).toHaveBeenCalledTimes(1);
    expect(result.success).toBe(true);
    expect(result.execution).toBeDefined();
    expect(result.execution?.status).toBe('success');
    expect(result.execution?.duration_ms).toBe(120);
    expect(result.execution?.failed_nodes).toEqual([]);
  });

  it('当 auto_run 出现执行异常时，平滑捕获并在 execution 字段中返回失败信息，不抛出异常', async () => {
    vi.spyOn(executionToolHandlers, 'run_flow').mockRejectedValue(
      new Error('Engine connection timeout'),
    );

    const result = await canvasToolHandlers.batch_create_pipeline({
      nodes: [
        {
          id: 'n1',
          type: 'http:request',
          name: 'Timeout Node',
        },
      ],
      auto_run: true,
    });

    expect(result.success).toBe(true);
    expect(result.created_count).toBe(1);
    expect(result.execution).toBeDefined();
    expect(result.execution?.status).toBe('failed');
    expect(result.execution?.flow_error).toContain('Engine connection timeout');
  });

  it('未指定 auto_run 时默认执行流程并返回执行结果', async () => {
    const runFlowSpy = vi.spyOn(executionToolHandlers, 'run_flow').mockResolvedValue({
      success: true,
      execution_time_ms: 80,
      status: 'success',
      failed_nodes: [],
      failed_node_reports: [],
      diagnostic_summary: 'All nodes passed',
      flow_error: undefined,
      error: undefined,
    });

    const result = await canvasToolHandlers.batch_create_pipeline({
      nodes: [
        {
          id: 'test_node',
          type: 'http:request',
          name: 'Default Exec Endpoint',
        },
      ],
    });

    expect(runFlowSpy).toHaveBeenCalledTimes(1);
    expect(result.execution).toBeDefined();
    expect(result.execution?.status).toBe('success');
  });

  it('显式指定 auto_run: false 时不触发流程执行', async () => {
    const runFlowSpy = vi.spyOn(executionToolHandlers, 'run_flow').mockResolvedValue({
      success: true,
      execution_time_ms: 50,
      status: 'success',
      failed_nodes: [],
      failed_node_reports: [],
      diagnostic_summary: '',
      flow_error: undefined,
      error: undefined,
    });

    const result = await canvasToolHandlers.batch_create_pipeline({
      nodes: [
        {
          id: 'no_run_node',
          type: 'http:request',
          name: 'No Run Endpoint',
        },
      ],
      auto_run: false,
    });

    expect(runFlowSpy).not.toHaveBeenCalled();
    expect(result.execution).toBeUndefined();
  });

  it('当 variable:extractor 漏配 source_node_id 时，自动跨越中间的 assertion 节点查找到正确的上游数据生产节点', async () => {
    vi.spyOn(executionToolHandlers, 'run_flow').mockResolvedValue({
      success: true,
      status: 'success',
      failed_nodes: [],
      failed_node_reports: [],
      execution_time_ms: 10,
      diagnostic_summary: '',
      flow_error: undefined,
      error: undefined,
    });

    const result = await canvasToolHandlers.batch_create_pipeline({
      nodes: [
        {
          id: 'login',
          type: 'http:request',
          name: '用户登录',
          config: { url: 'https://api.test/login', method: 'post' },
        },
        {
          id: 'assert_status',
          type: 'assertion',
          name: '断言200',
          config: {
            assertions: [{ target: 'status_code', operator: 'equal', expected: 200 }],
          },
        },
        {
          id: 'extract_token',
          type: 'variable:extractor',
          name: '提取Token',
          config: {
            // 模拟 AI 漏掉了 source_node_id
            output_mappings: [{ source: '$.body.token', target: 'AUTH_TOKEN' }],
          },
        },
      ],
      edges: [
        { source: 'login', target: 'assert_status', source_port: 'true' },
        { source: 'assert_status', target: 'extract_token', source_port: 'true' },
      ],
      auto_run: false,
    });

    expect(result.success).toBe(true);

    const flow = useFlowStore.getState().flow;
    const loginNode = flow?.nodes.find((n) => n.name === '用户登录');
    const extractorNode = flow?.nodes.find((n) => n.name === '提取Token');

    expect(loginNode).toBeDefined();
    expect(extractorNode).toBeDefined();

    expect(isVariableExtractorConfig(extractorNode!.data)).toBe(true);
    if (isVariableExtractorConfig(extractorNode!.data)) {
      expect(extractorNode!.data.source_node_id).toBe(loginNode?.uuid);
    }
  });

  it('当 variable:extractor 传入别名 id 时，正确解析为目标节点的实际 UUID', async () => {
    const result = await canvasToolHandlers.batch_create_pipeline({
      nodes: [
        {
          id: 'fetch_data',
          type: 'http:request',
          name: '获取数据',
        },
        {
          id: 'extractor_node',
          type: 'variable:extractor',
          name: '提取数据',
          config: {
            source_node_id: 'fetch_data', // 传入了别名
            output_mappings: [{ source: '$.body.data.id', target: 'DATA_ID' }],
          },
        },
      ],
      auto_run: false,
    });

    expect(result.success).toBe(true);
    const flow = useFlowStore.getState().flow;
    const fetchNode = flow?.nodes.find((n) => n.name === '获取数据');
    const extractorNode = flow?.nodes.find((n) => n.name === '提取数据');

    expect(isVariableExtractorConfig(extractorNode!.data)).toBe(true);
    if (isVariableExtractorConfig(extractorNode!.data)) {
      expect(extractorNode!.data.source_node_id).toBe(fetchNode?.uuid);
    }
  });

  it('add_node 与 update_node_config: 自动推导或解析 variable:extractor 的 source_node_id', async () => {
    // 1. 添加一个 http producer
    const httpNodeRes = await canvasToolHandlers.add_node({
      type: 'http:request',
      name: 'Producer HTTP',
    });

    // 2. 添加 variable:extractor 且未指定 source_node_id -> 自动关联到最近 producer
    const extRes1 = await canvasToolHandlers.add_node({
      type: 'variable:extractor',
      name: 'Auto Extractor',
      config: {
        output_mappings: [{ source: '$.id', target: 'ID' }],
      },
    });

    let flow = useFlowStore.getState().flow;
    let extNode1 = flow?.nodes.find((n) => n.uuid === extRes1.node_uuid);
    expect(isVariableExtractorConfig(extNode1!.data)).toBe(true);
    if (isVariableExtractorConfig(extNode1!.data)) {
      expect(extNode1!.data.source_node_id).toBe(httpNodeRes.node_uuid);
    }

    // 3. update_node_config 使用节点名称指定 source_node_id -> 自动解析为 UUID
    await canvasToolHandlers.update_node_config({
      node_uuid: extRes1.node_uuid,
      config: {
        source_node_id: 'Producer HTTP',
      },
    });

    flow = useFlowStore.getState().flow;
    extNode1 = flow?.nodes.find((n) => n.uuid === extRes1.node_uuid);
    expect(isVariableExtractorConfig(extNode1!.data)).toBe(true);
    if (isVariableExtractorConfig(extNode1!.data)) {
      expect(extNode1!.data.source_node_id).toBe(httpNodeRes.node_uuid);
    }
  });

  it('当 variable:extractor 既无 name 也无 id 时，通过 list fallback 准确绑定到前驱 producer 节点', async () => {
    const result = await canvasToolHandlers.batch_create_pipeline({
      nodes: [
        {
          type: 'http:request',
        },
        {
          type: 'variable:extractor',
          config: {
            output_mappings: [{ source: '$.token', target: 'ANON_TOKEN' }],
          },
        },
      ],
      auto_run: false,
    });

    expect(result.success).toBe(true);
    const flow = useFlowStore.getState().flow;
    const httpNode = flow?.nodes.find((n) => n.type === 'http:request');
    const extractorNode = flow?.nodes.find((n) => n.type === 'variable:extractor');

    expect(httpNode).toBeDefined();
    expect(extractorNode).toBeDefined();
    expect(isVariableExtractorConfig(extractorNode!.data)).toBe(true);
    if (isVariableExtractorConfig(extractorNode!.data)) {
      expect(extractorNode!.data.source_node_id).toBe(httpNode?.uuid);
    }
  });

  it('单流程包含多个提取节点时，各提取节点只向回搜索前驱节点，不污染绑定下游节点', async () => {
    const result = await canvasToolHandlers.batch_create_pipeline({
      nodes: [
        {
          id: 'step1_http',
          type: 'http:request',
          name: 'Step 1 HTTP',
        },
        {
          id: 'step1_ext',
          type: 'variable:extractor',
          name: 'Step 1 Extractor',
          config: {
            output_mappings: [{ source: '$.token1', target: 'T1' }],
          },
        },
        {
          id: 'step2_http',
          type: 'http:request',
          name: 'Step 2 HTTP',
        },
        {
          id: 'step2_ext',
          type: 'variable:extractor',
          name: 'Step 2 Extractor',
          config: {
            output_mappings: [{ source: '$.token2', target: 'T2' }],
          },
        },
      ],
      auto_run: false,
    });

    expect(result.success).toBe(true);
    const flow = useFlowStore.getState().flow;
    const step1Http = flow?.nodes.find((n) => n.name === 'Step 1 HTTP');
    const step1Ext = flow?.nodes.find((n) => n.name === 'Step 1 Extractor');
    const step2Http = flow?.nodes.find((n) => n.name === 'Step 2 HTTP');
    const step2Ext = flow?.nodes.find((n) => n.name === 'Step 2 Extractor');

    expect(step1Http).toBeDefined();
    expect(step1Ext).toBeDefined();
    expect(step2Http).toBeDefined();
    expect(step2Ext).toBeDefined();

    if (isVariableExtractorConfig(step1Ext!.data)) {
      // step1Ext 必须绑定 step1Http，绝不能错误绑定下游的 step2Http
      expect(step1Ext!.data.source_node_id).toBe(step1Http?.uuid);
    }
    if (isVariableExtractorConfig(step2Ext!.data)) {
      expect(step2Ext!.data.source_node_id).toBe(step2Http?.uuid);
    }
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
