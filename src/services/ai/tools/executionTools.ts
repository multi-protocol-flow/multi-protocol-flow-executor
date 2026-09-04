/**
 * AI Flow Execution & Debugging Tools
 */

import { useFlowStore } from '@/stores/flow';
import { useExecutionStore } from '@/stores/execution';
import { useUIStore } from '@/stores/ui';
import { useConsoleStore } from '@/stores/console';
import { useWsStreamStore } from '@/stores/wsStream';
import { executeFlowWithChannel } from '@/services/tauri';
import type { LogEvent } from '@/types/events';
import type { ExecutionEvent } from '@/types/execution_event';
import type { DebugEvent } from '@/types/debug';
import type { AIToolDefinition } from '@/types/ai';

export const executionToolDefinitions: AIToolDefinition[] = [
  {
    type: 'function',
    function: {
      name: 'run_flow',
      description: 'Trigger full execution of the currently active flow and record node status.',
      parameters: {
        type: 'object',
        properties: {},
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'cancel_flow_execution',
      description: 'Cancel and terminate the currently running flow execution.',
      parameters: {
        type: 'object',
        properties: {},
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'get_last_execution_report',
      description: 'Retrieve the detailed execution report of the last flow execution run.',
      parameters: {
        type: 'object',
        properties: {},
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'diagnose_flow_errors',
      description:
        'Perform a comprehensive diagnostic analysis of current workflow errors, combining the latest execution failure reports, node configurations, topological connection defects, and variable interpolation issues. Returns root-cause analysis and actionable repair directions to ask the user.',
      parameters: {
        type: 'object',
        properties: {
          node_uuid: {
            type: 'string',
            description:
              'Optional UUID of a specific failed node to focus the diagnosis on. If omitted, diagnoses all failures across the workflow.',
          },
        },
      },
    },
  },
];

export const executionToolHandlers = {
  async run_flow() {
    const flowState = useFlowStore.getState();
    const execStore = useExecutionStore.getState();
    const currentFlow = flowState.flow;

    if (!currentFlow) {
      throw new Error('No active flow to execute.');
    }
    if (execStore.isExecuting) {
      throw new Error('Flow is already executing.');
    }

    execStore.startExecution();
    execStore.clearExecutionState();
    useUIStore.setState({ isBottomPanelExpanded: true, bottomPanelTab: 'running' });
    useConsoleStore.getState().clearLogs();

    const handleLog = (event: LogEvent) => {
      useConsoleStore.getState().addLog(event);
    };

    const handleExecutionEvent = (event: ExecutionEvent) => {
      useExecutionStore.getState().handleExecutionEvent(event);
    };

    const handleWsEvent = (event: DebugEvent) => {
      useWsStreamStore.getState().handleWsEvent(event);
    };

    try {
      const result = await executeFlowWithChannel(
        currentFlow,
        handleLog,
        handleExecutionEvent,
        handleWsEvent,
      );

      if (result.flow_report) {
        execStore.setReport(result.flow_report);
      }

      const failedNodeReports = (result.flow_report?.node_reports || [])
        .filter((nr) => nr.status === 'failed')
        .map((nr) => ({
          node_uuid: nr.node_uuid,
          node_name: nr.node_name,
          node_type: nr.node_type,
          duration_ms: nr.duration_ms,
          error: nr.error
            ? {
                error_kind: nr.error.error_kind,
                error_code: nr.error.error_code,
                message: nr.error.message,
                stack_trace: nr.error.stack_trace,
              }
            : null,
          resolved_config: nr.resolved_config,
          output_data: nr.output_data,
          input_data: nr.input_data,
        }));

      const failedNodes = failedNodeReports.map((nr) => nr.node_name);

      const diagnosticSummary =
        failedNodeReports.length > 0
          ? failedNodeReports
              .map(
                (f) =>
                  `Node [${f.node_type}] "${f.node_name}" failed: ${f.error?.message || 'Execution error'} (kind: ${f.error?.error_kind || 'execution'}, code: ${f.error?.error_code || 'none'})`,
              )
              .join('\n')
          : result.error
            ? `Flow execution error: ${result.error}`
            : 'All executed nodes succeeded.';

      return {
        success: result.success,
        execution_time_ms: result.execution_time,
        status: result.flow_report?.status,
        failed_nodes: failedNodes,
        failed_node_reports: failedNodeReports,
        diagnostic_summary: diagnosticSummary,
        flow_error: result.flow_report?.flow_error,
        error: result.error,
      };
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      execStore.setError(message);
      return {
        success: false,
        error: message,
      };
    }
  },
  async cancel_flow_execution() {
    const execStore = useExecutionStore.getState();
    await execStore.cancelExecution();
    return {
      success: true,
      message: 'Flow execution cancel requested.',
    };
  },

  async get_last_execution_report() {
    const execStore = useExecutionStore.getState();
    const report = execStore.report;

    if (!report) {
      return {
        success: false,
        message: 'No execution report available. Run the flow first.',
      };
    }

    return {
      success: true,
      flow_name: report.flow_name,
      status: report.status,
      duration_ms: report.duration_ms,
      statistics: report.statistics,
      node_reports: (report.node_reports || []).map((nr) => ({
        node_uuid: nr.node_uuid,
        name: nr.node_name,
        type: nr.node_type,
        status: nr.status,
        duration_ms: nr.duration_ms,
        error: nr.error,
      })),
      failed_nodes: (report.node_reports || [])
        .filter((nr) => nr.status === 'failed')
        .map((nr) => nr.node_name),
    };
  },

  async diagnose_flow_errors(args?: { node_uuid?: string }) {
    const flowState = useFlowStore.getState();
    const execStore = useExecutionStore.getState();
    const consoleStore = useConsoleStore.getState();
    const currentFlow = flowState.flow;

    if (!currentFlow) {
      throw new Error('No active flow to diagnose.');
    }

    const report = execStore.report;
    let failedReports = (report?.node_reports || []).filter((nr) => nr.status === 'failed');
    if (args?.node_uuid) {
      failedReports = failedReports.filter((nr) => nr.node_uuid === args.node_uuid);
    }

    // 1. 结构与拓扑校验
    const topologyWarnings: string[] = [];
    const connectedTargetUuids = new Set(currentFlow.connections.map((c) => c.target_node_uuid));
    const connectedSourceUuids = new Set(currentFlow.connections.map((c) => c.source_node_uuid));

    for (const node of currentFlow.nodes) {
      if (node.type !== 'entry' && !connectedTargetUuids.has(node.uuid)) {
        topologyWarnings.push(
          `Node [${node.type}] "${node.name}" has no incoming connection from upstream.`,
        );
      }
      if (node.type !== 'end' && !connectedSourceUuids.has(node.uuid)) {
        topologyWarnings.push(
          `Node [${node.type}] "${node.name}" has no outgoing connection to downstream.`,
        );
      }
    }

    // 2. 失败节点根因诊断与建议
    const diagnoses: Array<{
      node_uuid: string;
      node_name: string;
      node_type: string;
      error_message: string;
      possible_root_cause: string;
      suggested_repair_directions: string[];
    }> = [];

    for (const nr of failedReports) {
      const nodeObj = currentFlow.nodes.find((n) => n.uuid === nr.node_uuid);
      const errMsg = nr.error?.message || 'Execution failed';
      const errKind = nr.error?.error_kind || 'execution';
      const errCode = nr.error?.error_code || '';

      let possibleRootCause = `[${errKind}] Execution error: ${errMsg}`;
      const suggestedDirections: string[] = [];

      if (nr.node_type === 'http') {
        const cfg = (nodeObj?.data || {}) as Record<string, unknown>;
        const url = String(cfg.url || '');
        if (!url || url === 'https://httpbin.org/get') {
          possibleRootCause = 'HTTP node is using empty or placeholder URL.';
          suggestedDirections.push('Direction 1: Update node config with the actual target API URL.');
        } else if (errMsg.includes('401') || errMsg.includes('Unauthorized') || errCode === 'E_HTTP_STATUS_ERROR') {
          possibleRootCause = 'Target API returned 401 Unauthorized, likely due to missing, expired, or invalid authorization token.';
          suggestedDirections.push('Direction 1: Check and update the Authorization header with a valid token or variable interpolation like {{variables.token}}.');
          suggestedDirections.push('Direction 2: Prepend an authentication/login request step to extract the token automatically.');
        } else if (errMsg.includes('404') || errMsg.includes('Not Found')) {
          possibleRootCause = 'Target API returned 404 Not Found. The URL path or resource identifier does not exist.';
          suggestedDirections.push('Direction 1: Verify the URL endpoint path and parameter values.');
        } else if (errMsg.includes('Connection refused') || errCode === 'E_CONN_REFUSED') {
          possibleRootCause = 'Connection refused. The target server is either not running or the port is incorrect.';
          suggestedDirections.push('Direction 1: Verify target host and port in node config.');
          suggestedDirections.push('Direction 2: Ensure local test server or backend service is active.');
        } else {
          suggestedDirections.push('Direction 1: Review request method, headers, and request body format.');
          suggestedDirections.push('Direction 2: Increase timeout_ms or enable retry_count.');
        }
      } else if (nr.node_type === 'assertion') {
        possibleRootCause = `Assertion condition not met: ${errMsg}`;
        suggestedDirections.push('Direction 1: Adjust assertion operator or expected value to match upstream actual output.');
        suggestedDirections.push('Direction 2: Check upstream step to see why it produced unexpected output.');
      } else if (nr.node_type === 'script') {
        possibleRootCause = `JavaScript runtime error in sandbox: ${errMsg}`;
        suggestedDirections.push('Direction 1: Fix JavaScript code syntax or object property access.');
        suggestedDirections.push('Direction 2: Check preceding step response structure accessed via context.steps.<id>.response.body.');
      } else {
        suggestedDirections.push(`Direction 1: Review ${nr.node_type} node parameters and connection credentials.`);
      }

      diagnoses.push({
        node_uuid: nr.node_uuid,
        node_name: nr.node_name,
        node_type: nr.node_type,
        error_message: errMsg,
        possible_root_cause: possibleRootCause,
        suggested_repair_directions: suggestedDirections,
      });
    }

    // 3. 控制台最近错误
    const recentConsoleErrors = consoleStore.logs
      .filter((l) => l.level === 'ERROR')
      .slice(-5)
      .map((l) => l.message);

    return {
      success: true,
      has_execution_report: !!report,
      overall_status: report?.status || 'not_executed',
      failed_count: failedReports.length,
      diagnoses,
      topology_warnings: topologyWarnings,
      recent_console_errors: recentConsoleErrors,
      instruction_for_assistant:
        'Present the root causes clearly to the user, propose the suggested repair directions, and explicitly ask the user whether to proceed with repair and which direction they prefer.',
    };
  },
};
