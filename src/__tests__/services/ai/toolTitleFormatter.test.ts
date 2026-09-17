import { describe, it, expect, beforeEach } from 'vitest';
import { formatToolTitle, isUuid, cleanName } from '@/services/ai/toolTitleFormatter';
import { useFlowStore } from '@/stores/flow';

describe('toolTitleFormatter', () => {
  beforeEach(() => {
    useFlowStore.setState({
      flow: {
        uuid: 'flow-1',
        name: 'Test Flow',
        nodes: [
          {
            uuid: 'node-auth-123',
            name: '用户登录',
            type: 'http:request',
            position: { x: 0, y: 0 },
            data: {} as never,
          },
          {
            uuid: 'node-order-456',
            name: '创建订单',
            type: 'http:request',
            position: { x: 100, y: 0 },
            data: {} as never,
          },
          {
            uuid: '550e8400-e29b-41d4-a716-446655440000',
            name: 'node-550e8400-e29b-41d4-a716-446655440000',
            type: 'script',
            position: { x: 200, y: 0 },
            data: {} as never,
          },
        ],
        connections: [],
      },
    });
  });

  describe('isUuid & cleanName', () => {
    it('正确识别标准 UUID 与带前缀的 UUID', () => {
      expect(isUuid('550e8400-e29b-41d4-a716-446655440000')).toBe(true);
      expect(isUuid('node-550e8400-e29b-41d4-a716-446655440000')).toBe(true);
      expect(isUuid('用户登录')).toBe(false);
      expect(isUuid('HTTP 请求')).toBe(false);
    });

    it('cleanName 过滤掉纯 UUID，保留人类可读名称', () => {
      expect(cleanName('550e8400-e29b-41d4-a716-446655440000')).toBeUndefined();
      expect(cleanName('node-550e8400-e29b-41d4-a716-446655440000')).toBeUndefined();
      expect(cleanName('用户登录')).toBe('用户登录');
    });
  });

  describe('formatToolTitle', () => {
    it('正确提取 update_node_config 的目标节点名称 (工具名: 目标名)', () => {
      const info = formatToolTitle('update_node_config', { node_uuid: 'node-auth-123' });
      expect(info.rawName).toBe('update_node_config');
      expect(info.toolDisplayName).toBe('修改节点');
      expect(info.target).toBe('"用户登录"');
      expect(info.fullTitle).toBe('修改节点: "用户登录"');
    });

    it('update_node_config 没有名字时绝不展示长 UUID，仅展示类型', () => {
      const info = formatToolTitle('update_node_config', {
        node_uuid: '550e8400-e29b-41d4-a716-446655440000',
      });
      // 节点名字为 UUID，过滤后仅展示类型 script，绝不展示长 UUID 或括号
      expect(info.target).toBe('script');
      expect(info.fullTitle).toBe('修改节点: script');
      expect(info.fullTitle).not.toContain('550e8400');
    });

    it('完全未知且无类型的节点不展示 target，仅展示工具名，绝不展示长 UUID', () => {
      const info = formatToolTitle('update_node_config', {
        node_uuid: 'non-existent-uuid-12345678-abcd',
      });
      expect(info.target).toBeUndefined();
      expect(info.fullTitle).toBe('修改节点');
      expect(info.fullTitle).not.toContain('non-existent-uuid');
    });

    it('update_node_config 结果中包含有效人类可读 node_name 时使用', () => {
      const info = formatToolTitle(
        'update_node_config',
        { node_uuid: 'unknown-id' },
        JSON.stringify({ success: true, node_name: '支付校验' }),
      );
      expect(info.target).toBe('"支付校验"');
      expect(info.fullTitle).toBe('修改节点: "支付校验"');
    });

    it('正确格式化 add_node 节点名称', () => {
      const info = formatToolTitle('add_node', { name: '数据校验', node_type: 'script' });
      expect(info.rawName).toBe('add_node');
      expect(info.toolDisplayName).toBe('新增节点');
      expect(info.target).toBe('"数据校验"');
      expect(info.fullTitle).toBe('新增节点: "数据校验"');
    });

    it('正确解析 connect_nodes 的源与目标节点名称', () => {
      const info = formatToolTitle('connect_nodes', {
        source_node_uuid: 'node-auth-123',
        target_node_uuid: 'node-order-456',
      });
      expect(info.toolDisplayName).toBe('连接节点');
      expect(info.target).toBe('"用户登录" ➔ "创建订单"');
      expect(info.fullTitle).toBe('连接节点: "用户登录" ➔ "创建订单"');
    });

    it('正确格式化 browse_datasource 数据源名称与资源', () => {
      const info = formatToolTitle('browse_datasource', {
        datasource_id: 'apifox',
        resource_id: 'proj_order_center',
      });
      expect(info.toolDisplayName).toBe('查询数据源');
      expect(info.target).toBe('apifox · proj_order_center');
      expect(info.fullTitle).toBe('查询数据源: apifox · proj_order_center');
    });

    it('正确格式化 import_datasource_item_to_flow 接口导入', () => {
      const info = formatToolTitle('import_datasource_item_to_flow', {
        datasource_id: 'postman',
        name: 'GET /api/v1/users',
      });
      expect(info.toolDisplayName).toBe('导入接口');
      expect(info.target).toBe('"GET /api/v1/users"');
      expect(info.fullTitle).toBe('导入接口: "GET /api/v1/users"');
    });

    it('正确格式化 run_flow，从当前 flowStore 获取流程名称作为 target', () => {
      const info = formatToolTitle('run_flow');
      expect(info.rawName).toBe('run_flow');
      expect(info.toolDisplayName).toBe('执行流程');
      expect(info.target).toBe('"Test Flow"');
      expect(info.fullTitle).toBe('执行流程: "Test Flow"');
    });

    it('当 args 传入 flow_name 时优先使用入参名称', () => {
      const info = formatToolTitle('run_flow', { flow_name: '自定义执行流程' });
      expect(info.toolDisplayName).toBe('执行流程');
      expect(info.target).toBe('"自定义执行流程"');
      expect(info.fullTitle).toBe('执行流程: "自定义执行流程"');
    });

    it('当 store 无激活流程且未传参时，不展示 target 仅展示工具名', () => {
      useFlowStore.getState().reset();
      const info = formatToolTitle('run_flow');
      expect(info.toolDisplayName).toBe('执行流程');
      expect(info.target).toBeUndefined();
      expect(info.fullTitle).toBe('执行流程');
    });

    it('正确格式化 save_current_flow 与 cancel_flow_execution', () => {
      const saveInfo = formatToolTitle('save_current_flow', { name: '支付流程' });
      expect(saveInfo.toolDisplayName).toBe('保存流程');
      expect(saveInfo.target).toBe('"支付流程"');
      expect(saveInfo.fullTitle).toBe('保存流程: "支付流程"');

      const cancelInfo = formatToolTitle('cancel_flow_execution');
      expect(cancelInfo.toolDisplayName).toBe('终止流程');
    });

    it('正确格式化 run_terminal_command，提取具体命令作为 target', () => {
      const info = formatToolTitle('run_terminal_command', { command: 'git status' });
      expect(info.rawName).toBe('run_terminal_command');
      expect(info.toolDisplayName).toBe('终端命令');
      expect(info.target).toBe('"git status"');
      expect(info.fullTitle).toBe('终端命令: "git status"');
    });
  });
});
