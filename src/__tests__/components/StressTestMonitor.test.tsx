/**
 * StressTestMonitor 组件测试
 */
import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { StressTestMonitor } from '@/components/stress-test/StressTestMonitor';
import { useStressTestStore } from '@/stores/stressTest';

describe('StressTestMonitor', () => {
  beforeEach(() => {
    useStressTestStore.getState().reset();
  });

  it('未运行时不渲染任何内容', () => {
    const { container } = render(<StressTestMonitor />);
    expect(container.firstChild).toBeNull();
  });

  it('运行时渲染监控面板及 3 个指标卡片', () => {
    useStressTestStore.setState({
      status: 'running',
      progress: {
        status: 'running',
        elapsedSeconds: 15,
        totalIterations: 1200,
        successfulIterations: 1180,
        failedIterations: 20,
        currentRps: 80.5,
        avgLatencyMs: 12.3,
        nodeMetrics: [],
      },
      progressHistory: [],
    });

    render(<StressTestMonitor />);

    expect(screen.getByText('压力测试')).toBeDefined();
    expect(screen.getByText('运行中')).toBeDefined();
    expect(screen.getByText('1200')).toBeDefined();
    expect(screen.getByText('80.50')).toBeDefined();
    expect(screen.getByText('12')).toBeDefined();
  });
});
