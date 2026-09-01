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
      name: 'run_flow_debug',
      description: 'Trigger full execution of the currently active flow in debug/run mode and record node status.',
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
];

export const executionToolHandlers = {
  async run_flow_debug() {
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

      const failedNodes = result.flow_report?.node_reports
        .filter((nr) => nr.status === 'failed')
        .map((nr) => nr.node_name);

      return {
        success: result.success,
        execution_time_ms: result.execution_time,
        status: result.flow_report?.status,
        failed_nodes: failedNodes,
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
};
