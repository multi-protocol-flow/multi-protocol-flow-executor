/**
 * SubFlow 节点配置组件
 *
 * 目标子流程采用「二选一」引用方式：
 * - 已保存流程（target_flow_id）：文件夹层级树形下拉选择
 * - 流程文件（target_flow_path）：路径输入 + 原生文件选择器
 * 两者互斥，切换方式时自动清空另一种引用。
 */
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  Plus,
  Trash2,
  ExternalLink,
  FolderOpen,
  FileText,
  Workflow,
  AlertTriangle,
  X,
} from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { FlowTreeSelect } from '@/components/ui/FlowTreeSelect';
import type { SubFlowConfig as SubFlowConfigType, VariableMapping } from '@/types/node';
import { useFlowListStore, useFlowStore } from '@/stores';
import { useCurrentLocale } from '@/hooks/useLocale';
import { flowStorage } from '@/services/storage';
import { getLL } from '@/i18n/i18n-lazy';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';

interface SubFlowConfigProps {
  /** 当前节点数据 */
  data: SubFlowConfigType;
  /** 保存回调 */
  onSave: (data: SubFlowConfigType) => void;
}

/** 目标流程引用方式 */
type TargetMode = 'flow' | 'path';

/**
 * 面板内部配置：映射列表在此层必定存在，
 * 避免每处使用点重复做 `?? []` 兜底（TS 类型中二者为可选，对应 JSON 的缺省序列化）
 */
type PanelConfig = Omit<SubFlowConfigType, 'variable_mappings' | 'output_mappings'> & {
  variable_mappings: VariableMapping[];
  output_mappings: VariableMapping[];
};

/** 补齐缺省字段，保证面板状态与后端结构一致 */
function normalize(data: SubFlowConfigType): PanelConfig {
  return {
    type: 'sub_flow',
    target_flow_id: data.target_flow_id || undefined,
    target_flow_path: data.target_flow_path || undefined,
    inherit_all_variables: data.inherit_all_variables ?? true,
    variable_mappings: data.variable_mappings ?? [],
    export_all_variables: data.export_all_variables ?? true,
    output_mappings: data.output_mappings ?? [],
  };
}

/** 解析目标流程引用方式（路径优先，与后端解析顺序一致） */
function resolveMode(config: SubFlowConfigType): TargetMode {
  return config.target_flow_path ? 'path' : 'flow';
}

/**
 * 动态导入 Tauri 文件对话框（浏览器模式返回 null）
 *
 * 必须动态导入：静态导入会让打包器在浏览器（playground）构建期解析原生插件模块，
 * 而该模块只在 Tauri 运行时存在；动态导入 + try/catch 在浏览器模式下降级为「取消」。
 * 与 EntryConfig / HttpConfig / DatasetExecutionPanel 的处理保持一致。
 */
async function pickFlowFile(): Promise<string | null> {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'Flow', extensions: ['json', 'mpf'] }],
    });
    return typeof selected === 'string' ? selected : null;
  } catch {
    return null;
  }
}

export function SubFlowConfig({ data, onSave }: SubFlowConfigProps) {
  useCurrentLocale();
  const ll = getLL().panels.subflow;
  const flows = useFlowListStore((state) => state.flows);
  const setStoreFlows = useFlowListStore((state) => state.setFlows);
  const setFlow = useFlowStore((state) => state.setFlow);
  const currentFlowUuid = useFlowStore((state) => state.flow?.uuid);

  const [config, setConfig] = useState<PanelConfig>(() => normalize(data));
  const [mode, setMode] = useState<TargetMode>(() => resolveMode(data));
  const [pathDraft, setPathDraft] = useState<string>(() => data.target_flow_path ?? '');
  const configRef = useRef(config);
  configRef.current = config;

  // 外部数据变化时同步（与本地面板状态一致则跳过，避免保存回环打断输入）
  useEffect(() => {
    const incoming = normalize(data);
    if (JSON.stringify(incoming) === JSON.stringify(configRef.current)) return;
    setConfig(incoming);
    setMode(resolveMode(data));
    setPathDraft(incoming.target_flow_path ?? '');
  }, [data]);

  // 流程列表为空时补一次加载（面板可能先于侧边栏渲染）
  useEffect(() => {
    if (flows.length > 0) return;
    let cancelled = false;
    flowStorage.listFlows().then((metas) => {
      if (cancelled || metas.length === 0) return;
      setStoreFlows(
        metas.map((meta) => ({
          uuid: meta.uuid,
          name: meta.name,
          updated_at: meta.updatedAt,
          folder_path: meta.folderPath ?? '',
          file_path: meta.filePath,
        }))
      );
    });
    return () => {
      cancelled = true;
    };
  }, [flows.length, setStoreFlows]);

  const currentFlow = flows.find(
    (flow) =>
      (config.target_flow_id && flow.uuid === config.target_flow_id) ||
      (config.target_flow_path && (flow.file_path === config.target_flow_path || flow.uuid === config.target_flow_path))
  );
  const isTargetMissing =
    Boolean(config.target_flow_id || config.target_flow_path) &&
    flows.length > 0 &&
    !currentFlow;

  /** 选择目标流程（文件夹层级树） */
  const flowItems = useMemo(
    () =>
      flows
        .filter((flow) => flow.uuid !== currentFlowUuid)
        .map((flow) => ({ uuid: flow.uuid, name: flow.name, folderPath: flow.folder_path ?? '' })),
    [flows, currentFlowUuid]
  );

  /**
   * 归一化并保存：
   * - 未设置的目标引用统一写空串。NodeConfigPanel → flow store 对 `data` 是浅合并，
   *   被删除的键会被旧值重新填回，因此"清空"只能用空串覆盖表达；
   *   后端 `resolve_flow` 与校验器均把空串视为未设置。
   * - 调用方每次同时给出两种引用，保证二选一约束不依赖调用顺序。
   */
  const update = useCallback(
    (patch: Partial<PanelConfig>) => {
      const next = { ...configRef.current, ...patch };
      next.target_flow_id = next.target_flow_id || '';
      next.target_flow_path = next.target_flow_path || '';
      setConfig(next);
      onSave(next);
    },
    [onSave]
  );

  /** 切换引用方式：清空另一种引用，保证二选一 */
  const switchMode = (next: TargetMode) => {
    if (next === mode) return;
    setMode(next);
    if (next === 'flow') {
      setPathDraft('');
      update({ target_flow_path: '', target_flow_id: config.target_flow_id });
    } else {
      update({ target_flow_id: '', target_flow_path: config.target_flow_path });
    }
  };

  /** 提交路径草稿 */
  const commitPath = (value: string) => {
    const trimmed = value.trim();
    if (trimmed === (config.target_flow_path ?? '')) return;
    update({ target_flow_path: trimmed, target_flow_id: '' });
  };

  /** 打开原生文件选择器 */
  const handleBrowse = async () => {
    const selected = await pickFlowFile();
    if (selected === null) return;
    setPathDraft(selected);
    commitPath(selected);
  };

  /** 在新标签页打开目标流程（仅已保存流程可直接打开） */
  const handleOpenFlow = async () => {
    const target = config.target_flow_path || config.target_flow_id;
    if (!target) {
      toast.error(ll.select_target_flow());
      return;
    }
    try {
      const flow = await flowStorage.loadFlow(target);
      if (flow) {
        setFlow(flow);
        toast.success(flow.name);
      } else {
        toast.error(ll.missing_target());
      }
    } catch (err) {
      toast.error(String(err));
    }
  };

  const addMapping = (key: 'variable_mappings' | 'output_mappings') => {
    update({ [key]: [...config[key], { source: '', target: '' }] });
  };

  const updateMapping = (
    key: 'variable_mappings' | 'output_mappings',
    index: number,
    field: 'source' | 'target',
    value: string
  ) => {
    const next = config[key].map((mapping, i) =>
      i === index ? { ...mapping, [field]: value } : mapping
    );
    update({ [key]: next });
  };

  const removeMapping = (key: 'variable_mappings' | 'output_mappings', index: number) => {
    update({ [key]: config[key].filter((_, i) => i !== index) });
  };

  const renderMappingRows = (
    key: 'variable_mappings' | 'output_mappings',
    sourcePlaceholder: string,
    targetPlaceholder: string,
    emptyText: string
  ) => {
    const mappings: VariableMapping[] = config[key];
    if (mappings.length === 0) {
      return (
        <p className="py-2 text-center text-xs italic text-muted-foreground bg-muted/20 rounded">
          {emptyText}
        </p>
      );
    }
    return (
      <div className="space-y-2">
        {mappings.map((mapping, index) => (
          <div key={index} className="flex items-center gap-2">
            <Input
              className="h-8 flex-1 font-mono text-xs"
              placeholder={sourcePlaceholder}
              value={mapping.source}
              onChange={(e) => updateMapping(key, index, 'source', e.target.value)}
            />
            <span className="text-xs text-muted-foreground">→</span>
            <Input
              className="h-8 flex-1 font-mono text-xs"
              placeholder={targetPlaceholder}
              value={mapping.target}
              onChange={(e) => updateMapping(key, index, 'target', e.target.value)}
            />
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="h-8 w-8 shrink-0 text-muted-foreground hover:text-destructive"
              onClick={() => removeMapping(key, index)}
            >
              <Trash2 className="w-3.5 h-3.5" />
            </Button>
          </div>
        ))}
      </div>
    );
  };

  return (
    <div className="space-y-6 text-sm">
      {/* 1. 目标流程（二选一：已保存流程 / 流程文件） */}
      <div className="space-y-3 rounded-lg border border-border bg-card p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Workflow className="w-4 h-4 text-violet-500" />
            <Label className="font-semibold">{ll.target_flow()}</Label>
          </div>
          {(config.target_flow_id || config.target_flow_path) && (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-7 gap-1 border-violet-200 text-xs text-violet-600 hover:bg-violet-50 dark:border-violet-800 dark:hover:bg-violet-950/30"
              onClick={handleOpenFlow}
            >
              <ExternalLink className="w-3.5 h-3.5" />
              {ll.open_target_flow()}
            </Button>
          )}
        </div>

        {/* 引用方式切换 */}
        <div className="flex gap-1 rounded-lg bg-muted p-0.5">
          <button
            type="button"
            onClick={() => switchMode('flow')}
            className={cn(
              'flex-1 rounded-md px-3 py-1.5 text-xs font-medium transition-all',
              mode === 'flow'
                ? 'bg-background text-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground'
            )}
          >
            {ll.mode_flow_select()}
          </button>
          <button
            type="button"
            onClick={() => switchMode('path')}
            className={cn(
              'flex-1 rounded-md px-3 py-1.5 text-xs font-medium transition-all',
              mode === 'path'
                ? 'bg-background text-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground'
            )}
          >
            {ll.mode_file_path()}
          </button>
        </div>

        {mode === 'flow' ? (
          <div className="space-y-1.5">
            <Label className="text-xs text-muted-foreground">{ll.select_target_flow()}</Label>
            <FlowTreeSelect
              value={config.target_flow_id ?? ''}
              onValueChange={(selectedId) => {
                update({
                  target_flow_id: selectedId,
                  target_flow_path: '',
                });
              }}
              items={flowItems}
              placeholder={ll.select_target_flow()}
              searchPlaceholder={ll.search_flow_placeholder()}
              emptyText={ll.no_flows()}
              noResultsText={ll.no_matching_flows()}
              clearLabel={ll.clear_selection()}
            />
            {isTargetMissing && (
              <p className="flex items-center gap-1.5 pt-1 text-xs text-amber-600 dark:text-amber-500">
                <AlertTriangle className="w-3.5 h-3.5 shrink-0" />
                {ll.missing_target()}
              </p>
            )}
          </div>
        ) : (
          <div className="space-y-1.5">
            <Label className="text-xs text-muted-foreground">{ll.target_flow_path()}</Label>
            <div className="flex items-center gap-2">
              <div className="relative flex-1">
                <FileText className="absolute left-2.5 top-1/2 w-3.5 h-3.5 -translate-y-1/2 text-muted-foreground" />
                <Input
                  value={pathDraft}
                  placeholder={ll.target_flow_path_placeholder()}
                  onChange={(e) => setPathDraft(e.target.value)}
                  onBlur={() => commitPath(pathDraft)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') commitPath(pathDraft);
                  }}
                  className="h-9 pl-8 pr-8 font-mono text-xs"
                />
                {pathDraft && (
                  <button
                    type="button"
                    onClick={() => {
                      setPathDraft('');
                      commitPath('');
                    }}
                    className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                    title={ll.clear_selection()}
                  >
                    <X className="w-3.5 h-3.5" />
                  </button>
                )}
              </div>
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="h-9 shrink-0 gap-1.5 text-xs"
                onClick={handleBrowse}
              >
                <FolderOpen className="w-3.5 h-3.5" />
                {ll.browse_file()}
              </Button>
            </div>
            <p className="text-[11px] text-muted-foreground">{ll.target_flow_path_hint()}</p>
          </div>
        )}
      </div>

      {/* 2. 变量继承与输入映射 */}
      <div className="space-y-4 rounded-lg border border-border bg-card p-4">
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <Label className="font-semibold cursor-pointer" htmlFor="inherit-all-vars">
              {ll.inherit_all_variables()}
            </Label>
            <p className="text-xs text-muted-foreground">{ll.inherit_all_variables_desc()}</p>
          </div>
          <Switch
            id="inherit-all-vars"
            checked={config.inherit_all_variables}
            onCheckedChange={(checked) => update({ inherit_all_variables: checked })}
          />
        </div>

        <div className="space-y-2 border-t border-border/50 pt-2">
          <div className="flex items-center justify-between">
            <Label className="text-xs font-medium text-muted-foreground">
              {ll.input_mappings()}
            </Label>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => addMapping('variable_mappings')}
              className="h-7 gap-1 text-xs text-primary hover:text-primary"
            >
              <Plus className="w-3.5 h-3.5" />
              {ll.add_input_mapping()}
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">{ll.input_mappings_desc()}</p>
          {renderMappingRows(
            'variable_mappings',
            ll.source_expr_placeholder(),
            ll.target_var_placeholder(),
            ll.no_input_mappings()
          )}
        </div>
      </div>

      {/* 3. 变量导出与输出映射 */}
      <div className="space-y-4 rounded-lg border border-border bg-card p-4">
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <Label className="font-semibold cursor-pointer" htmlFor="export-all-vars">
              {ll.export_all_variables()}
            </Label>
            <p className="text-xs text-muted-foreground">{ll.export_all_variables_desc()}</p>
          </div>
          <Switch
            id="export-all-vars"
            checked={config.export_all_variables}
            onCheckedChange={(checked) => update({ export_all_variables: checked })}
          />
        </div>

        {!config.export_all_variables && (
          <div className="space-y-2 border-t border-border/50 pt-2">
            <div className="flex items-center justify-between">
              <Label className="text-xs font-medium text-muted-foreground">
                {ll.output_mappings()}
              </Label>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => addMapping('output_mappings')}
                className="h-7 gap-1 text-xs text-primary hover:text-primary"
              >
                <Plus className="w-3.5 h-3.5" />
                {ll.add_output_mapping()}
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">{ll.output_mappings_desc()}</p>
            {renderMappingRows(
              'output_mappings',
              ll.child_var_placeholder(),
              ll.parent_var_placeholder(),
              ll.no_output_mappings()
            )}
          </div>
        )}
      </div>
    </div>
  );
}
