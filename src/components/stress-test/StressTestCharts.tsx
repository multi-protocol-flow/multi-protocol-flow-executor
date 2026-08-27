/**
 * StressTestCharts - 压力测试可视化图表
 * 包含 RPS 实时曲线和响应时间分布图
 */
import { useMemo } from 'react';
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

// ============================================================================
// Types
// ============================================================================

interface StressTestChartsProps {
  /** 进度历史数据 - 从父组件传入 */
  progressHistory: StressTestProgress[];
  /** 压测报告（可选，用于在采样未捕获到最终完成点时补充完整指标） */
  report?: StressTestReport | null;
}

interface RpsChartDataPoint {
  time: number;
  rps: number;
}

interface LatencyBucketData {
  name: string;
  count: number;
  fill: string;
}

// ============================================================================
// Constants
// ============================================================================

/** 最大显示数据点数 */
const MAX_DATA_POINTS = 60;

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
 * 将进度历史转换为 RPS 图表数据
 * 限制为最近 60 个数据点
 */
function prepareRpsChartData(
  history: StressTestProgress[],
  report?: StressTestReport | null
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

  const sliced = baseHistory.slice(-MAX_DATA_POINTS);
  return sliced.map((progress) => ({
    time: progress.elapsedSeconds,
    rps: progress.currentRps,
  }));
}

/**
 * 根据延迟值计算延迟分布桶
 * 结合进度历史与最终报告统计
 */
function prepareLatencyBuckets(
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
}

interface LatencyTooltipProps {
  active?: boolean;
  payload?: Array<{ payload: LatencyBucketData }>;
}

function RpsTooltip({ active, payload, label }: RpsTooltipProps) {
  if (!active || !payload || payload.length === 0) {
    return null;
  }

  return (
    <div className="bg-popover border border-border rounded-lg px-3 py-2 shadow-md">
      <p className="text-xs text-muted-foreground mb-1">
        {getLL().flow.stress_chart_tooltip_time()}<span className="font-medium text-foreground">{label}{getLL().flow.stress_chart_tooltip_seconds()}</span>
      </p>
      <p className="text-xs text-muted-foreground">
        RPS: <span className="font-medium text-blue-600">{formatRps(payload[0].value)}</span>
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
        {data.name}: <span className="font-medium text-foreground">{data.count}{getLL().flow.stress_chart_tooltip_seconds()}</span>
      </p>
    </div>
  );
}

// ============================================================================
// Component
// ============================================================================

export function StressTestCharts({ progressHistory, report }: StressTestChartsProps) {
  // 准备 RPS 图表数据
  const rpsChartData = useMemo(
    () => prepareRpsChartData(progressHistory, report),
    [progressHistory, report]
  );

  // 准备延迟分布数据
  const latencyBucketData = useMemo(
    () => prepareLatencyBuckets(progressHistory, report),
    [progressHistory, report]
  );

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
        <h3 className="text-sm font-semibold text-foreground">{getLL().flow.stress_chart_rps()}</h3>
        <div className="h-40">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart
              data={rpsChartData}
              margin={{ top: 5, right: 20, left: 0, bottom: 5 }}
            >
              <CartesianGrid strokeDasharray="3 3" stroke="oklch(0.88 0 0)" />
              <XAxis
                dataKey="time"
                tick={{ fontSize: 11, fill: 'oklch(0.556 0 0)' }}
                tickLine={{ stroke: 'oklch(0.88 0 0)' }}
                axisLine={{ stroke: 'oklch(0.88 0 0)' }}
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
                tickLine={{ stroke: 'oklch(0.88 0 0)' }}
                axisLine={{ stroke: 'oklch(0.88 0 0)' }}
                label={{
                  value: 'RPS',
                  angle: -90,
                  position: 'insideLeft',
                  offset: 10,
                  fontSize: 11,
                  fill: 'oklch(0.556 0 0)',
                }}
              />
              <Tooltip content={<RpsTooltip />} />
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
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-semibold text-foreground">{getLL().flow.stress_chart_response_dist()}</h3>
          <span className="text-xs text-muted-foreground">
            · {getLL().flow.stress_chart_response_note()}
          </span>
        </div>
        <div className="h-40">
          <ResponsiveContainer width="100%" height="100%">
            <BarChart
              data={latencyBucketData}
              margin={{ top: 5, right: 20, left: 0, bottom: 5 }}
            >
              <CartesianGrid strokeDasharray="3 3" stroke="oklch(0.88 0 0)" vertical={false} />
              <XAxis
                dataKey="name"
                tick={{ fontSize: 10, fill: 'oklch(0.556 0 0)' }}
                tickLine={{ stroke: 'oklch(0.88 0 0)' }}
                axisLine={{ stroke: 'oklch(0.88 0 0)' }}
              />
              <YAxis
                tick={{ fontSize: 11, fill: 'oklch(0.556 0 0)' }}
                tickLine={{ stroke: 'oklch(0.88 0 0)' }}
                axisLine={{ stroke: 'oklch(0.88 0 0)' }}
                label={{
                  value: getLL().flow.stress_chart_latency_y(),
                  angle: -90,
                  position: 'insideLeft',
                  offset: 10,
                  fontSize: 11,
                  fill: 'oklch(0.556 0 0)',
                }}
              />
              <Tooltip content={<LatencyTooltip />} />
              <Bar
                dataKey="count"
                isAnimationActive={false}
              />
            </BarChart>
          </ResponsiveContainer>
        </div>
      </div>
    </div>
  );
}
