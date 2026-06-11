/**
 * Tauri IPC 服务主导出文件
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 */

import type { Flow } from '@/types/flow';
import type { ExecuteFlowCommandOutput } from '@/types/ipc';
import { invoke } from '@tauri-apps/api/core';
import type { FlowExecutionReport } from '@/types/report';
import type { ReportMetadata } from '@/types/ipc';

// 检测是否在 Tauri 环境中运行
function isTauriEnv(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}

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
 * @param name - 流程名称
 * @returns 创建的流程对象
 */
export async function createFlow(name: string): Promise<Flow> {
  const { createFlow: createFlowImpl } = await getCachedService();
  return createFlowImpl(name);
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
  return await invoke<void>('save_flow_report', { report, reportsDir });
}

/**
 * 加载流程执行报告
 * @param uuid 报告 UUID
 * @param reportsDir 报告存储目录
 */
export async function loadFlowReport(uuid: string, reportsDir: string): Promise<FlowExecutionReport> {
  return await invoke<FlowExecutionReport>('load_flow_report', { uuid, reportsDir });
}

/**
 * 列出所有报告元数据
 * @param reportsDir 报告存储目录
 */
export async function listFlowReports(reportsDir: string): Promise<ReportMetadata[]> {
  return await invoke<ReportMetadata[]>('list_flow_reports', { reportsDir });
}

/**
 * 删除流程执行报告
 * @param uuid 报告 UUID
 * @param reportsDir 报告存储目录
 */
export async function deleteFlowReport(uuid: string, reportsDir: string): Promise<void> {
  return await invoke<void>('delete_flow_report', { uuid, reportsDir });
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
// gRPC 命令
// ============================================================================

export type {
  GrpcDiscoverServicesInput,
  GrpcDiscoverServicesOutput,
  GrpcGenerateRequestSkeletonInput,
  GrpcGenerateRequestSkeletonOutput,
  GrpcScanProtoDirectoryOutput,
  GrpcParseDescriptorSetOutput,
  GrpcReadProtoFilesOutput,
  ProtoFileOutput,
  ValidationError,
  ValidationResult,
  GrpcValidateRequestOutput,
} from './grpc';

/**
 * 发现 gRPC 服务
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @param input - 服务发现请求参数
 * @returns 服务发现结果
 */
export async function discoverGrpcServices(
  input: import('./grpc').GrpcDiscoverServicesInput
): Promise<import('./grpc').GrpcDiscoverServicesOutput> {
  if (isTauriEnv()) {
    const { discoverGrpcServices: discoverGrpcServicesImpl } = await import('./grpc');
    return discoverGrpcServicesImpl(input);
  } else {
    const { discoverGrpcServices: discoverGrpcServicesImpl } = await import('./mock');
    return discoverGrpcServicesImpl(input);
  }
}

/**
 * 生成 gRPC 请求消息的 JSON 骨架
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @param input - 骨架生成请求参数
 * @returns 骨架生成结果
 */
export async function generateGrpcRequestSkeleton(
  input: import('./grpc').GrpcGenerateRequestSkeletonInput
): Promise<import('./grpc').GrpcGenerateRequestSkeletonOutput> {
  if (isTauriEnv()) {
    const { generateGrpcRequestSkeleton: generateGrpcRequestSkeletonImpl } = await import('./grpc');
    return generateGrpcRequestSkeletonImpl(input);
  } else {
    const { generateGrpcRequestSkeleton: generateGrpcRequestSkeletonImpl } = await import('./mock');
    return generateGrpcRequestSkeletonImpl(input);
  }
}

/**
 * 通过 Server Reflection 生成 gRPC 请求消息的 JSON 骨架
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @param input - Reflection 骨架生成请求参数
 * @returns 骨架生成结果
 */
export async function generateGrpcRequestSkeletonFromReflection(
  input: import('./grpc').GrpcGenerateRequestSkeletonFromReflectionInput
): Promise<import('./grpc').GrpcGenerateRequestSkeletonOutput> {
  if (isTauriEnv()) {
    const { generateGrpcRequestSkeletonFromReflection: impl } = await import('./grpc');
    return impl(input);
  } else {
    const { generateGrpcRequestSkeletonFromReflection: impl } = await import('./mock');
    return impl(input);
  }
}

/**
 * 扫描目录，发现所有 .proto 文件
 * 仅在 Tauri 环境中可用
 *
 * @param dirPath - 目录路径
 * @returns 扫描结果（文件列表和 import 路径）
 */
export async function scanProtoDirectory(
  dirPath: string
): Promise<import('./grpc').GrpcScanProtoDirectoryOutput> {
  if (isTauriEnv()) {
    const { scanProtoDirectory: scanProtoDirectoryImpl } = await import('./grpc');
    return scanProtoDirectoryImpl(dirPath);
  } else {
    // Mock 环境返回空结果
    return { success: false, files: [], import_path: null, error: 'Mock environment does not support directory scanning' };
  }
}

/**
 * 读取多个 .proto 文件的内容
 * 仅在 Tauri 环境中可用
 *
 * @param filePaths - 文件路径列表
 * @returns 文件列表（路径和内容）
 */
export async function readProtoFiles(
  filePaths: string[]
): Promise<import('./grpc').GrpcReadProtoFilesOutput> {
  if (isTauriEnv()) {
    const { readProtoFiles: readProtoFilesImpl } = await import('./grpc');
    return readProtoFilesImpl(filePaths);
  } else {
    return { success: false, files: [], error: 'Mock environment does not support file reading' };
  }
}

/**
 * 解析 FileDescriptorSet 二进制文件
 * 仅在 Tauri 环境中可用
 *
 * @param filePath - 文件路径
 * @returns 服务发现结果
 */
export async function parseDescriptorSet(
  filePath: string
): Promise<import('./grpc').GrpcParseDescriptorSetOutput> {
  if (isTauriEnv()) {
    const { parseDescriptorSet: parseDescriptorSetImpl } = await import('./grpc');
    return parseDescriptorSetImpl(filePath);
  } else {
    // Mock 环境返回空结果
    return { success: false, services: [], error: 'Mock environment does not support Descriptor Set parsing' };
  }
}

/**
 * 验证请求 JSON 是否符合 proto 消息定义
 * 仅在 Tauri 环境中可用
 *
 * @param json - 待验证的 JSON 字符串
 * @param messageName - 消息全名
 * @param protoFiles - Proto 文件列表
 * @returns 验证结果
 */
export async function validateGrpcRequest(
  json: string,
  messageName: string,
  protoFiles?: [string, string][]
): Promise<import('./grpc').GrpcValidateRequestOutput> {
  if (isTauriEnv()) {
    const { validateGrpcRequest: validateGrpcRequestImpl } = await import('./grpc');
    return validateGrpcRequestImpl(json, messageName, protoFiles);
  } else {
    // Mock 环境 — 仅做 JSON 语法检查
    try {
      const parsed = JSON.parse(json);
      if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
        return {
          success: true,
          result: { valid: false, errors: [{ path: '', message: 'Expected a JSON object' }] },
          error: null,
        };
      }
      return { success: true, result: { valid: true, errors: [] }, error: null };
    } catch (e) {
      return {
        success: true,
        result: { valid: false, errors: [{ path: '', message: `JSON syntax error: ${e}` }] },
        error: null,
      };
    }
  }
}

// ============================================================================
// MCP 命令
// ============================================================================

export type { McpToolInfo, McpResourceInfo, McpServerInfo, McpDiscoverToolsOutput, McpDiscoverToolsInput } from './mcp';

/**
 * 发现 MCP 服务器的工具和资源
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @param input - MCP 发现工具输入
 * @returns MCP 工具和资源列表
 */
export async function discoverMcpTools(
  input: import('./mcp').McpDiscoverToolsInput
): Promise<import('./mcp').McpDiscoverToolsOutput> {
  if (isTauriEnv()) {
    const { discoverMcpTools: discoverMcpToolsImpl } = await import('./mcp');
    return discoverMcpToolsImpl(input);
  } else {
    const { discoverMcpTools: discoverMcpToolsImpl } = await import('./mock');
    return discoverMcpToolsImpl(input);
  }
}

// ============================================================================
// MQTT 命令
// ============================================================================

export type {
  MqttTestConnectionInput,
  MqttBrokerInfo,
  MqttTestConnectionOutput,
} from './mqtt';

/**
 * 测试 MQTT Broker 连接
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @param input - 测试连接请求参数
 * @returns 测试连接结果
 */
export async function testMqttConnection(
  input: import('./mqtt').MqttTestConnectionInput
): Promise<import('./mqtt').MqttTestConnectionOutput> {
  if (isTauriEnv()) {
    const { testMqttConnection: testMqttConnectionImpl } = await import('./mqtt');
    return testMqttConnectionImpl(input);
  } else {
    const { testMqttConnection: testMqttConnectionImpl } = await import('./mock');
    return testMqttConnectionImpl(input);
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
// AMQP 命令
// ============================================================================

export type { AmqpTestConnectionInput } from './amqp';

/**
 * 测试 AMQP Broker 连接
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @param input - 测试连接请求参数
 * @returns true 如果 URL 格式有效
 */
export async function testAmqpConnection(
  input: import('./amqp').AmqpTestConnectionInput
): Promise<boolean> {
  if (isTauriEnv()) {
    const { testAmqpConnection: testAmqpConnectionImpl } = await import('./amqp');
    return testAmqpConnectionImpl(input);
  } else {
    const { testAmqpConnection: testAmqpConnectionImpl } = await import('./mock');
    return testAmqpConnectionImpl(input);
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

// ============================================================================
// 更新命令
// ============================================================================

/**
 * 检查更新
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @returns 更新信息或 null
 */
export async function checkForUpdates(): Promise<import('@/types/updater').UpdateInfo | null> {
  if (isTauriEnv()) {
    const { checkForUpdates: checkForUpdatesImpl } = await import('./updater');
    return checkForUpdatesImpl();
  } else {
    const { checkForUpdates: checkForUpdatesImpl } = await import('./updater.mock');
    return checkForUpdatesImpl();
  }
}

/**
 * 下载并安装更新
 * 根据运行环境自动选择真实 IPC 或 Mock 服务
 *
 * @param onEvent - 下载进度事件回调
 */
export async function downloadAndInstallUpdate(
  onEvent?: (event: import('@/types/updater').DownloadEvent) => void
): Promise<void> {
  if (isTauriEnv()) {
    const { downloadAndInstallUpdate: downloadAndInstallUpdateImpl } = await import('./updater');
    return downloadAndInstallUpdateImpl(onEvent);
  } else {
    const { downloadAndInstallUpdate: downloadAndInstallUpdateImpl } = await import('./updater.mock');
    return downloadAndInstallUpdateImpl();
  }
}
