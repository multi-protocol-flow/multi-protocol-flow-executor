/**
 * 本地命令节点配置面板（`local:exec`）
 *
 * 安全语义（与后端一致）：
 * - argv 模式：`args` 逐项做变量替换后作为单个参数传递，不经 shell、不拆词；
 * - shell 模式：`script` 内**禁止** `{{ }}`，变量只能通过注入的 `MPE_VAR_*` 环境变量访问。
 */
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { ChevronDown, ChevronUp, Plus, Trash2 } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { VariableInput } from '@/components/ui/VariableInput';
import { VariableTextarea } from '@/components/ui/VariableTextarea';
import { useFlowVariables } from '@/hooks';
import { useCurrentLocale } from '@/hooks/useLocale';
import { getLL } from '@/i18n/i18n-lazy';
import type { LocalExecConfig } from '@/types/node';

interface LocalExecConfigProps {
  /** 当前节点数据 */
  data: LocalExecConfig;
  /** 保存回调 */
  onSave: (data: LocalExecConfig) => void;
}

const SHELL_OPTIONS = ['bash', 'sh', 'cmd', 'powershell', 'pwsh'];

/** 面板内保持草稿形态，缺省值补齐后交给后端 */
function normalize(data: LocalExecConfig): LocalExecConfig {
  return {
    type: 'local:exec',
    mode: data.mode ?? 'argv',
    program: data.program ?? '',
    args: data.args ?? [],
    shell: data.shell ?? '',
    script: data.script ?? '',
    env: data.env ?? {},
    inherit_env: data.inherit_env ?? [],
    cwd: data.cwd ?? '',
    timeout_ms: data.timeout_ms ?? 600000,
    max_output_bytes: data.max_output_bytes ?? 1048576,
    capture: data.capture,
  };
}

/**
 * 本地命令节点配置面板
 */
export function LocalExecConfig({ data, onSave }: LocalExecConfigProps) {
  useCurrentLocale();
  const ll = getLL().panels.localExec;
  const [config, setConfig] = useState<LocalExecConfig>(() => normalize(data));
  const configRef = useRef(config);
  configRef.current = config;
  const { variables } = useFlowVariables();

  // 行标识：参数顺序即语义（ffmpeg 的 -vf/-i 等位置敏感），因此上移/下移必须
  // 交换「行」而不是「索引对应的值」——用 index 作 React key 会让被点击的按钮
  // 在交换后仍绑定原索引，连续点击就会移动错的那一条。
  const [rowIds, setRowIds] = useState<number[]>(() =>
    Array.from({ length: normalize(data).args?.length ?? 0 }, (_, index) => index),
  );
  const nextRowId = useRef(rowIds.length);

  const update = useCallback(
    (patch: Partial<LocalExecConfig>) => {
      const next = { ...configRef.current, ...patch };
      setConfig(next);
      onSave(next);
    },
    [onSave],
  );

  const mode = config.mode ?? 'argv';
  const args = useMemo(() => config.args ?? [], [config.args]);
  const envEntries = useMemo(() => Object.entries(config.env ?? {}), [config.env]);
  const scriptHasTemplate = mode === 'shell' && (config.script ?? '').includes('{{');

  // 外部（切换节点 / 加载流程）改变参数条数时重建行标识
  useEffect(() => {
    if (rowIds.length !== args.length) {
      setRowIds(Array.from({ length: args.length }, () => nextRowId.current++));
    }
  }, [args.length, rowIds.length]);

  const setArg = useCallback(
    (index: number, value: string) => {
      const nextArgs = [...(configRef.current.args ?? [])];
      nextArgs[index] = value;
      update({ args: nextArgs });
    },
    [update],
  );

  const addArg = useCallback(() => {
    update({ args: [...(configRef.current.args ?? []), ''] });
    setRowIds((ids) => [...ids, nextRowId.current++]);
  }, [update]);

  const removeArg = useCallback(
    (index: number) => {
      const nextArgs = [...(configRef.current.args ?? [])];
      nextArgs.splice(index, 1);
      update({ args: nextArgs });
      setRowIds((ids) => ids.filter((_, i) => i !== index));
    },
    [update],
  );

  /** 上移 / 下移一个参数（`delta` 取 -1 或 1） */
  const moveArg = useCallback(
    (index: number, delta: -1 | 1) => {
      const current = [...(configRef.current.args ?? [])];
      const target = index + delta;
      if (target < 0 || target >= current.length) return;
      [current[index], current[target]] = [current[target], current[index]];
      update({ args: current });
      setRowIds((ids) => {
        const next = [...ids];
        [next[index], next[target]] = [next[target], next[index]];
        return next;
      });
    },
    [update],
  );

  const setEnvKey = useCallback(
    (oldKey: string, newKey: string) => {
      const current = { ...(configRef.current.env ?? {}) };
      const value = current[oldKey] ?? '';
      delete current[oldKey];
      current[newKey] = value;
      update({ env: current });
    },
    [update],
  );

  const setEnvValue = useCallback(
    (key: string, value: string) => {
      const current = { ...(configRef.current.env ?? {}) };
      current[key] = value;
      update({ env: current });
    },
    [update],
  );

  const removeEnv = useCallback(
    (key: string) => {
      const current = { ...(configRef.current.env ?? {}) };
      delete current[key];
      update({ env: current });
    },
    [update],
  );

  return (
    <div className="w-full min-w-0 space-y-4">
      {/* 模式切换 */}
      <div className="space-y-2 rounded-lg border border-border bg-card p-4">
        <Label className="text-xs font-medium text-muted-foreground">{ll.mode()}</Label>
        <Select
          value={mode}
          onValueChange={(value) => update({ mode: value === 'shell' ? 'shell' : 'argv' })}
        >
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="argv">{ll.mode_argv()}</SelectItem>
            <SelectItem value="shell">{ll.mode_shell()}</SelectItem>
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          {mode === 'argv' ? ll.argv_hint() : ll.shell_hint()}
        </p>
      </div>

      {mode === 'argv' ? (
        <>
          <div className="space-y-1.5 rounded-lg border border-border bg-card p-4">
            <Label className="text-xs font-medium text-muted-foreground" htmlFor="local-exec-program">
              {ll.program()}
            </Label>
            <VariableInput
              id="local-exec-program"
              className="font-mono text-sm"
              variables={variables}
              value={config.program ?? ''}
              onValueChange={(value) => update({ program: value })}
            />
            <p className="text-xs text-muted-foreground">{ll.program_hint()}</p>
          </div>

          <div className="space-y-2 rounded-lg border border-border bg-card p-4">
            <Label className="text-xs font-medium text-muted-foreground">{ll.args()}</Label>
            {args.map((arg, index) => (
              <div key={rowIds[index] ?? index} className="flex items-center gap-1.5">
                <VariableInput
                  containerClassName="min-w-0 flex-1"
                  className="font-mono text-sm"
                  variables={variables}
                  value={arg}
                  placeholder={ll.arg_placeholder()}
                  onValueChange={(value) => setArg(index, value)}
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 shrink-0"
                  title={ll.move_up()}
                  aria-label={ll.move_up()}
                  disabled={index === 0}
                  onClick={() => moveArg(index, -1)}
                >
                  <ChevronUp className="h-4 w-4" />
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 shrink-0"
                  title={ll.move_down()}
                  aria-label={ll.move_down()}
                  disabled={index === args.length - 1}
                  onClick={() => moveArg(index, 1)}
                >
                  <ChevronDown className="h-4 w-4" />
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 shrink-0"
                  title={ll.remove_arg()}
                  aria-label={ll.remove_arg()}
                  onClick={() => removeArg(index)}
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </div>
            ))}
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="w-full justify-center"
              onClick={addArg}
            >
              <Plus className="mr-1 h-4 w-4" />
              {ll.add_arg()}
            </Button>
            <p className="text-xs text-muted-foreground">{ll.args_order_hint()}</p>
          </div>
        </>
      ) : (
        <div className="space-y-1.5 rounded-lg border border-border bg-card p-4">
          <Label className="text-xs font-medium text-muted-foreground">{ll.shell()}</Label>
          <Select
            value={config.shell === '' ? 'bash' : (config.shell ?? 'bash')}
            onValueChange={(value) => update({ shell: value })}
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {SHELL_OPTIONS.map((option) => (
                <SelectItem key={option} value={option}>
                  {option}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <Label className="mt-3 text-xs font-medium text-muted-foreground" htmlFor="local-exec-script">
            {ll.script()}
          </Label>
          <VariableTextarea
            id="local-exec-script"
            className="font-mono text-sm"
            rows={6}
            variables={variables}
            value={config.script ?? ''}
            onValueChange={(value) => update({ script: value })}
          />
          <p className={`text-xs ${scriptHasTemplate ? 'text-red-500' : 'text-muted-foreground'}`}>
            {scriptHasTemplate ? ll.script_template_error() : ll.shell_hint()}
          </p>
        </div>
      )}

      {/* env 注入 */}
      <div className="space-y-2 rounded-lg border border-border bg-card p-4">
        <Label className="text-xs font-medium text-muted-foreground">{ll.env()}</Label>
        <p className="text-xs text-muted-foreground">{ll.env_hint()}</p>
        {envEntries.map(([key, value]) => (
          <div key={key} className="flex items-start gap-2">
            <div className="grid min-w-0 flex-1 gap-2">
              <Input
                className="min-w-0 font-mono text-sm"
                value={key}
                placeholder={ll.env_name()}
                onChange={(event) => setEnvKey(key, event.target.value)}
              />
              <VariableInput
                className="min-w-0 font-mono text-sm"
                variables={variables}
                value={value}
                placeholder={ll.env_value()}
                onValueChange={(next) => setEnvValue(key, next)}
              />
            </div>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              title={ll.remove_arg()}
              onClick={() => removeEnv(key)}
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          </div>
        ))}
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="w-full justify-center"
          onClick={() => update({ env: { ...(config.env ?? {}), [`var${envEntries.length + 1}`]: '' } })}
        >
          <Plus className="mr-1 h-4 w-4" />
          {ll.add_env()}
        </Button>
      </div>

      {/* cwd / 继承 / 上限 */}
      <div className="space-y-3 rounded-lg border border-border bg-card p-4">
        <div className="space-y-1.5">
          <Label className="text-xs font-medium text-muted-foreground" htmlFor="local-exec-cwd">
            {ll.cwd()}
          </Label>
          <VariableInput
            id="local-exec-cwd"
            className="font-mono text-sm"
            variables={variables}
            value={config.cwd ?? ''}
            onValueChange={(value) => update({ cwd: value })}
          />
          <p className="text-xs text-muted-foreground">{ll.cwd_hint()}</p>
        </div>

        <div className="space-y-1.5">
          <Label
            className="text-xs font-medium text-muted-foreground"
            htmlFor="local-exec-inherit-env"
          >
            {ll.inherit_env()}
          </Label>
          <Input
            id="local-exec-inherit-env"
            className="font-mono text-sm"
            value={(config.inherit_env ?? []).join(',')}
            onChange={(event) =>
              update({
                inherit_env: event.target.value
                  .split(',')
                  .map((item) => item.trim())
                  .filter((item) => item !== ''),
              })
            }
          />
          <p className="text-xs text-muted-foreground">{ll.inherit_env_hint()}</p>
        </div>

        <div className="space-y-1.5">
          <Label className="text-xs font-medium text-muted-foreground" htmlFor="local-exec-timeout">
            {ll.timeout_ms()}
          </Label>
          <Input
            id="local-exec-timeout"
            type="number"
            min={1}
            max={3600000}
            value={config.timeout_ms ?? 600000}
            onChange={(event) => {
              const value = Number.parseInt(event.target.value, 10);
              update({ timeout_ms: Number.isNaN(value) ? 600000 : value });
            }}
          />
          <p className="text-xs text-muted-foreground">{ll.timeout_hint()}</p>
        </div>

        <div className="space-y-1.5">
          <Label className="text-xs font-medium text-muted-foreground" htmlFor="local-exec-max-output">
            {ll.max_output_bytes()}
          </Label>
          <Input
            id="local-exec-max-output"
            type="number"
            min={1}
            max={16 * 1024 * 1024}
            value={config.max_output_bytes ?? 1048576}
            onChange={(event) => {
              const value = Number.parseInt(event.target.value, 10);
              update({ max_output_bytes: Number.isNaN(value) ? 1048576 : value });
            }}
          />
          <p className="text-xs text-muted-foreground">{ll.max_output_bytes_hint()}</p>
        </div>
      </div>

      {/* 产物捕获 */}
      <div className="space-y-2 rounded-lg border border-border bg-card p-4">
        <Label className="text-xs font-medium text-muted-foreground">{ll.capture()}</Label>
        <VariableInput
          className="min-w-0 font-mono text-sm"
          variables={variables}
          value={config.capture?.path ?? ''}
          placeholder={ll.capture_path()}
          onValueChange={(value) =>
            update({
              capture: value === '' ? undefined : { path: value, max_bytes: config.capture?.max_bytes },
            })
          }
        />
        <p className="text-xs text-muted-foreground">{ll.capture_path_hint()}</p>
        {config.capture && (
          <>
            <Input
              type="number"
              min={1}
              max={64 * 1024 * 1024}
              value={config.capture.max_bytes ?? 4 * 1024 * 1024}
              onChange={(event) => {
                const value = Number.parseInt(event.target.value, 10);
                update({
                  capture: {
                    path: config.capture?.path ?? '',
                    max_bytes: Number.isNaN(value) ? 4 * 1024 * 1024 : value,
                  },
                });
              }}
            />
            <p className="text-xs text-muted-foreground">{ll.capture_max_bytes_hint()}</p>
          </>
        )}
      </div>
    </div>
  );
}
