/**
 * 变量提取节点配置组件
 * 支持数据来源选择 + 输出变量映射编辑
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useRef, useMemo } from 'react';
import { variableExtractorConfigSchema, type VariableExtractorConfigFormData } from './schemas';
import { Label } from '@/components/ui/label';
import { SearchableNodeSelect } from '@/components/ui/SearchableNodeSelect';
import { OutputMappingsEditor } from '@/components/panels/shared/OutputMappingsEditor';
import type { VariableExtractorConfig as VariableExtractorConfigType } from '@/types';
import { useFlowStore } from '@/stores';
import { getLL } from '@/i18n/i18n-lazy';

interface VariableExtractorConfigProps {
  /** 当前节点数据 */
  data: VariableExtractorConfigType;
  /** 保存回调 */
  onSave: (data: VariableExtractorConfigType) => void;
}

/** 不应出现在数据来源下拉中的节点类型（无输出数据或无意义） */
const EXCLUDED_SOURCE_TYPES: Set<string> = new Set([
  'entry',
  'variable:extractor',
]);

/**
 * 将 VariableExtractorConfigType 转换为表单数据
 */
function toFormData(data: VariableExtractorConfigType): VariableExtractorConfigFormData {
  return {
    type: 'variable:extractor',
    output_mappings: data.output_mappings || [],
    source_node_id: data.source_node_id || undefined,
  };
}

/**
 * 将表单数据转换为 VariableExtractorConfigType
 */
function fromFormData(formData: VariableExtractorConfigFormData): VariableExtractorConfigType {
  return {
    type: 'variable:extractor',
    output_mappings: formData.output_mappings && formData.output_mappings.length > 0
      ? formData.output_mappings
      : undefined,
    source_node_id: formData.source_node_id || undefined,
  };
}

export function VariableExtractorConfig({ data, onSave }: VariableExtractorConfigProps) {
  const ll = getLL().panels.variableExtractor;
  const presetsLL = getLL().panels.variableExtractor.presets;
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

  // 获取当前流程节点列表和当前选中节点
  const flow = useFlowStore((state) => state.flow);
  const selectedNodeId = useFlowStore((state) => state.selectedNodeId);

  // 可选的来源节点：排除自身和无输出节点类型
  const sourceNodes = flow?.nodes.filter(
    (node) => node.uuid !== selectedNodeId && !EXCLUDED_SOURCE_TYPES.has(node.type)
  ) ?? [];

  // 数据来源下拉选项：自动 + 所有上游节点
  const sourceSelectItems = useMemo(() => {
    const _ll = getLL().panels.variableExtractor;
    return [
      { uuid: "", name: _ll.sourceAuto(), type: "" },
      ...sourceNodes.map((node) => ({
        uuid: node.uuid,
        name: node.name,
        type: node.type,
      })),
    ];
  }, [sourceNodes]);

  /** JSONPath 预设列表 */
  const jsonPathPresets = useMemo(() => [
    { value: '$.status', label: presetsLL.statusCode() },
    { value: '$.body', label: presetsLL.body() },
    { value: '$.body.data', label: presetsLL.responseData() },
    { value: '$.headers', label: presetsLL.headers() },
    { value: '$.rows', label: presetsLL.rows() },
    { value: '$.rows[0]', label: presetsLL.firstRow() },
    { value: '$.response', label: presetsLL.response() },
    { value: '$.data', label: presetsLL.data() },
    { value: '$.message', label: presetsLL.message() },
    { value: '$.connection_id', label: presetsLL.connectionId() },
  ], [presetsLL]);

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
  const handleSourceNodeChange = useCallback((value: string) => {
    setValue('source_node_id', value || undefined, { shouldDirty: true });
    // 延迟保存，确保表单值已更新
    safeSetTimeout(saveData, 100);
  }, [setValue, saveData, safeSetTimeout]);

  return (
    <form onBlur={() => { safeSetTimeout(() => { if (isActuallyDirtyRef.current) saveData(); }, 100); }} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 数据来源选择 */}
      <div className="space-y-2">
        <Label htmlFor="source_node_id" className="text-sm font-medium text-foreground">
          {ll.dataSource()}
        </Label>
        <SearchableNodeSelect
          value={watch('source_node_id') || ''}
          onValueChange={handleSourceNodeChange}
          items={sourceSelectItems}
          placeholder={ll.sourceAuto()}
          noResultsText={ll.sourceHelp()}
          searchPlaceholder={getLL().panels.nodeSearchPlaceholder()}
        />
        <p className="text-xs text-muted-foreground">
          {ll.sourceHelp()}
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
