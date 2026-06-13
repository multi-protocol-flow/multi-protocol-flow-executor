/**
 * OnErrorSection - 节点执行错误处理策略选择组件
 *
 * 允许用户选择执行失败时的处理策略:
 * - route_to_false: 路由到 false 分支，流程继续执行
 * - abort_flow: 立即终止整个流程
 * - ignore: 忽略错误，继续正常执行
 */
import { getLL } from '@/i18n/i18n-lazy';
import type { ExecutionErrorStrategy } from '@/types/node';

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

export interface OnErrorSectionProps {
  /** 当前选中的错误处理策略 */
  value?: ExecutionErrorStrategy;
  /** 策略变更回调 */
  onChange: (value: ExecutionErrorStrategy) => void;
}

export function OnErrorSection({ value, onChange }: OnErrorSectionProps) {
  // undefined (unset) defaults to RouteToFalse behavior
  const effective = value ?? 'route_to_false';

  return (
    <div className="space-y-2 pb-3">
      {/* Section title */}
      <div className="text-sm font-medium text-foreground">
        {getLL().panels.on_error_section.title()}
      </div>

      {/* Select dropdown */}
      <Select
        value={effective}
        onValueChange={(v) => onChange(v as ExecutionErrorStrategy)}
      >
        <SelectTrigger className="w-full">
          <SelectValue placeholder={getLL().panels.on_error_section.title()} />
        </SelectTrigger>
        <SelectContent>
          {/* Route to False */}
          <SelectItem value="route_to_false">
            {getLL().panels.on_error_section.route_to_false()}
          </SelectItem>

          {/* Abort Flow */}
          <SelectItem value="abort_flow">
            {getLL().panels.on_error_section.abort_flow()}
          </SelectItem>

          {/* Ignore */}
          <SelectItem value="ignore">
            {getLL().panels.on_error_section.ignore()}
          </SelectItem>
        </SelectContent>
      </Select>

      {/* Dynamic description based on selection */}
      <p className="text-xs text-muted-foreground">
        {effective === 'route_to_false'
          ? getLL().panels.on_error_section.route_to_false_desc()
          : effective === 'abort_flow'
            ? getLL().panels.on_error_section.abort_flow_desc()
            : getLL().panels.on_error_section.ignore_desc()}
      </p>
    </div>
  );
}

export default OnErrorSection;
