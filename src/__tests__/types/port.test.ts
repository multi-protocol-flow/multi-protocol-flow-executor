/**
 * Port 类型定义测试
 */

import { describe, it, expect } from 'vitest';
import type { PortType, Port } from '@/types/port';

describe('Port types', () => {
  describe('PortType type', () => {
    it('should accept input type', () => {
      const portType: PortType = 'input';
      expect(portType).toBe('input');
    });

    it('should accept output type', () => {
      const portType: PortType = 'output';
      expect(portType).toBe('output');
    });
  });

  describe('Port interface', () => {
    it('should have correct structure with required fields', () => {
      const port: Port = {
        id: 'input',
        type: 'input',
      };

      expect(port.id).toBe('input');
      expect(port.type).toBe('input');
    });

    it('should accept optional label', () => {
      const port: Port = {
        id: 'true',
        type: 'output',
        label: 'True',
      };

      expect(port.id).toBe('true');
      expect(port.type).toBe('output');
      expect(port.label).toBe('True');
    });

    it('should support output port with label', () => {
      const port: Port = {
        id: 'output',
        type: 'output',
        label: 'Result',
      };

      expect(port.type).toBe('output');
      expect(port.label).toBe('Result');
    });

    it('should support input port without label', () => {
      const port: Port = {
        id: 'input',
        type: 'input',
      };

      expect(port.label).toBeUndefined();
    });

    it('should support multiple ports with different configurations', () => {
      const ports: Port[] = [
        { id: 'input', type: 'input' },
        { id: 'output', type: 'output', label: 'Success' },
        { id: 'true', type: 'output', label: 'True' },
        { id: 'false', type: 'output', label: 'False' },
      ];

      expect(ports).toHaveLength(4);
      expect(ports[0].type).toBe('input');
      expect(ports[1].label).toBe('Success');
      expect(ports[2].id).toBe('true');
      expect(ports[3].id).toBe('false');
    });
  });
});
