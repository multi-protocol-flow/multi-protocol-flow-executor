/**
 * StressTestCharts 组件与辅助函数测试
 */
import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import {
  StressTestCharts,
  formatTimeTick,
  downsampleRpsData,
  prepareRpsChartData,
} from '@/components/stress-test/StressTestCharts';
import type { StressTestReport, StressTestProgress } from '@/types/stressTest';

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

  it('当压测时间超过 60 秒时，显示时间范围选择器并可点击切换', () => {
    const longHistory: StressTestProgress[] = Array.from({ length: 400 }, (_, i) => ({
      status: 'running',
      elapsedSeconds: i + 1,
      totalIterations: (i + 1) * 10,
      successfulIterations: (i + 1) * 10,
      failedIterations: 0,
      currentRps: 10 + (i % 5),
      avgLatencyMs: 20,
      nodeMetrics: [],
    }));

    render(<StressTestCharts progressHistory={longHistory} report={null} />);

    expect(screen.getByText('全部')).toBeDefined();
    expect(screen.getByText('近 1 分钟')).toBeDefined();
    expect(screen.getByText('近 5 分钟')).toBeDefined();

    // 切换到近 1 分钟
    fireEvent.click(screen.getByText('近 1 分钟'));
    expect(screen.getByText('RPS 实时曲线')).toBeDefined();
  });
});

describe('formatTimeTick', () => {
  it('短时间 (<60s) 展示为秒', () => {
    expect(formatTimeTick(0, 30)).toBe('0s');
    expect(formatTimeTick(15, 30)).toBe('15s');
    expect(formatTimeTick(45, 59)).toBe('45s');
  });

  it('中等时间 (60s - 3599s) 展示为 mm:ss', () => {
    expect(formatTimeTick(0, 120)).toBe('00:00');
    expect(formatTimeTick(65, 120)).toBe('01:05');
    expect(formatTimeTick(600, 600)).toBe('10:00');
    expect(formatTimeTick(3599, 3599)).toBe('59:59');
  });

  it('长时间 (>=3600s) 展示为 hh:mm:ss', () => {
    expect(formatTimeTick(0, 3600)).toBe('00:00:00');
    expect(formatTimeTick(3665, 3665)).toBe('01:01:05');
    expect(formatTimeTick(7200, 7200)).toBe('02:00:00');
  });
});

describe('downsampleRpsData', () => {
  it('数据点少于或等于最大限制时不降采样', () => {
    const points = [
      { time: 1, rps: 10 },
      { time: 2, rps: 20 },
      { time: 3, rps: 30 },
    ];
    expect(downsampleRpsData(points, 120)).toEqual(points);
  });

  it('数据点超过最大限制时自适应降采样并保持总点数和终点对齐', () => {
    const points = Array.from({ length: 600 }, (_, i) => ({
      time: i + 1,
      rps: 50 + (i % 10),
    }));

    const downsampled = downsampleRpsData(points, 120);
    expect(downsampled.length).toBeLessThanOrEqual(120);
    expect(downsampled[downsampled.length - 1].time).toBe(600);
    expect(downsampled[downsampled.length - 1].rps).toBe(points[points.length - 1].rps);
  });
});

describe('prepareRpsChartData', () => {
  it('支持全量与时间范围过滤', () => {
    const history: StressTestProgress[] = Array.from({ length: 200 }, (_, i) => ({
      status: 'running',
      elapsedSeconds: i + 1,
      totalIterations: (i + 1) * 10,
      successfulIterations: (i + 1) * 10,
      failedIterations: 0,
      currentRps: 10,
      avgLatencyMs: 20,
      nodeMetrics: [],
    }));

    const allData = prepareRpsChartData(history, null, 'all');
    expect(allData.length).toBeGreaterThan(0);
    expect(allData[0].time).toBeLessThan(100);

    const last1mData = prepareRpsChartData(history, null, '1m');
    expect(last1mData[0].time).toBeGreaterThanOrEqual(140);
    expect(last1mData[last1mData.length - 1].time).toBe(200);
  });
});
