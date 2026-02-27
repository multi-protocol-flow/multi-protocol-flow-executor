/**
 * Entry 节点配置组件
 * 入口节点没有配置项，仅显示说明文字
 */
import { PlayCircle, Info, AlertCircle } from 'lucide-react';

export function EntryConfig() {
  return (
    <div className="space-y-4">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-emerald-200/50 bg-gradient-to-br from-emerald-50/80 to-teal-50/50 p-4">
        <div className="flex items-start gap-3">
          <div className="mt-0.5 rounded-full bg-emerald-100 p-1.5">
            <PlayCircle className="h-4 w-4 text-emerald-600" />
          </div>
          <div className="flex-1 space-y-2">
            <h4 className="text-sm font-semibold text-emerald-900">
              入口节点
            </h4>
            <p className="text-sm leading-relaxed text-emerald-700/90">
              入口节点是流程的起点。每个流程必须有且仅有一个入口节点，
              流程执行将从这里开始，沿着连接线依次执行各个节点。
            </p>
          </div>
        </div>
      </div>

      {/* 功能说明列表 */}
      <div className="space-y-3">
        <div className="flex items-start gap-3 rounded-md bg-muted/30 p-3">
          <Info className="mt-0.5 h-4 w-4 shrink-0 text-sky-500" />
          <div>
            <p className="text-sm font-medium text-foreground">无需配置</p>
            <p className="mt-1 text-xs text-muted-foreground">
              入口节点没有配置参数，只需将其作为流程的起始点连接到其他节点即可。
            </p>
          </div>
        </div>

        <div className="flex items-start gap-3 rounded-md bg-muted/30 p-3">
          <AlertCircle className="mt-0.5 h-4 w-4 shrink-0 text-amber-500" />
          <div>
            <p className="text-sm font-medium text-foreground">执行流程</p>
            <p className="mt-1 text-xs text-muted-foreground">
              点击工具栏中的「执行」按钮即可开始执行整个流程。
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
