/**
 * Mock 服务 - 用于非 Tauri 环境测试
 * 模拟后端 IPC 命令行为
 */

import type { Flow } from '@/types/flow';
import type { ExecuteFlowCommandOutput } from '@/types/ipc';
import type { FlowExecutionReport } from '@/types/report';
import type { LogEvent } from '@/types/events';
import type { ExecutionEvent } from '@/types/execution_event';

/**
 * 生成 UUID v4
 */
function generateUUID(): string {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

/**
 * Mock 创建流程
 * 模拟后端 create_flow 命令行为
 *
 * @param name - 流程名称
 * @returns 创建的流程对象
 */
export async function createFlow(name: string): Promise<Flow> {
  const flowId = generateUUID();

  return {
    uuid: flowId,
    name,
    nodes: [
      {
        uuid: `start-${flowId}`,
        type: 'entry',
        name: '开始',
        position: { x: 0, y: 0 },
        data: { type: 'entry' },
      },
    ],
    connections: [],
    initial_variables: {},
  };
}

/**
 * Mock 执行流程并订阅日志
 * 模拟执行过程中发送日志事件和执行事件
 *
 * @param flow - 要执行的流程对象
 * @param onLog - 日志回调函数
 * @param onExecutionEvent - 执行事件回调函数（实时节点报告）
 * @param _onWsEvent - WS 流事件回调函数（Mock 中未使用）
 * @param _variables - 可选的执行变量（Mock 中未使用）
 * @returns 执行结果
 */
export async function executeFlowWithChannel(
  flow: Flow,
  onLog: (event: LogEvent) => void,
  onExecutionEvent: (event: ExecutionEvent) => void = () => {},
  _onWsEvent: (event: import('@/types/debug').DebugEvent) => void = () => {},
  _variables?: Record<string, unknown>
): Promise<ExecuteFlowCommandOutput> {
  const startTime = Date.now();

  // 发送流程开始日志
  onLog({
    level: 'INFO',
    message: `流程 '${flow.name}' 开始执行`,
    timestamp: startTime,
    flowUuid: flow.uuid,
  });

  // 发送 FlowStarted 执行事件
  onExecutionEvent({
    flowStarted: { flowUuid: flow.uuid, flowName: flow.name, totalNodes: flow.nodes.length },
  });

  // 查找开始节点
  const startNode = flow.nodes.find((node) => node.type === 'entry');

  // 验证开始节点存在
  if (!startNode) {
    onLog({
      level: 'ERROR',
      message: '流程缺少开始节点',
      timestamp: Date.now(),
      flowUuid: flow.uuid,
    });
    return {
      success: false,
      error: '流程缺少开始节点',
      execution_time: Date.now() - startTime,
    };
  }

  // 发送节点开始日志
  onLog({
    level: 'INFO',
    message: `执行节点 ${startNode.name} (entry)`,
    timestamp: Date.now(),
    flowUuid: flow.uuid,
    nodeUuid: startNode.uuid,
    nodeName: startNode.name,
  });

  // 模拟节点执行延迟
  await new Promise((resolve) => setTimeout(resolve, 100));

  // 发送节点完成日志
  onLog({
    level: 'INFO',
    message: `节点 ${startNode.name} 执行完成`,
    timestamp: Date.now(),
    flowUuid: flow.uuid,
    nodeUuid: startNode.uuid,
    nodeName: startNode.name,
  });

  // 发送 NodeCompleted 执行事件
  const nodeEndTime = Date.now();
  onExecutionEvent({
    nodeCompleted: {
      report: {
        node_uuid: startNode.uuid,
        node_type: startNode.type,
        node_name: startNode.name,
        start_time: startTime,
        end_time: nodeEndTime,
        duration_ms: nodeEndTime - startTime,
        status: 'success',
        input_data: null,
        output_data: null,
      },
    },
  });

  // 检查开始节点是否有输出连接
  const hasConnection = flow.connections.some(
    (conn) => conn.source_node_uuid === startNode.uuid
  );

  if (!hasConnection) {
    onLog({
      level: 'INFO',
      message: `节点 ${startNode.name} 没有下一个节点，执行完成`,
      timestamp: Date.now(),
      flowUuid: flow.uuid,
      nodeUuid: startNode.uuid,
      nodeName: startNode.name,
    });

    const endTime = Date.now();
    onLog({
      level: 'INFO',
      message: '流程执行成功',
      timestamp: endTime,
      flowUuid: flow.uuid,
    });

    // 生成模拟的执行报告（单个 Entry 节点无连接是正常终止）
    const flowReport: FlowExecutionReport = {
      uuid: `report-${Date.now()}`,
      flow_uuid: flow.uuid,
      flow_name: flow.name,
      start_time: startTime,
      end_time: endTime,
      duration_ms: endTime - startTime,
      total_nodes: flow.nodes.length,
      executed_count: 1,
      status: 'success',
      node_reports: [{
        node_uuid: startNode.uuid,
        node_type: startNode.type,
        node_name: startNode.name,
        start_time: startTime,
        end_time: endTime,
        duration_ms: endTime - startTime,
        status: 'success',
        input_data: null,
        output_data: null,
      }],
      statistics: {
        success_count: 1,
        failed_count: 0,
        avg_node_duration_ms: endTime - startTime,
        max_node_duration_ms: endTime - startTime,
        min_node_duration_ms: endTime - startTime,
        completion_rate: 1,
        error_rate: 0,
      },
    };

    // 发送 FlowCompleted 执行事件
    onExecutionEvent({
      flowCompleted: { report: flowReport },
    });

    return {
      success: true,
      flow_report: flowReport,
      execution_time: endTime - startTime,
    };
  }

  // 模拟执行后续节点
  let executedCount = 1; // 已执行开始节点
  const nodeReports: FlowExecutionReport['node_reports'] = [
    {
      node_uuid: startNode.uuid,
      node_type: startNode.type,
      node_name: startNode.name,
      start_time: startTime,
      end_time: startTime + 100,
      duration_ms: 100,
      status: 'success',
      input_data: null,
      output_data: { result: 'ok' },
    },
  ];

  // 遍历并模拟执行其他节点
  for (let i = 0; i < flow.nodes.length; i++) {
    const node = flow.nodes[i];
    if (node.type === 'entry') continue; // 跳过已执行的开始节点

    const nodeStartTime = Date.now();

    // 发送节点开始日志
    onLog({
      level: 'INFO',
      message: `执行节点 ${node.name} (${node.type})`,
      timestamp: nodeStartTime,
      flowUuid: flow.uuid,
      nodeUuid: node.uuid,
      nodeName: node.name,
    });

    // 模拟节点执行延迟
    await new Promise((resolve) => setTimeout(resolve, 50 + Math.random() * 100));

    const nodeEndTime = Date.now();

    // 发送节点完成日志
    onLog({
      level: 'INFO',
      message: `节点 ${node.name} 执行完成`,
      timestamp: nodeEndTime,
      flowUuid: flow.uuid,
      nodeUuid: node.uuid,
      nodeName: node.name,
    });

    executedCount++;
    const nodeReport: FlowExecutionReport['node_reports'][number] = {
      node_uuid: node.uuid,
      node_type: node.type,
      node_name: node.name,
      start_time: nodeStartTime,
      end_time: nodeEndTime,
      duration_ms: nodeEndTime - nodeStartTime,
      status: 'success',
      input_data: { from: flow.nodes[i - 1]?.uuid },
      output_data: { result: 'ok' },
    };
    nodeReports.push(nodeReport);

    // 发送 NodeCompleted 执行事件
    onExecutionEvent({
      nodeCompleted: { report: nodeReport },
    });
  }

  const endTime = Date.now();

  // 发送流程完成日志
  onLog({
    level: 'INFO',
    message: '流程执行成功',
    timestamp: endTime,
    flowUuid: flow.uuid,
  });

  // 生成模拟的执行报告
  const flowReport: FlowExecutionReport = {
    uuid: `report-${Date.now()}`,
    flow_uuid: flow.uuid,
    flow_name: flow.name,
    start_time: startTime,
    end_time: endTime,
    duration_ms: endTime - startTime,
    total_nodes: flow.nodes.length,
    executed_count: executedCount,
    status: 'success',
    node_reports: nodeReports,
    statistics: {
      success_count: executedCount,
      failed_count: 0,
      avg_node_duration_ms: Math.round((endTime - startTime) / executedCount),
      max_node_duration_ms: Math.max(...nodeReports.map((r) => r.duration_ms)),
      min_node_duration_ms: Math.min(...nodeReports.map((r) => r.duration_ms)),
      completion_rate: 1,
      error_rate: 0,
    },
  };

  // 发送 FlowCompleted 执行事件
  onExecutionEvent({
    flowCompleted: { report: flowReport },
  });

  return {
    success: true,
    flow_report: flowReport,
    execution_time: endTime - startTime,
  };
}

/**
 * Mock 取消流程
 * 模拟后端 cancel_flow 命令行为
 */
export async function cancelFlow(): Promise<void> {
  // Mock 环境 - 无操作
}

/**
 * Mock 获取节点配置 JSON Schema
 * 模拟后端 get_node_config_schema 命令行为
 *
 * @param _typeId - 节点类型 ID（Mock 中未使用）
 * @returns null（Mock 环境不提供真实 schema）
 */
export async function getNodeConfigSchema(_typeId: string): Promise<null> {
  return null;
}

/**
 * Mock 获取所有已注册插件列表
 * 模拟后端 list_registered_plugins 命令行为
 *
 * @returns 空数组（Mock 环境不提供真实插件数据）
 */
export async function listRegisteredPlugins(): Promise<[]> {
  return [];
}

/**
 * Mock 发现 MCP 工具
 * 模拟后端 mcp_discover_tools 命令行为
 *
 * @param _input - MCP 发现工具输入（Mock 中未使用）
 * @returns 模拟的 MCP 工具和资源列表
 */
/**
 * Mock 发现 gRPC 服务
 * 模拟后端 grpc_discover_services 命令行为
 *
 * @param _input - gRPC 服务发现请求参数（Mock 中未使用）
 * @returns 模拟的 gRPC 服务发现结果
 */
export async function discoverGrpcServices(
  _input: import('./grpc').GrpcDiscoverServicesInput
): Promise<import('./grpc').GrpcDiscoverServicesOutput> {
  // 模拟网络延迟
  await new Promise((resolve) => setTimeout(resolve, 300));

  return {
    success: true,
    services: [
      {
        service_name: 'grpc.health.v1.Health',
        methods: [
          {
            method_name: 'Check',
            input_type: 'grpc.health.v1.HealthCheckRequest',
            output_type: 'grpc.health.v1.HealthCheckResponse',
            is_server_streaming: false,
            is_client_streaming: false,
          },
          {
            method_name: 'Watch',
            input_type: 'grpc.health.v1.HealthCheckRequest',
            output_type: 'grpc.health.v1.HealthCheckResponse',
            is_server_streaming: true,
            is_client_streaming: false,
          },
        ],
      },
      {
        service_name: 'example.Greeter',
        methods: [
          {
            method_name: 'SayHello',
            input_type: 'example.HelloRequest',
            output_type: 'example.HelloReply',
            is_server_streaming: false,
            is_client_streaming: false,
          },
          {
            method_name: 'SayHelloStream',
            input_type: 'example.HelloRequest',
            output_type: 'example.HelloReply',
            is_server_streaming: true,
            is_client_streaming: false,
          },
          {
            method_name: 'StreamGreetings',
            input_type: 'example.HelloRequest',
            output_type: 'example.HelloReply',
            is_server_streaming: true,
            is_client_streaming: true,
          },
        ],
      },
    ],
    error: null,
  };
}

/**
 * Mock 生成 gRPC 请求骨架
 * 模拟后端 grpc_generate_request_skeleton 命令行为
 *
 * @param _input - 骨架生成请求参数（Mock 中未使用）
 * @returns 模拟的骨架生成结果
 */
export async function generateGrpcRequestSkeleton(
  _input: import('./grpc').GrpcGenerateRequestSkeletonInput
): Promise<import('./grpc').GrpcGenerateRequestSkeletonOutput> {
  // 模拟网络延迟
  await new Promise((resolve) => setTimeout(resolve, 200));

  return {
    success: true,
    skeleton: JSON.stringify({ field: '' }, null, 2),
    error: null,
  };
}

export async function discoverMcpTools(
  _input: import('./mcp').McpDiscoverToolsInput
): Promise<import('./mcp').McpDiscoverToolsOutput> {
  return {
    success: true,
    tools: [
      {
        name: 'mock_tool',
        description: 'Mock MCP tool for testing',
        input_schema: { type: 'object', properties: {} },
      },
    ],
    resources: [
      {
        uri: 'mock://resource',
        name: 'Mock Resource',
        description: 'Mock MCP resource for testing',
        mime_type: 'text/plain',
      },
    ],
    prompts: [
      { name: 'example_prompt', description: '示例提示词', arguments: [{ name: 'topic', description: '主题', required: true }] }
    ],
    server_info: {
      name: 'Mock MCP Server',
      version: '1.0.0',
    },
  };
}

// ============================================================================
// MQTT Mock
// ============================================================================

/**
 * Mock 测试 MQTT Broker 连接
 * 模拟后端 mqtt_test_connection 命令行为
 *
 * @param _input - MQTT 测试连接请求参数（Mock 中未使用）
 * @returns 模拟的连接测试结果
 */
export async function testMqttConnection(
  _input: import('./mqtt').MqttTestConnectionInput
): Promise<import('./mqtt').MqttTestConnectionOutput> {
  await new Promise((resolve) => setTimeout(resolve, 200));

  return {
    success: true,
    broker_info: {
      version: 'MQTT 5.0',
      session_present: false,
      assigned_client_id: 'mock-client-001',
      server_keep_alive: 60,
      topic_alias_max: 10,
      receive_max: 20,
      shared_subscription_available: true,
      wildcard_sub_available: true,
    },
  };
}

// ============================================================================
// SSE Mock
// ============================================================================

/**
 * Mock 测试 SSE 连接
 * 模拟后端 sse_test_connection 命令行为
 *
 * @param _input - SSE 测试连接请求参数（Mock 中未使用）
 * @returns 模拟的连接测试结果
 */
export async function testSseConnection(
  _input: import('./sse').SseTestConnectionInput
): Promise<boolean> {
  await new Promise((resolve) => setTimeout(resolve, 200));
  return true;
}

// ============================================================================
// AMQP Mock
// ============================================================================

/**
 * Mock 测试 AMQP 连接
 * 模拟后端 amqp_test_connection 命令行为
 *
 * @param _input - AMQP 测试连接请求参数（Mock 中未使用）
 * @returns 模拟的连接测试结果
 */
export async function testAmqpConnection(
  _input: import('./amqp').AmqpTestConnectionInput
): Promise<boolean> {
  await new Promise((resolve) => setTimeout(resolve, 200));
  return true;
}

// ============================================================================
// GraphQL Mock
// ============================================================================

/**
 * Mock GraphQL Schema 内省
 * 模拟后端 graphql_introspect_schema 命令行为
 *
 * @param _input - 内省请求参数（Mock 中未使用）
 * @returns 模拟的内省结果
 */
export async function introspectSchema(
  _input: import('./graphql').GraphqlIntrospectInput
): Promise<import('./graphql').GraphqlIntrospectOutput> {
  await new Promise((resolve) => setTimeout(resolve, 300));

  return {
    success: true,
    schema: {
      __schema: {
        queryType: { name: 'Query' },
        mutationType: { name: 'Mutation' },
        subscriptionType: { name: 'Subscription' },
        types: [
          {
            kind: 'OBJECT',
            name: 'Query',
            description: '根查询类型',
            fields: [
              {
                name: 'user',
                description: '获取用户信息',
                type: { kind: 'OBJECT', name: 'User', ofType: null },
                args: [
                  { name: 'id', description: '用户 ID', type: { kind: 'NON_NULL', name: null, ofType: { kind: 'SCALAR', name: 'ID' } }, defaultValue: null },
                ],
              },
            ],
            inputFields: null,
            enumValues: null,
          },
          {
            kind: 'OBJECT',
            name: 'User',
            description: '用户类型',
            fields: [
              { name: 'id', description: '用户 ID', type: { kind: 'NON_NULL', name: null, ofType: { kind: 'SCALAR', name: 'ID' } }, args: [] },
              { name: 'name', description: '用户名', type: { kind: 'SCALAR', name: 'String', ofType: null }, args: [] },
              { name: 'email', description: '邮箱', type: { kind: 'SCALAR', name: 'String', ofType: null }, args: [] },
            ],
            inputFields: null,
            enumValues: null,
          },
        ],
      },
    },
    error: null,
  };
}

// ============================================================================
// TCP/UDP 录制 Mock
// ============================================================================

/** 模拟录制数据 */
function createMockRecording(): import('@/types/node').TcpUdpRecording {
  return {
    id: 'rec_mock_001',
    name: '模拟 TCP 录制',
    created_at: Date.now(),
    entries: [
      {
        timestamp_ms: 0,
        direction: 'send',
        connection_id: 'conn-mock-1',
        data_base64: btoa('Hello, TCP Server!'),
        data_size: 18,
        metadata: {},
      },
      {
        timestamp_ms: 50,
        direction: 'receive',
        connection_id: 'conn-mock-1',
        data_base64: btoa('Hello, Client!'),
        data_size: 15,
        metadata: {},
      },
      {
        timestamp_ms: 120,
        direction: 'send',
        connection_id: 'conn-mock-1',
        data_base64: btoa('GET /api/status HTTP/1.1\r\nHost: example.com\r\n\r\n'),
        data_size: 46,
        metadata: {},
      },
      {
        timestamp_ms: 200,
        direction: 'receive',
        connection_id: 'conn-mock-1',
        data_base64: btoa('HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK'),
        data_size: 38,
        metadata: {},
      },
    ],
    duration_ms: 250,
  };
}

/**
 * Mock 停止 TCP 录制
 * @param _connectionId 连接 ID（Mock 中未使用）
 */
export async function stopTcpRecording(_connectionId: string): Promise<import('@/types/node').TcpUdpRecording> {
  await new Promise(resolve => setTimeout(resolve, 100));
  return createMockRecording();
}

/**
 * Mock 保存 TCP 录制
 * @param _recording 录制数据（Mock 中未使用）
 * @param _dir 目标目录（Mock 中未使用）
 */
export async function saveTcpRecording(_recording: import('@/types/node').TcpUdpRecording, _dir?: string): Promise<string> {
  return '/mock/recordings/rec_mock_001.json';
}

/**
 * Mock 加载 TCP 录制
 * @param _path 文件路径（Mock 中未使用）
 */
export async function loadTcpRecording(_path: string): Promise<import('@/types/node').TcpUdpRecording> {
  return createMockRecording();
}

/**
 * Mock 列出 TCP 录制文件
 * @param _dir 目录路径（Mock 中未使用）
 */
export async function listTcpRecordings(_dir?: string): Promise<string[]> {
  return [];
}

// ============================================================================
// 数据集执行 Mock
// ============================================================================

import type {
  DatasetExecutionConfig,
  CsvInfo,
  DatasetExecutionReport,
  DatasetReportSummary,
  DatasetEvent,
} from '@/types/dataset';

/**
 * Mock 验证 CSV 文件
 * 模拟后端 validate_dataset_csv 命令行为
 *
 * @param _path CSV 文件路径（Mock 中未使用）
 * @param _encoding 编码（Mock 中未使用）
 * @param _delimiter 分隔符（Mock 中未使用）
 */
export async function validateDatasetCsv(
  _path: string,
  _encoding?: string,
  _delimiter?: string
): Promise<CsvInfo> {
  await new Promise((resolve) => setTimeout(resolve, 100));
  return {
    row_count: 3,
    column_names: ['name', 'email', 'age'],
    detected_encoding: 'utf-8',
  };
}

/**
 * Mock 启动数据驱动执行
 * 模拟发送 started → row_completed × 3 → progress × 3 → ended 事件序列
 *
 * @param _flow 流程对象（Mock 中未使用）
 * @param _config 数据集执行配置（Mock 中未使用）
 * @param onEvent 事件回调函数
 */
export async function executeFlowWithDataset(
  _flow: import('@/types/flow').Flow,
  _config: DatasetExecutionConfig,
  onEvent: (event: DatasetEvent) => void
): Promise<void> {
  const totalRows = 3;
  const datasetName = 'mock_data.csv';

  // 发送 Started 事件
  onEvent({ type: 'started', total_rows: totalRows, dataset_name: datasetName });

  for (let i = 0; i < totalRows; i++) {
    await new Promise((resolve) => setTimeout(resolve, 50));

    // 发送 RowCompleted 事件
    onEvent({
      type: 'row_completed',
      row_index: i,
      status: 'success',
      duration_ms: 50 + Math.round(Math.random() * 50),
    });

    // 发送 Progress 事件
    onEvent({
      type: 'progress',
      completed: i + 1,
      total: totalRows,
      success_count: i + 1,
      fail_count: 0,
      error_count: 0,
      elapsed_ms: (i + 1) * 80,
      estimated_remaining_ms: (totalRows - i - 1) * 80,
    });
  }

  // 发送 Ended 事件
  const now = Date.now();
  onEvent({
    type: 'ended',
    report: {
      uuid: `mock-report-${now}`,
      flow_uuid: 'mock-flow-uuid',
      flow_name: 'Mock 测试流程',
      dataset_path: '/mock/data.csv',
      dataset_name: datasetName,
      encoding: 'utf-8',
      executed_at: now,
      total_duration_ms: totalRows * 80,
      total_rows: totalRows,
      success_count: totalRows,
      fail_count: 0,
      error_count: 0,
      avg_duration_ms: 75,
      p50_duration_ms: 70,
      p95_duration_ms: 100,
      p99_duration_ms: 100,
      row_summaries: Array.from({ length: totalRows }, (_, i) => ({
        row_index: i,
        input_data: { name: `user_${i}`, email: `user${i}@test.com`, age: String(20 + i) },
        status: 'success' as const,
        duration_ms: 50 + i * 10,
        has_detail_report: false,
      })),
    },
  });
}

/**
 * Mock 取消数据驱动执行
 */
export async function cancelDatasetExecution(): Promise<void> {
  // Mock 环境 - 无操作
}

/**
 * Mock 列出数据集报告
 */
export async function listDatasetReports(
  _reportsDir: string
): Promise<DatasetReportSummary[]> {
  return [
    {
      report_id: 'mock-report-001',
      flow_name: '测试流程 1',
      dataset_name: 'users.csv',
      executed_at: Date.now() - 86400000,
      total_rows: 100,
      success_count: 95,
      fail_count: 3,
      error_count: 2,
    },
  ];
}

/**
 * Mock 加载数据集报告
 */
export async function loadDatasetReport(
  _reportId: string,
  _reportsDir: string
): Promise<DatasetExecutionReport> {
  const now = Date.now();
  return {
    uuid: 'mock-report-001',
    flow_uuid: 'mock-flow-001',
    flow_name: '测试流程 1',
    dataset_path: '/data/users.csv',
    dataset_name: 'users.csv',
    encoding: 'utf-8',
    executed_at: now - 86400000,
    total_duration_ms: 5000,
    total_rows: 100,
    success_count: 95,
    fail_count: 3,
    error_count: 2,
    avg_duration_ms: 50,
    p50_duration_ms: 45,
    p95_duration_ms: 80,
    p99_duration_ms: 95,
    row_summaries: Array.from({ length: 5 }, (_, i) => ({
      row_index: i,
      input_data: { id: String(i + 1), name: `user_${i}` },
      status: i < 4 ? ('success' as const) : ('fail' as const),
      duration_ms: 40 + i * 5,
      error_message: i >= 4 ? '断言失败' : undefined,
      has_detail_report: i >= 4,
    })),
  };
}

/**
 * Mock 加载失败行详情
 */
export async function loadDatasetRowDetail(
  _reportId: string,
  _rowIndex: number,
  _reportsDir: string
): Promise<unknown | null> {
  return {
    uuid: 'mock-flow-report-001',
    flow_uuid: 'mock-flow-001',
    flow_name: '测试流程 1',
    status: 'failed',
    error: '断言失败: 状态码期望 200，实际 500',
  };
}

/**
 * Mock 删除数据集报告
 */
export async function deleteDatasetReport(
  _reportId: string,
  _reportsDir: string
): Promise<void> {
  // Mock 环境 - 无操作
}