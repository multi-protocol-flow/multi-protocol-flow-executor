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
import type { StressTestProgress } from '@/types/stressTest';

// ============================================================================
// Types
// ============================================================================

interface StressTestChartsProps {
  /** 进度历史数据 - 从父组件传入 */
  progressHistory: StressTestProgress[];
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
function prepareRpsChartData(history: StressTestProgress[]): RpsChartDataPoint[] {
  const sliced = history.slice(-MAX_DATA_POINTS);
  return sliced.map((progress) => ({
    time: progress.elapsedSeconds,
    rps: progress.currentRps,
  }));
}

/**
 * 根据延迟值计算延迟分布桶
 * 注意：由于只有 avgLatencyMs，这是按采样点（每秒一次）统计的分布，非请求级别分布
 */
function prepareLatencyBuckets(history: StressTestProgress[]): LatencyBucketData[] {
  // 初始化桶计数
  const bucketCounts = LATENCY_BUCKETS.map(() => 0);

  // 遍历历史数据，将每个数据点的 avgLatencyMs 分配到桶中
  history.forEach((progress) => {
    const latency = progress.avgLatencyMs;
    for (let i = 0; i < LATENCY_BUCKETS.length; i++) {
      if (latency < LATENCY_BUCKETS[i].max) {
        bucketCounts[i]++;
        break;
      }
    }
  });

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
        时间: <span className="font-medium text-foreground">{label}秒</span>
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
        {data.name}: <span className="font-medium text-foreground">{data.count} 秒</span>
      </p>
    </div>
  );
}

// ============================================================================
// Component
// ============================================================================

export function StressTestCharts({ progressHistory }: StressTestChartsProps) {
  // 准备 RPS 图表数据
  const rpsChartData = useMemo(
    () => prepareRpsChartData(progressHistory),
    [progressHistory]
  );

  // 准备延迟分布数据
  const latencyBucketData = useMemo(
    () => prepareLatencyBuckets(progressHistory),
    [progressHistory]
  );

  // 如果没有数据，显示占位符
  if (progressHistory.length === 0) {
    return (
      <div className="flex items-center justify-center h-48 text-muted-foreground text-sm">
        暂无数据
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      {/* RPS 实时曲线 */}
      <div className="flex flex-col gap-2">
        <h3 className="text-sm font-semibold text-foreground">RPS 实时曲线</h3>
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
                  value: '时间（秒）',
                  position: 'insideBottomRight',
                  offset: -5,
                  fontSize: 11,
                  fill: 'oklch(0.556 0 0)',
                }}
              />
              <YAxis
                tick={{ fontSize: 11, fill: 'oklch(0.556 0 0)' }}
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
                dot={false}
                isAnimationActive={false}
              />
            </LineChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* 响应时间分布 */}
      <div className="flex flex-col gap-2">
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-semibold text-foreground">响应时间分布</h3>
          <span className="text-xs text-muted-foreground">
            · 每秒采样一次平均延迟，统计各延迟区间对应的采样秒数
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
                  value: '采样秒数',
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
