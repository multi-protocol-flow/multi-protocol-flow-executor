/**
 * StressTestCharts 组件测试
 */
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { StressTestCharts } from '@/components/stress-test/StressTestCharts';
import type { StressTestReport } from '@/types/stressTest';

describe('StressTestCharts', () => {
  const mockReport: StressTestReport = {
    uuid: '11693799-f975-454f-9f37-2dea4bba4bda',
    flowUuid: '11e70721-51e1-435f-912d-71b5e46f435e',
    flowName: 'llm',
    startTime: 1787823661532,
    endTime: 1787823678580,
    durationMs: 17048,
    config: {
      concurrentUsers: 2,
      rampUpSeconds: 10,
      durationSeconds: 12,
      flowUuid: '11e70721-51e1-435f-912d-71b5e46f435e',
    },
    statistics: {
      totalIterations: 1,
      successful: 1,
      failed: 0,
      successRate: 1.0,
      rps: 0.058675,
    },
    responseTimeDistribution: {
      avgMs: 16440.0,
      minMs: 16440,
      maxMs: 16440,
      p50Ms: 16440,
      p90Ms: 16440,
      p95Ms: 16440,
      p99Ms: 16440,
    },
    progressHistory: [
      {
        status: 'rampUp',
        elapsedSeconds: 1,
        totalIterations: 0,
        successfulIterations: 0,
        failedIterations: 0,
        currentRps: 0.0,
        avgLatencyMs: 0.0,
        nodeMetrics: [],
      },
      {
        status: 'running',
        elapsedSeconds: 11,
        totalIterations: 0,
        successfulIterations: 0,
        failedIterations: 0,
        currentRps: 0.0,
        avgLatencyMs: 0.0,
        nodeMetrics: [],
      },
    ],
  };

  it('当没有数据且无 report 时显示暂无数据提示', () => {
    render(<StressTestCharts progressHistory={[]} report={null} />);
    expect(screen.getByText(/暂无数据/)).toBeDefined();
  });

  it('单次长耗时请求且采样均为 0ms 时，结合 report 正确渲染图表', () => {
    render(
      <StressTestCharts
        progressHistory={mockReport.progressHistory ?? []}
        report={mockReport}
      />
    );

    expect(screen.getByText(/RPS 实时曲线/)).toBeDefined();
    expect(screen.getByText(/响应时间分布/)).toBeDefined();
  });
});
