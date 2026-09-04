import { describe, it, expect, beforeEach } from 'vitest';
import {
  executionToolDefinitions,
  executionToolHandlers,
} from '@/services/ai/tools/executionTools';
import { useFlowStore } from '@/stores/flow';
import { useExecutionStore } from '@/stores/execution';
import type { Flow, FlowExecutionReport } from '@/types';

describe('executionTools', () => {
  beforeEach(() => {
    useFlowStore.getState().reset();
    useExecutionStore.getState().reset();
  });

  it('定义了完整的执行与调试工具，包含 diagnose_flow_errors', () => {
    const names = executionToolDefinitions.map((t) => t.function.name);
    expect(names).toContain('run_flow');
    expect(names).toContain('cancel_flow_execution');
    expect(names).toContain('get_last_execution_report');
    expect(names).toContain('diagnose_flow_errors');
  });

  describe('diagnose_flow_errors', () => {
    it('当没有打开流程时抛出错误', async () => {
      await expect(executionToolHandlers.diagnose_flow_errors()).rejects.toThrow(
        'No active flow to diagnose.',
      );
    });

    it('检测流程中的孤立节点和拓扑连线缺失', async () => {
      const mockFlow: Flow = {
        uuid: 'flow-test',
        name: 'Test Flow',
        nodes: [
          {
            uuid: 'entry-1',
            type: 'entry',
            name: 'Entry',
            position: { x: 0, y: 0 },
            data: { type: 'entry' } as unknown as Flow['nodes'][0]['data'],
          },
          {
            uuid: 'http-1',
            type: 'http',
            name: 'Orphan HTTP',
            position: { x: 200, y: 0 },
            data: { type: 'http', url: 'https://example.com' } as unknown as Flow['nodes'][0]['data'],
          },
        ],
        connections: [],
      };
      useFlowStore.getState().setFlow(mockFlow);

      const res = await executionToolHandlers.diagnose_flow_errors();
      expect(res.success).toBe(true);
      expect(res.topology_warnings.length).toBeGreaterThan(0);
      expect(res.topology_warnings.some((w) => w.includes('Orphan HTTP'))).toBe(true);
    });

    it('深度诊断失败节点并提供根本原因与 1~3 个修复方向', async () => {
      const mockFlow: Flow = {
        uuid: 'flow-err',
        name: 'Error Flow',
        nodes: [
          {
            uuid: 'http-err-1',
            type: 'http',
            name: 'API Auth Request',
            position: { x: 100, y: 100 },
            data: {
              type: 'http',
              url: 'https://api.example.com/protected',
              method: 'get',
            } as unknown as Flow['nodes'][0]['data'],
          },
        ],
        connections: [],
      };
      useFlowStore.getState().setFlow(mockFlow);

      const mockReport: FlowExecutionReport = {
        uuid: 'report-1',
        flow_uuid: 'flow-err',
        flow_name: 'Error Flow',
        start_time: 1000,
        end_time: 1200,
        duration_ms: 200,
        total_nodes: 1,
        executed_count: 1,
        status: 'failed',
        statistics: { success_count: 0, failed_count: 1 },
        node_reports: [
          {
            node_uuid: 'http-err-1',
            node_name: 'API Auth Request',
            node_type: 'http',
            start_time: 1000,
            end_time: 1200,
            duration_ms: 200,
            status: 'failed',
            error: {
              error_kind: 'execution',
              error_code: 'E_HTTP_STATUS_ERROR',
              message: 'HTTP 401 Unauthorized',
            },
          },
        ],
      };
      useExecutionStore.getState().setReport(mockReport);

      const res = await executionToolHandlers.diagnose_flow_errors();
      expect(res.success).toBe(true);
      expect(res.failed_count).toBe(1);
      expect(res.diagnoses[0].node_name).toBe('API Auth Request');
      expect(res.diagnoses[0].possible_root_cause).toContain('401 Unauthorized');
      expect(res.diagnoses[0].suggested_repair_directions.length).toBeGreaterThan(0);
      expect(res.instruction_for_assistant).toContain('propose the suggested repair directions');
    });
  });

  describe('get_last_execution_report', () => {
    it('当没有报告时返回明确提示', async () => {
      const res = await executionToolHandlers.get_last_execution_report();
      expect(res.success).toBe(false);
      expect(res.message).toContain('No execution report available');
    });
  });
});
