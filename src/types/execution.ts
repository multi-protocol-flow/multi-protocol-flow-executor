/**
 * 执行上下文接口
 */
export interface ExecutionContext {
  flowId: string;
  executionId: string;
  currentNodeId: string;
  variables: Record<string, any>;
  startTime: number;
  endTime?: number;
  status: 'running' | 'completed' | 'failed';
  error?: Error;
}

/**
 * 执行结果接口
 */
export interface ExecutionResult {
  executionId: string;
  flowId: string;
  success: boolean;
  data?: any;
  error?: string;
  executionTime: number;
}
