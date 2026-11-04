/**
 * Delay 节点配置面板
 *
 * 只配置等待时长（支持变量模板）与随机抖动上限，上限与后端一致（1 小时）。
 */
import { useCallback, useRef, useState } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { DelayConfig as DelayConfigType } from '@/types/node';
import { useFlowVariables } from '@/hooks';
import { useCurrentLocale } from '@/hooks/useLocale';
import { getLL } from '@/i18n/i18n-lazy';
import { VariableInput } from '@/components/ui/VariableInput';

interface DelayConfigProps {
  /** 当前节点数据 */
  data: DelayConfigType;
  /** 保存回调 */
  onSave: (data: DelayConfigType) => void;
}

/** 面板内保持字符串草稿，避免输入中间态被数字强转吞掉 */
function normalize(data: DelayConfigType): DelayConfigType {
  return {
    type: 'flow:delay',
    duration_ms: data.duration_ms ?? '1000',
    jitter_ms: data.jitter_ms ?? 0,
  };
}

/**
 * Delay 节点配置面板
 */
export function DelayConfig({ data, onSave }: DelayConfigProps) {
  useCurrentLocale();
  const ll = getLL().panels.delay;
  const [config, setConfig] = useState<DelayConfigType>(() => normalize(data));
  const configRef = useRef(config);
  configRef.current = config;
  const { variables } = useFlowVariables();

  const update = useCallback(
    (patch: Partial<DelayConfigType>) => {
      const next = { ...configRef.current, ...patch };
      setConfig(next);
      onSave(next);
    },
    [onSave],
  );

  return (
    <div className="space-y-4 p-4">
      <div className="space-y-1.5 rounded-lg border border-border bg-card p-4">
        <Label className="text-xs font-medium text-muted-foreground" htmlFor="delay-duration">
          {ll.duration_ms()}
        </Label>
        <VariableInput
          id="delay-duration"
          className="font-mono text-sm"
          variables={variables}
          value={config.duration_ms}
          onValueChange={(value) => update({ duration_ms: value })}
        />
        <p className="text-xs text-muted-foreground">{ll.duration_hint()}</p>
      </div>

      <div className="space-y-1.5 rounded-lg border border-border bg-card p-4">
        <Label className="text-xs font-medium text-muted-foreground" htmlFor="delay-jitter">
          {ll.jitter_ms()}
        </Label>
        <Input
          id="delay-jitter"
          type="number"
          min={0}
          value={config.jitter_ms}
          onChange={(e) => {
            const value = Number.parseInt(e.target.value, 10);
            update({ jitter_ms: Number.isNaN(value) ? 0 : value });
          }}
        />
      </div>
    </div>
  );
}
