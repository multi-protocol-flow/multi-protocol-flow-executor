/**
 * ErrorSampleDialog 组件测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ErrorSampleDialog } from '@/components/stress-test/ErrorSampleDialog';
import type { ErrorSample } from '@/types/stressTest';

describe('ErrorSampleDialog', () => {
  const mockOnOpenChange = vi.fn();

  const mockSamples: ErrorSample[] = [
    {
      iteration: 1,
      timestamp: 1700000000000,
      nodeUuid: 'node-001',
      nodeName: 'HTTP Request 1',
      nodeType: 'http',
      errorMessage: 'Connection refused: 127.0.0.1:8080',
      durationMs: 45,
      resolvedConfig: { url: 'http://127.0.0.1:8080/api', method: 'GET' },
      inputData: { query: 'test' },
      outputData: { status: 500, body: 'Internal Server Error' },
    },
    {
      iteration: 5,
      timestamp: 1700000005000,
      nodeUuid: 'node-002',
      nodeName: 'HTTP Request 2',
      nodeType: 'http',
      errorMessage: 'Timeout after 30000ms',
      durationMs: 30000,
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('当 open=false 时不渲染对话框', () => {
    render(
      <ErrorSampleDialog
        open={false}
        onOpenChange={mockOnOpenChange}
        errorKind="connection"
        samples={mockSamples}
      />
    );

    expect(screen.queryByText(/错误采样详情/)).toBeNull();
  });

  it('渲染错误采样详情与首个样本', () => {
    render(
      <ErrorSampleDialog
        open={true}
        onOpenChange={mockOnOpenChange}
        errorKind="connection"
        samples={mockSamples}
      />
    );

    expect(screen.getByText(/错误采样详情/)).toBeDefined();
    expect(screen.getByText('connection')).toBeDefined();
    expect(screen.getAllByText(/HTTP Request 1/).length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText(/Connection refused: 127.0.0.1:8080/)).toBeDefined();
  });

  it('支持在多个采样记录间切换', async () => {
    const user = userEvent.setup();
    render(
      <ErrorSampleDialog
        open={true}
        onOpenChange={mockOnOpenChange}
        errorKind="connection"
        samples={mockSamples}
      />
    );

    const secondSampleBtn = screen.getByRole('button', { name: /迭代 #5/i });
    await user.click(secondSampleBtn);

    expect(screen.getAllByText(/HTTP Request 2/).length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText(/Timeout after 30000ms/)).toBeDefined();
  });
});
