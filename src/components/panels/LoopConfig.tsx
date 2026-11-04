/**
 * Loop 节点配置面板
 *
 * 来源三选一（`count` / `items` / `until`），节奏与护栏分区配置，
 * `output_mappings` 用列表编辑器，`body` 只透传（由画布下钻编辑）。
 */
import { useCallback, useMemo, useRef, useState } from 'react';
import { Plus, Trash2, RotateCw } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { VariableTextarea } from '@/components/ui/VariableTextarea';
import type { LoopConfig as LoopConfigType, LoopSource } from '@/types/node';
import type { OutputMapping } from '@/types/variable';
import { useFlowStore } from '@/stores';
import { useFlowVariables } from '@/hooks';
import { useCurrentLocale } from '@/hooks/useLocale';
import { getLL } from '@/i18n/i18n-lazy';
import { cn } from '@/lib/utils';

interface LoopConfigProps {
  /** 当前节点数据 */
  data: LoopConfigType;
  /** 保存回调 */
  onSave: (data: LoopConfigType) => void;
}

/** 补齐缺省字段，保证面板状态与后端结构一致 */
function normalize(data: LoopConfigType): LoopConfigType {
  return {
    type: 'flow:loop',
    source: data.source ?? 'count',
    items: data.items,
    count: data.count ?? 1,
    until: data.until,
    interval_ms: data.interval_ms ?? 0,
    interval_backoff: data.interval_backoff ?? 1,
    interval_max_ms: data.interval_max_ms ?? 10000,
    initial_delay_ms: data.initial_delay_ms ?? 0,
    max_iterations: data.max_iterations ?? 100,
    concurrency: data.concurrency ?? 1,
    body: data.body ?? { nodes: [], connections: [] },
    output_mappings: data.output_mappings ?? [],
    break_on_error: data.break_on_error ?? true,
    timeout_ms: data.timeout_ms ?? 600000,
    report: data.report ?? { detail_iterations: 20, always_keep_failed: true },
  };
}

/** 输出路径 JSONPath 预设 */
const PATH_PRESETS = ['$.last.status', '$.last.body', '$.last.progress'];

/**
 * Loop 节点配置面板
 */
export function LoopConfig({ data, onSave }: LoopConfigProps) {
  useCurrentLocale();
  const ll = getLL().panels.loop;
  const [config, setConfig] = useState<LoopConfigType>(() => normalize(data));
  const configRef = useRef(config);
  configRef.current = config;

  const { variables, nodeLabels } = useFlowVariables();
  const selectedNodeId = useFlowStore((state) => state.selectedNodeId);
  const enterLoop = useFlowStore((state) => state.enterLoop);

  const bodyNodeCount = useMemo(() => config.body?.nodes?.length ?? 0, [config.body]);

  /**
   * 保存：`body` 永远取自入参，面板不编辑也不丢弃循环体
   */
  const update = useCallback(
    (patch: Partial<LoopConfigType>) => {
      const next: LoopConfigType = { ...configRef.current, ...patch, body: data.body };
      setConfig(next);
      onSave(next);
    },
    [data.body, onSave],
  );

  const setSource = (source: LoopSource) => update({ source });

  const updateMapping = (index: number, patch: Partial<OutputMapping>) => {
    const mappings = [...(configRef.current.output_mappings ?? [])];
    mappings[index] = { ...mappings[index], ...patch };
    update({ output_mappings: mappings });
  };

  const addMapping = () => {
    const mappings = [...(configRef.current.output_mappings ?? [])];
    mappings.push({ source: '', target: '', description: undefined });
    update({ output_mappings: mappings });
  };

  const removeMapping = (index: number) => {
    const mappings = [...(configRef.current.output_mappings ?? [])];
    mappings.splice(index, 1);
    update({ output_mappings: mappings });
  };

  const sourceOptions: Array<{ value: LoopSource; label: string }> = [
    { value: 'count', label: ll.source_count() },
    { value: 'items', label: ll.source_items() },
    { value: 'until', label: ll.source_until() },
  ];

  return (
    <div className="space-y-6 text-sm">
      {/* 1. 来源 */}
      <div className="space-y-3 rounded-lg border border-border bg-card p-4">
        <Label className="font-semibold">{ll.source()}</Label>
        <div className="grid grid-cols-3 gap-2">
          {sourceOptions.map((opt) => {
            const isActive = config.source === opt.value;
            return (
              <button
                key={opt.value}
                type="button"
                aria-pressed={isActive}
                onClick={() => setSource(opt.value)}
                className={cn(
                  'flex items-center justify-center gap-1.5 rounded-lg border-2 p-2.5 text-xs font-medium transition-all',
                  isActive
                    ? 'border-sky-500 bg-sky-50 text-sky-700 dark:bg-sky-900/30 dark:text-sky-300'
                    : 'border-border text-muted-foreground hover:bg-accent',
                )}
              >
                {opt.label}
              </button>
            );
          })}
        </div>

        {config.source === 'count' && (
          <div className="space-y-1.5">
            <Label className="text-xs font-medium text-muted-foreground" htmlFor="loop-count">
              {ll.count()}
            </Label>
            <Input
              id="loop-count"
              type="number"
              min={1}
              value={config.count ?? 1}
              onChange={(e) => {
                const value = Number.parseInt(e.target.value, 10);
                update({ count: Number.isNaN(value) ? 0 : value });
              }}
            />
          </div>
        )}

        {config.source === 'items' && (
          <div className="space-y-1.5">
            <Label className="text-xs font-medium text-muted-foreground" htmlFor="loop-items">
              {ll.items()}
            </Label>
            <VariableTextarea
              id="loop-items"
              rows={3}
              className="font-mono text-xs"
              variables={variables}
              nodeLabels={nodeLabels}
              value={config.items ?? ''}
              onValueChange={(value) => update({ items: value })}
            />
            <p className="text-xs text-muted-foreground">{ll.items_hint()}</p>
          </div>
        )}

        {config.source === 'until' && (
          <div className="space-y-1.5">
            <Label className="text-xs font-medium text-muted-foreground" htmlFor="loop-until">
              {ll.until()}
            </Label>
            <VariableTextarea
              id="loop-until"
              rows={4}
              className="font-mono text-xs"
              variables={variables}
              nodeLabels={nodeLabels}
              value={config.until ?? ''}
              onValueChange={(value) => update({ until: value })}
            />
            <p className="text-xs text-muted-foreground">{ll.until_hint()}</p>
            <div className="space-y-1 rounded-md bg-muted/50 p-2.5 text-xs text-muted-foreground">
              <p className="font-medium text-foreground">{ll.until_help_title()}</p>
              <ul className="ml-3 list-disc space-y-0.5">
                <li>
                  <code className="font-mono text-foreground">loop.index</code> —{' '}
                  {ll.until_help_index()}
                </li>
                <li>
                  <code className="font-mono text-foreground">loop.count</code> —{' '}
                  {ll.until_help_count()}
                </li>
                <li>
                  <code className="font-mono text-foreground">loop.item</code> —{' '}
                  {ll.until_help_item()}
                </li>
                <li>
                  <code className="font-mono text-foreground">last</code> — {ll.until_help_last()}
                </li>
              </ul>
              <p>{ll.until_help_syntax()}</p>
              <p className="font-mono text-[11px]">{ll.until_help_example()}</p>
            </div>
          </div>
        )}

        {config.source !== 'until' && (
          <div className="space-y-1.5">
            <Label className="text-xs font-medium text-muted-foreground" htmlFor="loop-until-opt">
              {ll.until()}
            </Label>
            <VariableTextarea
              id="loop-until-opt"
              rows={3}
              className="font-mono text-xs"
              variables={variables}
              nodeLabels={nodeLabels}
              value={config.until ?? ''}
              onValueChange={(value) => update({ until: value })}
            />
            <p className="text-xs text-muted-foreground">{ll.until_hint()}</p>
            <p className="text-xs text-muted-foreground">{ll.until_help_syntax()}</p>
          </div>
        )}
      </div>

      {/* 2. 节奏 */}
      <div className="space-y-3 rounded-lg border border-border bg-card p-4">
        <Label className="font-semibold">{ll.rhythm()}</Label>
        <div className="grid grid-cols-2 gap-3">
          <div className="space-y-1.5">
            <Label className="text-xs font-medium text-muted-foreground" htmlFor="loop-initial">
              {ll.initial_delay_ms()}
            </Label>
            <Input
              id="loop-initial"
              type="number"
              min={0}
              value={config.initial_delay_ms ?? 0}
              onChange={(e) => {
                const value = Number.parseInt(e.target.value, 10);
                update({ initial_delay_ms: Number.isNaN(value) ? 0 : value });
              }}
            />
          </div>
          <div className="space-y-1.5">
            <Label className="text-xs font-medium text-muted-foreground" htmlFor="loop-interval">
              {ll.interval_ms()}
            </Label>
            <Input
              id="loop-interval"
              type="number"
              min={0}
              value={config.interval_ms ?? 0}
              onChange={(e) => {
                const value = Number.parseInt(e.target.value, 10);
                update({ interval_ms: Number.isNaN(value) ? 0 : value });
              }}
            />
          </div>
          <div className="space-y-1.5">
            <Label className="text-xs font-medium text-muted-foreground" htmlFor="loop-backoff">
              {ll.interval_backoff()}
            </Label>
            <Input
              id="loop-backoff"
              type="number"
              min={1}
              step={0.1}
              value={config.interval_backoff ?? 1}
              onChange={(e) => {
                const value = Number.parseFloat(e.target.value);
                update({ interval_backoff: Number.isNaN(value) ? 1 : value });
              }}
            />
          </div>
          <div className="space-y-1.5">
            <Label className="text-xs font-medium text-muted-foreground" htmlFor="loop-interval-max">
              {ll.interval_max_ms()}
            </Label>
            <Input
              id="loop-interval-max"
              type="number"
              min={0}
              value={config.interval_max_ms ?? 10000}
              onChange={(e) => {
                const value = Number.parseInt(e.target.value, 10);
                update({ interval_max_ms: Number.isNaN(value) ? 0 : value });
              }}
            />
          </div>
        </div>
      </div>

      {/* 3. 护栏 */}
      <div className="space-y-3 rounded-lg border border-border bg-card p-4">
        <Label className="font-semibold">{ll.guards()}</Label>
        <div className="grid grid-cols-2 gap-3">
          <div className="space-y-1.5">
            <Label className="text-xs font-medium text-muted-foreground" htmlFor="loop-max-iterations">
              {ll.max_iterations()}
            </Label>
            <Input
              id="loop-max-iterations"
              type="number"
              min={1}
              value={config.max_iterations}
              onChange={(e) => {
                const value = Number.parseInt(e.target.value, 10);
                update({ max_iterations: Number.isNaN(value) ? 1 : value });
              }}
            />
          </div>
          <div className="space-y-1.5">
            <Label className="text-xs font-medium text-muted-foreground" htmlFor="loop-timeout">
              {ll.timeout_ms()}
            </Label>
            <Input
              id="loop-timeout"
              type="number"
              min={0}
              value={config.timeout_ms ?? 600000}
              onChange={(e) => {
                const value = Number.parseInt(e.target.value, 10);
                update({ timeout_ms: Number.isNaN(value) ? 0 : value });
              }}
            />
          </div>
          <div className="space-y-1.5">
            <Label className="text-xs font-medium text-muted-foreground" htmlFor="loop-concurrency">
              {ll.concurrency()}
            </Label>
            <Input
              id="loop-concurrency"
              type="number"
              min={1}
              value={config.concurrency}
              onChange={(e) => {
                const value = Number.parseInt(e.target.value, 10);
                update({ concurrency: Number.isNaN(value) ? 1 : value });
              }}
            />
            {config.concurrency !== 1 && (
              <p className="text-xs text-rose-600">{ll.concurrency_unsupported()}</p>
            )}
          </div>
        </div>
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <Label className="font-medium" htmlFor="loop-break-on-error">
              {ll.break_on_error()}
            </Label>
          </div>
          <Switch
            id="loop-break-on-error"
            checked={config.break_on_error}
            onCheckedChange={(checked) => update({ break_on_error: checked })}
          />
        </div>
      </div>

      {/* 4. 输出变量 */}
      <div className="space-y-2 rounded-lg border border-border bg-card p-4">
        <div className="flex items-center justify-between">
          <Label className="font-semibold">{ll.outputs()}</Label>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="h-7 gap-1 text-xs text-primary"
            onClick={addMapping}
          >
            <Plus className="w-3.5 h-3.5" />
            {ll.add_output()}
          </Button>
        </div>
        <p className="text-xs text-muted-foreground">{ll.outputs_desc()}</p>
        {(config.output_mappings ?? []).length === 0 ? (
          <p className="text-xs text-muted-foreground">{ll.no_outputs()}</p>
        ) : (
          <div className="space-y-2">
            {(config.output_mappings ?? []).map((mapping, index) => (
              <div key={index} className="flex items-start gap-2">
                <Input
                  className="font-mono text-xs"
                  list="loop-path-presets"
                  placeholder={ll.source_path_placeholder()}
                  value={mapping.source}
                  onChange={(e) => updateMapping(index, { source: e.target.value })}
                />
                <Input
                  className="font-mono text-xs"
                  placeholder={ll.target_var_placeholder()}
                  value={mapping.target}
                  onChange={(e) => updateMapping(index, { target: e.target.value })}
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="h-9 w-9 shrink-0 text-muted-foreground hover:text-rose-600"
                  onClick={() => removeMapping(index)}
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </Button>
              </div>
            ))}
            <datalist id="loop-path-presets">
              {PATH_PRESETS.map((preset) => (
                <option key={preset} value={preset} />
              ))}
            </datalist>
          </div>
        )}
      </div>

      {/* 5. 报告 */}
      <div className="space-y-3 rounded-lg border border-border bg-card p-4">
        <Label className="font-semibold">{ll.report()}</Label>
        <div className="space-y-1.5">
          <Label className="text-xs font-medium text-muted-foreground" htmlFor="loop-detail-iterations">
            {ll.detail_iterations()}
          </Label>
          <Input
            id="loop-detail-iterations"
            type="number"
            min={0}
            value={config.report?.detail_iterations ?? 20}
            onChange={(e) => {
              const value = Number.parseInt(e.target.value, 10);
              update({
                report: {
                  detail_iterations: Number.isNaN(value) ? 0 : value,
                  always_keep_failed: configRef.current.report?.always_keep_failed ?? true,
                },
              });
            }}
          />
        </div>
        <div className="flex items-center justify-between">
          <Label className="font-medium" htmlFor="loop-keep-failed">
            {ll.always_keep_failed()}
          </Label>
          <Switch
            id="loop-keep-failed"
            checked={config.report?.always_keep_failed ?? true}
            onCheckedChange={(checked) =>
              update({
                report: {
                  detail_iterations: configRef.current.report?.detail_iterations ?? 20,
                  always_keep_failed: checked,
                },
              })
            }
          />
        </div>
      </div>

      {/* 6. 循环体 */}
      <div className="space-y-2 rounded-lg border border-border bg-card p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <RotateCw className="w-3.5 h-3.5 text-sky-500" />
            {ll.body_node_count({ count: bodyNodeCount })}
          </div>
          <Button
            type="button"
            size="sm"
            className="h-7 gap-1 text-xs"
            disabled={!selectedNodeId}
            onClick={() => {
              if (selectedNodeId) enterLoop(selectedNodeId);
            }}
          >
            {ll.enter_body()}
          </Button>
        </div>
        <p className="text-xs text-muted-foreground">{ll.body_entry_hint()}</p>
        <p className="text-xs text-muted-foreground">{ll.body_roots_hint()}</p>
      </div>
    </div>
  );
}
