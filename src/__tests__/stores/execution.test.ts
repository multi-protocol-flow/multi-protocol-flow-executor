/**
 * Execution Store 单元测试
 */
import { describe, it, expect, beforeEach } from 'vitest';
import {
  useExecutionStore,
  selectIsExecuting,
  selectReport,
  selectError,
  selectHistory,
  selectExecutionStatus,
  selectNodeReportsMap,
  selectCompletedNodeCount,
  selectTotalNodes,
} from '@/stores/execution';
import type { FlowExecutionReport, NodeExecutionReport } from '@/types';
import type { ExecutionEvent } from '@/types/execution_event';

/**
 * 创建模拟的流程执行报告
 */
function createMockReport(overrides: Partial<FlowExecutionReport> = {}): FlowExecutionReport {
  return {
    uuid: 'report-1',
    flow_uuid: 'flow-1',
    flow_name: '测试流程',
    start_time: 1000,
    end_time: 2000,
    duration_ms: 1000,
    total_nodes: 2,
    executed_count: 2,
    status: 'success',
    node_reports: [],
    statistics: {
      success_count: 2,
      failed_count: 0,
    },
    ...overrides,
  };
}

/**
 * 创建模拟的节点执行报告
 */
function createMockNodeReport(
  nodeUuid: string,
  overrides: Partial<NodeExecutionReport> = {}
): NodeExecutionReport {
  return {
    node_uuid: nodeUuid,
    node_type: 'http',
    node_name: 'HTTP请求',
    start_time: 1000,
    end_time: 1500,
    duration_ms: 500,
    status: 'success',
    ...overrides,
  };
}

describe('ExecutionStore', () => {
  beforeEach(() => {
    useExecutionStore.getState().reset();
  });

  describe('初始状态', () => {
    it('初始状态正确', () => {
      const state = useExecutionStore.getState();

      expect(state.isExecuting).toBe(false);
      expect(state.report).toBeNull();
      expect(state.error).toBeNull();
      expect(state.history).toEqual([]);
      expect(state.nodeReportsMap).toEqual({});
      expect(state.currentNodeUuid).toBeNull();
      expect(state.completedNodeCount).toBe(0);
      expect(state.totalNodes).toBe(0);
      expect(state.isReportDialogOpen).toBe(false);
      expect(state.selectedHistoryReport).toBeNull();
    });
  });

  describe('startExecution', () => {
    it('开始执行时设置 isExecuting 为 true', () => {
      useExecutionStore.getState().startExecution();

      expect(useExecutionStore.getState().isExecuting).toBe(true);
    });

    it('开始执行时清除之前的报告', () => {
      const report = createMockReport();
      useExecutionStore.getState().setReport(report);
      expect(useExecutionStore.getState().report).not.toBeNull();

      useExecutionStore.getState().startExecution();

      expect(useExecutionStore.getState().report).toBeNull();
    });

    it('开始执行时清除之前的错误', () => {
      useExecutionStore.getState().setError('之前的错误');
      expect(useExecutionStore.getState().error).not.toBeNull();

      useExecutionStore.getState().startExecution();

      expect(useExecutionStore.getState().error).toBeNull();
    });
  });

  describe('setReport', () => {
    it('设置报告后 isExecuting 变为 false', () => {
      useExecutionStore.getState().startExecution();
      expect(useExecutionStore.getState().isExecuting).toBe(true);

      const report = createMockReport();
      useExecutionStore.getState().setReport(report);

      expect(useExecutionStore.getState().isExecuting).toBe(false);
    });

    it('可以设置报告', () => {
      const report = createMockReport();
      useExecutionStore.getState().setReport(report);

      expect(useExecutionStore.getState().report).toEqual(report);
    });

    it('设置报告时添加到历史记录', () => {
      const report = createMockReport();
      useExecutionStore.getState().setReport(report);

      expect(useExecutionStore.getState().history).toHaveLength(1);
      expect(useExecutionStore.getState().history[0]).toEqual(report);
    });

    it('多次设置报告按时间倒序存储（最新在前）', () => {
      const report1 = createMockReport({ uuid: 'report-1' });
      const report2 = createMockReport({ uuid: 'report-2' });
      const report3 = createMockReport({ uuid: 'report-3' });

      useExecutionStore.getState().setReport(report1);
      useExecutionStore.getState().setReport(report2);
      useExecutionStore.getState().setReport(report3);

      const history = useExecutionStore.getState().history;
      expect(history).toHaveLength(3);
      expect(history[0].uuid).toBe('report-3');
      expect(history[1].uuid).toBe('report-2');
      expect(history[2].uuid).toBe('report-1');
    });

    it('历史记录最多保留 10 条', () => {
      // 设置 12 条报告
      for (let i = 1; i <= 12; i++) {
        const report = createMockReport({ uuid: `report-${i}` });
        useExecutionStore.getState().setReport(report);
      }

      const history = useExecutionStore.getState().history;
      expect(history).toHaveLength(10);
      // 最新的报告在前
      expect(history[0].uuid).toBe('report-12');
      // 最旧的报告是第 3 条（1 和 2 被丢弃）
      expect(history[9].uuid).toBe('report-3');
    });

    it('设置 null 报告时不添加到历史', () => {
      const report = createMockReport();
      useExecutionStore.getState().setReport(report);
      useExecutionStore.getState().setReport(null);

      expect(useExecutionStore.getState().report).toBeNull();
      expect(useExecutionStore.getState().history).toHaveLength(1);
    });
  });

  describe('setError', () => {
    it('设置错误后 isExecuting 变为 false', () => {
      useExecutionStore.getState().startExecution();
      expect(useExecutionStore.getState().isExecuting).toBe(true);

      useExecutionStore.getState().setError('执行失败');

      expect(useExecutionStore.getState().isExecuting).toBe(false);
    });

    it('可以设置错误信息', () => {
      useExecutionStore.getState().setError('网络超时');

      expect(useExecutionStore.getState().error).toBe('网络超时');
    });

    it('可以清除错误（设置 null）', () => {
      useExecutionStore.getState().setError('错误');
      useExecutionStore.getState().setError(null);

      expect(useExecutionStore.getState().error).toBeNull();
    });
  });

  describe('reset', () => {
    it('重置后恢复初始状态', () => {
      // 设置各种状态
      useExecutionStore.getState().startExecution();
      const report = createMockReport();
      useExecutionStore.getState().setReport(report);
      useExecutionStore.getState().setError('测试错误');
      useExecutionStore.getState().handleExecutionEvent({
        flowStarted: { flowUuid: 'f1', flowName: 'Test', totalNodes: 3 },
      });

      // 重置
      useExecutionStore.getState().reset();

      const state = useExecutionStore.getState();
      expect(state.isExecuting).toBe(false);
      expect(state.report).toBeNull();
      expect(state.error).toBeNull();
      expect(state.history).toEqual([]);
      expect(state.nodeReportsMap).toEqual({});
      expect(state.currentNodeUuid).toBeNull();
      expect(state.completedNodeCount).toBe(0);
      expect(state.totalNodes).toBe(0);
    });
  });

  describe('clearError', () => {
    it('仅清除错误，不影响其他状态', () => {
      useExecutionStore.getState().setError('错误');
      useExecutionStore.getState().startExecution();

      useExecutionStore.getState().clearError();

      expect(useExecutionStore.getState().error).toBeNull();
      expect(useExecutionStore.getState().isExecuting).toBe(true);
    });
  });

  describe('clearHistory', () => {
    it('仅清除历史，不影响其他状态', () => {
      const report = createMockReport();
      useExecutionStore.getState().setReport(report);
      useExecutionStore.getState().setError('错误');

      useExecutionStore.getState().clearHistory();

      expect(useExecutionStore.getState().history).toEqual([]);
      expect(useExecutionStore.getState().report).not.toBeNull();
      expect(useExecutionStore.getState().error).toBe('错误');
    });
  });

  describe('getNodeReport', () => {
    it('根据 UUID 返回节点报告', () => {
      const nodeReport1 = createMockNodeReport('node-1');
      const nodeReport2 = createMockNodeReport('node-2');
      const report = createMockReport({
        node_reports: [nodeReport1, nodeReport2],
      });
      useExecutionStore.getState().setReport(report);

      const result = useExecutionStore.getState().getNodeReport('node-2');

      expect(result).toEqual(nodeReport2);
    });

    it('找不到节点时返回 null', () => {
      const report = createMockReport({
        node_reports: [createMockNodeReport('node-1')],
      });
      useExecutionStore.getState().setReport(report);

      const result = useExecutionStore.getState().getNodeReport('non-existent');

      expect(result).toBeNull();
    });

    it('report 为 null 时返回 null', () => {
      const result = useExecutionStore.getState().getNodeReport('node-1');

      expect(result).toBeNull();
    });
  });

  describe('hasResult', () => {
    it('有报告时返回 true', () => {
      const report = createMockReport();
      useExecutionStore.getState().setReport(report);

      expect(useExecutionStore.getState().hasResult()).toBe(true);
    });

    it('有错误时返回 true', () => {
      useExecutionStore.getState().setError('错误');

      expect(useExecutionStore.getState().hasResult()).toBe(true);
    });

    it('同时有报告和错误时返回 true', () => {
      const report = createMockReport();
      useExecutionStore.getState().setReport(report);
      useExecutionStore.getState().setError('错误');

      expect(useExecutionStore.getState().hasResult()).toBe(true);
    });

    it('无报告无错误时返回 false', () => {
      expect(useExecutionStore.getState().hasResult()).toBe(false);
    });
  });

  describe('Selectors', () => {
    it('selectIsExecuting 返回执行状态', () => {
      expect(selectIsExecuting(useExecutionStore.getState())).toBe(false);

      useExecutionStore.getState().startExecution();
      expect(selectIsExecuting(useExecutionStore.getState())).toBe(true);
    });

    it('selectReport 返回报告', () => {
      expect(selectReport(useExecutionStore.getState())).toBeNull();

      const report = createMockReport();
      useExecutionStore.getState().setReport(report);
      expect(selectReport(useExecutionStore.getState())).toEqual(report);
    });

    it('selectError 返回错误', () => {
      expect(selectError(useExecutionStore.getState())).toBeNull();

      useExecutionStore.getState().setError('测试错误');
      expect(selectError(useExecutionStore.getState())).toBe('测试错误');
    });

    it('selectHistory 返回历史记录', () => {
      expect(selectHistory(useExecutionStore.getState())).toEqual([]);

      const report = createMockReport();
      useExecutionStore.getState().setReport(report);
      expect(selectHistory(useExecutionStore.getState())).toHaveLength(1);
    });

    describe('selectExecutionStatus', () => {
      it('执行中返回 executing', () => {
        useExecutionStore.getState().startExecution();
        expect(selectExecutionStatus(useExecutionStore.getState())).toBe('executing');
      });

      it('有错误返回 error', () => {
        useExecutionStore.getState().setError('错误');
        expect(selectExecutionStatus(useExecutionStore.getState())).toBe('error');
      });

      it('有报告时返回报告状态', () => {
        const report = createMockReport({ status: 'success' });
        useExecutionStore.getState().setReport(report);
        expect(selectExecutionStatus(useExecutionStore.getState())).toBe('success');
      });

      it('有报告但状态为 failed 时返回 failed', () => {
        const report = createMockReport({ status: 'failed' });
        useExecutionStore.getState().setReport(report);
        expect(selectExecutionStatus(useExecutionStore.getState())).toBe('failed');
      });

      it('默认返回 idle', () => {
        expect(selectExecutionStatus(useExecutionStore.getState())).toBe('idle');
      });

      it('优先级：isExecuting > error > report.status', () => {
        // 设置报告
        const report = createMockReport({ status: 'success' });
        useExecutionStore.getState().setReport(report);
        expect(selectExecutionStatus(useExecutionStore.getState())).toBe('success');

        // 设置错误（但 report 还在）
        useExecutionStore.getState().setError('错误');
        expect(selectExecutionStatus(useExecutionStore.getState())).toBe('error');

        // 开始执行（error 被清除，但这里手动设置 isExecuting）
        useExecutionStore.setState({ isExecuting: true });
        expect(selectExecutionStatus(useExecutionStore.getState())).toBe('executing');
      });
    });
  });

  describe('历史记录边界情况', () => {
    it('刚好 10 条记录时不会丢弃', () => {
      for (let i = 1; i <= 10; i++) {
        const report = createMockReport({ uuid: `report-${i}` });
        useExecutionStore.getState().setReport(report);
      }

      const history = useExecutionStore.getState().history;
      expect(history).toHaveLength(10);
      expect(history[0].uuid).toBe('report-10');
      expect(history[9].uuid).toBe('report-1');
    });

    it('第 11 条记录时丢弃最旧的', () => {
      for (let i = 1; i <= 10; i++) {
        const report = createMockReport({ uuid: `report-${i}` });
        useExecutionStore.getState().setReport(report);
      }

      // 添加第 11 条
      const report11 = createMockReport({ uuid: 'report-11' });
      useExecutionStore.getState().setReport(report11);

      const history = useExecutionStore.getState().history;
      expect(history).toHaveLength(10);
      expect(history[0].uuid).toBe('report-11');
      expect(history[9].uuid).toBe('report-2'); // report-1 被丢弃
    });
  });

  // ===== Incremental state tests =====

  describe('handleExecutionEvent', () => {
    it('should handle FlowStarted event', () => {
      const event: ExecutionEvent = {
        flowStarted: { flowUuid: 'f1', flowName: 'Test Flow', totalNodes: 3 },
      };
      useExecutionStore.getState().handleExecutionEvent(event);

      const state = useExecutionStore.getState();
      expect(state.isExecuting).toBe(true);
      expect(state.totalNodes).toBe(3);
      expect(state.completedNodeCount).toBe(0);
      expect(Object.keys(state.nodeReportsMap)).toHaveLength(0);
    });

    it('should handle NodeCompleted event', () => {
      // Setup
      useExecutionStore.getState().handleExecutionEvent({
        flowStarted: { flowUuid: 'f1', flowName: 'Test', totalNodes: 2 },
      });

      const nodeReport: NodeExecutionReport = {
        node_uuid: 'n1',
        node_type: 'http',
        node_name: 'HTTP Request',
        start_time: 1000,
        end_time: 2000,
        duration_ms: 1000,
        status: 'success',
        input_data: null,
        output_data: null,
      };

      useExecutionStore.getState().handleExecutionEvent({
        nodeCompleted: { report: nodeReport },
      });

      const state = useExecutionStore.getState();
      expect(state.nodeReportsMap['n1']).toBeDefined();
      expect(state.nodeReportsMap['n1'].status).toBe('success');
      expect(state.completedNodeCount).toBe(1);
    });

    it('should handle FlowCompleted event', () => {
      // Setup
      useExecutionStore.getState().handleExecutionEvent({
        flowStarted: { flowUuid: 'f1', flowName: 'Test', totalNodes: 1 },
      });

      const flowReport: FlowExecutionReport = {
        uuid: 'r1',
        flow_uuid: 'f1',
        flow_name: 'Test',
        start_time: 1000,
        end_time: 2000,
        duration_ms: 1000,
        total_nodes: 1,
        executed_count: 1,
        status: 'success',
        node_reports: [{
          node_uuid: 'n1', node_type: 'entry', node_name: 'Entry',
          start_time: 1000, end_time: 1100, duration_ms: 100,
          status: 'success', input_data: null, output_data: null,
        }],
        statistics: { success_count: 1, failed_count: 0 },
      };

      useExecutionStore.getState().handleExecutionEvent({
        flowCompleted: { report: flowReport },
      });

      const state = useExecutionStore.getState();
      expect(state.isExecuting).toBe(false);
      expect(state.report).toBeDefined();
      expect(state.report!.uuid).toBe('r1');
      // 验证完成计数已从报告同步
      expect(state.completedNodeCount).toBe(1);
      expect(state.totalNodes).toBe(1);
    });

    it('should handle full event sequence', () => {
      const { handleExecutionEvent } = useExecutionStore.getState();

      // FlowStarted
      handleExecutionEvent({
        flowStarted: { flowUuid: 'f1', flowName: 'Test', totalNodes: 2 },
      });
      expect(useExecutionStore.getState().isExecuting).toBe(true);

      // Node 1 completed
      handleExecutionEvent({
        nodeCompleted: {
          report: {
            node_uuid: 'n1', node_type: 'entry', node_name: 'Entry',
            start_time: 1000, end_time: 1100, duration_ms: 100,
            status: 'success', input_data: null, output_data: null,
          },
        },
      });
      expect(useExecutionStore.getState().completedNodeCount).toBe(1);

      // Node 2 completed (failed)
      handleExecutionEvent({
        nodeCompleted: {
          report: {
            node_uuid: 'n2', node_type: 'http', node_name: 'HTTP',
            start_time: 1100, end_time: 1500, duration_ms: 400,
            status: 'failed', input_data: null, output_data: null,
            error: { error_kind: 'connection', message: 'Connection refused' },
          },
        },
      });
      expect(useExecutionStore.getState().completedNodeCount).toBe(2);
      expect(useExecutionStore.getState().nodeReportsMap['n2'].status).toBe('failed');

      // FlowCompleted — 使用包含之前已完成节点报告的 node_reports
      handleExecutionEvent({
        flowCompleted: {
          report: {
            uuid: 'r1', flow_uuid: 'f1', flow_name: 'Test',
            start_time: 1000, end_time: 1500, duration_ms: 500,
            total_nodes: 2, executed_count: 2, status: 'failed',
            node_reports: [
              {
                node_uuid: 'n1', node_type: 'entry', node_name: 'Entry',
                start_time: 1000, end_time: 1100, duration_ms: 100,
                status: 'success', input_data: null, output_data: null,
              },
              {
                node_uuid: 'n2', node_type: 'http', node_name: 'HTTP',
                start_time: 1100, end_time: 1500, duration_ms: 400,
                status: 'failed', input_data: null, output_data: null,
                error: { error_kind: 'connection', message: 'Connection refused' },
              },
            ],
            statistics: { success_count: 1, failed_count: 1 },
          },
        },
      });
      expect(useExecutionStore.getState().isExecuting).toBe(false);
      // 验证完成计数已从报告同步（覆盖之前的增量值）
      expect(useExecutionStore.getState().completedNodeCount).toBe(2);
      expect(useExecutionStore.getState().totalNodes).toBe(2);
    });

    it('FlowStarted should clear previous incremental state', () => {
      // First execution
      useExecutionStore.getState().handleExecutionEvent({
        flowStarted: { flowUuid: 'f1', flowName: 'Test', totalNodes: 2 },
      });
      useExecutionStore.getState().handleExecutionEvent({
        nodeCompleted: {
          report: {
            node_uuid: 'n1', node_type: 'http', node_name: 'HTTP',
            start_time: 1000, end_time: 2000, duration_ms: 1000,
            status: 'success', input_data: null, output_data: null,
          },
        },
      });
      expect(useExecutionStore.getState().completedNodeCount).toBe(1);

      // Second execution - should reset
      useExecutionStore.getState().handleExecutionEvent({
        flowStarted: { flowUuid: 'f2', flowName: 'Test 2', totalNodes: 5 },
      });
      const state = useExecutionStore.getState();
      expect(state.nodeReportsMap).toEqual({});
      expect(state.completedNodeCount).toBe(0);
      expect(state.totalNodes).toBe(5);
    });
  });

  describe('getNodeExecutionStatus', () => {
    it('should return pending for unknown nodes', () => {
      expect(useExecutionStore.getState().getNodeExecutionStatus('n1')).toBe('pending');
    });

    it('should return success for completed nodes', () => {
      useExecutionStore.getState().handleExecutionEvent({
        flowStarted: { flowUuid: 'f1', flowName: 'Test', totalNodes: 1 },
      });
      useExecutionStore.getState().handleExecutionEvent({
        nodeCompleted: {
          report: {
            node_uuid: 'n1', node_type: 'http', node_name: 'HTTP',
            start_time: 1000, end_time: 2000, duration_ms: 1000,
            status: 'success', input_data: null, output_data: null,
          },
        },
      });
      expect(useExecutionStore.getState().getNodeExecutionStatus('n1')).toBe('success');
    });

    it('should return failed for failed nodes', () => {
      useExecutionStore.getState().handleExecutionEvent({
        flowStarted: { flowUuid: 'f1', flowName: 'Test', totalNodes: 1 },
      });
      useExecutionStore.getState().handleExecutionEvent({
        nodeCompleted: {
          report: {
            node_uuid: 'n1', node_type: 'http', node_name: 'HTTP',
            start_time: 1000, end_time: 2000, duration_ms: 1000,
            status: 'failed', input_data: null, output_data: null,
          },
        },
      });
      expect(useExecutionStore.getState().getNodeExecutionStatus('n1')).toBe('failed');
    });

    it('should return pending for non-executing state', () => {
      // No execution started, node is pending
      expect(useExecutionStore.getState().getNodeExecutionStatus('n1')).toBe('pending');
    });
  });

  describe('getNodeExecutionReport', () => {
    it('should return report from incremental map', () => {
      useExecutionStore.getState().handleExecutionEvent({
        flowStarted: { flowUuid: 'f1', flowName: 'Test', totalNodes: 1 },
      });
      useExecutionStore.getState().handleExecutionEvent({
        nodeCompleted: {
          report: {
            node_uuid: 'n1', node_type: 'http', node_name: 'HTTP',
            start_time: 1000, end_time: 2000, duration_ms: 1000,
            status: 'success', input_data: null, output_data: null,
          },
        },
      });

      const result = useExecutionStore.getState().getNodeExecutionReport('n1');
      expect(result).not.toBeNull();
      expect(result!.node_uuid).toBe('n1');
    });

    it('should fall back to final report', () => {
      // Set a final report (e.g. from setReport, not incremental)
      const report = createMockReport({
        node_reports: [createMockNodeReport('n1')],
      });
      useExecutionStore.getState().setReport(report);

      const result = useExecutionStore.getState().getNodeExecutionReport('n1');
      expect(result).not.toBeNull();
      expect(result!.node_uuid).toBe('n1');
    });

    it('should return null for unknown nodes', () => {
      expect(useExecutionStore.getState().getNodeExecutionReport('unknown')).toBeNull();
    });
  });

  describe('clearExecutionState', () => {
    it('should reset incremental state', () => {
      useExecutionStore.getState().handleExecutionEvent({
        flowStarted: { flowUuid: 'f1', flowName: 'Test', totalNodes: 3 },
      });
      useExecutionStore.getState().clearExecutionState();

      const state = useExecutionStore.getState();
      expect(state.nodeReportsMap).toEqual({});
      expect(state.currentNodeUuid).toBeNull();
      expect(state.completedNodeCount).toBe(0);
      expect(state.totalNodes).toBe(0);
    });

    it('should also clear report and error', () => {
      const report = createMockReport();
      useExecutionStore.getState().setReport(report);
      useExecutionStore.getState().setError('test error');
      useExecutionStore.getState().clearExecutionState();

      const state = useExecutionStore.getState();
      expect(state.report).toBeNull();
      expect(state.error).toBeNull();
      // history 不受影响
      expect(state.history).toHaveLength(1);
    });
  });

  describe('Incremental Selectors', () => {
    it('selectNodeReportsMap returns node reports map', () => {
      expect(selectNodeReportsMap(useExecutionStore.getState())).toEqual({});

      useExecutionStore.getState().handleExecutionEvent({
        flowStarted: { flowUuid: 'f1', flowName: 'Test', totalNodes: 1 },
      });
      useExecutionStore.getState().handleExecutionEvent({
        nodeCompleted: {
          report: {
            node_uuid: 'n1', node_type: 'http', node_name: 'HTTP',
            start_time: 1000, end_time: 2000, duration_ms: 1000,
            status: 'success', input_data: null, output_data: null,
          },
        },
      });

      const map = selectNodeReportsMap(useExecutionStore.getState());
      expect(map['n1']).toBeDefined();
    });

    it('selectCompletedNodeCount returns completed count', () => {
      expect(selectCompletedNodeCount(useExecutionStore.getState())).toBe(0);

      useExecutionStore.getState().handleExecutionEvent({
        flowStarted: { flowUuid: 'f1', flowName: 'Test', totalNodes: 2 },
      });
      useExecutionStore.getState().handleExecutionEvent({
        nodeCompleted: {
          report: {
            node_uuid: 'n1', node_type: 'http', node_name: 'HTTP',
            start_time: 1000, end_time: 2000, duration_ms: 1000,
            status: 'success', input_data: null, output_data: null,
          },
        },
      });

      expect(selectCompletedNodeCount(useExecutionStore.getState())).toBe(1);
    });

    it('selectTotalNodes returns total nodes', () => {
      expect(selectTotalNodes(useExecutionStore.getState())).toBe(0);

      useExecutionStore.getState().handleExecutionEvent({
        flowStarted: { flowUuid: 'f1', flowName: 'Test', totalNodes: 5 },
      });

      expect(selectTotalNodes(useExecutionStore.getState())).toBe(5);
    });
  });

  describe('reportDialog', () => {
    it('初始状态 isReportDialogOpen 为 false 且 selectedHistoryReport 为 null', () => {
      const state = useExecutionStore.getState();
      expect(state.isReportDialogOpen).toBe(false);
      expect(state.selectedHistoryReport).toBeNull();
    });

    it('openReportDialog() 设置 isReportDialogOpen = true，selectedHistoryReport 不变', () => {
      useExecutionStore.getState().openReportDialog();

      const state = useExecutionStore.getState();
      expect(state.isReportDialogOpen).toBe(true);
      expect(state.selectedHistoryReport).toBeNull();
    });

    it('openReportDialog(report) 设置 isReportDialogOpen = true 且 selectedHistoryReport = report', () => {
      const report = createMockReport({ uuid: 'history-report-1' });
      useExecutionStore.getState().openReportDialog(report);

      const state = useExecutionStore.getState();
      expect(state.isReportDialogOpen).toBe(true);
      expect(state.selectedHistoryReport).toEqual(report);
    });

    it('closeReportDialog() 设置 isReportDialogOpen = false 且 selectedHistoryReport = null', () => {
      const report = createMockReport({ uuid: 'history-report-2' });
      useExecutionStore.getState().openReportDialog(report);
      expect(useExecutionStore.getState().isReportDialogOpen).toBe(true);

      useExecutionStore.getState().closeReportDialog();

      const state = useExecutionStore.getState();
      expect(state.isReportDialogOpen).toBe(false);
      expect(state.selectedHistoryReport).toBeNull();
    });

    it('closeReportDialog() 不清空 history 和 nodeReportsMap', () => {
      // 添加 nodeReportsMap
      useExecutionStore.getState().handleExecutionEvent({
        flowStarted: { flowUuid: 'f1', flowName: 'Test', totalNodes: 1 },
      });
      useExecutionStore.getState().handleExecutionEvent({
        nodeCompleted: {
          report: {
            node_uuid: 'n1', node_type: 'http', node_name: 'HTTP',
            start_time: 1000, end_time: 2000, duration_ms: 1000,
            status: 'success', input_data: null, output_data: null,
          },
        },
      });
      expect(Object.keys(useExecutionStore.getState().nodeReportsMap).length).toBeGreaterThan(0);

      // 通过 flowCompleted 产生 history 和 report
      useExecutionStore.getState().handleExecutionEvent({
        flowCompleted: {
          report: {
            uuid: 'r1', flow_uuid: 'f1', flow_name: 'Test',
            start_time: 1000, end_time: 2000, duration_ms: 1000,
            total_nodes: 1, executed_count: 1, status: 'success',
            node_reports: [], statistics: { success_count: 1, failed_count: 0 },
          },
        },
      });
      expect(useExecutionStore.getState().history.length).toBeGreaterThan(0);

      // 打开再关闭对话框
      const report = useExecutionStore.getState().report!;
      useExecutionStore.getState().openReportDialog(report);
      useExecutionStore.getState().closeReportDialog();

      // history 和 nodeReportsMap 仍保留
      const state = useExecutionStore.getState();
      expect(state.history.length).toBeGreaterThan(0);
      expect(Object.keys(state.nodeReportsMap).length).toBeGreaterThan(0);
      // report 也保留
      expect(state.report).not.toBeNull();
    });
  });
});
