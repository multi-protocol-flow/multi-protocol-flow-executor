import { describe, it, expect } from 'vitest';
import type {
  ExecutionMode,
  ReportMetadata,
  LoadFlowCommandInput,
  LoadFlowCommandOutput,
  SaveFlowCommandInput,
  SaveFlowCommandOutput,
  CreateFlowCommandInput,
  CreateFlowCommandOutput,
  ExecuteFlowCommandInput,
  ExecuteFlowCommandOutput,
  UpdateNodeCommandInput,
  UpdateNodeCommandOutput,
  DeleteNodeCommandInput,
  DeleteNodeCommandOutput,
} from '@/types/ipc';

describe('IPC Types', () => {
  describe('ExecutionMode', () => {
    it('should accept "normal" as valid ExecutionMode', () => {
      const mode: ExecutionMode = 'normal';
      expect(mode).toBe('normal');
    });

    it('should accept "debug" as valid ExecutionMode', () => {
      const mode: ExecutionMode = 'debug';
      expect(mode).toBe('debug');
    });
  });

  describe('ReportMetadata', () => {
    it('should create a valid ReportMetadata with required fields', () => {
      const metadata: ReportMetadata = {
        uuid: 'report-001',
        flow_uuid: 'flow-001',
        flow_name: 'Test Flow',
        status: 'success',
        timestamp: 1709500000000,
        duration_ms: 1234,
      };
      expect(metadata.uuid).toBe('report-001');
      expect(metadata.flow_uuid).toBe('flow-001');
      expect(metadata.flow_name).toBe('Test Flow');
      expect(metadata.status).toBe('success');
      expect(metadata.timestamp).toBe(1709500000000);
      expect(metadata.duration_ms).toBe(1234);
      expect(metadata.execution_mode).toBeUndefined();
    });

    it('should create a valid ReportMetadata with optional execution_mode', () => {
      const metadata: ReportMetadata = {
        uuid: 'report-002',
        flow_uuid: 'flow-002',
        flow_name: 'Debug Flow',
        status: 'failed',
        timestamp: 1709500000000,
        duration_ms: 5000,
        execution_mode: 'debug',
      };
      expect(metadata.execution_mode).toBe('debug');
      expect(metadata.status).toBe('failed');
    });

    it('should accept both status values', () => {
      const successMetadata: ReportMetadata = {
        uuid: 'r1', flow_uuid: 'f1', flow_name: 'F1', status: 'success', timestamp: 1, duration_ms: 1,
      };
      const failedMetadata: ReportMetadata = {
        uuid: 'r2', flow_uuid: 'f2', flow_name: 'F2', status: 'failed', timestamp: 2, duration_ms: 2,
      };
      expect(successMetadata.status).toBe('success');
      expect(failedMetadata.status).toBe('failed');
    });
  });

  describe('LoadFlowCommandInput', () => {
    it('should create a valid input with path', () => {
      const input: LoadFlowCommandInput = {
        path: '/flows/example.json',
      };
      expect(input.path).toBe('/flows/example.json');
    });
  });

  describe('LoadFlowCommandOutput', () => {
    it('should create a valid output with flow', () => {
      const output: LoadFlowCommandOutput = {
        flow: {
          uuid: 'flow-001',
          name: 'Test Flow',
          nodes: [],
          connections: [],
        },
      };
      expect(output.flow.uuid).toBe('flow-001');
      expect(output.flow.name).toBe('Test Flow');
    });
  });

  describe('SaveFlowCommandInput', () => {
    it('should create a valid input with flow and path', () => {
      const input: SaveFlowCommandInput = {
        flow: {
          uuid: 'flow-001',
          name: 'Test Flow',
          nodes: [],
          connections: [],
        },
        path: '/flows/test.json',
      };
      expect(input.flow.uuid).toBe('flow-001');
      expect(input.path).toBe('/flows/test.json');
    });
  });

  describe('SaveFlowCommandOutput', () => {
    it('should create a valid output with success', () => {
      const output: SaveFlowCommandOutput = {
        success: true,
      };
      expect(output.success).toBe(true);
    });
  });

  describe('CreateFlowCommandInput', () => {
    it('should create a valid input with name', () => {
      const input: CreateFlowCommandInput = {
        name: 'New Flow',
      };
      expect(input.name).toBe('New Flow');
    });
  });

  describe('CreateFlowCommandOutput', () => {
    it('should create a valid output with flow', () => {
      const output: CreateFlowCommandOutput = {
        flow: {
          uuid: 'flow-new',
          name: 'New Flow',
          nodes: [],
          connections: [],
        },
      };
      expect(output.flow.uuid).toBe('flow-new');
    });
  });

  describe('ExecuteFlowCommandInput', () => {
    it('should create a valid input with required fields', () => {
      const input: ExecuteFlowCommandInput = {
        flow_id: 'flow-001',
      };
      expect(input.flow_id).toBe('flow-001');
      expect(input.variables).toBeUndefined();
    });

    it('should create a valid input with optional variables', () => {
      const input: ExecuteFlowCommandInput = {
        flow_id: 'flow-002',
        variables: { api_key: 'secret' },
      };
      expect(input.variables).toEqual({ api_key: 'secret' });
    });
  });

  describe('ExecuteFlowCommandOutput', () => {
    it('should create a successful output', () => {
      const output: ExecuteFlowCommandOutput = {
        success: true,
        execution_time: 1500,
      };
      expect(output.success).toBe(true);
      expect(output.execution_time).toBe(1500);
      expect(output.error).toBeUndefined();
    });

    it('should create a failed output with error', () => {
      const output: ExecuteFlowCommandOutput = {
        success: false,
        execution_time: 100,
        error: 'Connection failed',
      };
      expect(output.success).toBe(false);
      expect(output.error).toBe('Connection failed');
    });
  });

  describe('UpdateNodeCommandInput', () => {
    it('should create a valid input', () => {
      const input: UpdateNodeCommandInput = {
        flow_id: 'flow-001',
        node_id: 'node-001',
        data: { type: 'http', url: 'https://api.example.com' },
      };
      expect(input.flow_id).toBe('flow-001');
      expect(input.node_id).toBe('node-001');
    });
  });

  describe('UpdateNodeCommandOutput', () => {
    it('should create a successful output', () => {
      const output: UpdateNodeCommandOutput = {
        success: true,
      };
      expect(output.success).toBe(true);
      expect(output.error).toBeUndefined();
    });

    it('should create a failed output with error', () => {
      const output: UpdateNodeCommandOutput = {
        success: false,
        error: 'Node not found',
      };
      expect(output.success).toBe(false);
      expect(output.error).toBe('Node not found');
    });
  });

  describe('DeleteNodeCommandInput', () => {
    it('should create a valid input', () => {
      const input: DeleteNodeCommandInput = {
        flow_id: 'flow-001',
        node_id: 'node-001',
      };
      expect(input.flow_id).toBe('flow-001');
      expect(input.node_id).toBe('node-001');
    });
  });

  describe('DeleteNodeCommandOutput', () => {
    it('should create a successful output', () => {
      const output: DeleteNodeCommandOutput = {
        success: true,
      };
      expect(output.success).toBe(true);
    });

    it('should create a failed output with error', () => {
      const output: DeleteNodeCommandOutput = {
        success: false,
        error: 'Cannot delete entry node',
      };
      expect(output.success).toBe(false);
      expect(output.error).toBe('Cannot delete entry node');
    });
  });
});
