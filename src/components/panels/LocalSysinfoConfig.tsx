/**
 * 本机信息节点配置面板（`local:sysinfo`）
 *
 * 只读探测，不需要审批；采集项多选 + 可选磁盘路径。
 */
import { useCallback, useRef, useState } from 'react';
import { Label } from '@/components/ui/label';
import { Checkbox } from '@/components/ui/checkbox';
import { VariableInput } from '@/components/ui/VariableInput';
import { useFlowVariables } from '@/hooks';
import { useCurrentLocale } from '@/hooks/useLocale';
import { getLL } from '@/i18n/i18n-lazy';
import type { LocalSysinfoConfig } from '@/types/node';

interface LocalSysinfoConfigProps {
  /** 当前节点数据 */
  data: LocalSysinfoConfig;
  /** 保存回调 */
  onSave: (data: LocalSysinfoConfig) => void;
}

/** 支持的采集项（与后端 `SUPPORTED_CHECKS` 一致） */
const CHECK_OPTIONS = ['os', 'cpu', 'mem', 'disk', 'ffmpeg'] as const;

function normalize(data: LocalSysinfoConfig): LocalSysinfoConfig {
  return {
    type: 'local:sysinfo',
    checks: data.checks ?? ['os', 'cpu', 'mem', 'ffmpeg'],
    disk_path: data.disk_path ?? '',
  };
}

/**
 * 本机信息节点配置面板
 */
export function LocalSysinfoConfig({ data, onSave }: LocalSysinfoConfigProps) {
  useCurrentLocale();
  const ll = getLL().panels.localSysinfo;
  const [config, setConfig] = useState<LocalSysinfoConfig>(() => normalize(data));
  const configRef = useRef(config);
  configRef.current = config;
  const { variables } = useFlowVariables();

  const update = useCallback(
    (patch: Partial<LocalSysinfoConfig>) => {
      const next = { ...configRef.current, ...patch };
      setConfig(next);
      onSave(next);
    },
    [onSave],
  );

  const toggleCheck = useCallback(
    (check: string, checked: boolean) => {
      const current = configRef.current.checks ?? [];
      const next = checked ? [...current.filter((item) => item !== check), check] : current.filter((item) => item !== check);
      update({ checks: CHECK_OPTIONS.filter((option) => next.includes(option)) });
    },
    [update],
  );

  const labelFor = (check: string): string => {
    switch (check) {
      case 'os':
        return ll.check_os();
      case 'cpu':
        return ll.check_cpu();
      case 'mem':
        return ll.check_mem();
      case 'disk':
        return ll.check_disk();
      default:
        return ll.check_ffmpeg();
    }
  };

  return (
    <div className="w-full min-w-0 space-y-4">
      <div className="space-y-2 rounded-lg border border-border bg-card p-4">
        <Label className="text-xs font-medium text-muted-foreground">{ll.checks()}</Label>
        <p className="text-xs text-muted-foreground">{ll.checks_hint()}</p>
        {CHECK_OPTIONS.map((check) => (
          <div key={check} className="flex items-center gap-2">
            <Checkbox
              id={`local-sysinfo-${check}`}
              checked={(config.checks ?? []).includes(check)}
              onCheckedChange={(checked) => toggleCheck(check, checked === true)}
            />
            <Label htmlFor={`local-sysinfo-${check}`} className="text-sm">
              {labelFor(check)}
            </Label>
          </div>
        ))}
      </div>

      <div className="space-y-1.5 rounded-lg border border-border bg-card p-4">
        <Label className="text-xs font-medium text-muted-foreground" htmlFor="local-sysinfo-disk-path">
          {ll.disk_path()}
        </Label>
        <VariableInput
          id="local-sysinfo-disk-path"
          className="font-mono text-sm"
          variables={variables}
          value={config.disk_path ?? ''}
          onValueChange={(value) => update({ disk_path: value })}
        />
        <p className="text-xs text-muted-foreground">{ll.disk_path_hint()}</p>
      </div>
    </div>
  );
}
