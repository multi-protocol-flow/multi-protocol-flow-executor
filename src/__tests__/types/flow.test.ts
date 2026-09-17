/**
 * 类型序列化测试
 * 验证 TypeScript 类型与后端 Rust 结构体的序列化/反序列化对齐
 */

import { describe, it, expect } from 'vitest';
import type {
  Flow,
  Node,
  Connection,
  Port,
  EntryConfig,
  HttpConfig,
  ConditionalConfig,
  ErrorDetail,
  FlowExecutionReport,
  FlowStatistics,
} from '../../types';

describe('Flow 类型序列化测试', () => {
  it('应该正确序列化和反序列化完整的 Flow 对象', () => {
    const flow: Flow = {
      uuid: 'flow-001',
      name: '测试流程',
      initial_variables: {
        api_base: 'https://api.example.com',
        timeout: 5000,
      },
      nodes: [
        {
          uuid: 'node-001',
          type: 'entry',
          name: '入口节点',
          position: { x: 0, y: 0 },
          data: { type: 'entry' },
        },
        {
          uuid: 'node-002',
          type: 'http:request',
          name: 'HTTP 请求',
          position: { x: 200, y: 0 },
          data: {
            type: 'http:request',
            url: 'https://api.example.com/users',
            method: 'get',
            headers: {
              'Content-Type': 'application/json',
            },
          },
        },
      ],
      connections: [
        {
          id: 'conn-001',
          source_node_uuid: 'node-001',
          target_node_uuid: 'node-002',
          source_port_id: 'out',
          target_port_id: 'in',
        },
      ],
    };

    // 序列化
    const json = JSON.stringify(flow);
    
    // 反序列化
    const deserialized: Flow = JSON.parse(json);

    expect(deserialized.uuid).toBe('flow-001');
    expect(deserialized.name).toBe('测试流程');
    expect(deserialized.initial_variables?.api_base).toBe('https://api.example.com');
    expect(deserialized.nodes).toHaveLength(2);
    expect(deserialized.connections).toHaveLength(1);
    expect(deserialized.nodes[0].type).toBe('entry');
    expect(deserialized.nodes[1].type).toBe('http:request');
  });
});

describe('Node 类型序列化测试', () => {
  it('应该正确处理 Entry 节点', () => {
    const node: Node = {
      uuid: 'entry-001',
      type: 'entry',
      name: '入口',
      position: { x: 100, y: 100 },
      data: { type: 'entry' },
    };

    const json = JSON.stringify(node);
    const deserialized: Node = JSON.parse(json);

    expect(deserialized.uuid).toBe('entry-001');
    expect(deserialized.type).toBe('entry');
    expect((deserialized.data as EntryConfig).type).toBe('entry');
  });

  it('应该正确处理 HTTP 节点', () => {
    const node: Node = {
      uuid: 'http-001',
      type: 'http:request',
      name: '获取用户数据',
      position: { x: 200, y: 100 },
      data: {
        type: 'http:request',
        url: 'https://api.example.com/users',
        method: 'post',
        headers: {
          Authorization: 'Bearer token123',
        },
        body: JSON.stringify({ page: 1, limit: 10 }),
      },
      ports: [
        { id: 'in', type: 'input', label: '输入' },
        { id: 'out', type: 'output', label: '输出' },
      ],
    };

    const json = JSON.stringify(node);
    const deserialized: Node = JSON.parse(json);

    expect(deserialized.uuid).toBe('http-001');
    expect(deserialized.type).toBe('http:request');
    const httpData = deserialized.data as HttpConfig;
    expect(httpData.type).toBe('http:request');
    expect(httpData.url).toBe('https://api.example.com/users');
    expect(httpData.method).toBe('post');
    expect(httpData.headers?.Authorization).toBe('Bearer token123');
    expect(deserialized.ports).toHaveLength(2);
  });

  it('应该正确处理 Conditional 节点（简单条件）', () => {
    const node: Node = {
      uuid: 'cond-001',
      type: 'conditional',
      name: '条件判断',
      position: { x: 300, y: 100 },
      data: {
        type: 'conditional',
        condition_type: 'simple',
        field_path: 'http_response.status',
        operator: 'equal',
        expected_value: 200,
      },
      ports: [
        { id: 'in', type: 'input', label: '输入' },
        { id: 'true', type: 'output', label: '是' },
        { id: 'false', type: 'output', label: '否' },
      ],
    };

    const json = JSON.stringify(node);
    const deserialized: Node = JSON.parse(json);

    expect(deserialized.uuid).toBe('cond-001');
    expect(deserialized.type).toBe('conditional');
    const condData = deserialized.data as ConditionalConfig;
    expect(condData.type).toBe('conditional');
    expect(condData.condition_type).toBe('simple');
    expect(condData.field_path).toBe('http_response.status');
    expect(condData.operator).toBe('equal');
    expect(condData.expected_value).toBe(200);
    expect(deserialized.ports).toHaveLength(3);
  });

  it('应该正确处理 Conditional 节点（高级条件）', () => {
    const node: Node = {
      uuid: 'cond-002',
      type: 'conditional',
      name: '高级条件判断',
      position: { x: 400, y: 100 },
      data: {
        type: 'conditional',
        condition_type: 'advanced',
        advanced_expression: 'http_response.status >= 200 && http_response.status < 300',
      },
    };

    const json = JSON.stringify(node);
    const deserialized: Node = JSON.parse(json);

    const condData = deserialized.data as ConditionalConfig;
    expect(condData.condition_type).toBe('advanced');
    expect(condData.advanced_expression).toBe('http_response.status >= 200 && http_response.status < 300');
  });
});

describe('Connection 类型序列化测试', () => {
  it('应该正确序列化 Connection', () => {
    const connection: Connection = {
      id: 'conn-001',
      source_node_uuid: 'node-001',
      target_node_uuid: 'node-002',
      source_port_id: 'out',
      target_port_id: 'in',
    };

    const json = JSON.stringify(connection);
    const deserialized: Connection = JSON.parse(json);

    expect(deserialized.id).toBe('conn-001');
    expect(deserialized.source_node_uuid).toBe('node-001');
    expect(deserialized.target_node_uuid).toBe('node-002');
    expect(deserialized.source_port_id).toBe('out');
    expect(deserialized.target_port_id).toBe('in');
  });
});

describe('Port 类型序列化测试', () => {
  it('应该正确序列化 Port', () => {
    const port: Port = {
      id: 'port-001',
      type: 'input',
      label: '数据输入',
    };

    const json = JSON.stringify(port);
    const deserialized: Port = JSON.parse(json);

    expect(deserialized.id).toBe('port-001');
    expect(deserialized.type).toBe('input');
    expect(deserialized.label).toBe('数据输入');
  });

  it('应该正确处理可选的 label', () => {
    const port: Port = {
      id: 'port-002',
      type: 'output',
    };

    const json = JSON.stringify(port);
    const deserialized: Port = JSON.parse(json);

    expect(deserialized.id).toBe('port-002');
    expect(deserialized.type).toBe('output');
    expect(deserialized.label).toBeUndefined();
  });
});

describe('ErrorDetail 类型序列化测试', () => {
  it('应该正确序列化 ErrorDetail', () => {
    const error: ErrorDetail = {
      error_kind: 'execution',
      error_code: 'E_EXECUTION_ERROR',
      message: '执行失败',
      stack_trace: 'at line 42',
      context: { node_uuid: 'node-001' },
    };

    const json = JSON.stringify(error);
    const deserialized: ErrorDetail = JSON.parse(json);

    expect(deserialized.error_kind).toBe('execution');
    expect(deserialized.error_code).toBe('E_EXECUTION_ERROR');
    expect(deserialized.message).toBe('执行失败');
    expect(deserialized.stack_trace).toBe('at line 42');
  });
});

describe('FlowExecutionReport 类型序列化测试', () => {
  it('应该正确序列化 FlowExecutionReport', () => {
    const report: FlowExecutionReport = {
      uuid: 'report-001',
      flow_uuid: 'flow-001',
      flow_name: '测试流程',
      start_time: 1000,
      end_time: 2000,
      duration_ms: 1000,
      total_nodes: 3,
      executed_count: 3,
      status: 'success',
      node_reports: [
        {
          node_uuid: 'node-001',
          node_type: 'entry',
          node_name: '入口',
          start_time: 1000,
          end_time: 1100,
          duration_ms: 100,
          status: 'success',
          used_port_id: 'out',
        },
        {
          node_uuid: 'node-002',
          node_type: 'http:request',
          node_name: 'HTTP 请求',
          start_time: 1100,
          end_time: 1800,
          duration_ms: 700,
          status: 'success',
          input_data: { url: 'https://api.example.com' },
          output_data: { status: 200, data: [] },
          used_port_id: 'out',
        },
        {
          node_uuid: 'node-003',
          node_type: 'conditional',
          node_name: '条件判断',
          start_time: 1800,
          end_time: 2000,
          duration_ms: 200,
          status: 'success',
          used_port_id: 'true',
        },
      ],
      statistics: {
        success_count: 3,
        failed_count: 0,
        avg_node_duration_ms: 333,
        max_node_duration_ms: 700,
        min_node_duration_ms: 100,
        error_rate: 0,
        completion_rate: 1.0,
      },
    };

    const json = JSON.stringify(report);
    const deserialized: FlowExecutionReport = JSON.parse(json);

    expect(deserialized.flow_uuid).toBe('flow-001');
    expect(deserialized.status).toBe('success');
    expect(deserialized.node_reports).toHaveLength(3);
    expect(deserialized.statistics.success_count).toBe(3);
    expect(deserialized.statistics.error_rate).toBe(0);
  });

  it('应该正确处理失败的执行报告', () => {
    const report: FlowExecutionReport = {
      uuid: 'report-002',
      flow_uuid: 'flow-002',
      flow_name: '失败流程',
      start_time: 1000,
      end_time: 1500,
      duration_ms: 500,
      total_nodes: 2,
      executed_count: 1,
      status: 'failed',
      node_reports: [
        {
          node_uuid: 'node-001',
          node_type: 'http:request',
          node_name: 'HTTP 请求',
          start_time: 1000,
          end_time: 1500,
          duration_ms: 500,
          status: 'failed',
          error: {
            error_kind: 'connection',
            message: '连接超时',
          },
        },
      ],
      statistics: {
        success_count: 0,
        failed_count: 1,
        error_rate: 1.0,
        completion_rate: 0.5,
      },
      flow_error: {
        error_kind: 'execution',
        message: '流程执行失败',
      },
    };

    const json = JSON.stringify(report);
    const deserialized: FlowExecutionReport = JSON.parse(json);

    expect(deserialized.status).toBe('failed');
    expect(deserialized.node_reports[0].status).toBe('failed');
    expect(deserialized.node_reports[0].error?.error_kind).toBe('connection');
    expect(deserialized.flow_error?.error_kind).toBe('execution');
  });
});

describe('snake_case 字段名验证', () => {
  it('Connection 应该使用 snake_case 字段名', () => {
    const connection: Connection = {
      id: 'conn-001',
      source_node_uuid: 'source',
      target_node_uuid: 'target',
      source_port_id: 'out',
      target_port_id: 'in',
    };

    const json = JSON.stringify(connection);
    
    // 验证 JSON 字段名是 snake_case
    expect(json).toContain('source_node_uuid');
    expect(json).toContain('target_node_uuid');
    expect(json).toContain('source_port_id');
    expect(json).toContain('target_port_id');
    
    // 验证不是 camelCase
    expect(json).not.toContain('sourceNodeUuid');
    expect(json).not.toContain('targetNodeUuid');
  });

  it('ConditionalConfig 应该使用 snake_case 字段名', () => {
    const config: ConditionalConfig = {
      type: 'conditional',
      condition_type: 'simple',
      field_path: 'data.value',
      advanced_expression: 'data.value > 10', // 提供值而不是 undefined
    };

    const json = JSON.stringify(config);
    
    // 验证 JSON 字段名是 snake_case
    expect(json).toContain('condition_type');
    expect(json).toContain('field_path');
    expect(json).toContain('advanced_expression');
    
    // 验证不是 camelCase
    expect(json).not.toContain('conditionType');
    expect(json).not.toContain('fieldPath');
    expect(json).not.toContain('advancedExpression');
  });

  it('FlowStatistics 应该使用 snake_case 字段名', () => {
    const stats: FlowStatistics = {
      success_count: 5,
      failed_count: 2,
      avg_node_duration_ms: 150,
      max_node_duration_ms: 300,
      min_node_duration_ms: 100,
      total_input_size_bytes: 50,
      total_output_size_bytes: 40,
      error_rate: 0.28,
      completion_rate: 0.71,
    };

    const json = JSON.stringify(stats);
    
    // 验证 JSON 字段名是 snake_case
    expect(json).toContain('success_count');
    expect(json).toContain('failed_count');
    expect(json).toContain('avg_node_duration_ms');
    expect(json).toContain('max_node_duration_ms');
    expect(json).toContain('min_node_duration_ms');
    expect(json).toContain('total_input_size_bytes');
    expect(json).toContain('total_output_size_bytes');
    expect(json).toContain('error_rate');
    expect(json).toContain('completion_rate');
  });
});

describe('YAML 与双格式流程导入解析测试', () => {
  it('应该正确解析 YAML 格式的 FlowFile 与 Flow 数据', async () => {
    const YAML = await import('yaml');
    const yamlText = `
_version: 1
flow:
  uuid: flow-yaml-001
  name: YAML测试流程
  nodes:
    - uuid: node-001
      type: entry
      name: 入口
      position:
        x: 100
        y: 200
      data:
        type: entry
  connections: []
`;
    let parsed: Record<string, unknown>;
    try {
      parsed = JSON.parse(yamlText);
    } catch {
      parsed = YAML.parse(yamlText) as Record<string, unknown>;
    }

    const flowData = (parsed.flow ?? parsed) as Record<string, unknown>;
    expect(flowData.uuid).toBe('flow-yaml-001');
    expect(flowData.name).toBe('YAML测试流程');
    expect(Array.isArray(flowData.nodes)).toBe(true);
    expect((flowData.nodes as unknown[]).length).toBe(1);
  });
});
