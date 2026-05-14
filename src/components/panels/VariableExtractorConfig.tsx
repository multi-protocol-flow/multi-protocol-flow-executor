/**
 * 变量提取节点配置组件
 * 支持数据来源选择 + 输出变量映射编辑
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef } from 'react';
import { variableExtractorConfigSchema, type VariableExtractorConfigFormData } from './schemas';
import { Label } from '@/components/ui/label';
import { OutputMappingsEditor } from '@/components/panels/shared/OutputMappingsEditor';
import type { VariableExtractorConfig as VariableExtractorConfigType } from '@/types';
import { useFlowStore } from '@/stores';

interface VariableExtractorConfigProps {
  /** 当前节点数据 */
  data: VariableExtractorConfigType;
  /** 保存回调 */
  onSave: (data: VariableExtractorConfigType) => void;
}

/** 不应出现在数据来源下拉中的节点类型（无输出数据或无意义） */
const EXCLUDED_SOURCE_TYPES: Set<string> = new Set([
  'entry',
  'variable_extractor',
]);

/** JSONPath 预设列表 */
const jsonPathPresets = [
  { value: '$.status', label: '状态码' },
  { value: '$.body', label: '响应体' },
  { value: '$.body.data', label: '响应数据' },
  { value: '$.headers', label: '响应头' },
  { value: '$.rows', label: '数据行' },
  { value: '$.rows[0]', label: '首行数据' },
  { value: '$.response', label: '响应' },
  { value: '$.data', label: '数据' },
  { value: '$.message', label: '消息' },
  { value: '$.connection_id', label: '连接ID' },
];

/**
 * 将 VariableExtractorConfigType 转换为表单数据
 */
function toFormData(data: VariableExtractorConfigType): VariableExtractorConfigFormData {
  return {
    type: 'variable_extractor',
    output_mappings: data.output_mappings || [],
    source_node_id: data.source_node_id || undefined,
  };
}

/**
 * 将表单数据转换为 VariableExtractorConfigType
 */
function fromFormData(formData: VariableExtractorConfigFormData): VariableExtractorConfigType {
  return {
    type: 'variable_extractor',
    output_mappings: formData.output_mappings && formData.output_mappings.length > 0
      ? formData.output_mappings
      : undefined,
    source_node_id: formData.source_node_id || undefined,
  };
}

export function VariableExtractorConfig({ data, onSave }: VariableExtractorConfigProps) {
  const {
    register,
    handleSubmit,
    control,
    watch,
    getValues,
    reset,
    setValue,
    formState: { errors, isDirty },
  } = useForm<VariableExtractorConfigFormData>({
    resolver: zodResolver(variableExtractorConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 追踪表单实际修改状态
  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

  // 获取当前流程节点列表和当前选中节点
  const flow = useFlowStore((state) => state.flow);
  const selectedNodeId = useFlowStore((state) => state.selectedNodeId);

  // 可选的来源节点：排除自身和无输出节点类型
  const sourceNodes = flow?.nodes.filter(
    (node) => node.uuid !== selectedNodeId && !EXCLUDED_SOURCE_TYPES.has(node.type)
  ) ?? [];

  // 当外部数据变化时，重置表单
  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

  // 组件卸载时同步保存
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
        console.error('保存变量提取配置失败:', error);
      }
    })();
  }, [handleSubmit, onSave]);

  // 监听 Ctrl+S 刷新事件
  useEffect(() => {
    const handleFlush = () => { saveData(); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
  }, [saveData]);

  /**
   * 处理数据来源选择变更
   */
  const handleSourceNodeChange = useCallback((e: React.ChangeEvent<HTMLSelectElement>) => {
    const value = e.target.value;
    setValue('source_node_id', value || undefined, { shouldDirty: true });
    // 延迟保存，确保表单值已更新
    setTimeout(saveData, 100);
  }, [setValue, saveData]);

  return (
    <form onBlur={() => { setTimeout(() => { if (isActuallyDirtyRef.current) saveData(); }, 100); }} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 数据来源选择 */}
      <div className="space-y-2">
        <Label htmlFor="source_node_id" className="text-sm font-medium text-foreground">
          数据来源
        </Label>
        <select
          id="source_node_id"
          value={watch('source_node_id') || ''}
          onChange={handleSourceNodeChange}
          className="flex h-9 w-full rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
        >
          <option value="">自动（最近上游节点输出）</option>
          {sourceNodes.map((node) => (
            <option key={node.uuid} value={node.uuid}>
              {node.name} ({node.type})
            </option>
          ))}
        </select>
        <p className="text-xs text-muted-foreground">
          选择变量提取的数据来源节点，默认使用最近的上游节点输出。指定后将从该节点的 output_data 中提取数据。
        </p>
      </div>

      {/* 输出变量映射编辑器 */}
      <OutputMappingsEditor
        control={control}
        errors={errors}
        watch={watch}
        register={register}
        saveData={saveData}
        jsonPathPresets={jsonPathPresets}
        defaultSource="$.status"
      />
    </form>
  );
}
