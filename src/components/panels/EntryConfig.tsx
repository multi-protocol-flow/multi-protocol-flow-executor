/**
 * Entry 节点配置组件
 * 支持 CSV 数据集关联
 *
 * 注意：初始变量已统一到流程级 initial_variables + 环境管理，
 * Entry 节点不再独立管理变量。
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { getLL } from '@/i18n/i18n-lazy';
import { useCallback, useEffect, useRef, useState } from 'react';
import { entryConfigSchema, type EntryConfigFormData } from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Switch } from '@/components/ui/switch';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { PlayCircle, Plus, Trash2, FileText, AlertTriangle, Table, Bell } from 'lucide-react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { datasetService } from '@/services/dataset';
import type { EntryConfig as EntryConfigType } from '@/types';
import type { FlowHook } from '@/types';
import type { CsvInfo } from '@/types/dataset';

interface EntryConfigProps {
  /** 当前节点数据 */
  data: EntryConfigType;
  /** 保存回调 */
  onSave: (data: EntryConfigType) => void;
}

/**
 * 将 EntryConfigType 转换为表单数据
 */
function toFormData(data: EntryConfigType): EntryConfigFormData {
  return {
    type: 'entry',
    dataset_path: data.dataset_path,
    dataset_encoding: data.dataset_encoding,
  };
}

/**
 * 将表单数据转换为 EntryConfigType
 */
function fromFormData(formData: EntryConfigFormData): EntryConfigType {
  return {
    type: 'entry',
    dataset_path: formData.dataset_path || undefined,
    dataset_encoding: formData.dataset_encoding || undefined,
  };
}

export function EntryConfig({ data, onSave }: EntryConfigProps) {
  const {
    handleSubmit,
    getValues,
    reset,
    formState: { isDirty },
  } = useForm<EntryConfigFormData>({
    resolver: zodResolver(entryConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 当外部数据变化时，重置表单
  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

  // 追踪表单实际修改状态（useRef 用于卸载时读取）
  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

  // 追踪待执行的 setTimeout，组件卸载时清除，防止保存到错误的节点
  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  useEffect(() => {
    return () => {
      pendingTimersRef.current.forEach((id) => clearTimeout(id));
      pendingTimersRef.current.clear();
    };
  }, []);

  /**
   * 安全的延迟执行包装器，注册定时器以便卸载时清理
   */
  const safeSetTimeout = useCallback((fn: () => void, ms: number) => {
    const id = setTimeout(() => {
      pendingTimersRef.current.delete(id);
      fn();
    }, ms);
    pendingTimersRef.current.add(id);
  }, []);

  // 组件卸载时同步保存（仅当用户实际修改过表单时）
  useEffect(() => {
    return () => {
      if (!isActuallyDirtyRef.current) return;
      try {
        const formData = getValues();
        const config = fromFormData(formData);
        onSave(config);
      } catch {
        // 静默处理卸载时的保存错误
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /**
   * 保存表单数据
   */
  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存 Entry 配置失败:', error);
      }
    })();
  }, [handleSubmit, onSave]);

  /**
   * 处理失焦事件
   */
  const handleBlur = useCallback(() => {
    // 延迟保存，确保表单校验完成且 isDirty ref 已更新
    safeSetTimeout(() => {
      if (!isActuallyDirtyRef.current) return;
      saveData();
    }, 100);
  }, [safeSetTimeout, saveData]);

  // 监听 Ctrl+S 刷新事件，将表单数据同步到 store
  useEffect(() => {
    const handleFlush = () => { saveData(); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
  }, [saveData]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-emerald-200/50 bg-gradient-to-br from-emerald-50/80 to-teal-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-emerald-900 flex items-center gap-2">
            <PlayCircle className="h-4 w-4" />
            {getLL().panels.entryNode.title()}
          </h4>
          <p className="text-sm leading-relaxed text-emerald-700/90 whitespace-normal break-words">
            {getLL().panels.entryNode.description()}
          </p>
        </div>
      </div>

      {/* 流程钩子配置 */}
      <HookConfigSection
        hooks={data.hooks}
        onHooksChange={(hooks) => {
          const formData = getValues();
          const updatedConfig = fromFormData(formData);
          updatedConfig.hooks = hooks.length > 0 ? hooks : undefined;
          onSave(updatedConfig);
        }}
      />

      {/* 关联数据集 */}
      <DatasetAssociationSection
        datasetPath={data.dataset_path}
        datasetEncoding={data.dataset_encoding}
        onConfigChange={(path, encoding) => {
          const formData = getValues();
          const updatedConfig = fromFormData({
            ...formData,
            dataset_path: path,
            dataset_encoding: encoding,
          });
          onSave(updatedConfig);
        }}
      />
    </form>
  );
}

// ============================================================================
// 数据集关联子组件
// ============================================================================

/** CSV 预览行数 */
const PREVIEW_ROW_COUNT = 5;

interface DatasetAssociationProps {
  /** 当前关联的 CSV 路径 */
  datasetPath?: string;
  /** 当前编码设置 */
  datasetEncoding?: string;
  /** 配置变更回调 */
  onConfigChange: (path: string | undefined, encoding: string | undefined) => void;
}

/**
 * 数据集关联区块
 * 独立于 react-hook-form 管理，通过 onConfigChange 直接同步到父组件
 */
function DatasetAssociationSection({ datasetPath, datasetEncoding, onConfigChange }: DatasetAssociationProps) {
  const [csvInfo, setCsvInfo] = useState<CsvInfo | null>(null);
  const [fileExists, setFileExists] = useState(true);
  const [isLoading, setIsLoading] = useState(false);
  const [previewRows, setPreviewRows] = useState<string[][]>([]);

  /** 编码选项 */
  const encodingOptions = [
    { value: 'auto', label: getLL().panels.entryNode.encodingAuto() },
    { value: 'utf-8', label: 'UTF-8' },
    { value: 'gbk', label: 'GBK' },
    { value: 'gb2312', label: 'GB2312' },
    { value: 'big5', label: 'Big5' },
  ] as const;

  /** 当前选中的编码值（UI 状态） */
  const encoding = datasetEncoding || 'auto';

  /** 从路径中提取文件名 */
  const datasetName = datasetPath
    ? datasetPath.replace(/\\/g, '/').split('/').pop() || datasetPath
    : '';

  /**
   * 加载 CSV 元信息并生成预览
   */
  const loadCsvInfo = useCallback(async (path: string, enc?: string) => {
    setIsLoading(true);
    try {
      const info = await datasetService.validateCsv(path, enc === 'auto' ? undefined : enc);
      setCsvInfo(info);
      setFileExists(true);

      // 从 validateCsv 返回的 column_names 构建模拟预览
      // 真正的行数据需要后端提供 preview 接口，这里先显示列信息
      setPreviewRows([]);
    } catch {
      // 文件不存在或解析失败
      setCsvInfo(null);
      setFileExists(false);
      setPreviewRows([]);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // 初始加载：已有 dataset_path 时验证文件
  useEffect(() => {
    if (datasetPath) {
      loadCsvInfo(datasetPath, datasetEncoding);
    }
    // 仅在挂载和路径变化时触发
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [datasetPath]);

  /**
   * 打开文件选择对话框
   */
  const handleSelectFile = useCallback(async () => {
    try {
      const selected = await openDialog({
        filters: [{ name: 'CSV', extensions: ['csv'] }],
        multiple: false,
      });
      if (selected) {
        const enc = encoding === 'auto' ? undefined : encoding;
        await loadCsvInfo(selected, enc);
        onConfigChange(selected, enc);
      }
    } catch (error) {
      console.error('选择 CSV 文件失败:', error);
    }
  }, [encoding, loadCsvInfo, onConfigChange]);

  /**
   * 切换编码
   */
  const handleEncodingChange = useCallback(async (newEncoding: string) => {
    const enc = newEncoding === 'auto' ? undefined : newEncoding;
    if (datasetPath) {
      await loadCsvInfo(datasetPath, enc);
    }
    onConfigChange(datasetPath, enc);
  }, [datasetPath, loadCsvInfo, onConfigChange]);

  /**
   * 移除关联
   */
  const handleRemoveAssociation = useCallback(() => {
    setCsvInfo(null);
    setFileExists(true);
    setPreviewRows([]);
    onConfigChange(undefined, undefined);
  }, [onConfigChange]);

  return (
    <div className="space-y-3 pt-4 border-t">
      <div className="flex items-center gap-2">
        <Table className="h-4 w-4 text-muted-foreground" />
        <Label className="text-sm font-medium text-foreground">{getLL().panels.entryNode.dataset()}</Label>
      </div>

      {!datasetPath ? (
        <Button
          type="button"
          variant="outline"
          onClick={handleSelectFile}
          className="w-full h-9 gap-2 text-sm"
        >
          <FileText className="h-4 w-4" />
          {getLL().panels.entryNode.selectCsv()}
        </Button>
      ) : (
        <div className="space-y-2">
          {/* 文件路径展示 + 删除按钮 */}
          <div className="flex items-center gap-2">
            <FileText className="h-4 w-4 shrink-0 text-muted-foreground" />
            <span className="text-sm truncate flex-1" title={datasetPath}>
              {datasetName}
            </span>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              onClick={handleRemoveAssociation}
              className="h-7 w-7 shrink-0 text-muted-foreground hover:text-rose-500"
              title={getLL().panels.entryNode.removeAssociation()}
            >
              <Trash2 className="h-3.5 w-3.5" />
            </Button>
          </div>

          {/* 编码选择器 */}
          <Select value={encoding} onValueChange={handleEncodingChange}>
            <SelectTrigger className="h-8 text-xs">
              <SelectValue placeholder={getLL().panels.entryNode.encodingAuto()} />
            </SelectTrigger>
            <SelectContent>
              {encodingOptions.map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>
                  {opt.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          {/* 加载中状态 */}
          {isLoading && (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-3 text-center">
              <p className="text-xs text-muted-foreground">{getLL().panels.entryNode.readingFile()}</p>
            </div>
          )}

          {/* 文件不存在警告 */}
          {!isLoading && !fileExists && (
            <div className="rounded-md border border-amber-200/50 bg-amber-50/50 p-3">
              <div className="flex items-center gap-2">
                <AlertTriangle className="h-4 w-4 shrink-0 text-amber-600" />
                <span className="text-sm text-amber-700 flex-1">{getLL().panels.entryNode.fileMissing()}</span>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={handleSelectFile}
                  className="h-7 gap-1.5 text-xs shrink-0"
                >
                  {getLL().panels.entryNode.reselect()}
                </Button>
              </div>
            </div>
          )}

          {/* 文件信息 + 预览 */}
          {!isLoading && fileExists && csvInfo && (
            <div className="rounded-md border bg-muted/20 p-3 space-y-2">
              <p className="text-xs text-muted-foreground">
                {csvInfo.row_count} 行 × {csvInfo.column_names.length} 列
                {csvInfo.detected_encoding && (
                  <span className="ml-2 text-muted-foreground/60">
                    {getLL().panels.entryNode.detectedEncoding()} {csvInfo.detected_encoding}
                  </span>
                )}
              </p>

              {/* 列名预览 */}
              <div className="overflow-x-auto">
                <div className="flex gap-1 flex-wrap">
                  {csvInfo.column_names.slice(0, PREVIEW_ROW_COUNT).map((col, i) => (
                    <span
                      key={i}
                      className="inline-flex items-center rounded bg-background border px-1.5 py-0.5 font-mono text-xs text-foreground"
                    >
                      {col}
                    </span>
                  ))}
                  {csvInfo.column_names.length > PREVIEW_ROW_COUNT && (
                    <span className="inline-flex items-center text-xs text-muted-foreground">
                      +{getLL().panels.entryNode.moreColumns({ count: csvInfo.column_names.length - PREVIEW_ROW_COUNT })}
                    </span>
                  )}
                </div>
              </div>

              {/* 预览表格 */}
              {previewRows.length > 0 && (
                <div className="overflow-x-auto rounded border">
                  <table className="w-full text-xs">
                    <thead>
                      <tr className="border-b bg-muted/50">
                        {csvInfo.column_names.map((col) => (
                          <th key={col} className="px-2 py-1 text-left font-medium text-muted-foreground whitespace-nowrap">
                            {col}
                          </th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {previewRows.map((row, ri) => (
                        <tr key={ri} className={ri > 0 ? 'border-t' : ''}>
                          {row.map((cell, ci) => (
                            <td key={ci} className="px-2 py-1 font-mono whitespace-nowrap max-w-[200px] truncate">
                              {cell}
                            </td>
                          ))}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </div>
          )}
        </div>
      )}
     </div>
   );
 }

// ============================================================================
// 钩子配置子组件
// ============================================================================

interface HookConfigProps {
  hooks?: FlowHook[];
  onHooksChange: (hooks: FlowHook[]) => void;
}

/**
 * 钩子配置区块
 * 独立于 react-hook-form 管理，通过 onHooksChange 直接同步到父组件
 */
function HookConfigSection({ hooks, onHooksChange }: HookConfigProps) {
  const [localHooks, setLocalHooks] = useState<FlowHook[]>(hooks || []);

  // 同步外部 hooks 变更（节点切换、undo/redo 时从 props 同步）
  useEffect(() => {
    setLocalHooks(hooks || []);
  }, [hooks]);

  /** 触发条件选项 */
  const triggerOptions = [
    { value: 'on_success', label: getLL().panels.entryNode.triggerOnSuccess() },
    { value: 'on_failure', label: getLL().panels.entryNode.triggerOnFailure() },
    { value: 'on_cancel', label: getLL().panels.entryNode.triggerOnCancel() },
    { value: 'on_complete', label: getLL().panels.entryNode.triggerOnComplete() },
  ] as const;

  /** 动作类型选项 */
  const actionTypeOptions = [
    { value: 'notify', label: getLL().panels.entryNode.actionNotify() },
    { value: 'webhook', label: getLL().panels.entryNode.actionWebhook() },
    { value: 'script', label: getLL().panels.entryNode.actionScript() },
  ] as const;

  /** 添加新钩子 */
  const handleAddHook = useCallback(() => {
    const newHook: FlowHook = {
      uuid: `hook-${Date.now()}`,
      name: '',
      enabled: true,
      trigger: 'on_complete',
      action: { type: 'notify', title_template: '', message_template: '' },
    };
    const updated = [...localHooks, newHook];
    setLocalHooks(updated);
    onHooksChange(updated);
  }, [localHooks, onHooksChange]);

  /** 删除钩子 */
  const handleRemoveHook = useCallback((index: number) => {
    const updated = localHooks.filter((_, i) => i !== index);
    setLocalHooks(updated);
    onHooksChange(updated);
  }, [localHooks, onHooksChange]);

  /** 更新钩子字段 */
  const handleUpdateHook = useCallback((index: number, updates: Partial<FlowHook>) => {
    const updated = localHooks.map((h, i) => (i === index ? { ...h, ...updates } : h));
    setLocalHooks(updated);
    onHooksChange(updated);
  }, [localHooks, onHooksChange]);

  /** 更新动作字段 */
  const handleUpdateAction = useCallback((index: number, actionUpdates: Record<string, unknown>) => {
    const updated = localHooks.map((h, i) =>
      i === index ? { ...h, action: { ...h.action, ...actionUpdates } as FlowHook['action'] } : h
    );
    setLocalHooks(updated);
    onHooksChange(updated);
  }, [localHooks, onHooksChange]);

  /** 切换动作类型（需要重置动作字段） */
  const handleActionTypeChange = useCallback((index: number, newType: string) => {
    let newAction: FlowHook['action'];
    switch (newType) {
      case 'webhook':
        newAction = { type: 'webhook', url: '', method: 'POST', headers: {} };
        break;
      case 'script':
        newAction = { type: 'script', code: '' };
        break;
      case 'notify':
      default:
        newAction = { type: 'notify', title_template: '', message_template: '' };
        break;
    }
    const updated = localHooks.map((h, i) => (i === index ? { ...h, action: newAction } : h));
    setLocalHooks(updated);
    onHooksChange(updated);
  }, [localHooks, onHooksChange]);

  return (
    <div className="space-y-3 pt-4 border-t">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Bell className="h-4 w-4 text-muted-foreground" />
          <Label className="text-sm font-medium text-foreground">{getLL().panels.entryNode.flowHooks()}</Label>
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={handleAddHook}
          className="h-7 gap-1.5 text-xs"
          disabled={localHooks.length >= 10}
        >
          <Plus className="h-3.5 w-3.5" />
          {getLL().panels.entryNode.addHook()}
        </Button>
      </div>

      {localHooks.length === 0 ? (
        <div className="rounded-md bg-muted/30 p-3">
          <p className="text-xs text-muted-foreground">
            {getLL().panels.entryNode.hooksHint()}
          </p>
        </div>
      ) : (
        <div className="space-y-3">
          {localHooks.map((hook, index) => (
            <div key={hook.uuid} className="rounded-md border bg-muted/10 p-3 space-y-2">
              {/* 第一行：启用开关 + 名称 + 删除 */}
              <div className="flex items-center gap-2">
                <Switch
                  checked={hook.enabled}
                  onCheckedChange={(checked) => handleUpdateHook(index, { enabled: checked })}
                />
                <Input
                  value={hook.name}
                  onChange={(e) => handleUpdateHook(index, { name: e.target.value })}
                  placeholder={getLL().panels.entryNode.hookNamePlaceholder()}
                  className="h-7 text-xs flex-1"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  onClick={() => handleRemoveHook(index)}
                  className="h-7 w-7 shrink-0 text-muted-foreground hover:text-rose-500"
                  title={getLL().panels.entryNode.deleteHook()}
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>

              {/* 第二行：触发条件 + 动作类型 */}
              <div className="flex gap-2">
                <Select
                  value={hook.trigger}
                  onValueChange={(v) => handleUpdateHook(index, { trigger: v as FlowHook['trigger'] })}
                >
                  <SelectTrigger className="h-7 text-xs flex-1">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {triggerOptions.map((opt) => (
                      <SelectItem key={opt.value} value={opt.value}>
                        {opt.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>

                <Select
                  value={hook.action.type}
                  onValueChange={(v) => handleActionTypeChange(index, v)}
                >
                  <SelectTrigger className="h-7 text-xs flex-1">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {actionTypeOptions.map((opt) => (
                      <SelectItem key={opt.value} value={opt.value}>
                        {opt.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              {/* 动作配置（根据类型显示不同字段） */}
              {hook.action.type === 'webhook' && (
                <div className="space-y-2">
                  <div className="flex gap-2">
                    <Select
                      value={hook.action.method}
                      onValueChange={(v) => handleUpdateAction(index, { method: v })}
                    >
                      <SelectTrigger className="h-7 text-xs w-24 shrink-0">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {['GET', 'POST', 'PUT', 'PATCH', 'DELETE'].map((m) => (
                          <SelectItem key={m} value={m}>{m}</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <Input
                      value={hook.action.url}
                      onChange={(e) => handleUpdateAction(index, { url: e.target.value })}
                      placeholder={getLL().panels.entryNode.hookUrlPlaceholder()}
                      className="h-7 text-xs flex-1 font-mono"
                    />
                  </div>
                  <Textarea
                    value={hook.action.body_template || ''}
                    onChange={(e) => handleUpdateAction(index, { body_template: e.target.value || undefined })}
                    placeholder={getLL().panels.entryNode.hookBodyPlaceholder('')}
                    className="text-xs font-mono min-h-[48px] resize-y"
                    rows={2}
                  />
                </div>
              )}

              {hook.action.type === 'script' && (
                <Textarea
                  value={hook.action.code}
                  onChange={(e) => handleUpdateAction(index, { code: e.target.value })}
                  placeholder={getLL().panels.entryNode.hookCodePlaceholder()}
                  className="text-xs font-mono min-h-[48px] resize-y"
                  rows={3}
                />
              )}

              {hook.action.type === 'notify' && (
                <div className="space-y-2">
                  <Input
                    value={hook.action.title_template}
                    onChange={(e) => handleUpdateAction(index, { title_template: e.target.value })}
                    placeholder={getLL().panels.entryNode.hookTitlePlaceholder('')}
                    className="h-7 text-xs"
                  />
                  <Input
                    value={hook.action.message_template}
                    onChange={(e) => handleUpdateAction(index, { message_template: e.target.value })}
                    placeholder={getLL().panels.entryNode.hookMessagePlaceholder('')}
                    className="h-7 text-xs"
                  />
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
