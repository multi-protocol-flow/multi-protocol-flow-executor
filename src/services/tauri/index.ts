/**
 * Tauri IPC 服务主导出文件
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 */

import type { Flow } from '@/types/flow';
import type { ExecuteFlowCommandOutput } from '@/types/ipc';
import { invoke } from '@tauri-apps/api/core';
import type { FlowExecutionReport, NodeExecutionReport } from '@/types/report';
import type { ReportMetadata } from '@/types/ipc';

import { isTauriEnv } from '@/lib/platform';
import { getLL } from '@/i18n/i18n-lazy';

// 检测是否在 Tauri 环境中运行

// 动态导入正确的服务
async function getService() {
  if (isTauriEnv()) {
    // 在 Tauri 环境中使用真实 IPC
    const { createFlow, IpcError } = await import('./flow');
    return { createFlow, IpcError };
  } else {
    // 在非 Tauri 环境中使用 Mock
    const { createFlow } = await import('./mock');
    // Mock 环境中返回一个简单的错误类
    class IpcError extends Error {
      constructor(
        message: string,
        public readonly command: string,
        public readonly cause?: unknown
      ) {
        super(message);
        this.name = 'IpcError';
      }
    }
    return { createFlow, IpcError };
  }
}

// 缓存服务实例
let servicePromise: ReturnType<typeof getService> | null = null;

/**
 * 获取服务实例（带缓存）
 */
async function getCachedService() {
  if (!servicePromise) {
    servicePromise = getService();
  }
  return servicePromise;
}

/**
 * 创建流程
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * 创建后按当前界面语言本地化入口/结束节点的默认名称
 * （后端默认名称为 "开始"/"End"，不随界面语言变化）
 *
 * @param name - 流程名称
 * @returns 创建的流程对象
 */
export async function createFlow(name: string): Promise<Flow> {
  const { createFlow: createFlowImpl } = await getCachedService();
  const flow = await createFlowImpl(name);
  return localizeDefaultNodeNames(flow);
}

/**
 * 按当前界面语言本地化入口/结束节点的默认名称
 */
function localizeDefaultNodeNames(flow: Flow): Flow {
  const ll = getLL();
  return {
    ...flow,
    nodes: flow.nodes.map((node) => {
      if (node.type === 'entry') return { ...node, name: ll.nodes.entry() };
      if (node.type === 'end') return { ...node, name: ll.nodes.end() };
      return node;
    }),
  };
}

/**
 * 执行流程并订阅日志
 * 使用 Channel API 接收后端日志事件、执行事件和 WS 流事件
 *
 * @param flow - 要执行的流程对象
 * @param onLog - 日志回调函数
 * @param onExecutionEvent - 执行事件回调函数（实时节点报告）
 * @param onWsEvent - WS 流事件回调函数（DebugEvent 中的 WS 变体）
 * @param variables - 可选的执行变量
 * @returns 执行结果
 */
export async function executeFlowWithChannel(
  flow: Flow,
  onLog: (event: import('@/types/events').LogEvent) => void,
  onExecutionEvent: (event: import('@/types/execution_event').ExecutionEvent) => void = () => {},
  onWsEvent: (event: import('@/types/debug').DebugEvent) => void = () => {},
  variables?: Record<string, unknown>
): Promise<ExecuteFlowCommandOutput> {
  // 检测是否在 Tauri 环境中
  if (isTauriEnv()) {
    const { executeFlowWithChannel: executeFlowWithChannelImpl } = await import('./flow');
    return executeFlowWithChannelImpl(flow, onLog, onExecutionEvent, onWsEvent, variables);
  } else {
    // Mock 环境
    const { executeFlowWithChannel: executeFlowWithChannelImpl } = await import('./mock');
    return executeFlowWithChannelImpl(flow, onLog, onExecutionEvent, onWsEvent, variables);
  }
}

// ============================================================================
// 报告存储命令
// ============================================================================

/**
 * 保存流程执行报告
 * @param report 流程执行报告
 * @param reportsDir 报告存储目录
 */
export async function saveFlowReport(report: FlowExecutionReport, reportsDir: string): Promise<void> {
  if (isTauriEnv()) {
    await invoke<void>('save_flow_report', { report, reportsDir });
  } else {
    const { saveFlowReport: saveFlowReportImpl } = await import('./mock');
    return saveFlowReportImpl(report, reportsDir);
  }
}

/**
 * 加载流程执行报告
 * @param uuid 报告 UUID
 * @param reportsDir 报告存储目录
 */
export async function loadFlowReport(uuid: string, reportsDir: string): Promise<FlowExecutionReport> {
  if (isTauriEnv()) {
    return await invoke<FlowExecutionReport>('load_flow_report', { uuid, reportsDir });
  } else {
    const { loadFlowReport: loadFlowReportImpl } = await import('./mock');
    return loadFlowReportImpl(uuid, reportsDir);
  }
}

/**
 * 列出所有报告元数据
 * @param reportsDir 报告存储目录
 */
export async function listFlowReports(reportsDir: string): Promise<ReportMetadata[]> {
  if (isTauriEnv()) {
    return await invoke<ReportMetadata[]>('list_flow_reports', { reportsDir });
  } else {
    const { listFlowReports: listFlowReportsImpl } = await import('./mock');
    return listFlowReportsImpl(reportsDir);
  }
}

/**
 * 删除流程执行报告
 * @param uuid 报告 UUID
 * @param reportsDir 报告存储目录
 */
export async function deleteFlowReport(uuid: string, reportsDir: string): Promise<void> {
  if (isTauriEnv()) {
    await invoke<void>('delete_flow_report', { uuid, reportsDir });
  } else {
    const { deleteFlowReport: deleteFlowReportImpl } = await import('./mock');
    return deleteFlowReportImpl(uuid, reportsDir);
  }
}

// ============================================================================
// 调试命令
// ============================================================================

/**
 * 启动调试会话
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @param flow - 要调试的流程对象
 * @param breakpoints - 断点列表（节点 UUID）
 * @param onEvent - 调试事件回调函数
 * @param onLog - 日志回调函数
 */
export async function startDebug(
  flow: import('@/types/flow').Flow,
  breakpoints: string[],
  onEvent: (event: import('@/types/debug').DebugEvent) => void,
  onLog: (event: import('@/types/events').LogEvent) => void
): Promise<void> {
  if (isTauriEnv()) {
    const { startDebug: startDebugImpl } = await import('./debug');
    return startDebugImpl(flow, breakpoints, onEvent, onLog);
  } else {
    // Mock 环境 - 暂不实现
    console.warn('[Mock] Debug features are not available in mock environment');
  }
}

/**
 * 发送调试命令
 *
 * @param command - 调试命令
 */
export async function sendDebugCommand(command: import('@/types/debug').DebugCommand): Promise<void> {
  if (isTauriEnv()) {
    const { sendDebugCommand: sendDebugCommandImpl } = await import('./debug');
    return sendDebugCommandImpl(command);
  } else {
    // Mock 环境 - 暂不实现
    console.warn('[Mock] Debug features are not available in mock environment');
  }
}

/**
 * 结束调试会话
 */
export async function endDebug(): Promise<void> {
  if (isTauriEnv()) {
    const { endDebug: endDebugImpl } = await import('./debug');
    return endDebugImpl();
  } else {
    // Mock 环境 - 暂不实现
    console.warn('[Mock] Debug features are not available in mock environment');
  }
}

/**
 * 获取调试会话状态
 *
 * 检查后端是否有活跃的调试会话
 * @returns true 如果有活跃的调试会话，false 否则
 */
export async function get_debug_status(): Promise<boolean> {
  if (isTauriEnv()) {
    const { get_debug_status: get_debug_statusImpl } = await import('./debug');
    return get_debug_statusImpl();
  } else {
    // Mock 环境 - 返回 false
    return false;
  }
}

/**
 * 强制结束调试会话
 *
 * 强制清理残留的调试会话状态（用于页面刷新后恢复）
 */
export async function force_end_debug(): Promise<void> {
  if (isTauriEnv()) {
    const { force_end_debug: force_end_debugImpl } = await import('./debug');
    return force_end_debugImpl();
  } else {
    // Mock 环境 - 暂不实现
    console.warn('[Mock] Debug features are not available in mock environment');
  }
}

// ============================================================================
// SSE 命令
// ============================================================================

export type { SseTestConnectionInput } from './sse';

/**
 * 测试 SSE 端点连接
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @param input - 测试连接请求参数
 * @returns true 如果 URL 格式有效
 */
export async function testSseConnection(
  input: import('./sse').SseTestConnectionInput
): Promise<boolean> {
  if (isTauriEnv()) {
    const { testSseConnection: testSseConnectionImpl } = await import('./sse');
    return testSseConnectionImpl(input);
  } else {
    const { testSseConnection: testSseConnectionImpl } = await import('./mock');
    return testSseConnectionImpl(input);
  }
}

// ============================================================================
// GraphQL 命令
// ============================================================================

export type { GraphqlIntrospectInput, GraphqlIntrospectOutput } from './graphql';

/**
 * GraphQL Schema 内省
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @param input - 内省请求参数
 * @returns 内省结果
 */
export async function introspectSchema(
  input: import('./graphql').GraphqlIntrospectInput
): Promise<import('./graphql').GraphqlIntrospectOutput> {
  if (isTauriEnv()) {
    const { introspectSchema: introspectSchemaImpl } = await import('./graphql');
    return introspectSchemaImpl(input);
  } else {
    const { introspectSchema: introspectSchemaImpl } = await import('./mock');
    return introspectSchemaImpl(input);
  }
}

// ============================================================================
// 压力测试命令
// ============================================================================

/**
 * 取消正在执行的流程
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @throws IpcError 如果取消失败
 */
export async function cancelFlow(): Promise<void> {
  if (isTauriEnv()) {
    const { cancelFlow: cancelFlowImpl } = await import('./flow');
    return cancelFlowImpl();
  } else {
    // Mock 环境 - 无操作
  }
}

/**
 * 获取指定节点的完整执行详情（冷通道按需拉取）
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @param flowUuid - 流程 UUID
 * @param nodeUuid - 节点 UUID
 * @returns 节点执行报告详情
 */
export async function getNodeExecutionDetail(
  flowUuid: string,
  nodeUuid: string
): Promise<NodeExecutionReport | null> {
  if (isTauriEnv()) {
    const { getNodeExecutionDetail: getNodeExecutionDetailImpl } = await import('./flow');
    return getNodeExecutionDetailImpl(flowUuid, nodeUuid);
  } else {
    const { getNodeExecutionDetail: getNodeExecutionDetailMock } = await import('./mock');
    return getNodeExecutionDetailMock(flowUuid, nodeUuid);
  }
}

/**
 * 验证流程结构
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @param flow - 要验证的流程
 * @returns 验证结果
 */
export async function validateFlow(flow: Flow): Promise<{ valid: boolean; errors: { code: string; message: string }[]; warnings: string[] }> {
  if (isTauriEnv()) {
    const { validateFlow: validateFlowImpl } = await import('./flow');
    return validateFlowImpl(flow);
  } else {
    const { validateFlow: validateFlowMock } = await import('./mock');
    return validateFlowMock();
  }
}

// ============================================================================
// 压力测试命令
// ============================================================================

/**
 * 启动压力测试
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @param flow - 要压测的流程对象
 * @param config - 压测配置
 * @param onEvent - 压测事件回调函数
 * @returns 取消监听的函数
 */
export async function startStressTest(
  flow: import('@/types/flow').Flow,
  config: import('@/types/stressTest').StressTestConfig,
  onEvent: (event: import('@/types/stressTest').StressTestEvent) => void
): Promise<() => void> {
  if (isTauriEnv()) {
    const { startStressTest: startStressTestImpl } = await import('./stressTest');
    return startStressTestImpl(flow, config, onEvent);
  } else {
    // Mock 环境 - 暂不实现
    console.warn('[Mock] Stress test features are not available in mock environment');
    return () => {};
  }
}

/**
 * 停止压力测试
 */
export async function stopStressTest(): Promise<void> {
  if (isTauriEnv()) {
    const { stopStressTest: stopStressTestImpl } = await import('./stressTest');
    return stopStressTestImpl();
  } else {
    // Mock 环境 - 暂不实现
    console.warn('[Mock] Stress test features are not available in mock environment');
  }
}

/**
 * 获取压力测试状态
 *
 * @returns 当前压测状态和进度信息
 */
export async function getStressTestStatus(): Promise<import('@/types/stressTest').GetStressTestStatusOutput> {
  if (isTauriEnv()) {
    const { getStressTestStatus: getStressTestStatusImpl } = await import('./stressTest');
    return getStressTestStatusImpl();
  } else {
    // Mock 环境 - 返回空闲状态
    return {
      status: 'idle',
      progress: undefined,
    };
  }
}

// ============================================================================
// TCP/UDP 录制命令
// ============================================================================

/**
 * 开始 TCP/UDP 录制
 * @param connectionId 连接 ID
 */
export async function startTcpRecording(connectionId: string): Promise<void> {
  if (isTauriEnv()) {
    await invoke<void>('start_tcp_recording', { connectionId });
  } else {
    const { startTcpRecording: startTcpRecordingImpl } = await import('./mock');
    return startTcpRecordingImpl(connectionId);
  }
}

/**
 * 停止 TCP/UDP 录制并返回录制结果
 * @param connectionId 连接 ID
 * @returns 录制数据
 */
export async function stopTcpRecording(connectionId: string): Promise<import('@/types/node').TcpUdpRecording> {
  if (isTauriEnv()) {
    return await invoke<import('@/types/node').TcpUdpRecording>('stop_tcp_recording', { connectionId });
  } else {
    const { stopTcpRecording: stopTcpRecordingImpl } = await import('./mock');
    return stopTcpRecordingImpl(connectionId);
  }
}

/**
 * 保存 TCP/UDP 录制到文件
 * @param recording 录制数据
 * @param dir 目标目录（可选）
 * @returns 保存的文件路径
 */
export async function saveTcpRecording(recording: import('@/types/node').TcpUdpRecording, dir?: string): Promise<string> {
  if (isTauriEnv()) {
    return await invoke<string>('save_tcp_recording', { recording, dir });
  } else {
    const { saveTcpRecording: saveTcpRecordingImpl } = await import('./mock');
    return saveTcpRecordingImpl(recording, dir);
  }
}

/**
 * 从文件加载 TCP/UDP 录制
 * @param path 文件路径
 * @returns 录制数据
 */
export async function loadTcpRecording(path: string): Promise<import('@/types/node').TcpUdpRecording> {
  if (isTauriEnv()) {
    return await invoke<import('@/types/node').TcpUdpRecording>('load_tcp_recording', { path });
  } else {
    const { loadTcpRecording: loadTcpRecordingImpl } = await import('./mock');
    return loadTcpRecordingImpl(path);
  }
}

/**
 * 列出目录中的 TCP/UDP 录制文件
 * @param dir 目录路径（可选）
 * @returns 录制文件路径列表
 */
export async function listTcpRecordings(dir?: string): Promise<string[]> {
  if (isTauriEnv()) {
    return await invoke<string[]>('list_tcp_recordings', { dir });
  } else {
    const { listTcpRecordings: listTcpRecordingsImpl } = await import('./mock');
    return listTcpRecordingsImpl(dir);
  }
}

// ============================================================================
// 数据集执行命令
// ============================================================================

import type {
  DatasetExecutionConfig,
  CsvInfo,
  DatasetExecutionReport,
  DatasetReportSummary,
  DatasetEvent,
} from '@/types/dataset';

/**
 * 验证 CSV 文件元信息
 */
export async function validateDatasetCsv(
  path: string,
  encoding?: string,
  delimiter?: string
): Promise<CsvInfo> {
  if (isTauriEnv()) {
    const { datasetService } = await import('../dataset');
    return datasetService.validateCsv(path, encoding, delimiter);
  } else {
    const { validateDatasetCsv: validateDatasetCsvImpl } = await import('./mock');
    return validateDatasetCsvImpl(path, encoding, delimiter);
  }
}

/**
 * 启动数据驱动执行
 * @param flow 流程对象
 * @param config 数据集执行配置
 * @param onEvent 事件回调函数
 * @returns 取消监听的函数
 */
export async function executeFlowWithDataset(
  flow: import('@/types/flow').Flow,
  config: DatasetExecutionConfig,
  onEvent: (event: DatasetEvent) => void
): Promise<() => void> {
  if (isTauriEnv()) {
    const { datasetService } = await import('../dataset');
    return datasetService.executeWithDataset(flow, config, onEvent);
  } else {
    const { executeFlowWithDataset: executeFlowWithDatasetImpl } = await import('./mock');
    await executeFlowWithDatasetImpl(flow, config, onEvent);
    return () => {};
  }
}

/**
 * 取消数据驱动执行
 */
export async function cancelDatasetExecution(): Promise<void> {
  if (isTauriEnv()) {
    const { datasetService } = await import('../dataset');
    return datasetService.cancelExecution();
  } else {
    const { cancelDatasetExecution: cancelDatasetExecutionImpl } = await import('./mock');
    return cancelDatasetExecutionImpl();
  }
}

/**
 * 列出数据集报告
 */
export async function listDatasetReports(reportsDir: string): Promise<DatasetReportSummary[]> {
  if (isTauriEnv()) {
    const { datasetService } = await import('../dataset');
    return datasetService.listReports(reportsDir);
  } else {
    const { listDatasetReports: listDatasetReportsImpl } = await import('./mock');
    return listDatasetReportsImpl(reportsDir);
  }
}

/**
 * 加载数据集报告
 */
export async function loadDatasetReport(
  reportId: string,
  reportsDir: string
): Promise<DatasetExecutionReport> {
  if (isTauriEnv()) {
    const { datasetService } = await import('../dataset');
    return datasetService.loadReport(reportId, reportsDir);
  } else {
    const { loadDatasetReport: loadDatasetReportImpl } = await import('./mock');
    return loadDatasetReportImpl(reportId, reportsDir);
  }
}

/**
 * 加载失败行详情
 */
export async function loadDatasetRowDetail(
  reportId: string,
  rowIndex: number,
  reportsDir: string
): Promise<unknown | null> {
  if (isTauriEnv()) {
    const { datasetService } = await import('../dataset');
    return datasetService.loadRowDetail(reportId, rowIndex, reportsDir);
  } else {
    const { loadDatasetRowDetail: loadDatasetRowDetailImpl } = await import('./mock');
    return loadDatasetRowDetailImpl(reportId, rowIndex, reportsDir);
  }
}

/**
 * 删除数据集报告
 */
export async function deleteDatasetReport(
  reportId: string,
  reportsDir: string
): Promise<void> {
  if (isTauriEnv()) {
    const { datasetService } = await import('../dataset');
    return datasetService.deleteReport(reportId, reportsDir);
  } else {
    const { deleteDatasetReport: deleteDatasetReportImpl } = await import('./mock');
    return deleteDatasetReportImpl(reportId, reportsDir);
  }
}

/**
 * 保存数据集报告到磁盘
 */
export async function saveDatasetReport(report: DatasetExecutionReport, reportsDir: string): Promise<string> {
  if (isTauriEnv()) {
    const { datasetService } = await import('../dataset');
    return datasetService.saveReport(report, reportsDir);
  } else {
    const { saveDatasetReport: saveDatasetReportImpl } = await import('./mock');
    return saveDatasetReportImpl(report, reportsDir);
  }
}
