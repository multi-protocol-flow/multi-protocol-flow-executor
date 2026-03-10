/**
 * Connection 类型定义测试
 */

import { describe, it, expect } from 'vitest';
import type { Connection } from '@/types/connection';

describe('Connection types', () => {
  describe('Connection interface', () => {
    it('should have correct structure', () => {
      const connection: Connection = {
        id: 'conn-1',
        source_node_uuid: 'node-entry',
        target_node_uuid: 'node-http',
        source_port_id: 'output',
        target_port_id: 'input',
      };

      expect(connection.id).toBe('conn-1');
      expect(connection.source_node_uuid).toBe('node-entry');
      expect(connection.target_node_uuid).toBe('node-http');
      expect(connection.source_port_id).toBe('output');
      expect(connection.target_port_id).toBe('input');
    });

    it('should support conditional port connections', () => {
      const connection: Connection = {
        id: 'conn-2',
        source_node_uuid: 'node-conditional',
        target_node_uuid: 'node-action',
        source_port_id: 'true',
        target_port_id: 'input',
      };

      expect(connection.source_port_id).toBe('true');
      expect(connection.target_port_id).toBe('input');
    });

    it('should support false port connections', () => {
      const connection: Connection = {
        id: 'conn-3',
        source_node_uuid: 'node-conditional',
        target_node_uuid: 'node-fallback',
        source_port_id: 'false',
        target_port_id: 'input',
      };

      expect(connection.source_port_id).toBe('false');
      expect(connection.target_node_uuid).toBe('node-fallback');
    });

    it('should support multiple connections between different nodes', () => {
      const connections: Connection[] = [
        {
          id: 'conn-1',
          source_node_uuid: 'entry',
          target_node_uuid: 'http-1',
          source_port_id: 'output',
          target_port_id: 'input',
        },
        {
          id: 'conn-2',
          source_node_uuid: 'http-1',
          target_node_uuid: 'conditional',
          source_port_id: 'output',
          target_port_id: 'input',
        },
        {
          id: 'conn-3',
          source_node_uuid: 'conditional',
          target_node_uuid: 'http-2',
          source_port_id: 'true',
          target_port_id: 'input',
        },
      ];

      expect(connections).toHaveLength(3);
      expect(connections[0].source_node_uuid).toBe('entry');
      expect(connections[2].source_port_id).toBe('true');
    });
  });
});
