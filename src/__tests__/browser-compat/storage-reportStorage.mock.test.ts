/**
 * 报告存储 Mock 服务测试
 * 测试 reportStorage.mock CRUD
 */
import { describe, it, expect, beforeEach } from 'vitest';
import {
  saveReport,
  loadReport,
  listReports,
  deleteReport,
  clearReports,
  getReportCount,
} from '@/services/storage/reportStorage.mock';
import type { FlowExecutionReport } from '@/types/report';

/** 创建测试用的执行报告 */
function createTestReport(overrides: Partial<FlowExecutionReport> = {}): FlowExecutionReport {
  const uuid = `report-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
  return {
    uuid,
    flow_uuid: 'test-flow-1',
    flow_name: '测试流程',
    start_time: Date.now(),
    end_time: Date.now() + 100,
    duration_ms: 100,
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

describe('ReportStorage Mock', () => {
  const testDir = '/test/reports';

  beforeEach(() => {
    clearReports();
  });

  describe('saveReport', () => {
    it('可以保存报告', async () => {
      const report = createTestReport();
      await saveReport(report, testDir);

      expect(getReportCount()).toBe(1);
    });

    it('更新已存在的报告覆盖内容', async () => {
      const report = createTestReport({ flow_name: '原始名称' });
      await saveReport(report, testDir);

      const updated = { ...report, flow_name: '更新后的名称' };
      await saveReport(updated, testDir);

      expect(getReportCount()).toBe(1);
      const loaded = await loadReport(report.uuid, testDir);
      expect(loaded.flow_name).toBe('更新后的名称');
    });

    it('可以保存多个不同 UUID 的报告', async () => {
      await saveReport(createTestReport(), testDir);
      await saveReport(createTestReport(), testDir);
      await saveReport(createTestReport(), testDir);

      expect(getReportCount()).toBe(3);
    });
  });

  describe('loadReport', () => {
    it('可以加载已保存的报告', async () => {
      const report = createTestReport();
      await saveReport(report, testDir);

      const loaded = await loadReport(report.uuid, testDir);

      expect(loaded).toBeDefined();
      expect(loaded.uuid).toBe(report.uuid);
      expect(loaded.flow_name).toBe(report.flow_name);
    });

    it('加载不存在的报告抛出错误', async () => {
      await expect(loadReport('non-existent', testDir)).rejects.toThrow(
        'Report not found: non-existent',
      );
    });

    it('加载的报告包含 node_reports 数据', async () => {
      const report = createTestReport({
        node_reports: [
          {
            node_uuid: 'node-1',
            node_type: 'entry',
            node_name: '入口',
            start_time: Date.now(),
            end_time: Date.now() + 10,
            duration_ms: 10,
            status: 'success',
            input_data: { key: 'value' },
            output_data: { result: 'ok' },
          },
        ],
      });
      await saveReport(report, testDir);

      const loaded = await loadReport(report.uuid, testDir);

      expect(loaded.node_reports).toHaveLength(1);
      expect(loaded.node_reports[0].node_uuid).toBe('node-1');
      expect(loaded.node_reports[0].input_data).toEqual({ key: 'value' });
    });

    it('加载的报告包含 statistics 数据', async () => {
      const report = createTestReport({
        statistics: {
          success_count: 5,
          failed_count: 1,
          avg_node_duration_ms: 200,
          max_node_duration_ms: 500,
          min_node_duration_ms: 50,
          completion_rate: 0.85,
          error_rate: 0.15,
        },
      });
      await saveReport(report, testDir);

      const loaded = await loadReport(report.uuid, testDir);

      expect(loaded.statistics.success_count).toBe(5);
      expect(loaded.statistics.failed_count).toBe(1);
    });
  });

  describe('listReports', () => {
    it('空存储返回空数组', async () => {
      const reports = await listReports(testDir);
      expect(reports).toEqual([]);
    });

    it('可以列出报告元数据', async () => {
      await saveReport(createTestReport(), testDir);
      await saveReport(createTestReport(), testDir);

      const reports = await listReports(testDir);

      expect(reports).toHaveLength(2);
    });

    it('报告按 start_time 倒序排列', async () => {
      const oldReport = createTestReport({
        uuid: 'old-report',
        start_time: 1000,
        end_time: 1100,
      });
      const newReport = createTestReport({
        uuid: 'new-report',
        start_time: 2000,
        end_time: 2100,
      });

      await saveReport(oldReport, testDir);
      await saveReport(newReport, testDir);

      const reports = await listReports(testDir);

      expect(reports[0].uuid).toBe('new-report');
      expect(reports[1].uuid).toBe('old-report');
    });

    it('过滤掉 skipped 状态的报告', async () => {
      const successReport = createTestReport({ uuid: 'success-1', status: 'success' });
      const skippedReport = createTestReport({ uuid: 'skipped-1', status: 'skipped' });

      await saveReport(successReport, testDir);
      await saveReport(skippedReport, testDir);

      const reports = await listReports(testDir);

      expect(reports).toHaveLength(1);
      expect(reports[0].uuid).toBe('success-1');
    });

    it('包含失败状态的报告', async () => {
      const failedReport = createTestReport({ uuid: 'failed-1', status: 'failed' });
      await saveReport(failedReport, testDir);

      const reports = await listReports(testDir);

      expect(reports).toHaveLength(1);
      expect(reports[0].status).toBe('failed');
    });

    it('返回的是元数据而非完整报告', async () => {
      const report = createTestReport({
        node_reports: [
          {
            node_uuid: 'node-1',
            node_type: 'http',
            node_name: 'HTTP 请求',
            start_time: Date.now(),
            end_time: Date.now() + 50,
            duration_ms: 50,
            status: 'success',
          },
        ],
      });
      await saveReport(report, testDir);

      const reports = await listReports(testDir);

      expect(reports[0]).toHaveProperty('uuid');
      expect(reports[0]).toHaveProperty('flow_uuid');
      expect(reports[0]).toHaveProperty('flow_name');
      expect(reports[0]).toHaveProperty('status');
      expect(reports[0]).toHaveProperty('timestamp');
      expect(reports[0]).toHaveProperty('duration_ms');
      expect(reports[0]).not.toHaveProperty('node_reports');
      expect(reports[0]).not.toHaveProperty('statistics');
    });
  });

  describe('deleteReport', () => {
    it('可以删除已保存的报告', async () => {
      const report = createTestReport();
      await saveReport(report, testDir);

      await deleteReport(report.uuid, testDir);

      expect(getReportCount()).toBe(0);
      await expect(loadReport(report.uuid, testDir)).rejects.toThrow(/Report not found/);
    });

    it('删除不存在的报告抛出错误', async () => {
      await expect(deleteReport('non-existent', testDir)).rejects.toThrow(
        'Report not found: non-existent',
      );
    });

    it('删除后不影响其他报告', async () => {
      await saveReport(createTestReport({ uuid: 'report-1' }), testDir);
      await saveReport(createTestReport({ uuid: 'report-2' }), testDir);

      await deleteReport('report-1', testDir);

      expect(getReportCount()).toBe(1);
      const loaded = await loadReport('report-2', testDir);
      expect(loaded.uuid).toBe('report-2');
    });

    it('删除后可从列表可见', async () => {
      await saveReport(createTestReport({ uuid: 'report-1' }), testDir);
      await saveReport(createTestReport({ uuid: 'report-2' }), testDir);

      await deleteReport('report-1', testDir);

      const reports = await listReports(testDir);
      expect(reports).toHaveLength(1);
      expect(reports[0].uuid).toBe('report-2');
    });
  });

  describe('clearReports', () => {
    it('清空所有报告', async () => {
      await saveReport(createTestReport(), testDir);
      await saveReport(createTestReport(), testDir);
      expect(getReportCount()).toBe(2);

      clearReports();

      expect(getReportCount()).toBe(0);
      const reports = await listReports(testDir);
      expect(reports).toHaveLength(0);
    });
  });

  describe('集成场景', () => {
    it('保存→列出→加载→删除完整流程', async () => {
      const report = createTestReport({ flow_name: '集成测试' });
      await saveReport(report, testDir);

      let list = await listReports(testDir);
      expect(list).toHaveLength(1);
      expect(list[0].uuid).toBe(report.uuid);
      expect(list[0].flow_name).toBe('集成测试');

      const loaded = await loadReport(report.uuid, testDir);
      expect(loaded.uuid).toBe(report.uuid);
      expect(loaded.flow_name).toBe('集成测试');
      expect(loaded.duration_ms).toBe(100);

      await deleteReport(report.uuid, testDir);
      list = await listReports(testDir);
      expect(list).toHaveLength(0);
    });

    it('多个报告并发保存', async () => {
      const reports = [
        createTestReport({ uuid: 'parallel-1' }),
        createTestReport({ uuid: 'parallel-2' }),
        createTestReport({ uuid: 'parallel-3' }),
      ];

      await Promise.all(reports.map((r) => saveReport(r, testDir)));

      expect(getReportCount()).toBe(3);

      const list = await listReports(testDir);
      expect(list).toHaveLength(3);
    });
  });
});
