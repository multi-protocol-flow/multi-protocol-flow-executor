import { describe, it, expect } from 'vitest';
import { isFatalError, type ErrorKind, type ErrorDetail } from '@/types/error';

describe('Error Types', () => {
  describe('ErrorKind', () => {
    it('should define fatal error kinds', () => {
      // 致命错误 - 需要立即终止流程
      const fatalErrorKinds: ErrorKind[] = ['configuration', 'condition_syntax'];
      
      fatalErrorKinds.forEach((kind) => {
        expect(isFatalError(kind)).toBe(true);
      });
    });

    it('should define recoverable error kinds', () => {
      // 可恢复错误 - 容错模式下可继续执行
      const recoverableErrorKinds: ErrorKind[] = [
        'execution',
        'connection',
        'timeout',
        'condition_evaluation',
        'validation',
        'parse',
      ];

      recoverableErrorKinds.forEach((kind) => {
        expect(isFatalError(kind)).toBe(false);
      });
    });

    it('should allow all ErrorKind values as valid types', () => {
      // 验证所有 ErrorKind 值都是有效的类型
      const allKinds: ErrorKind[] = [
        'configuration',
        'condition_syntax',
        'execution',
        'connection',
        'timeout',
        'condition_evaluation',
        'validation',
        'parse',
      ];

      // TypeScript 编译通过即证明类型正确
      expect(allKinds).toHaveLength(8);
    });
  });

  describe('isFatalError', () => {
    describe('fatal errors (返回 true)', () => {
      it('should return true for configuration error', () => {
        expect(isFatalError('configuration')).toBe(true);
      });

      it('should return true for condition_syntax error', () => {
        expect(isFatalError('condition_syntax')).toBe(true);
      });
    });

    describe('recoverable errors (返回 false)', () => {
      it('should return false for execution error', () => {
        expect(isFatalError('execution')).toBe(false);
      });

      it('should return false for connection error', () => {
        expect(isFatalError('connection')).toBe(false);
      });

      it('should return false for timeout error', () => {
        expect(isFatalError('timeout')).toBe(false);
      });

      it('should return false for condition_evaluation error', () => {
        expect(isFatalError('condition_evaluation')).toBe(false);
      });

      it('should return false for validation error', () => {
        expect(isFatalError('validation')).toBe(false);
      });

      it('should return false for parse error', () => {
        expect(isFatalError('parse')).toBe(false);
      });
    });
  });

  describe('ErrorDetail', () => {
    it('should create a valid ErrorDetail with required fields', () => {
      const errorDetail: ErrorDetail = {
        error_kind: 'execution',
        message: 'Failed to execute HTTP request',
      };

      expect(errorDetail.error_kind).toBe('execution');
      expect(errorDetail.message).toBe('Failed to execute HTTP request');
    });

    it('should create a valid ErrorDetail with all optional fields', () => {
      const errorDetail: ErrorDetail = {
        error_kind: 'configuration',
        error_code: 'E_CONFIG_ERROR',
        message: 'Invalid node configuration',
        stack_trace: 'at NodeExecutor.execute...',
        context: { nodeId: 'node-123', config: { url: '' } },
      };

      expect(errorDetail.error_kind).toBe('configuration');
      expect(errorDetail.error_code).toBe('E_CONFIG_ERROR');
      expect(errorDetail.message).toBe('Invalid node configuration');
      expect(errorDetail.stack_trace).toBe('at NodeExecutor.execute...');
      expect(errorDetail.context).toEqual({ nodeId: 'node-123', config: { url: '' } });
    });

    it('should have undefined optional fields when not specified', () => {
      const errorDetail: ErrorDetail = {
        error_kind: 'timeout',
        message: 'Request timed out',
      };

      expect(errorDetail.error_code).toBeUndefined();
      expect(errorDetail.stack_trace).toBeUndefined();
      expect(errorDetail.context).toBeUndefined();
    });

    it('should allow various context types', () => {
      const stringContext: ErrorDetail = {
        error_kind: 'parse',
        message: 'Parse error',
        context: 'Raw response data',
      };

      const objectContext: ErrorDetail = {
        error_kind: 'validation',
        message: 'Validation failed',
        context: { field: 'url', value: '', reason: 'required' },
      };

      const arrayContext: ErrorDetail = {
        error_kind: 'execution',
        message: 'Multiple errors',
        context: ['error1', 'error2'],
      };

      const nullContext: ErrorDetail = {
        error_kind: 'connection',
        message: 'Connection lost',
        context: null,
      };

      expect(stringContext.context).toBe('Raw response data');
      expect(objectContext.context).toEqual({ field: 'url', value: '', reason: 'required' });
      expect(arrayContext.context).toEqual(['error1', 'error2']);
      expect(nullContext.context).toBeNull();
    });

    it('should support all ErrorKind values in error_kind field', () => {
      const fatalConfigError: ErrorDetail = {
        error_kind: 'configuration',
        message: 'Configuration error',
      };

      const fatalSyntaxError: ErrorDetail = {
        error_kind: 'condition_syntax',
        message: 'Syntax error in condition',
      };

      const recoverableExecutionError: ErrorDetail = {
        error_kind: 'execution',
        message: 'Execution failed',
      };

      expect(isFatalError(fatalConfigError.error_kind)).toBe(true);
      expect(isFatalError(fatalSyntaxError.error_kind)).toBe(true);
      expect(isFatalError(recoverableExecutionError.error_kind)).toBe(false);
    });
  });

  describe('Error classification documentation', () => {
    it('should classify errors correctly: fatal errors terminate flow immediately', () => {
      // 致命错误类型列表（与 FATAL_ERROR_KINDS 对齐）
      const fatalErrors: ErrorKind[] = ['configuration', 'condition_syntax'];
      
      fatalErrors.forEach((kind) => {
        expect(isFatalError(kind)).toBe(true);
      });
    });

    it('should classify errors correctly: recoverable errors route to false port', () => {
      // 可恢复错误类型列表（容错模式下继续执行）
      const recoverableErrors: ErrorKind[] = [
        'execution',
        'connection', 
        'timeout',
        'condition_evaluation',
        'validation',
        'parse',
      ];

      recoverableErrors.forEach((kind) => {
        expect(isFatalError(kind)).toBe(false);
      });
    });
  });
});
