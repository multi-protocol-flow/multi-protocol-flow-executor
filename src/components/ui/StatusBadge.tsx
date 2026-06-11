/**
 * StatusBadge - 统一的状态展示 Badge
 *
 * 基于 shadcn/ui Badge 的条件渲染封装：
 * - isSuccess=true  → 绿色 (variant="default")
 * - isSuccess=false → 红色 (variant="destructive")
 *
 * 替代各个 viewer 文件中内联的 <Badge variant={...}> 模式
 */
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';

export interface StatusBadgeProps {
  /** 是否成功状态 */
  isSuccess: boolean;
  /** 成功时显示的文本 */
  successText: string;
  /** 失败时显示的文本 */
  failText: string;
  /** 自定义 class 覆盖 */
  className?: string;
}

function StatusBadge({ isSuccess, successText, failText, className }: StatusBadgeProps) {
  return (
    <Badge
      variant={isSuccess ? 'default' : 'destructive'}
      className={cn('font-mono text-xs', className)}
    >
      {isSuccess ? successText : failText}
    </Badge>
  );
}

export { StatusBadge };
export default StatusBadge;
