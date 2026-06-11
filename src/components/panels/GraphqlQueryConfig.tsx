/**
 * GraphQL Query 节点配置组件
 * 支持连接 ID、查询编辑器、变量编辑器、操作名称、文件上传、超时
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  graphqlQueryConfigSchema,
  type GraphqlQueryConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { Plus, Trash2, Info, Send } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { GraphqlQueryConfig as GraphqlQueryConfigType, GraphqlConnectConfig as GraphqlConnectConfigType } from '@/types';
import { useFlowStore } from '@/stores';
import { isGraphqlConnectConfig } from '@/types/node';
import { buildOperationFieldTrees } from '@/lib/graphqlSchema';
import type { FieldNode } from '@/lib/graphqlSchema';
import { GraphqlFieldTree } from '@/components/ui/GraphqlFieldTree';

interface GraphqlQueryConfigProps {
  data: GraphqlQueryConfigType;
  onSave: (data: GraphqlQueryConfigType) => void;
}

interface FileEntry {
  field_name: string;
  file_path: string;
}

function variablesToString(variables?: Record<string, unknown>): string {
  if (!variables) return '';
  try {
    return JSON.stringify(variables, null, 2);
  } catch {
    return '';
  }
}

function variablesFromString(json: string | undefined): Record<string, unknown> | undefined {
  if (!json || !json.trim()) return undefined;
  try {
    return JSON.parse(json) as Record<string, unknown>;
  } catch {
    return undefined;
  }
}

function toFormData(data: GraphqlQueryConfigType): GraphqlQueryConfigFormData {
  return {
    type: 'graphql_query',
    connection_id: data.connection_id || '',
    query: data.query || '',
    variables: variablesToString(data.variables),
    operation_name: data.operation_name || '',
    timeout_ms: data.timeout_ms ?? 30000,
  };
}

function fromFormData(formData: GraphqlQueryConfigFormData, files?: Array<{ field_name: string; file_path: string }>): GraphqlQueryConfigType {
  return {
    type: 'graphql_query',
    connection_id: formData.connection_id,
    query: formData.query,
    variables: variablesFromString(formData.variables),
    operation_name: formData.operation_name || undefined,
    files: files && files.length > 0 ? files : undefined,
    timeout_ms: formData.timeout_ms || undefined,
  };
}

export function GraphqlQueryConfig({ data, onSave }: GraphqlQueryConfigProps) {
  const {
    register,
    handleSubmit,
    reset,
    getValues,
    setValue,
    watch,
    formState: { errors, isDirty },
  } = useForm<GraphqlQueryConfigFormData>({
    resolver: zodResolver(graphqlQueryConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  useEffect(() => {
    return () => {
      pendingTimersRef.current.forEach((id) => clearTimeout(id));
      pendingTimersRef.current.clear();
    };
  }, []);

  useEffect(() => {
    return () => {
      if (!isActuallyDirtyRef.current) return;
      try {
        const formData = getValues();
        const config = fromFormData(formData, fileEntries);
        onSave(config);
      } catch {
        // 静默处理卸载时的保存错误
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const safeSetTimeout = useCallback((fn: () => void, ms: number) => {
    const id = setTimeout(() => {
      pendingTimersRef.current.delete(id);
      fn();
    }, ms);
    pendingTimersRef.current.add(id);
  }, []);

  useEffect(() => {
    reset(toFormData(data));
    setFileEntries(data.files || []);
  }, [data, reset]);

  const flow = useFlowStore((state) => state.flow);

  const availableConnections = useMemo(() => {
    if (!flow) return [];
    const connections: Array<{ uuid: string; name: string }> = [];
    flow.nodes.forEach(node => {
      if (isGraphqlConnectConfig(node.data)) {
        connections.push({ uuid: node.uuid, name: node.name });
      }
    });
    return connections;
  }, [flow]);

  // 获取选中的 connect 节点中的 discovered_schema，构建字段树
  const selectedSchema = useMemo<{
    queryFields: FieldNode[];
    mutationFields: FieldNode[];
    hasSchema: boolean;
  }>(() => {
    if (!flow || !watch('connection_id')) {
      return { queryFields: [], mutationFields: [], hasSchema: false };
    }
    const connectNode = flow.nodes.find(
      n => n.uuid === watch('connection_id') && isGraphqlConnectConfig(n.data)
    );
    if (!connectNode) return { queryFields: [], mutationFields: [], hasSchema: false };
    const config = connectNode.data as GraphqlConnectConfigType;
    if (!config.discovered_schema) return { queryFields: [], mutationFields: [], hasSchema: false };

    const query = buildOperationFieldTrees(config.discovered_schema, 'query');
    const mutation = buildOperationFieldTrees(config.discovered_schema, 'mutation');
    return {
      queryFields: query.rootFields,
      mutationFields: mutation.rootFields,
      hasSchema: true,
    };
  }, [flow, watch('connection_id')]);

  const [fileEntries, setFileEntries] = useState<FileEntry[]>(data.files || []);

  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData, fileEntries);
        onSave(config);
      } catch (error) {
        console.error('保存 GraphQL Query 配置失败:', error);
      }
    })();
  }, [handleSubmit, onSave, fileEntries]);

  const handleBlur = useCallback(() => {
    safeSetTimeout(() => {
      if (!isActuallyDirtyRef.current) return;
      saveData();
    }, 100);
  }, [safeSetTimeout, saveData]);

  useEffect(() => {
    const handleFlush = () => { saveData(); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
  }, [saveData]);

  const [variablesError, setVariablesError] = useState<string | null>(null);

  const handleVariablesBlur = useCallback(() => {
    const val = getValues('variables');
    if (val && val.trim()) {
      try {
        JSON.parse(val);
        setVariablesError(null);
      } catch {
        setVariablesError('JSON 格式无效');
        return;
      }
    } else {
      setVariablesError(null);
    }
    safeSetTimeout(() => {
      if (!isActuallyDirtyRef.current) return;
      saveData();
    }, 100);
  }, [getValues, safeSetTimeout, saveData]);

  const handleAddFile = useCallback(() => {
    setFileEntries(prev => [...prev, { field_name: '', file_path: '' }]);
  }, []);

  const handleRemoveFile = useCallback((index: number) => {
    const newFiles = fileEntries.filter((_, i) => i !== index);
    setFileEntries(newFiles);
    safeSetTimeout(saveData, 100);
  }, [fileEntries, safeSetTimeout, saveData]);

  const handleFileChange = useCallback((index: number, field: 'field_name' | 'file_path', val: string) => {
    setFileEntries(prev => prev.map((entry, i) =>
      i === index ? { ...entry, [field]: val } : entry
    ));
  }, []);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-indigo-200/50 bg-gradient-to-br from-indigo-50/80 to-violet-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-indigo-900 flex items-center gap-2">
            <Send className="h-4 w-4" />
            {getLL().panels.graphqlQuery.title()}
          </h4>
          <p className="text-sm leading-relaxed text-indigo-700/90 whitespace-normal break-words">
            {getLL().panels.graphqlQuery.description()}
          </p>
        </div>
      </div>

      {/* 连接选择 */}
      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">
          {getLL().panels.connection()} <span className="text-indigo-500">*</span>
        </Label>
        {availableConnections.length > 0 ? (
          <Select
            value={watch('connection_id') || ''}
            onValueChange={(value) => {
              setValue('connection_id', value, { shouldDirty: true });
              saveData();
            }}
          >
            <SelectTrigger id="connection_id" className="font-mono">
              <SelectValue placeholder={getLL().panels.selectConnection()} />
            </SelectTrigger>
            <SelectContent>
              {availableConnections.map((conn) => (
                <SelectItem key={conn.uuid} value={conn.uuid}>
                  <span className="font-mono">{conn.name}</span>
                  <span className="ml-2 text-xs text-muted-foreground">
                    ({conn.uuid.substring(0, 8)}...)
                  </span>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        ) : (
          <Input
            id="connection_id"
            {...register('connection_id')}
            placeholder={getLL().panels.enterConnectionId()}
            className="font-mono text-sm"
          />
        )}
        {errors.connection_id && (
          <p className="text-xs text-indigo-500">{errors.connection_id.message}</p>
        )}
        {availableConnections.length === 0 && (
          <p className="text-xs text-muted-foreground">
            {getLL().panels.graphqlQuery.noConnectNode()}
          </p>
        )}
      </div>

      {/* Schema 字段浏览器 */}
      {selectedSchema.hasSchema && (selectedSchema.queryFields.length > 0 || selectedSchema.mutationFields.length > 0) && (
        <div className="space-y-3">
          <Label className="text-sm font-medium text-foreground">
            Schema 字段浏览器
          </Label>
          <p className="text-xs text-muted-foreground">
            勾选需要的字段，点击"生成查询"自动填充查询编辑器
          </p>

          {/* Query 字段 */}
          {selectedSchema.queryFields.length > 0 && (
            <GraphqlFieldTree
              title="query"
              displayTitle={getLL().panels.graphql_field.query_type()}
              fields={selectedSchema.queryFields}
              onGenerate={(q) => {
                if (!q) return;
                // 使用 reset 替代 setValue，确保表单状态完整刷新到 Textarea 组件
                const vals = getValues();
                const updated = { ...vals, query: q };
                reset(updated as GraphqlQueryConfigFormData);
                onSave(fromFormData(updated, fileEntries));
              }}
            />
          )}

          {/* Mutation 字段 */}
          {selectedSchema.mutationFields.length > 0 && (
            <GraphqlFieldTree
              title="mutation"
              displayTitle={getLL().panels.graphql_field.mutation_type()}
              fields={selectedSchema.mutationFields}
              onGenerate={(q) => {
                if (!q) return;
                // 使用 reset 替代 setValue，确保表单状态完整刷新到 Textarea 组件
                const vals = getValues();
                const updated = { ...vals, query: q };
                reset(updated as GraphqlQueryConfigFormData);
                onSave(fromFormData(updated, fileEntries));
              }}
            />
          )}
        </div>
      )}

      {/* 查询编辑器 */}
      <div className="space-y-2">
        <Label htmlFor="query" className="text-sm font-medium text-foreground">
          {getLL().panels.graphqlQuery.query()} <span className="text-indigo-500">*</span>
        </Label>
        <Textarea
          id="query"
          {...register('query')}
          placeholder={`query GetUsers($limit: Int) {\n  users(limit: $limit) {\n    id\n    name\n    email\n  }\n}`}
          className="min-h-[160px] font-mono text-sm resize-y"
          style={{ tabSize: 2 }}
        />
        {errors.query && (
          <p className="text-xs text-indigo-500">{errors.query.message}</p>
        )}
      </div>

      {/* 变量编辑器 */}
      <div className="space-y-2">
        <Label htmlFor="variables" className="text-sm font-medium text-foreground">
          {getLL().panels.graphqlQuery.variables()}
        </Label>
        <Textarea
          id="variables"
          {...register('variables')}
          onBlur={handleVariablesBlur}
          placeholder={'{\n  "limit": 10\n}'}
          className="min-h-[80px] font-mono text-sm resize-y"
          style={{ tabSize: 2 }}
        />
        {variablesError && (
          <p className="text-xs text-indigo-500">{variablesError}</p>
        )}
      </div>

      {/* 操作名称 */}
      <div className="space-y-2">
        <Label htmlFor="operation_name" className="text-sm font-medium text-foreground">
          操作名称
        </Label>
        <Input
          id="operation_name"
          {...register('operation_name')}
          placeholder={getLL().panels.methodNameExample()}
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          当查询包含多个操作时，指定要执行的操作名称
        </p>
      </div>

      {/* 文件上传 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-medium text-foreground">文件上传（可选）</Label>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddFile}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />
            {getLL().panels.add()}
          </Button>
        </div>
        <div className="space-y-2">
          {fileEntries.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">暂无文件上传</p>
            </div>
          ) : (
            fileEntries.map((entry, index) => (
              <div key={index} className="flex items-center gap-2 p-2 rounded-md border bg-muted/30">
                <Input
                  value={entry.field_name}
                  onChange={(e) => handleFileChange(index, 'field_name', e.target.value)}
                  onBlur={() => safeSetTimeout(saveData, 100)}
                  placeholder={getLL().panels.fieldNameExample()}
                  className="h-8 w-32 font-mono text-xs"
                />
                <Input
                  value={entry.file_path}
                  onChange={(e) => handleFileChange(index, 'file_path', e.target.value)}
                  onBlur={() => safeSetTimeout(saveData, 100)}
                  placeholder={getLL().panels.filePath()}
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  onClick={() => handleRemoveFile(index)}
                  className="h-8 w-8 shrink-0 text-muted-foreground hover:text-indigo-500"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            ))
          )}
        </div>
      </div>

      {/* 超时 */}
      <div className="space-y-2">
        <Label htmlFor="timeout_ms" className="text-sm font-medium text-foreground">
          超时（毫秒）
        </Label>
        <Input
          id="timeout_ms"
          type="number"
          {...register('timeout_ms', { valueAsNumber: true })}
          placeholder="30000"
          className="font-mono text-sm"
        />
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-indigo-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">GraphQL 查询说明：</p>
          <p>支持 Query、Mutation 操作</p>
          <p>变量需以 JSON 格式输入</p>
          <p>文件上传使用 GraphQL multipart 规范</p>
        </div>
      </div>
    </form>
  );
}


