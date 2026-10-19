/**
 * Tauri IPC 压力测试相关调用
 * 封装压力测试命令的 invoke 调用
 * 
 * 事件通过 Tauri 全局事件系统 ("stress-test-event") 传递，
 * 而非 IPC Channel —— 因为压测是长时间后台任务，
 * Channel 在 IPC 命令返回后会被清理。
 */

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Flow } from '@/types/flow';
import { useEnvironmentStore } from '@/stores/environment';
import type {
  StressTestConfig,
  StressTestEvent,
  GetStressTestStatusOutput,
  StressTestReport,
} from '@/types/stressTest';
import { isTauriEnv } from '@/lib/platform';
import { formatError } from './flow';

/**
 * 启动压力测试
 * 通过 Tauri 全局事件系统接收压测事件
 *
 * @param flow - 要压测的流程对象
 * @param config - 压测配置
 * @param onEvent - 压测事件回调函数
 * @returns 取消监听的函数
 * @throws IpcError 如果启动失败
 */
export async function startStressTest(
  flow: Flow,
  config: StressTestConfig,
  onEvent: (event: StressTestEvent) => void
): Promise<UnlistenFn> {
  // 先注册事件监听器（确保在命令启动前注册完毕，避免丢失早期事件）
  const unlisten = await listen<StressTestEvent>('stress-test-event', (event) => {
    onEvent(event.payload);
  });

  try {
    const environmentName = useEnvironmentStore.getState().activeEnvironmentName;
    await invoke<void>('start_stress_test', {
      input: {
        flow,
        config,
      },
      ...(environmentName ? { environmentName } : {}),
    });
  } catch (error) {
    // 启动失败时清理监听器
    unlisten();
    throw formatError(error, 'start_stress_test');
  }

  return unlisten;
}

/**
 * 停止压力测试
 *
 * @throws IpcError 如果停止失败
 */
export async function stopStressTest(): Promise<void> {
  try {
    await invoke<void>('stop_stress_test');
  } catch (error) {
    throw formatError(error, 'stop_stress_test');
  }
}

/**
 * 获取压力测试状态
 *
 * @returns 当前压测状态和进度信息
 * @throws IpcError 如果查询失败
 */
export async function getStressTestStatus(): Promise<GetStressTestStatusOutput> {
  try {
    return await invoke<GetStressTestStatusOutput>('get_stress_test_status');
  } catch (error) {
    throw formatError(error, 'get_stress_test_status');
  }
}
/**
 * 获取压力测试报告的 Markdown 摘要
 */
export async function getStressReportMarkdown(
  report: StressTestReport
): Promise<string> {
  if (isTauriEnv()) {
    try {
      return await invoke<string>('get_stress_report_markdown', { report });
    } catch (error) {
      throw formatError(error, 'get_stress_report_markdown');
    }
  }
  return `## Stress Test: ${report.flowName}\n- Total Iterations: ${report.statistics.totalIterations}\n- Success Rate: ${(report.statistics.successRate * 100).toFixed(2)}%\n`;
}

/**
 * 导出压力测试报告为自包含 HTML 文件
 */
export async function exportStressReportHtml(
  report: StressTestReport,
  filePath: string
): Promise<void> {
  if (isTauriEnv()) {
    try {
      await invoke<void>('export_stress_report_html', { report, filePath });
    } catch (error) {
      throw formatError(error, 'export_stress_report_html');
    }
  }
}
