import { describe, it, expect } from 'vitest';
import type { Variable, VariablePool, OutputMapping } from '@/types/variable';

describe('Variable Types', () => {
  describe('Variable', () => {
    it('should create a valid Variable with required fields', () => {
      const variable: Variable = {
        name: 'test_var',
        value: 'test_value',
        sourceNodeUuid: 'node-123',
      };
      expect(variable.name).toBe('test_var');
      expect(variable.value).toBe('test_value');
      expect(variable.sourceNodeUuid).toBe('node-123');
    });

    it('should create a valid Variable with optional description', () => {
      const variable: Variable = {
        name: 'response_status',
        value: 200,
        sourceNodeUuid: 'http-node-001',
        description: 'HTTP response status code',
      };
      expect(variable.description).toBe('HTTP response status code');
    });

    it('should allow various value types', () => {
      const stringVar: Variable = { name: 'str', value: 'hello', sourceNodeUuid: 'n1' };
      const numberVar: Variable = { name: 'num', value: 42, sourceNodeUuid: 'n2' };
      const boolVar: Variable = { name: 'bool', value: true, sourceNodeUuid: 'n3' };
      const objectVar: Variable = { name: 'obj', value: { key: 'val' }, sourceNodeUuid: 'n4' };
      const arrayVar: Variable = { name: 'arr', value: [1, 2, 3], sourceNodeUuid: 'n5' };
      const nullVar: Variable = { name: 'null', value: null, sourceNodeUuid: 'n6' };

      expect(stringVar.value).toBe('hello');
      expect(numberVar.value).toBe(42);
      expect(boolVar.value).toBe(true);
      expect(objectVar.value).toEqual({ key: 'val' });
      expect(arrayVar.value).toEqual([1, 2, 3]);
      expect(nullVar.value).toBeNull();
    });
  });

  describe('VariablePool', () => {
    it('should create a valid VariablePool', () => {
      const pool: VariablePool = {
        variables: [
          { name: 'var1', value: 'value1', sourceNodeUuid: 'node-1' },
          { name: 'var2', value: 'value2', sourceNodeUuid: 'node-2' },
        ],
      };
      expect(pool.variables).toHaveLength(2);
    });

    it('should create an empty VariablePool', () => {
      const pool: VariablePool = { variables: [] };
      expect(pool.variables).toHaveLength(0);
    });
  });

  describe('OutputMapping', () => {
    it('should create a valid OutputMapping with required fields', () => {
      const mapping: OutputMapping = {
        source: 'status',
        target: 'response_status',
      };
      expect(mapping.source).toBe('status');
      expect(mapping.target).toBe('response_status');
    });

    it('should create a valid OutputMapping with optional fields', () => {
      const mapping: OutputMapping = {
        source: 'body.data.id',
        target: 'user_id',
        description: 'Extract user ID from response',
      };
      expect(mapping.description).toBe('Extract user ID from response');
    });

    it('should have undefined optional fields when not specified', () => {
      const mapping: OutputMapping = {
        source: 'status',
        target: 'response_status',
      };
      expect(mapping.description).toBeUndefined();
    });

    it('should support nested path expressions', () => {
      const mapping: OutputMapping = {
        source: 'body.data.items[0].name',
        target: 'first_item_name',
      };
      expect(mapping.source).toBe('body.data.items[0].name');
    });
  });

});
