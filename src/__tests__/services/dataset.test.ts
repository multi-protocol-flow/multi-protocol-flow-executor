/**
 * 数据集执行 Mock 服务测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  validateDatasetCsv,
  executeFlowWithDataset,
  cancelDatasetExecution,
  listDatasetReports,
  loadDatasetReport,
  loadDatasetRowDetail,
  deleteDatasetReport,
} from '@/services/tauri/mock';
import type { DatasetExecutionConfig } from '@/types/dataset';

// ============================================================================
// 测试
// ============================================================================

describe('Dataset Mock Services', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('validateDatasetCsv', () => {
    it('返回 CSV 元信息', async () => {
      const result = await validateDatasetCsv('/data/test.csv');

      expect(result).toHaveProperty('row_count');
      expect(result).toHaveProperty('column_names');
      expect(result).toHaveProperty('detected_encoding');
      expect(result.row_count).toBe(3);
      expect(result.column_names).toEqual(['name', 'email', 'age']);
      expect(result.detected_encoding).toBe('utf-8');
    });
  });

  describe('executeFlowWithDataset', () => {
    it('发送完整事件序列', async () => {
      const events: Array<{ type: string }> = [];
      const onEvent = vi.fn((event) => events.push(event));

      const mockFlow = {
        uuid: 'flow-1',
        name: 'Test',
        nodes: [],
        connections: [],
        initial_variables: {},
      };
      const mockConfig: DatasetExecutionConfig = {
        flow_uuid: 'flow-1',
        dataset_path: '/data/test.csv',
        concurrency: 1,
        failure_strategy: 'continue',
      };

      await executeFlowWithDataset(mockFlow, mockConfig, onEvent);

      // 验证事件序列
      expect(events[0].type).toBe('started');
      expect(events[events.length - 1].type).toBe('ended');

      // 验证有 row_completed 事件
      const rowEvents = events.filter((e) => e.type === 'row_completed');
      expect(rowEvents.length).toBe(3);

      // 验证有 progress 事件
      const progressEvents = events.filter((e) => e.type === 'progress');
      expect(progressEvents.length).toBe(3);

      // 验证 ended 事件包含报告
      const endedEvent = events.find((e) => e.type === 'ended');
      expect(endedEvent).toBeDefined();
      if (endedEvent && 'report' in endedEvent) {
        const report = endedEvent.report as Record<string, unknown>;
        expect(report['total_rows']).toBe(3);
        expect(report['success_count']).toBe(3);
        expect(report['uuid']).toBeTruthy();
      }
    });
  });

  describe('cancelDatasetExecution', () => {
    it('无错误返回', async () => {
      await expect(cancelDatasetExecution()).resolves.toBeUndefined();
    });
  });

  describe('listDatasetReports', () => {
    it('返回报告列表', async () => {
      const result = await listDatasetReports('/reports');

      expect(Array.isArray(result)).toBe(true);
      expect(result.length).toBeGreaterThan(0);

      const report = result[0];
      expect(report).toHaveProperty('report_id');
      expect(report).toHaveProperty('flow_name');
      expect(report).toHaveProperty('dataset_name');
      expect(report).toHaveProperty('total_rows');
      expect(report).toHaveProperty('success_count');
    });
  });

  describe('loadDatasetReport', () => {
    it('返回完整报告', async () => {
      const result = await loadDatasetReport('report-001', '/reports');

      expect(result).toHaveProperty('uuid');
      expect(result).toHaveProperty('flow_uuid');
      expect(result).toHaveProperty('flow_name');
      expect(result).toHaveProperty('dataset_name');
      expect(result).toHaveProperty('total_rows');
      expect(result).toHaveProperty('row_summaries');
      expect(Array.isArray(result.row_summaries)).toBe(true);
    });
  });

  describe('loadDatasetRowDetail', () => {
    it('返回行详情', async () => {
      const result = await loadDatasetRowDetail('report-001', 0, '/reports');

      expect(result).not.toBeNull();
    });
  });

  describe('deleteDatasetReport', () => {
    it('无错误返回', async () => {
      await expect(deleteDatasetReport('report-001', '/reports')).resolves.toBeUndefined();
    });
  });
});
