/**
 * 数据集执行 IPC 服务层
 * 封装数据集执行相关的 Tauri IPC 调用
 *
 * 事件通过 Tauri 全局事件系统 ("dataset-event") 传递，
 * 而非 IPC Channel —— 因为数据集执行是长时间后台任务，
 * Channel 在 IPC 命令返回后会被清理。
 */

import type { Flow } from '@/types/flow';
import type {
  DatasetExecutionConfig,
  CsvInfo,
  DatasetExecutionReport,
  DatasetReportSummary,
  DatasetEvent,
} from '@/types/dataset';

import { isTauriEnv } from '@/lib/platform';

// ============================================================================
// 环境检测
// ============================================================================

// ============================================================================
// 数据集执行服务
// ============================================================================

export const datasetService = {
  /**
   * 验证 CSV 文件元信息
   * @param path CSV 文件路径
   * @param encoding 指定编码（可选）
   * @param delimiter 分隔符（可选）
   */
  validateCsv: async (path: string, encoding?: string, delimiter?: string): Promise<CsvInfo> => {
    if (isTauriEnv()) {
      const { invoke } = await import('@tauri-apps/api/core');
      return invoke<CsvInfo>('validate_dataset_csv', { path, encoding, delimiter });
    } else {
      const { validateDatasetCsv } = await import('./tauri/mock');
      return validateDatasetCsv(path, encoding, delimiter);
    }
  },

  /**
   * 启动数据驱动执行
   * 通过 Tauri 全局事件系统接收执行事件
   * @param flow 要执行的流程
   * @param config 数据集执行配置
   * @param onEvent 事件回调函数
   * @returns 取消监听的函数
   */
  executeWithDataset: async (
    flow: Flow,
    config: DatasetExecutionConfig,
    onEvent: (event: DatasetEvent) => void
  ): Promise<() => void> => {
    if (isTauriEnv()) {
      const { invoke } = await import('@tauri-apps/api/core');
      const { listen } = await import('@tauri-apps/api/event');

      // 先注册事件监听器（确保在命令启动前注册完毕）
      const unlisten = await listen<DatasetEvent>('dataset-event', (event) => {
        onEvent(event.payload);
      });

      try {
        await invoke<void>('execute_flow_with_dataset', {
          input: { flow, config },
        });
      } catch (error) {
        // 启动失败时清理监听器
        unlisten();
        throw error;
      }

      return unlisten;
    } else {
      const { executeFlowWithDataset } = await import('./tauri/mock');
      await executeFlowWithDataset(flow, config, onEvent);
      return () => {};
    }
  },

  /**
   * 取消数据驱动执行
   */
  cancelExecution: async (): Promise<void> => {
    if (isTauriEnv()) {
      const { invoke } = await import('@tauri-apps/api/core');
      return invoke<void>('cancel_dataset_execution');
    } else {
      const { cancelDatasetExecution } = await import('./tauri/mock');
      return cancelDatasetExecution();
    }
  },

  /**
   * 列出数据集报告
   * @param reportsDir 报告存储目录
   */
  listReports: async (reportsDir: string): Promise<DatasetReportSummary[]> => {
    if (isTauriEnv()) {
      const { invoke } = await import('@tauri-apps/api/core');
      return invoke<DatasetReportSummary[]>('list_dataset_reports', { reportsDir });
    } else {
      const { listDatasetReports } = await import('./tauri/mock');
      return listDatasetReports(reportsDir);
    }
  },

  /**
   * 加载数据集报告
   * @param reportId 报告 ID
   * @param reportsDir 报告存储目录
   */
  loadReport: async (reportId: string, reportsDir: string): Promise<DatasetExecutionReport> => {
    if (isTauriEnv()) {
      const { invoke } = await import('@tauri-apps/api/core');
      return invoke<DatasetExecutionReport>('load_dataset_report', { reportId, reportsDir });
    } else {
      const { loadDatasetReport } = await import('./tauri/mock');
      return loadDatasetReport(reportId, reportsDir);
    }
  },

  /**
   * 加载失败行详情
   * @param reportId 报告 ID
   * @param rowIndex 行索引
   * @param reportsDir 报告存储目录
   */
  loadRowDetail: async (
    reportId: string,
    rowIndex: number,
    reportsDir: string
  ): Promise<unknown | null> => {
    if (isTauriEnv()) {
      const { invoke } = await import('@tauri-apps/api/core');
      return invoke<unknown | null>('load_dataset_row_detail', {
        reportId,
        rowIndex,
        reportsDir,
      });
    } else {
      const { loadDatasetRowDetail } = await import('./tauri/mock');
      return loadDatasetRowDetail(reportId, rowIndex, reportsDir);
    }
  },

  /**
   * 删除数据集报告
   * @param reportId 报告 ID
   * @param reportsDir 报告存储目录
   */
  deleteReport: async (reportId: string, reportsDir: string): Promise<void> => {
    if (isTauriEnv()) {
      const { invoke } = await import('@tauri-apps/api/core');
      return invoke<void>('delete_dataset_report', { reportId, reportsDir });
    } else {
      const { deleteDatasetReport } = await import('./tauri/mock');
      return deleteDatasetReport(reportId, reportsDir);
    }
  },

  /**
   * 保存数据集报告到磁盘
   * @param report 数据集执行报告
   * @param reportsDir 报告存储目录
   */
  saveReport: async (report: DatasetExecutionReport, reportsDir: string): Promise<string> => {
    if (isTauriEnv()) {
      const { invoke } = await import('@tauri-apps/api/core');
      return invoke<string>('save_dataset_report', { report, reportsDir });
    } else {
      const { saveDatasetReport } = await import('./tauri/mock');
      return saveDatasetReport(report, reportsDir);
    }
  },
};
