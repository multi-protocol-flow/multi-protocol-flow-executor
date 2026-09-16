import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { SubFlowViewer } from '@/components/execution/viewers/SubFlowViewer';
import type { NodeExecutionReport } from '@/types';

describe('SubFlowViewer', () => {
  it('正确渲染子流程执行概览与子节点树', () => {
    const report: NodeExecutionReport = {
      node_uuid: 'subflow-node-uuid',
      node_type: 'sub_flow',
      node_name: '嵌套认证流程',
      start_time: 1000,
      end_time: 1250,
      duration_ms: 250,
      status: 'success',
      sub_flow_report: {
        uuid: 'sub-report-uuid',
        flow_uuid: 'child-flow-uuid',
        flow_name: 'Auth SubFlow',
        start_time: 1000,
        end_time: 1250,
        duration_ms: 250,
        total_nodes: 2,
        executed_count: 2,
        status: 'success',
        node_reports: [
          {
            node_uuid: 'sub-node-1',
            node_type: 'entry',
            node_name: 'Sub Entry',
            start_time: 1000,
            end_time: 1050,
            duration_ms: 50,
            status: 'success',
          },
          {
            node_uuid: 'sub-node-2',
            node_type: 'script',
            node_name: 'Generate Token',
            start_time: 1050,
            end_time: 1250,
            duration_ms: 200,
            status: 'success',
          },
        ],
        statistics: {
          success_count: 2,
          failed_count: 0,
        },
        final_variables: {
          auth_token: 'jwt-xyz-123',
        },
        execution_path: ['sub-node-1', 'sub-node-2'],
        skipped_nodes: [],
        ws_streams: [],
      },
    };

    render(<SubFlowViewer report={report} />);

    // 检查子流程名称展示
    expect(screen.getByText('Auth SubFlow')).toBeInTheDocument();
    // 检查子流程节点列表渲染
    expect(screen.getAllByText('Sub Entry').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Generate Token').length).toBeGreaterThanOrEqual(1);
  });

  it('结构化展示调用配置与执行统计', () => {
    const report: NodeExecutionReport = {
      node_uuid: 'subflow-node-uuid',
      node_type: 'sub_flow',
      node_name: '嵌套认证流程',
      start_time: 1000,
      end_time: 1250,
      duration_ms: 250,
      status: 'success',
      input_data: {
        target_flow_path: './subflows/auth.json',
        inherit_all_variables: true,
        export_all_variables: false,
        variable_mappings: [{ source: 'parent_token', target: 'child_token' }],
        output_mappings: [],
      },
      sub_flow_report: {
        uuid: 'sub-report-uuid',
        flow_uuid: 'child-flow-uuid',
        flow_name: 'Auth SubFlow',
        start_time: 1000,
        end_time: 1250,
        duration_ms: 250,
        total_nodes: 4,
        executed_count: 3,
        status: 'failed',
        node_reports: [],
        statistics: { success_count: 3, failed_count: 1 },
        skipped_nodes: [
          { node_uuid: 'sub-node-9', node_name: 'Skipped Node', reason: '分支未命中' },
        ],
      },
    };

    render(<SubFlowViewer report={report} />);

    // 调用配置：目标引用与变量映射（默认展开）
    expect(screen.getByText('调用配置')).toBeInTheDocument();
    expect(screen.getByText('./subflows/auth.json')).toBeInTheDocument();
    expect(screen.getByText('parent_token')).toBeInTheDocument();
    expect(screen.getByText('child_token')).toBeInTheDocument();

    // 执行统计与失败态
    expect(screen.getByText('子流程节点数')).toBeInTheDocument();
    expect(screen.getByText('Failed')).toBeInTheDocument();

    // 跳过节点区块按数量展示
    expect(screen.getByText('跳过节点 (1)')).toBeInTheDocument();
  });

  it('嵌套子流程可继续下钻（任意层级）', () => {
    const report: NodeExecutionReport = {
      node_uuid: 'subflow-node-uuid',
      node_type: 'sub_flow',
      node_name: '外层子流程',
      start_time: 1000,
      end_time: 1250,
      duration_ms: 250,
      status: 'success',
      sub_flow_report: {
        uuid: 'sub-report-uuid',
        flow_uuid: 'child-flow-uuid',
        flow_name: 'Auth SubFlow',
        start_time: 1000,
        end_time: 1250,
        duration_ms: 250,
        total_nodes: 1,
        executed_count: 1,
        status: 'success',
        // 子节点自身也是子流程调用，其报告树内还有更深一层节点
        node_reports: [
          {
            node_uuid: 'inner-subflow',
            node_type: 'sub_flow',
            node_name: 'Nested Call',
            start_time: 1000,
            end_time: 1200,
            duration_ms: 200,
            status: 'success',
            sub_flow_report: {
              uuid: 'inner-report-uuid',
              flow_uuid: 'grandchild-flow-uuid',
              flow_name: 'Deep SubFlow',
              start_time: 1000,
              end_time: 1200,
              duration_ms: 200,
              total_nodes: 1,
              executed_count: 1,
              status: 'success',
              node_reports: [
                {
                  node_uuid: 'deep-node',
                  node_type: 'http',
                  node_name: 'Deep Node',
                  start_time: 1000,
                  end_time: 1100,
                  duration_ms: 100,
                  status: 'success',
                },
              ],
              statistics: { success_count: 1, failed_count: 0 },
            },
          },
        ],
        statistics: { success_count: 1, failed_count: 0 },
      },
    };

    render(<SubFlowViewer report={report} />);

    // 子节点采用平铺详情展示：嵌套层级直接渲染呈现
    expect(screen.getByText('Deep SubFlow')).toBeInTheDocument();
    expect(screen.getByText('Deep Node')).toBeInTheDocument();
  });
});
