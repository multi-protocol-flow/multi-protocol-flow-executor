/**
 * StressTestCharts - 压力测试可视化图表
 * 包含 RPS 实时曲线（支持自适应降采样、自适应时间刻度、时间范围筛选）和响应时间分布图
 */
import { useState, useMemo } from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  BarChart,
  Bar,
} from 'recharts';
import type { StressTestProgress, StressTestReport } from '@/types/stressTest';
import { getLL } from '@/i18n/i18n-lazy';
import { cn } from '@/lib/utils';

// ============================================================================
// Types
// ============================================================================

export interface StressTestChartsProps {
  /** 进度历史数据 - 从父组件传入 */
  progressHistory: StressTestProgress[];
  /** 压测报告（可选，用于在采样未捕获到最终完成点时补充完整指标） */
  report?: StressTestReport | null;
}

export interface RpsChartDataPoint {
  time: number;
  rps: number;
}

export interface LatencyBucketData {
  name: string;
  count: number;
  fill: string;
}

export type TimeRangeFilter = 'all' | '1m' | '5m' | '15m';

// ============================================================================
// Constants
// ============================================================================

/** 最大图表渲染点数，保证超长压测时流畅渲染同时保留全程趋势 */
export const MAX_CHART_POINTS = 120;

/** 延迟桶边界（毫秒） */
const LATENCY_BUCKETS: Array<{ max: number; label: string; color: string }> = [
  { max: 50, label: '0-50ms', color: '#22c55e' }, // green-500
  { max: 100, label: '50-100ms', color: '#3b82f6' }, // blue-500
  { max: 200, label: '100-200ms', color: '#f59e0b' }, // amber-500
  { max: 500, label: '200-500ms', color: '#f97316' }, // orange-500
  { max: Infinity, label: '500ms+', color: '#ef4444' }, // red-500
];

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * 格式化时间刻度
 * @param seconds 当前点的秒数
 * @param maxDuration 总时长（秒），用于确定时间粒度
 */
export function formatTimeTick(seconds: number, maxDuration = seconds): string {
  if (maxDuration < 60) {
    return `${seconds}s`;
  }
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;

  if (maxDuration >= 3600 || h > 0) {
    return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  }
  return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
}

/**
 * 对 RPS 历史数据自适应降采样
 */
export function downsampleRpsData(
  points: RpsChartDataPoint[],
  maxPoints = MAX_CHART_POINTS
): RpsChartDataPoint[] {
  if (points.length <= maxPoints) {
    return points;
  }

  const bucketSize = points.length / maxPoints;
  const result: RpsChartDataPoint[] = [];

  for (let i = 0; i < maxPoints; i++) {
    const start = Math.floor(i * bucketSize);
    const end = Math.floor((i + 1) * bucketSize);
    const bucket = points.slice(start, end);
    if (bucket.length === 0) continue;

    // 计算桶内的平均 RPS 和中间时间点
    const avgRps = bucket.reduce((sum, p) => sum + p.rps, 0) / bucket.length;
    const midPoint = bucket[Math.floor(bucket.length / 2)];
    result.push({
      time: midPoint.time,
      rps: Number(avgRps.toFixed(2)),
    });
  }

  // 确保最后一个点与最新点对齐
  const lastOriginal = points[points.length - 1];
  if (result.length > 0 && lastOriginal) {
    result[result.length - 1] = {
      time: lastOriginal.time,
      rps: lastOriginal.rps,
    };
  }

  return result;
}

/**
 * 将进度历史转换为 RPS 图表数据
 */
export function prepareRpsChartData(
  history: StressTestProgress[],
  report?: StressTestReport | null,
  range: TimeRangeFilter = 'all'
): RpsChartDataPoint[] {
  const baseHistory = [...history];

  // 如果已有完成报告，且历史点未包含最终完成时的非零迭代状态，则补充最终点
  if (report && report.statistics.totalIterations > 0) {
    const lastPoint = baseHistory[baseHistory.length - 1];
    const finalElapsed = Math.round(report.durationMs / 1000) || 1;
    if (
      !lastPoint ||
      lastPoint.totalIterations < report.statistics.totalIterations ||
      lastPoint.elapsedSeconds < finalElapsed
    ) {
      baseHistory.push({
        status: 'stopped',
        elapsedSeconds: finalElapsed,
        totalIterations: report.statistics.totalIterations,
        successfulIterations: report.statistics.successful,
        failedIterations: report.statistics.failed,
        currentRps: report.statistics.rps,
        avgLatencyMs: report.responseTimeDistribution.avgMs,
        nodeMetrics: [],
      });
    }
  }

  if (baseHistory.length === 0) {
    return [];
  }

  // 按时间范围过滤
  let filtered = baseHistory;
  const maxElapsed = baseHistory[baseHistory.length - 1].elapsedSeconds;
  if (range === '1m') {
    filtered = baseHistory.filter((p) => p.elapsedSeconds >= maxElapsed - 60);
  } else if (range === '5m') {
    filtered = baseHistory.filter((p) => p.elapsedSeconds >= maxElapsed - 300);
  } else if (range === '15m') {
    filtered = baseHistory.filter((p) => p.elapsedSeconds >= maxElapsed - 900);
  }

  const rawPoints = filtered.map((progress) => ({
    time: progress.elapsedSeconds,
    rps: progress.currentRps,
  }));

  // 降采样保证性能
  return downsampleRpsData(rawPoints, MAX_CHART_POINTS);
}

/**
 * 根据延迟值计算延迟分布桶
 * 结合进度历史与最终报告统计
 */
export function prepareLatencyBuckets(
  history: StressTestProgress[],
  report?: StressTestReport | null
): LatencyBucketData[] {
  // 初始化桶计数
  const bucketCounts = LATENCY_BUCKETS.map(() => 0);

  // 1. 优先统计历史进度采样点
  history.forEach((progress) => {
    // 过滤掉尚未产生任何完成请求或 avgLatencyMs <= 0 的无效采样点
    if (progress.totalIterations === 0 || progress.avgLatencyMs <= 0) {
      return;
    }
    const latency = progress.avgLatencyMs;
    for (let i = 0; i < LATENCY_BUCKETS.length; i++) {
      if (latency < LATENCY_BUCKETS[i].max) {
        bucketCounts[i]++;
        break;
      }
    }
  });

  // 2. 如果历史中全为未完成的 0ms 采样（无有效点），但最终报告中有真实耗时，使用报告整体指标保底
  const hasValidSample = bucketCounts.some((count) => count > 0);
  if (!hasValidSample && report && report.statistics.totalIterations > 0) {
    const latency = report.responseTimeDistribution.avgMs;
    if (latency > 0) {
      for (let i = 0; i < LATENCY_BUCKETS.length; i++) {
        if (latency < LATENCY_BUCKETS[i].max) {
          bucketCounts[i] = Math.max(1, report.statistics.totalIterations);
          break;
        }
      }
    }
  }

  // 构建图表数据
  return LATENCY_BUCKETS.map((bucket, index) => ({
    name: bucket.label,
    count: bucketCounts[index],
    fill: bucket.color,
  }));
}

/**
 * 格式化 RPS 值
 */
function formatRps(value: number): string {
  return value.toFixed(2);
}

// ============================================================================
// Custom Tooltips
// ============================================================================

interface RpsTooltipProps {
  active?: boolean;
  payload?: Array<{ value: number }>;
  label?: number;
  maxDuration?: number;
}

interface LatencyTooltipProps {
  active?: boolean;
  payload?: Array<{ payload: LatencyBucketData }>;
}

function RpsTooltip({ active, payload, label, maxDuration = 0 }: RpsTooltipProps) {
  if (!active || !payload || payload.length === 0 || label === undefined) {
    return null;
  }

  const formattedTime = formatTimeTick(label, maxDuration);
  const timeDisplay = maxDuration >= 60 ? `${formattedTime} (${label}s)` : `${label}s`;

  return (
    <div className="bg-popover border border-border rounded-lg px-3 py-2 shadow-md">
      <p className="text-xs text-muted-foreground mb-1">
        {getLL().flow.stress_chart_tooltip_time()}
        <span className="font-medium text-foreground ml-1">{timeDisplay}</span>
      </p>
      <p className="text-xs text-muted-foreground">
        RPS: <span className="font-medium text-blue-600 font-mono ml-1">{formatRps(payload[0].value)}</span>
      </p>
    </div>
  );
}

function LatencyTooltip({ active, payload }: LatencyTooltipProps) {
  if (!active || !payload || payload.length === 0) {
    return null;
  }

  const data = payload[0].payload;

  return (
    <div className="bg-popover border border-border rounded-lg px-3 py-2 shadow-md">
      <p className="text-xs text-muted-foreground">
        {data.name}: <span className="font-medium text-foreground ml-1">{data.count}{getLL().flow.stress_chart_tooltip_seconds()}</span>
      </p>
    </div>
  );
}

// ============================================================================
// Component
// ============================================================================

export function StressTestCharts({ progressHistory, report }: StressTestChartsProps) {
  const [timeRange, setTimeRange] = useState<TimeRangeFilter>('all');

  // 准备 RPS 图表数据
  const rpsChartData = useMemo(
    () => prepareRpsChartData(progressHistory, report, timeRange),
    [progressHistory, report, timeRange]
  );

  // 准备延迟分布数据
  const latencyBucketData = useMemo(
    () => prepareLatencyBuckets(progressHistory, report),
    [progressHistory, report]
  );

  // 计算全局总耗时（秒）用于决定时间刻度格式和范围选择器展示
  const totalElapsed = useMemo(() => {
    if (progressHistory.length > 0) {
      return progressHistory[progressHistory.length - 1].elapsedSeconds;
    }
    if (report && report.durationMs > 0) {
      return Math.round(report.durationMs / 1000);
    }
    return 0;
  }, [progressHistory, report]);

  // 当总时长超过 60 秒时，显示时间范围选择器
  const showRangeSelector = totalElapsed > 60;

  // 如果没有数据，显示占位符
  if (progressHistory.length === 0 && (!report || report.statistics.totalIterations === 0)) {
    return (
      <div className="flex items-center justify-center h-48 text-muted-foreground text-sm">
        {getLL().flow.stress_chart_no_data()}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      {/* RPS 实时曲线 */}
      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold text-foreground">{getLL().flow.stress_chart_rps()}</h3>
          {showRangeSelector && (
            <div className="flex items-center gap-1 bg-muted/40 p-0.5 rounded-md text-xs">
              <button
                type="button"
                onClick={() => setTimeRange('all')}
                className={cn(
                  'px-2 py-0.5 rounded text-xs font-medium transition-colors cursor-pointer',
                  timeRange === 'all'
                    ? 'bg-background text-foreground shadow-xs'
                    : 'text-muted-foreground hover:text-foreground'
                )}
              >
                {getLL().flow.stress_chart_range_all()}
              </button>
              <button
                type="button"
                onClick={() => setTimeRange('1m')}
                className={cn(
                  'px-2 py-0.5 rounded text-xs font-medium transition-colors cursor-pointer',
                  timeRange === '1m'
                    ? 'bg-background text-foreground shadow-xs'
                    : 'text-muted-foreground hover:text-foreground'
                )}
              >
                {getLL().flow.stress_chart_range_1m()}
              </button>
              {totalElapsed > 300 && (
                <button
                  type="button"
                  onClick={() => setTimeRange('5m')}
                  className={cn(
                    'px-2 py-0.5 rounded text-xs font-medium transition-colors cursor-pointer',
                    timeRange === '5m'
                      ? 'bg-background text-foreground shadow-xs'
                      : 'text-muted-foreground hover:text-foreground'
                  )}
                >
                  {getLL().flow.stress_chart_range_5m()}
                </button>
              )}
              {totalElapsed > 900 && (
                <button
                  type="button"
                  onClick={() => setTimeRange('15m')}
                  className={cn(
                    'px-2 py-0.5 rounded text-xs font-medium transition-colors cursor-pointer',
                    timeRange === '15m'
                      ? 'bg-background text-foreground shadow-xs'
                      : 'text-muted-foreground hover:text-foreground'
                  )}
                >
                  {getLL().flow.stress_chart_range_15m()}
                </button>
              )}
            </div>
          )}
        </div>
        <div className="h-44">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart
              data={rpsChartData}
              margin={{ top: 10, right: 15, left: 15, bottom: 5 }}
            >
              <CartesianGrid strokeDasharray="3 3" stroke="oklch(0.88 0 0)" />
              <XAxis
                dataKey="time"
                tick={{ fontSize: 11, fill: 'oklch(0.556 0 0)' }}
                tickLine={{ stroke: 'oklch(0.88 0 0)' }}
                axisLine={{ stroke: 'oklch(0.88 0 0)' }}
                minTickGap={25}
                tickFormatter={(val: number) => formatTimeTick(val, totalElapsed)}
                label={{
                  value: getLL().flow.stress_chart_rps_x(),
                  position: 'insideBottomRight',
                  offset: -5,
                  fontSize: 11,
                  fill: 'oklch(0.556 0 0)',
                }}
              />
              <YAxis
                domain={[0, 'auto']}
                allowDecimals={true}
                width={45}
                tick={{ fontSize: 11, fill: 'oklch(0.556 0 0)' }}
                tickLine={{ stroke: 'oklch(0.88 0 0)' }}
                axisLine={{ stroke: 'oklch(0.88 0 0)' }}
                label={{
                  value: 'RPS',
                  angle: -90,
                  position: 'insideLeft',
                  offset: 5,
                  style: { textAnchor: 'middle' },
                  fontSize: 11,
                  fill: 'oklch(0.556 0 0)',
                }}
              />
              <Tooltip content={<RpsTooltip maxDuration={totalElapsed} />} />
              <Line
                type="monotone"
                dataKey="rps"
                stroke="#3b82f6"
                strokeWidth={2}
                dot={rpsChartData.length <= 15 ? { r: 3, fill: '#3b82f6' } : false}
                activeDot={{ r: 5 }}
                isAnimationActive={false}
              />
            </LineChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* 响应时间分布 */}
      <div className="flex flex-col gap-2">
        <div className="flex flex-wrap items-baseline gap-x-2 gap-y-0.5">
          <h3 className="text-sm font-semibold text-foreground">{getLL().flow.stress_chart_response_dist()}</h3>
          <span className="text-xs text-muted-foreground">
            {getLL().flow.stress_chart_response_note()}
          </span>
        </div>
        <div className="h-44">
          <ResponsiveContainer width="100%" height="100%">
            <BarChart
              data={latencyBucketData}
              margin={{ top: 10, right: 15, left: 15, bottom: 5 }}
            >
              <CartesianGrid strokeDasharray="3 3" stroke="oklch(0.88 0 0)" vertical={false} />
              <XAxis
                dataKey="name"
                tick={{ fontSize: 11, fill: 'oklch(0.556 0 0)' }}
                tickLine={{ stroke: 'oklch(0.88 0 0)' }}
                axisLine={{ stroke: 'oklch(0.88 0 0)' }}
              />
              <YAxis
                allowDecimals={false}
                width={52}
                tick={{ fontSize: 11, fill: 'oklch(0.556 0 0)' }}
                tickLine={{ stroke: 'oklch(0.88 0 0)' }}
                axisLine={{ stroke: 'oklch(0.88 0 0)' }}
                label={{
                  value: getLL().flow.stress_chart_latency_y(),
                  angle: -90,
                  position: 'insideLeft',
                  offset: 5,
                  style: { textAnchor: 'middle' },
                  fontSize: 11,
                  fill: 'oklch(0.556 0 0)',
                }}
              />
              <Tooltip content={<LatencyTooltip />} />
              <Bar
                dataKey="count"
                radius={[4, 4, 0, 0]}
                isAnimationActive={false}
              />
            </BarChart>
          </ResponsiveContainer>
        </div>
      </div>
    </div>
  );
}
