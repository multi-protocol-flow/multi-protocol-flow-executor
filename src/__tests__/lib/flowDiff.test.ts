import { describe, it, expect } from 'vitest';
import { diffFlows } from '@/lib/flowDiff';
import type { Flow, Node, Connection, NodeData } from '@/types';

// ============================================================================
// 测试辅助函数
// ============================================================================

function createTestNode(uuid: string, type: string, name: string, data: NodeData): Node {
  return { uuid, type: type as Node['type'], name, position: { x: 0, y: 0 }, data };
}

function createTestConnection(id: string, source: string, target: string): Connection {
  return {
    id,
    source_node_uuid: source,
    target_node_uuid: target,
    source_port_id: 'output',
    target_port_id: 'input',
  };
}

function createTestFlow(overrides?: Partial<Flow>): Flow {
  return {
    uuid: 'flow-001',
    name: '测试流程',
    nodes: [],
    connections: [],
    ...overrides,
  };
}

// ============================================================================
// 测试
// ============================================================================

describe('diffFlows', () => {
  it('完全相同的流程应全部归类为 unchanged', () => {
    const node = createTestNode('n1', 'http', '请求', {
      type: 'http',
      url: 'https://api.example.com',
      method: 'get',
    });
    const conn = createTestConnection('c1', 'n1', 'n2');
    const flow = createTestFlow({ nodes: [node], connections: [conn] });

    const result = diffFlows(flow, flow);

    expect(result.addedNodes).toHaveLength(0);
    expect(result.removedNodes).toHaveLength(0);
    expect(result.modifiedNodes).toHaveLength(0);
    expect(result.unchangedNodes).toHaveLength(1);
    expect(result.unchangedNodes[0].uuid).toBe('n1');
    expect(result.addedConnections).toHaveLength(0);
    expect(result.removedConnections).toHaveLength(0);
    expect(result.unchangedConnections).toHaveLength(1);
    expect(result.flowChanges).toEqual({});
  });

  it('新增节点应归类为 addedNodes', () => {
    const n1 = createTestNode('n1', 'entry', '入口', { type: 'entry' });
    const n2 = createTestNode('n2', 'http', '请求', {
      type: 'http',
      url: 'https://api.example.com',
    });

    const snapshot = createTestFlow({ nodes: [n1] });
    const current = createTestFlow({ nodes: [n1, n2] });

    const result = diffFlows(snapshot, current);

    expect(result.addedNodes).toHaveLength(1);
    expect(result.addedNodes[0].uuid).toBe('n2');
    expect(result.removedNodes).toHaveLength(0);
    expect(result.unchangedNodes).toHaveLength(1);
  });

  it('删除节点应归类为 removedNodes', () => {
    const n1 = createTestNode('n1', 'entry', '入口', { type: 'entry' });
    const n2 = createTestNode('n2', 'http', '请求', {
      type: 'http',
      url: 'https://api.example.com',
    });

    const snapshot = createTestFlow({ nodes: [n1, n2] });
    const current = createTestFlow({ nodes: [n1] });

    const result = diffFlows(snapshot, current);

    expect(result.removedNodes).toHaveLength(1);
    expect(result.removedNodes[0].uuid).toBe('n2');
    expect(result.addedNodes).toHaveLength(0);
    expect(result.unchangedNodes).toHaveLength(1);
  });

  it('修改节点配置应归类为 modifiedNodes 并包含 dataChanges', () => {
    const n1 = createTestNode('n1', 'entry', '入口', { type: 'entry' });
    const n2Old = createTestNode('n2', 'http', '请求', {
      type: 'http',
      url: 'https://old.example.com',
      method: 'get',
    });
    const n2New = createTestNode('n2', 'http', '请求', {
      type: 'http',
      url: 'https://new.example.com',
      method: 'post',
    });

    const snapshot = createTestFlow({ nodes: [n1, n2Old] });
    const current = createTestFlow({ nodes: [n1, n2New] });

    const result = diffFlows(snapshot, current);

    expect(result.modifiedNodes).toHaveLength(1);
    expect(result.modifiedNodes[0].uuid).toBe('n2');
    expect(result.modifiedNodes[0].dataChanges).toBeDefined();
    expect(result.modifiedNodes[0].dataChanges!.fieldChanges).toHaveLength(2);

    const urlChange = result.modifiedNodes[0].dataChanges!.fieldChanges.find(
      (fc) => fc.field === 'url',
    );
    expect(urlChange?.oldValue).toBe('https://old.example.com');
    expect(urlChange?.newValue).toBe('https://new.example.com');

    const methodChange = result.modifiedNodes[0].dataChanges!.fieldChanges.find(
      (fc) => fc.field === 'method',
    );
    expect(methodChange?.oldValue).toBe('get');
    expect(methodChange?.newValue).toBe('post');

    expect(result.unchangedNodes).toHaveLength(1);
    expect(result.unchangedNodes[0].uuid).toBe('n1');
  });

  it('新增/删除连接应正确归类', () => {
    const n1 = createTestNode('n1', 'entry', '入口', { type: 'entry' });
    const n2 = createTestNode('n2', 'http', '请求', {
      type: 'http',
      url: 'https://api.example.com',
    });
    const n3 = createTestNode('n3', 'script', '脚本', { type: 'script', script: 'fn.log("hi")' });

    const c1 = createTestConnection('c1', 'n1', 'n2');
    const c2 = createTestConnection('c2', 'n2', 'n3');

    const snapshot = createTestFlow({ nodes: [n1, n2, n3], connections: [c1, c2] });
    const current = createTestFlow({ nodes: [n1, n2, n3], connections: [c1] });

    const result = diffFlows(snapshot, current);

    expect(result.removedConnections).toHaveLength(1);
    expect(result.removedConnections[0].id).toBe('c2');
    expect(result.unchangedConnections).toHaveLength(1);
    expect(result.unchangedConnections[0].id).toBe('c1');
    expect(result.addedConnections).toHaveLength(0);
  });

  it('流程名称变化应反映在 flowChanges.name', () => {
    const snapshot = createTestFlow({ name: '流程A' });
    const current = createTestFlow({ name: '流程B' });

    const result = diffFlows(snapshot, current);

    expect(result.flowChanges.name).toBeDefined();
    expect(result.flowChanges.name!.oldValue).toBe('流程A');
    expect(result.flowChanges.name!.newValue).toBe('流程B');
  });

  it('initial_variables 变化应反映 added/removed/modified', () => {
    const snapshot = createTestFlow({
      initial_variables: { a: 1, c: 'old' },
    });
    const current = createTestFlow({
      initial_variables: { a: 2, b: 3, c: 'old' },
    });

    const result = diffFlows(snapshot, current);

    expect(result.flowChanges.initialVariables).toBeDefined();
    expect(result.flowChanges.initialVariables!.added).toEqual(['b']);
    expect(result.flowChanges.initialVariables!.removed).toEqual([]);
    expect(result.flowChanges.initialVariables!.modified).toHaveLength(1);
    expect(result.flowChanges.initialVariables!.modified[0].field).toBe('a');
    expect(result.flowChanges.initialVariables!.modified[0].oldValue).toBe(1);
    expect(result.flowChanges.initialVariables!.modified[0].newValue).toBe(2);
  });

  it('节点名称变化应生成 nameChange', () => {
    const n1Old = createTestNode('n1', 'http', '旧名称', {
      type: 'http',
      url: 'https://api.example.com',
    });
    const n1New = createTestNode('n1', 'http', '新名称', {
      type: 'http',
      url: 'https://api.example.com',
    });

    const snapshot = createTestFlow({ nodes: [n1Old] });
    const current = createTestFlow({ nodes: [n1New] });

    const result = diffFlows(snapshot, current);

    expect(result.modifiedNodes).toHaveLength(1);
    expect(result.modifiedNodes[0].nameChange).toBeDefined();
    expect(result.modifiedNodes[0].nameChange!.oldValue).toBe('旧名称');
    expect(result.modifiedNodes[0].nameChange!.newValue).toBe('新名称');
    expect(result.modifiedNodes[0].dataChanges).toBeUndefined();
  });

  it('空流程对比应返回全部为空', () => {
    const empty = createTestFlow();
    const result = diffFlows(empty, empty);

    expect(result.addedNodes).toHaveLength(0);
    expect(result.removedNodes).toHaveLength(0);
    expect(result.modifiedNodes).toHaveLength(0);
    expect(result.unchangedNodes).toHaveLength(0);
    expect(result.addedConnections).toHaveLength(0);
    expect(result.removedConnections).toHaveLength(0);
    expect(result.unchangedConnections).toHaveLength(0);
    expect(result.flowChanges).toEqual({});
  });

  it('initial_variables 删除变量应反映在 removed', () => {
    const snapshot = createTestFlow({
      initial_variables: { x: 10, y: 20 },
    });
    const current = createTestFlow({
      initial_variables: { y: 20 },
    });

    const result = diffFlows(snapshot, current);

    expect(result.flowChanges.initialVariables!.removed).toEqual(['x']);
    expect(result.flowChanges.initialVariables!.added).toEqual([]);
    expect(result.flowChanges.initialVariables!.modified).toEqual([]);
  });

  it('节点类型改变应标记所有配置字段为变化', () => {
    const nOld = createTestNode('n1', 'http', '节点', {
      type: 'http',
      url: 'https://api.example.com',
    });
    const nNew = createTestNode('n1', 'script', '节点', {
      type: 'script',
      script: 'fn.log("hello")',
    });

    const snapshot = createTestFlow({ nodes: [nOld] });
    const current = createTestFlow({ nodes: [nNew] });

    const result = diffFlows(snapshot, current);

    expect(result.modifiedNodes).toHaveLength(1);
    expect(result.modifiedNodes[0].dataChanges).toBeDefined();
    expect(result.modifiedNodes[0].dataChanges!.nodeType).toBe('http → script');
  });

  // ==========================================================================
  // 各 NodeData 变体的配置 diff 测试
  // ==========================================================================

  it('HttpConfig 字段变化应被检测', () => {
    const nOld = createTestNode('n1', 'http', 'HTTP', {
      type: 'http',
      url: 'https://old.com',
      method: 'get',
      headers: { 'X-Key': 'old' },
      timeout_ms: 5000,
    });
    const nNew = createTestNode('n1', 'http', 'HTTP', {
      type: 'http',
      url: 'https://new.com',
      method: 'post',
      headers: { 'X-Key': 'new' },
      timeout_ms: 10000,
    });

    const result = diffFlows(createTestFlow({ nodes: [nOld] }), createTestFlow({ nodes: [nNew] }));

    expect(result.modifiedNodes).toHaveLength(1);
    const changes = result.modifiedNodes[0].dataChanges!.fieldChanges;
    const changedFields = changes.map((c) => c.field);

    expect(changedFields).toContain('url');
    expect(changedFields).toContain('method');
    expect(changedFields).toContain('headers');
    expect(changedFields).toContain('timeout_ms');

    const urlChange = changes.find((c) => c.field === 'url')!;
    expect(urlChange.oldValue).toBe('https://old.com');
    expect(urlChange.newValue).toBe('https://new.com');

    const methodChange = changes.find((c) => c.field === 'method')!;
    expect(methodChange.oldValue).toBe('get');
    expect(methodChange.newValue).toBe('post');

    const headerChange = changes.find((c) => c.field === 'headers')!;
    expect(headerChange.oldValue).toEqual({ 'X-Key': 'old' });
    expect(headerChange.newValue).toEqual({ 'X-Key': 'new' });

    const timeoutChange = changes.find((c) => c.field === 'timeout_ms')!;
    expect(timeoutChange.oldValue).toBe(5000);
    expect(timeoutChange.newValue).toBe(10000);
  });

  it('ConditionalConfig 字段变化应被检测', () => {
    const nOld = createTestNode('n1', 'conditional', '条件', {
      type: 'conditional',
      field_path: '{{status}}',
      operator: 'equal',
      expected_value: 200,
    });
    const nNew = createTestNode('n1', 'conditional', '条件', {
      type: 'conditional',
      field_path: '{{body.code}}',
      operator: 'not_equal',
      expected_value: 500,
    });

    const result = diffFlows(createTestFlow({ nodes: [nOld] }), createTestFlow({ nodes: [nNew] }));

    expect(result.modifiedNodes).toHaveLength(1);
    const changes = result.modifiedNodes[0].dataChanges!.fieldChanges;
    const changedFields = changes.map((c) => c.field);

    expect(changedFields).toContain('field_path');
    expect(changedFields).toContain('operator');
    expect(changedFields).toContain('expected_value');
    expect(changedFields).toHaveLength(3);
  });

  it('ScriptNodeConfig 脚本内容和超时变化应被检测', () => {
    const nOld = createTestNode('n1', 'script', '脚本', {
      type: 'script',
      script: 'fn.log("old")',
      timeout_ms: 5000,
    });
    const nNew = createTestNode('n1', 'script', '脚本', {
      type: 'script',
      script: 'fn.log("new")',
      timeout_ms: 10000,
    });

    const result = diffFlows(createTestFlow({ nodes: [nOld] }), createTestFlow({ nodes: [nNew] }));

    expect(result.modifiedNodes).toHaveLength(1);
    const changes = result.modifiedNodes[0].dataChanges!.fieldChanges;

    const scriptChange = changes.find((c) => c.field === 'script')!;
    expect(scriptChange.oldValue).toBe('fn.log("old")');
    expect(scriptChange.newValue).toBe('fn.log("new")');

    const timeoutChange = changes.find((c) => c.field === 'timeout_ms')!;
    expect(timeoutChange.oldValue).toBe(5000);
    expect(timeoutChange.newValue).toBe(10000);
  });

  it('AssertionNodeConfig 断言列表、模式和失败行为变化应被检测', () => {
    const nOld = createTestNode('n1', 'assertion', '断言', {
      type: 'assertion',
      assertions: [{ id: 'a1', target: '{{status}}', operator: 'eq', expected: 200 }],
      mode: 'all',
      on_failure: 'route_false',
      continue_on_failure: false,
    });
    const nNew = createTestNode('n1', 'assertion', '断言', {
      type: 'assertion',
      assertions: [{ id: 'a1', target: '{{status}}', operator: 'eq', expected: 404 }],
      mode: 'any',
      on_failure: 'throw',
      continue_on_failure: true,
    });

    const result = diffFlows(createTestFlow({ nodes: [nOld] }), createTestFlow({ nodes: [nNew] }));

    expect(result.modifiedNodes).toHaveLength(1);
    const changes = result.modifiedNodes[0].dataChanges!.fieldChanges;
    const changedFields = changes.map((c) => c.field);

    expect(changedFields).toContain('assertions');
    expect(changedFields).toContain('mode');
    expect(changedFields).toContain('on_failure');
    expect(changedFields).toContain('continue_on_failure');
  });

  it('WsConnectConfig url 和超时变化应被检测', () => {
    const nOld = createTestNode('n1', 'ws_connect', 'WS连接', {
      type: 'ws_connect',
      url: 'wss://old.example.com/ws',
      connect_timeout_ms: 3000,
    });
    const nNew = createTestNode('n1', 'ws_connect', 'WS连接', {
      type: 'ws_connect',
      url: 'wss://new.example.com/ws',
      connect_timeout_ms: 5000,
    });

    const result = diffFlows(createTestFlow({ nodes: [nOld] }), createTestFlow({ nodes: [nNew] }));

    expect(result.modifiedNodes).toHaveLength(1);
    const changes = result.modifiedNodes[0].dataChanges!.fieldChanges;
    const changedFields = changes.map((c) => c.field);

    expect(changedFields).toContain('url');
    expect(changedFields).toContain('connect_timeout_ms');
  });

  it('WsSendCollectConfig 消息和连接 ID 变化应被检测', () => {
    const nOld = createTestNode('n1', 'ws_send_collect', 'WS发送', {
      type: 'ws_send_collect',
      connection_id: 'conn-1',
      message: '{"action":"ping"}',
      collect_response: true,
    });
    const nNew = createTestNode('n1', 'ws_send_collect', 'WS发送', {
      type: 'ws_send_collect',
      connection_id: 'conn-2',
      message: '{"action":"pong"}',
      collect_response: true,
    });

    const result = diffFlows(createTestFlow({ nodes: [nOld] }), createTestFlow({ nodes: [nNew] }));

    expect(result.modifiedNodes).toHaveLength(1);
    const changes = result.modifiedNodes[0].dataChanges!.fieldChanges;
    const changedFields = changes.map((c) => c.field);

    expect(changedFields).toContain('connection_id');
    expect(changedFields).toContain('message');

    const connChange = changes.find((c) => c.field === 'connection_id')!;
    expect(connChange.oldValue).toBe('conn-1');
    expect(connChange.newValue).toBe('conn-2');
  });

  it('WsCloseConfig connection_id 和 close_code 变化应被检测', () => {
    const nOld = createTestNode('n1', 'ws_close', 'WS关闭', {
      type: 'ws_close',
      connection_id: 'conn-1',
      close_code: 1000,
    });
    const nNew = createTestNode('n1', 'ws_close', 'WS关闭', {
      type: 'ws_close',
      connection_id: 'conn-2',
      close_code: 1001,
    });

    const result = diffFlows(createTestFlow({ nodes: [nOld] }), createTestFlow({ nodes: [nNew] }));

    expect(result.modifiedNodes).toHaveLength(1);
    const changes = result.modifiedNodes[0].dataChanges!.fieldChanges;
    const changedFields = changes.map((c) => c.field);

    expect(changedFields).toContain('connection_id');
    expect(changedFields).toContain('close_code');
  });

  it('DbConnectConfig host/port/database/driver 变化应被检测', () => {
    const nOld = createTestNode('n1', 'db_connect', 'DB连接', {
      type: 'db_connect',
      driver: 'postgres',
      host: 'localhost',
      port: 5432,
      database: 'olddb',
    });
    const nNew = createTestNode('n1', 'db_connect', 'DB连接', {
      type: 'db_connect',
      driver: 'mysql',
      host: 'remotehost',
      port: 3306,
      database: 'newdb',
    });

    const result = diffFlows(createTestFlow({ nodes: [nOld] }), createTestFlow({ nodes: [nNew] }));

    expect(result.modifiedNodes).toHaveLength(1);
    const changes = result.modifiedNodes[0].dataChanges!.fieldChanges;
    const changedFields = changes.map((c) => c.field);

    expect(changedFields).toContain('driver');
    expect(changedFields).toContain('host');
    expect(changedFields).toContain('port');
    expect(changedFields).toContain('database');
  });

  it('DbQueryConfig sql/connection_uuid/fetch_mode 变化应被检测', () => {
    const nOld = createTestNode('n1', 'db_query', 'DB查询', {
      type: 'db_query',
      connection_uuid: 'conn-uuid-1',
      sql: 'SELECT * FROM users',
      fetch_mode: 'all',
    });
    const nNew = createTestNode('n1', 'db_query', 'DB查询', {
      type: 'db_query',
      connection_uuid: 'conn-uuid-2',
      sql: 'SELECT * FROM orders',
      fetch_mode: 'one',
    });

    const result = diffFlows(createTestFlow({ nodes: [nOld] }), createTestFlow({ nodes: [nNew] }));

    expect(result.modifiedNodes).toHaveLength(1);
    const changes = result.modifiedNodes[0].dataChanges!.fieldChanges;
    const changedFields = changes.map((c) => c.field);

    expect(changedFields).toContain('connection_uuid');
    expect(changedFields).toContain('sql');
    expect(changedFields).toContain('fetch_mode');
  });

  it('DbBeginConfig connection_uuid 和 isolation_level 变化应被检测', () => {
    const nOld = createTestNode('n1', 'db_begin', '事务开始', {
      type: 'db_begin',
      connection_uuid: 'conn-1',
      isolation_level: 'read_committed',
    });
    const nNew = createTestNode('n1', 'db_begin', '事务开始', {
      type: 'db_begin',
      connection_uuid: 'conn-1',
      isolation_level: 'serializable',
    });

    const result = diffFlows(createTestFlow({ nodes: [nOld] }), createTestFlow({ nodes: [nNew] }));

    expect(result.modifiedNodes).toHaveLength(1);
    const changes = result.modifiedNodes[0].dataChanges!.fieldChanges;

    const isoChange = changes.find((c) => c.field === 'isolation_level')!;
    expect(isoChange.oldValue).toBe('read_committed');
    expect(isoChange.newValue).toBe('serializable');
  });

  it('DbCommitConfig/DbRollbackConfig/DbCloseConfig connection_uuid 变化应被检测', () => {
    // DbCommitConfig
    const commitOld = createTestNode('n1', 'db_commit', '提交', { type: 'db_commit', connection_uuid: 'conn-1' });
    const commitNew = createTestNode('n1', 'db_commit', '提交', { type: 'db_commit', connection_uuid: 'conn-2' });
    const commitResult = diffFlows(createTestFlow({ nodes: [commitOld] }), createTestFlow({ nodes: [commitNew] }));
    expect(commitResult.modifiedNodes).toHaveLength(1);
    expect(commitResult.modifiedNodes[0].dataChanges!.fieldChanges.map((c) => c.field)).toContain('connection_uuid');

    // DbRollbackConfig
    const rbOld = createTestNode('n2', 'db_rollback', '回滚', { type: 'db_rollback', connection_uuid: 'conn-1' });
    const rbNew = createTestNode('n2', 'db_rollback', '回滚', { type: 'db_rollback', connection_uuid: 'conn-2' });
    const rbResult = diffFlows(createTestFlow({ nodes: [rbOld] }), createTestFlow({ nodes: [rbNew] }));
    expect(rbResult.modifiedNodes).toHaveLength(1);
    expect(rbResult.modifiedNodes[0].dataChanges!.fieldChanges.map((c) => c.field)).toContain('connection_uuid');

    // DbCloseConfig
    const closeOld = createTestNode('n3', 'db_close', '关闭', { type: 'db_close', connection_uuid: 'conn-1' });
    const closeNew = createTestNode('n3', 'db_close', '关闭', { type: 'db_close', connection_uuid: 'conn-2' });
    const closeResult = diffFlows(createTestFlow({ nodes: [closeOld] }), createTestFlow({ nodes: [closeNew] }));
    expect(closeResult.modifiedNodes).toHaveLength(1);
    expect(closeResult.modifiedNodes[0].dataChanges!.fieldChanges.map((c) => c.field)).toContain('connection_uuid');
  });

  it('空流程 vs 含节点流程应全部归类为 addedNodes', () => {
    const empty = createTestFlow();
    const n1 = createTestNode('n1', 'entry', '入口', { type: 'entry' });
    const n2 = createTestNode('n2', 'http', '请求', { type: 'http', url: 'https://api.example.com' });
    const flow = createTestFlow({ nodes: [n1, n2] });

    const result = diffFlows(empty, flow);

    expect(result.addedNodes).toHaveLength(2);
    expect(result.addedNodes.map((n) => n.uuid)).toEqual(['n1', 'n2']);
    expect(result.removedNodes).toHaveLength(0);
    expect(result.modifiedNodes).toHaveLength(0);
    expect(result.unchangedNodes).toHaveLength(0);
  });

  it('仅入口节点的流程与自身对比应全部 unchanged', () => {
    const n1 = createTestNode('n1', 'entry', '入口', { type: 'entry' });
    const flow = createTestFlow({ nodes: [n1] });

    const result = diffFlows(flow, flow);

    expect(result.unchangedNodes).toHaveLength(1);
    expect(result.unchangedNodes[0].nodeType).toBe('entry');
    expect(result.addedNodes).toHaveLength(0);
    expect(result.removedNodes).toHaveLength(0);
    expect(result.modifiedNodes).toHaveLength(0);
  });

  it('Unicode 节点名称应正确对比', () => {
    const n1 = createTestNode('n1', 'http', '日本語テスト 🚀', {
      type: 'http',
      url: 'https://example.com',
    });
    const n2 = createTestNode('n1', 'http', '中文测试 🎯', {
      type: 'http',
      url: 'https://example.com',
    });
    const result = diffFlows(
      createTestFlow({ nodes: [n1] }),
      createTestFlow({ nodes: [n2] }),
    );

    expect(result.modifiedNodes).toHaveLength(1);
    expect(result.modifiedNodes[0].nameChange?.oldValue).toBe('日本語テスト 🚀');
    expect(result.modifiedNodes[0].nameChange?.newValue).toBe('中文测试 🎯');
    expect(result.modifiedNodes[0].dataChanges).toBeUndefined();
  });
});
