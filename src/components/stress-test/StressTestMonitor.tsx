/**
 * StressTestMonitor - 压力测试实时监控面板
 * 显示压测进度、实时指标和停止按钮
 */
import { Activity, Gauge, Timer, Square } from 'lucide-react';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  useStressTestStore,
  selectIsStressTestRunning,
  selectStressTestProgress,
  selectStressTestStatus,
} from '@/stores/stressTest';
import { cn } from '@/lib/utils';

/**
 * 指标卡片组件
 */
function MetricCard({
  icon: Icon,
  label,
  value,
  subValue,
  iconColor,
  className,
}: {
  icon: React.ElementType;
  label: string;
  value: string | number;
  subValue?: string;
  iconColor?: string;
  className?: string;
}) {
  return (
    <Card className={cn('border-border/50 bg-gradient-to-br from-background to-muted/30', className)}>
      <CardContent className="p-4">
        <div className="flex items-center gap-2 mb-2">
          <Icon className={cn('w-4 h-4', iconColor || 'text-muted-foreground')} />
          <span className="text-xs text-muted-foreground">{label}</span>
        </div>
        <p className="text-xl font-bold tabular-nums">{value}</p>
        {subValue && (
          <p className="text-xs text-muted-foreground mt-1">{subValue}</p>
        )}
      </CardContent>
    </Card>
  );
}

/**
 * 压力测试监控面板
 * 仅在压力测试运行时（rampUp 或 running 状态）渲染
 */
export function StressTestMonitor() {
  const isRunning = useStressTestStore(selectIsStressTestRunning);
  const progress = useStressTestStore(selectStressTestProgress);
  const status = useStressTestStore(selectStressTestStatus);
  const stopStressTest = useStressTestStore((state) => state.stopStressTest);

  // 仅在运行时渲染
  if (!isRunning) {
    return null;
  }

  // 获取状态显示文本
  const getStatusText = () => {
    switch (status) {
      case 'rampUp':
        return '预热中';
      case 'running':
        return '运行中';
      default:
        return '';
    }
  };

  // 获取状态 Badge 样式
  const getStatusBadgeVariant = () => {
    switch (status) {
      case 'rampUp':
        return 'bg-amber-50 text-amber-700 border-amber-200';
      case 'running':
        return 'bg-emerald-50 text-emerald-700 border-emerald-200';
      default:
        return '';
    }
  };

  // 处理停止压测
  const handleStopStressTest = async () => {
    try {
      await stopStressTest();
    } catch (error) {
      console.error('停止压测失败:', error);
    }
  };

  return (
    <Card className="border-border shadow-lg">
      <CardContent className="p-4">
        {/* 头部：状态和时间 */}
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <Activity className="w-5 h-5 text-blue-500" />
            <span className="font-semibold">压力测试</span>
            <Badge
              variant="outline"
              className={cn('font-medium', getStatusBadgeVariant())}
            >
              {getStatusText()}
            </Badge>
          </div>
          <div className="flex items-center gap-1.5 text-sm text-muted-foreground">
            <Timer className="w-4 h-4" />
            <span className="font-medium tabular-nums">
              {progress?.elapsedSeconds ?? 0} 秒
            </span>
          </div>
        </div>

        {/* 指标卡片网格 */}
        <div className="grid grid-cols-3 gap-3 mb-4">
          <MetricCard
            icon={Activity}
            label="执行次数"
            value={progress?.totalIterations ?? 0}
            subValue={
              progress
                ? `成功 ${progress.successfulIterations} / 失败 ${progress.failedIterations}`
                : undefined
            }
            iconColor="text-blue-500"
          />
          <MetricCard
            icon={Gauge}
            label="当前 RPS"
            value={progress?.currentRps?.toFixed(2) ?? '0.00'}
            iconColor="text-violet-500"
          />
          <MetricCard
            icon={Timer}
            label="平均响应时间"
            value={progress?.avgLatencyMs?.toFixed(0) ?? 0}
            subValue="ms"
            iconColor="text-emerald-500"
          />
        </div>

        {/* 底部：停止按钮 */}
        <div className="flex justify-end">
          <Button
            variant="destructive"
            size="sm"
            onClick={handleStopStressTest}
            className="gap-1.5"
          >
            <Square className="w-4 h-4" />
            停止压测
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
