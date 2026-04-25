/**
 * Entry 节点配置组件
 * 支持初始变量配置 + CSV 数据集关联
 */
import { useForm, useFieldArray } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef, useState } from 'react';
import { entryConfigSchema, type EntryConfigFormData } from './schemas';
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
import { PlayCircle, Plus, Trash2, Variable, FileText, AlertTriangle, Table } from 'lucide-react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { datasetService } from '@/services/dataset';
import type { EntryConfig as EntryConfigType } from '@/types';
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
    initial_variables: data.initial_variables || [],
    dataset_path: data.dataset_path,
    dataset_encoding: data.dataset_encoding,
  };
}

/**
 * 将表单数据转换为 EntryConfigType
 */
function fromFormData(formData: EntryConfigFormData): EntryConfigType {
  const result: EntryConfigType = {
    type: 'entry',
    initial_variables: formData.initial_variables && formData.initial_variables.length > 0
      ? formData.initial_variables
      : undefined,
    dataset_path: formData.dataset_path || undefined,
    dataset_encoding: formData.dataset_encoding || undefined,
  };
  return result;
}

/**
 * 将 JSON 值转换为字符串（用于显示）
 */
function valueToString(value: unknown): string {
  if (value == null) return '';
  if (typeof value === 'string') return value;
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}

/**
 * 将字符串解析为 JSON 值
 */
function stringToValue(str: string): unknown {
  if (!str.trim()) return '';
  try {
    return JSON.parse(str);
  } catch {
    return str;
  }
}

export function EntryConfig({ data, onSave }: EntryConfigProps) {
  const {
    register,
    handleSubmit,
    control,
    watch,
    setValue,
    getValues,
    reset,
    formState: { errors, isDirty },
  } = useForm<EntryConfigFormData>({
    resolver: zodResolver(entryConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  const { fields, append, remove } = useFieldArray({
    control,
    name: 'initial_variables' as never,
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

  /**
   * 添加变量
   */
  const handleAddVariable = useCallback(() => {
    append({ name: '', value: '', description: '' });
  }, [append]);

  /**
   * 删除变量
   */
  const handleRemoveVariable = useCallback((index: number) => {
    remove(index);
    setTimeout(saveData, 100);
  }, [remove, saveData]);

  /**
   * 处理变量值变更（转换为 JSON）
   */
  const handleValueChange = useCallback((index: number, valueString: string) => {
    const value = stringToValue(valueString);
    setValue(`initial_variables.${index}.value`, value, { shouldDirty: true });
  }, [setValue]);

  // 监听所有变量名，用于唯一性检查
  const variables = watch('initial_variables') || [];

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-emerald-200/50 bg-gradient-to-br from-emerald-50/80 to-teal-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-emerald-900 flex items-center gap-2">
            <PlayCircle className="h-4 w-4" />
            入口节点
          </h4>
          <p className="text-sm leading-relaxed text-emerald-700/90 whitespace-normal break-words">
            入口节点是流程的起点。每个流程必须有且仅有一个入口节点，流程执行将从这里开始，沿着连接线依次执行各个节点。
          </p>
        </div>
      </div>

      {/* 初始变量配置 */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Variable className="h-4 w-4 text-muted-foreground" />
            <Label className="text-sm font-medium text-foreground">初始变量</Label>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddVariable}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />
            添加变量
          </Button>
        </div>

        {/* 变量列表 */}
        <div className="space-y-2">
          {fields.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">暂无初始变量</p>
              <p className="mt-1 text-xs text-muted-foreground/60">
                点击「添加变量」按钮创建初始变量
              </p>
            </div>
          ) : (
            fields.map((field, index) => {
              const nameError = errors.initial_variables?.[index]?.name;
              const valueString = valueToString(variables[index]?.value);
              
              // 检查变量名是否重复
              const currentName = variables[index]?.name || '';
              const duplicateNames = variables
                .map((v, i) => (v.name && i !== index && v.name === currentName ? i : -1))
                .filter(i => i !== -1);
              const hasDuplicate = duplicateNames.length > 0 && currentName.trim() !== '';
              
              return (
                <div key={field.id} className="space-y-2 p-3 rounded-md border bg-muted/30">
                  {/* 第一行：变量名 + 删除按钮 */}
                  <div className="flex items-center gap-2">
                    <div className="flex-1 min-w-0">
                      <Input
                        {...register(`initial_variables.${index}.name`)}
                        placeholder="变量名（如：api_base）"
                        className={`h-8 font-mono text-xs ${nameError || hasDuplicate ? 'border-rose-500 focus-visible:ring-rose-500' : ''}`}
                      />
                    </div>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => handleRemoveVariable(index)}
                      className="h-8 w-8 shrink-0 text-muted-foreground hover:text-violet-500"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                  
                  {/* 错误提示 */}
                  {nameError && (
                    <p className="text-xs text-rose-500">{nameError.message}</p>
                  )}
                  {hasDuplicate && !nameError && (
                    <p className="text-xs text-rose-500">变量名已存在</p>
                  )}
                  
                  {/* 第二行：默认值 */}
                  <div>
                    <Input
                      value={valueString}
                      onChange={(e) => handleValueChange(index, e.target.value)}
                      onBlur={handleBlur}
                      placeholder="默认值（支持 JSON：字符串、数字、对象、数组）"
                      className="h-8 font-mono text-xs"
                    />
                  </div>
                  
                  {/* 第三行：说明 */}
                  <div>
                    <Input
                      {...register(`initial_variables.${index}.description`)}
                      placeholder="说明（可选）"
                      className="h-8 text-xs"
                    />
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* 提示信息 */}
        {fields.length > 0 && (
          <div className="rounded-md bg-muted/30 p-2">
            <p className="text-xs text-muted-foreground whitespace-normal break-words">
              变量名只能包含字母、数字和下划线，长度 1-64 个字符。默认值支持 JSON 格式（字符串需加引号）。
            </p>
          </div>
        )}
      </div>

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

/** 编码选项 */
const ENCODING_OPTIONS = [
  { value: 'auto', label: '自动检测' },
  { value: 'utf-8', label: 'UTF-8' },
  { value: 'gbk', label: 'GBK' },
  { value: 'gb2312', label: 'GB2312' },
  { value: 'big5', label: 'Big5' },
] as const;

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
        <Label className="text-sm font-medium text-foreground">关联数据集</Label>
      </div>

      {!datasetPath ? (
        <Button
          type="button"
          variant="outline"
          onClick={handleSelectFile}
          className="w-full h-9 gap-2 text-sm"
        >
          <FileText className="h-4 w-4" />
          选择 CSV 文件
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
              title="移除关联"
            >
              <Trash2 className="h-3.5 w-3.5" />
            </Button>
          </div>

          {/* 编码选择器 */}
          <Select value={encoding} onValueChange={handleEncodingChange}>
            <SelectTrigger className="h-8 text-xs">
              <SelectValue placeholder="自动检测" />
            </SelectTrigger>
            <SelectContent>
              {ENCODING_OPTIONS.map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>
                  {opt.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          {/* 加载中状态 */}
          {isLoading && (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-3 text-center">
              <p className="text-xs text-muted-foreground">正在读取文件...</p>
            </div>
          )}

          {/* 文件不存在警告 */}
          {!isLoading && !fileExists && (
            <div className="rounded-md border border-amber-200/50 bg-amber-50/50 p-3">
              <div className="flex items-center gap-2">
                <AlertTriangle className="h-4 w-4 shrink-0 text-amber-600" />
                <span className="text-sm text-amber-700 flex-1">关联文件已移除或重命名</span>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={handleSelectFile}
                  className="h-7 gap-1.5 text-xs shrink-0"
                >
                  重新选择
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
                    检测编码: {csvInfo.detected_encoding}
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
                      +{csvInfo.column_names.length - PREVIEW_ROW_COUNT} 列
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
