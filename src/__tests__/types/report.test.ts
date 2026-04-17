/**
 * Report 类型定义测试
 */

import { describe, it, expect } from 'vitest';
import type {
  ExecutionStatus,
  ExecutionMode,
  VariableChange,
  SkippedNodeInfo,
  NodeExecutionReport,
  FlowStatistics,
  ReportMetadata,
  FlowExecutionReport,
} from '@/types/report';
import type { StressTestReport } from '@/types/stressTest';
import type { Flow } from '@/types/flow';

describe('Report types', () => {
  describe('ExecutionStatus type', () => {
    it('should accept valid status values', () => {
      const statusSuccess: ExecutionStatus = 'success';
      const statusFailed: ExecutionStatus = 'failed';
      const statusSkipped: ExecutionStatus = 'skipped';

      expect(statusSuccess).toBe('success');
      expect(statusFailed).toBe('failed');
      expect(statusSkipped).toBe('skipped');
    });
  });

  describe('ExecutionMode type', () => {
    it('should accept valid mode values', () => {
      const normalMode: ExecutionMode = 'normal';
      const debugMode: ExecutionMode = 'debug';

      expect(normalMode).toBe('normal');
      expect(debugMode).toBe('debug');
    });
  });

  describe('VariableChange interface', () => {
    it('should have correct structure', () => {
      const change: VariableChange = {
        key: 'myVar',
        old_value: 'old',
        new_value: 'new',
      };

      expect(change.key).toBe('myVar');
      expect(change.old_value).toBe('old');
      expect(change.new_value).toBe('new');
    });

    it('should accept null old_value for new variables', () => {
      const change: VariableChange = {
        key: 'newVar',
        old_value: null,
        new_value: 'initial',
      };

      expect(change.old_value).toBeNull();
      expect(change.new_value).toBe('initial');
    });
  });

  describe('SkippedNodeInfo interface', () => {
    it('should have correct structure', () => {
      const skipped: SkippedNodeInfo = {
        node_uuid: 'node-1',
        node_name: 'Skipped Node',
        reason: 'Condition not met',
      };

      expect(skipped.node_uuid).toBe('node-1');
      expect(skipped.node_name).toBe('Skipped Node');
      expect(skipped.reason).toBe('Condition not met');
    });
  });

  describe('NodeExecutionReport interface', () => {
    it('should have correct structure with required fields', () => {
      const report: NodeExecutionReport = {
        node_uuid: 'node-1',
        node_type: 'http',
        node_name: 'HTTP Request',
        start_time: 1000,
        end_time: 2000,
        duration_ms: 1000,
        status: 'success',
      };

      expect(report.node_uuid).toBe('node-1');
      expect(report.node_type).toBe('http');
      expect(report.status).toBe('success');
      expect(report.duration_ms).toBe(1000);
    });

    it('should accept optional fields', () => {
      const report: NodeExecutionReport = {
        node_uuid: 'node-1',
        node_type: 'conditional',
        node_name: 'Check Status',
        start_time: 1000,
        end_time: 2000,
        duration_ms: 1000,
        status: 'success',
        used_port_id: 'true',
        input_data: { key: 'value' },
        output_data: { result: 'passed' },
        variable_changes: [{ key: 'var1', new_value: 'updated' }],
        wait_duration_ms: 50,
      };

      expect(report.used_port_id).toBe('true');
      expect(report.input_data).toEqual({ key: 'value' });
      expect(report.variable_changes).toHaveLength(1);
      expect(report.wait_duration_ms).toBe(50);
    });

    it('should accept failed status with error', () => {
      const report: NodeExecutionReport = {
        node_uuid: 'node-1',
        node_type: 'http',
        node_name: 'Failed Request',
        start_time: 1000,
        end_time: 2000,
        duration_ms: 1000,
        status: 'failed',
        error: {
          error_kind: 'connection',
          message: 'Connection refused',
        },
      };

      expect(report.status).toBe('failed');
      expect(report.error?.error_kind).toBe('connection');
    });
  });

  describe('FlowStatistics interface', () => {
    it('should have correct structure with required fields', () => {
      const stats: FlowStatistics = {
        success_count: 5,
        failed_count: 1,
      };

      expect(stats.success_count).toBe(5);
      expect(stats.failed_count).toBe(1);
    });

    it('should accept optional timing fields', () => {
      const stats: FlowStatistics = {
        success_count: 10,
        failed_count: 0,
        avg_node_duration_ms: 150,
        max_node_duration_ms: 500,
        min_node_duration_ms: 50,
      };

      expect(stats.avg_node_duration_ms).toBe(150);
      expect(stats.max_node_duration_ms).toBe(500);
      expect(stats.min_node_duration_ms).toBe(50);
    });

    it('should accept optional data and efficiency fields', () => {
      const stats: FlowStatistics = {
        success_count: 8,
        failed_count: 2,
        total_input_size_bytes: 1024,
        total_output_size_bytes: 2048,
        error_rate: 0.2,
        completion_rate: 0.9,
      };

      expect(stats.total_input_size_bytes).toBe(1024);
      expect(stats.error_rate).toBe(0.2);
      expect(stats.completion_rate).toBe(0.9);
    });
  });

  describe('ReportMetadata interface', () => {
    it('should have correct structure', () => {
      const meta: ReportMetadata = {
        uuid: 'report-1',
        flow_uuid: 'flow-1',
        flow_name: 'Test Flow',
        status: 'success',
        timestamp: 1709500000000,
        duration_ms: 5000,
      };

      expect(meta.uuid).toBe('report-1');
      expect(meta.flow_uuid).toBe('flow-1');
      expect(meta.duration_ms).toBe(5000);
    });

    it('should accept optional execution_mode', () => {
      const meta: ReportMetadata = {
        uuid: 'report-1',
        flow_uuid: 'flow-1',
        flow_name: 'Debug Flow',
        status: 'success',
        timestamp: 1709500000000,
        duration_ms: 5000,
        execution_mode: 'debug',
      };

      expect(meta.execution_mode).toBe('debug');
    });
  });

  describe('FlowExecutionReport interface', () => {
    it('should have correct structure with required fields', () => {
      const report: FlowExecutionReport = {
        uuid: 'report-1',
        flow_uuid: 'flow-1',
        flow_name: 'Test Flow',
        start_time: 1000,
        end_time: 5000,
        duration_ms: 4000,
        total_nodes: 3,
        executed_count: 3,
        status: 'success',
        node_reports: [],
        statistics: {
          success_count: 3,
          failed_count: 0,
        },
      };

      expect(report.uuid).toBe('report-1');
      expect(report.flow_uuid).toBe('flow-1');
      expect(report.total_nodes).toBe(3);
      expect(report.status).toBe('success');
    });

    it('should accept optional fields', () => {
      const report: FlowExecutionReport = {
        uuid: 'report-1',
        flow_uuid: 'flow-1',
        flow_name: 'Complex Flow',
        start_time: 1000,
        end_time: 5000,
        duration_ms: 4000,
        total_nodes: 5,
        executed_count: 4,
        status: 'success',
        node_reports: [],
        statistics: {
          success_count: 4,
          failed_count: 0,
        },
        execution_mode: 'normal',
        initial_variables: { api_key: 'secret' },
        final_variables: { result: 'data' },
        execution_path: ['node-1', 'node-2', 'node-3'],
        skipped_nodes: [
          { node_uuid: 'node-4', node_name: 'Skip Me', reason: 'Skipped by user' },
        ],
      };

      expect(report.execution_path).toHaveLength(3);
      expect(report.skipped_nodes).toHaveLength(1);
      expect(report.initial_variables).toEqual({ api_key: 'secret' });
    });

    it('should accept flow_snapshot with Flow data', () => {
      const snapshot: Flow = {
        uuid: 'flow-1',
        name: 'Test Flow',
        nodes: [],
        connections: [],
      };
      const report: FlowExecutionReport = {
        uuid: 'report-1',
        flow_uuid: 'flow-1',
        flow_name: 'Test Flow',
        start_time: 1000,
        end_time: 5000,
        duration_ms: 4000,
        total_nodes: 1,
        executed_count: 1,
        status: 'success',
        node_reports: [],
        statistics: { success_count: 1, failed_count: 0 },
        flow_snapshot: snapshot,
      };

      expect(report.flow_snapshot).toBeDefined();
      expect(report.flow_snapshot?.uuid).toBe('flow-1');
      expect(report.flow_snapshot?.name).toBe('Test Flow');
    });

    it('should allow flow_snapshot to be undefined for backward compatibility', () => {
      const report: FlowExecutionReport = {
        uuid: 'report-1',
        flow_uuid: 'flow-1',
        flow_name: 'Test Flow',
        start_time: 1000,
        end_time: 5000,
        duration_ms: 4000,
        total_nodes: 1,
        executed_count: 1,
        status: 'success',
        node_reports: [],
        statistics: { success_count: 1, failed_count: 0 },
      };

      expect(report.flow_snapshot).toBeUndefined();
    });
  });

  describe('StressTestReport flowSnapshot', () => {
    it('should accept flowSnapshot with Flow data', () => {
      const snapshot: Flow = {
        uuid: 'flow-2',
        name: 'Stress Flow',
        nodes: [],
        connections: [],
      };
      const report: StressTestReport = {
        uuid: 'stress-report-1',
        flowUuid: 'flow-2',
        flowName: 'Stress Flow',
        startTime: 1000,
        endTime: 5000,
        durationMs: 4000,
        config: {
          concurrentUsers: 10,
          rampUpSeconds: 5,
          durationSeconds: 60,
          flowUuid: 'flow-2',
        },
        statistics: {
          totalIterations: 100,
          successful: 95,
          failed: 5,
          successRate: 0.95,
          rps: 10,
        },
        responseTimeDistribution: {
          avgMs: 100,
          minMs: 50,
          maxMs: 500,
          p50Ms: 90,
          p90Ms: 200,
          p95Ms: 300,
          p99Ms: 450,
        },
        flowSnapshot: snapshot,
      };

      expect(report.flowSnapshot).toBeDefined();
      expect(report.flowSnapshot?.uuid).toBe('flow-2');
      expect(report.flowSnapshot?.name).toBe('Stress Flow');
    });

    it('should allow flowSnapshot to be undefined for backward compatibility', () => {
      const report: StressTestReport = {
        uuid: 'stress-report-1',
        flowUuid: 'flow-2',
        flowName: 'Stress Flow',
        startTime: 1000,
        endTime: 5000,
        durationMs: 4000,
        config: {
          concurrentUsers: 10,
          rampUpSeconds: 5,
          durationSeconds: 60,
          flowUuid: 'flow-2',
        },
        statistics: {
          totalIterations: 100,
          successful: 95,
          failed: 5,
          successRate: 0.95,
          rps: 10,
        },
        responseTimeDistribution: {
          avgMs: 100,
          minMs: 50,
          maxMs: 500,
          p50Ms: 90,
          p90Ms: 200,
          p95Ms: 300,
          p99Ms: 450,
        },
      };

      expect(report.flowSnapshot).toBeUndefined();
    });
  });
});
