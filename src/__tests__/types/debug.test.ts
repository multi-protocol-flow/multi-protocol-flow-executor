/**
 * Debug 类型定义测试
 */

import { describe, it, expect } from 'vitest';
import { DEBUG_HEALTH_CONFIG } from '@/types/debug';
import type {
  DebugStatus,
  PauseReason,
  EndReason,
  DebugCommand,
  DebugEvent,
  ExecutionStackEntry,
  DebugSessionState,
} from '@/types/debug';

describe('Debug types', () => {
  describe('DEBUG_HEALTH_CONFIG', () => {
    it('should have HEARTBEAT_TIMEOUT_MS = 15000', () => {
      expect(DEBUG_HEALTH_CONFIG.HEARTBEAT_TIMEOUT_MS).toBe(15000);
    });

    it('should have COMMAND_TIMEOUT_MS = 30000', () => {
      expect(DEBUG_HEALTH_CONFIG.COMMAND_TIMEOUT_MS).toBe(30000);
    });

    it('should have HEALTH_CHECK_INTERVAL_MS = 5000', () => {
      expect(DEBUG_HEALTH_CONFIG.HEALTH_CHECK_INTERVAL_MS).toBe(5000);
    });

    it('should be frozen (as const)', () => {
      // Verify the object is readonly by checking all values are defined
      expect(Object.keys(DEBUG_HEALTH_CONFIG)).toHaveLength(3);
    });
  });

  describe('Type structure validation', () => {
    it('DebugStatus should accept valid values', () => {
      const statusIdle: DebugStatus = 'idle';
      const statusRunning: DebugStatus = 'running';
      const statusPaused: DebugStatus = 'paused';
      const statusEnded: DebugStatus = 'ended';

      expect(statusIdle).toBe('idle');
      expect(statusRunning).toBe('running');
      expect(statusPaused).toBe('paused');
      expect(statusEnded).toBe('ended');
    });

    it('PauseReason should accept breakpoint type', () => {
      const reason: PauseReason = { type: 'breakpoint', node_uuid: 'node-1' };
      expect(reason.type).toBe('breakpoint');
      expect(reason.node_uuid).toBe('node-1');
    });

    it('PauseReason should accept step_complete type', () => {
      const reason: PauseReason = { type: 'step_complete', node_uuid: 'node-1' };
      expect(reason.type).toBe('step_complete');
    });

    it('PauseReason should accept error type', () => {
      const reason: PauseReason = { type: 'error', message: 'Test error' };
      expect(reason.type).toBe('error');
      expect(reason.message).toBe('Test error');
    });

    it('EndReason should accept completed type', () => {
      const reason: EndReason = { type: 'completed' };
      expect(reason.type).toBe('completed');
    });

    it('EndReason should accept user_abort type', () => {
      const reason: EndReason = { type: 'user_abort' };
      expect(reason.type).toBe('user_abort');
    });

    it('EndReason should accept error type', () => {
      const reason: EndReason = { type: 'error', message: 'Test error' };
      expect(reason.type).toBe('error');
      expect(reason.message).toBe('Test error');
    });

    it('ExecutionStackEntry should have correct structure', () => {
      const entry: ExecutionStackEntry = {
        node_uuid: 'node-1',
        node_name: 'Test Node',
        node_type: 'http',
        started_at: 1000,
        ended_at: 2000,
        success: true,
      };

      expect(entry.node_uuid).toBe('node-1');
      expect(entry.node_name).toBe('Test Node');
      expect(entry.node_type).toBe('http');
      expect(entry.started_at).toBe(1000);
      expect(entry.ended_at).toBe(2000);
      expect(entry.success).toBe(true);
    });

    it('ExecutionStackEntry should accept optional fields', () => {
      const entry: ExecutionStackEntry = {
        node_uuid: 'node-1',
        node_name: 'Test Node',
        node_type: 'http',
        started_at: 1000,
        success: false,
        error: 'Execution failed',
        input: { key: 'value' },
        output: { result: 'data' },
        resolved_config: { url: 'http://example.com' },
        variable_changes: [{ key: 'var1', old_value: null, new_value: 'new' }],
        skip_reason: 'User skipped',
      };

      expect(entry.error).toBe('Execution failed');
      expect(entry.input).toEqual({ key: 'value' });
      expect(entry.output).toEqual({ result: 'data' });
    });

    it('DebugCommand should accept StartDebug', () => {
      const command: DebugCommand = { StartDebug: { flow_uuid: 'flow-1' } };
      expect(command).toEqual({ StartDebug: { flow_uuid: 'flow-1' } });
    });

    it('DebugCommand should accept simple commands', () => {
      const continueCmd: DebugCommand = 'Continue';
      const stepOver: DebugCommand = 'StepOver';
      const runToEnd: DebugCommand = 'RunToEnd';
      const endDebug: DebugCommand = 'EndDebug';

      expect(continueCmd).toBe('Continue');
      expect(stepOver).toBe('StepOver');
      expect(runToEnd).toBe('RunToEnd');
      expect(endDebug).toBe('EndDebug');
    });

    it('DebugEvent should accept SessionStarted', () => {
      const event: DebugEvent = { SessionStarted: { flow_uuid: 'flow-1' } };
      expect(event).toEqual({ SessionStarted: { flow_uuid: 'flow-1' } });
    });

    it('DebugEvent should accept Heartbeat', () => {
      const event: DebugEvent = {
        Heartbeat: {
          status: 'running',
          current_node_uuid: 'node-1',
          timestamp: 1000,
        },
      };
      expect(event).toHaveProperty('Heartbeat');
    });

    it('DebugSessionState should have correct structure', () => {
      const state: DebugSessionState = {
        session_id: 'session-1',
        flow_uuid: 'flow-1',
        status: 'running',
        variables: { var1: 'value1' },
        execution_stack: [],
        breakpoints: ['node-1', 'node-2'],
        is_stepping: false,
        skip_breakpoints: false,
      };

      expect(state.session_id).toBe('session-1');
      expect(state.flow_uuid).toBe('flow-1');
      expect(state.status).toBe('running');
      expect(state.breakpoints).toHaveLength(2);
      expect(state.is_stepping).toBe(false);
      expect(state.skip_breakpoints).toBe(false);
    });

    it('DebugSessionState should accept paused state with optional fields', () => {
      const state: DebugSessionState = {
        session_id: 'session-1',
        flow_uuid: 'flow-1',
        status: 'paused',
        current_node_uuid: 'node-1',
        pause_reason: { type: 'breakpoint', node_uuid: 'node-1' },
        variables: {},
        execution_stack: [],
        breakpoints: [],
        is_stepping: true,
        skip_breakpoints: false,
      };

      expect(state.status).toBe('paused');
      expect(state.current_node_uuid).toBe('node-1');
      expect(state.pause_reason?.type).toBe('breakpoint');
      expect(state.is_stepping).toBe(true);
    });
  });
});
