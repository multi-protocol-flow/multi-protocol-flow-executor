/**
 * 压测报告存储服务
 * 通过 Tauri IPC 调用后端压测报告存储命令
 */

import { invoke } from '@tauri-apps/api/core';
import type { StressTestReport, StressTestReportMetadata } from '@/types/stressTest';
import { isTauriEnv } from '@/lib/platform';

export const stressTestStorage = {
  /** 保存压测报告 */
  async saveReport(report: StressTestReport, reportsDir: string): Promise<string> {
    if (isTauriEnv()) {
      return invoke<string>('save_stress_test_report', { report, reportsDir });
    }
    const { saveReport: mock } = await import('./stressTestStorage.mock');
    return mock(report, reportsDir);
  },

  /** 加载压测报告 */
  async loadReport(filePath: string): Promise<StressTestReport> {
    if (isTauriEnv()) {
      return invoke<StressTestReport>('load_stress_test_report', { filePath });
    }
    const { loadReport: mock } = await import('./stressTestStorage.mock');
    return mock(filePath);
  },

  /** 列出所有压测报告 */
  async listReports(reportsDir: string): Promise<StressTestReportMetadata[]> {
    if (isTauriEnv()) {
      return invoke<StressTestReportMetadata[]>('list_stress_test_reports', { reportsDir });
    }
    const { listReports: mock } = await import('./stressTestStorage.mock');
    return mock(reportsDir);
  },

  /** 删除压测报告 */
  async deleteReport(filePath: string): Promise<void> {
    if (isTauriEnv()) {
      await invoke<void>('delete_stress_test_report', { filePath });
      return;
    }
    const { deleteReport: mock } = await import('./stressTestStorage.mock');
    return mock(filePath);
  },
};
