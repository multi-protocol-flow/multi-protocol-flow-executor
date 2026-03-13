/**
 * Script 节点配置组件
 * 支持脚本编辑、超时配置、内存限制、错误处理策略等
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect } from 'react';
import Editor, { type OnMount } from '@monaco-editor/react';
import { scriptConfigSchema, type ScriptConfigFormData } from './schemas';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Input } from '@/components/ui/input';
import { Info, Clock, Database, AlertTriangle, CheckCircle } from 'lucide-react';
import type { ScriptNodeConfig } from '@/types/node';

interface ScriptConfigProps {
  /** 当前节点数据 */
  data: ScriptNodeConfig;
  /** 保存回调 */
  onSave: (data: ScriptNodeConfig) => void;
}

/**
 * 将 ScriptNodeConfig 转换为表单数据
 */
function toFormData(data: ScriptNodeConfig): ScriptConfigFormData {
  return {
    type: 'script',
    script: data.script || '',
    timeout_ms: data.timeout_ms ?? 5000,
    memory_limit_mb: data.memory_limit_mb ?? 10,
    execution_phase: data.execution_phase ?? 'post_request',
    on_error: data.on_error ?? 'recoverable',
    on_assertion_fail: data.on_assertion_fail ?? 'route_false',
  };
}

/**
 * 将表单数据转换为 ScriptNodeConfig
 */
function fromFormData(formData: ScriptConfigFormData): ScriptNodeConfig {
  return {
    type: 'script',
    script: formData.script,
    timeout_ms: formData.timeout_ms,
    memory_limit_mb: formData.memory_limit_mb,
    execution_phase: formData.execution_phase,
    on_error: formData.on_error,
    on_assertion_fail: formData.on_assertion_fail,
  };
}

/** 执行时机选项 */
const EXECUTION_PHASE_OPTIONS = [
  { value: 'post_request', label: '请求后执行 (Post-Request)', description: '在 HTTP 请求完成后执行' },
  { value: 'pre_request', label: '请求前执行 (Pre-Request)', description: '在 HTTP 请求发送前执行' },
  { value: 'standalone', label: '独立执行 (Standalone)', description: '作为独立节点执行' },
] as const;

/** 错误处理策略选项 */
const ON_ERROR_OPTIONS = [
  { value: 'recoverable', label: '可恢复', description: '路由到失败端口，流程继续' },
  { value: 'fatal', label: '致命', description: '终止流程执行' },
] as const;

/** 断言失败策略选项 */
const ON_ASSERTION_FAIL_OPTIONS = [
  { value: 'route_false', label: '路由到失败端口', description: '断言失败时路由到 false 分支' },
  { value: 'ignore', label: '忽略', description: '忽略断言失败，继续执行' },
  { value: 'abort', label: '终止流程', description: '断言失败时终止整个流程' },
] as const;

export function ScriptConfig({ data, onSave }: ScriptConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    formState: { errors },
  } = useForm<ScriptConfigFormData>({
    resolver: zodResolver(scriptConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 监听表单值
  const watchedScript = watch('script');
  const watchedExecutionPhase = watch('execution_phase');
  const watchedOnError = watch('on_error');
  const watchedOnAssertionFail = watch('on_assertion_fail');

  // 当外部数据变化时，重置表单
  useEffect(() => {
    const formData = toFormData(data);
    setValue('script', formData.script);
    setValue('timeout_ms', formData.timeout_ms);
    setValue('memory_limit_mb', formData.memory_limit_mb);
    setValue('execution_phase', formData.execution_phase);
    setValue('on_error', formData.on_error);
    setValue('on_assertion_fail', formData.on_assertion_fail);
  }, [data, setValue]);

  /**
   * 保存表单数据
   */
  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存 Script 配置失败:', error);
      }
    })();
  }, [handleSubmit, onSave]);

  /**
   * 处理失焦事件
   */
  const handleBlur = useCallback(() => {
    // 延迟保存，确保表单校验完成
    setTimeout(saveData, 100);
  }, [saveData]);

  /**
   * 处理脚本编辑器变更
   */
  const handleScriptChange = useCallback((value: string | undefined) => {
    setValue('script', value || '');
    // 不立即保存，等用户离开编辑器时保存
  }, [setValue]);

  /**
   * 处理编辑器挂载，添加失焦保存功能
   */
  const handleEditorMount: OnMount = useCallback((editor) => {
    editor.onDidBlurEditorWidget(() => {
      saveData();
    });
  }, [saveData]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 脚本编辑器 */}
      <div className="space-y-2">
        <Label htmlFor="script" className="text-sm font-medium text-black flex items-center gap-2">
          <span>脚本代码</span>
          <span className="text-rose-500">*</span>
        </Label>
        <div className="rounded-md border overflow-hidden">
          <Editor
            height="200px"
            language="javascript"
            theme="vs-dark"
            value={watchedScript || ''}
            onChange={handleScriptChange}
            onMount={handleEditorMount}
            options={{
              minimap: { enabled: false },
              fontSize: 14,
              lineNumbers: 'on',
              scrollBeyondLastLine: false,
              wordWrap: 'on',
              automaticLayout: true,
              tabSize: 2,
              formatOnPaste: true,
              formatOnType: true,
            }}
          />
        </div>
        {errors.script && (
          <p className="text-xs text-rose-500">{errors.script.message}</p>
        )}
        {/* 脚本 API 提示 */}
        <div className="flex items-start gap-2 rounded-md bg-muted/30 p-2">
          <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
          <div className="text-xs text-muted-foreground space-y-1">
            <p className="font-medium text-black">可用 API：</p>
            <p><code className="rounded bg-muted px-1">variables.get(name)</code> 获取变量</p>
            <p><code className="rounded bg-muted px-1">variables.set(name, value)</code> 设置变量</p>
            <p><code className="rounded bg-muted px-1">assert(condition, name)</code> 断言</p>
            <p><code className="rounded bg-muted px-1">log(msg)</code> / <code className="rounded bg-muted px-1">console.log(msg)</code> 输出日志</p>
          </div>
        </div>
      </div>

      {/* 执行时机 */}
      <div className="space-y-2">
        <Label htmlFor="execution_phase" className="text-sm font-medium text-black flex items-center gap-2">
          <Clock className="h-4 w-4 text-muted-foreground" />
          执行时机
        </Label>
        <Select
          value={watchedExecutionPhase || 'post_request'}
          onValueChange={(value) => {
            setValue('execution_phase', value as ScriptConfigFormData['execution_phase']);
            saveData();
          }}
        >
          <SelectTrigger id="execution_phase" className="font-mono">
            <SelectValue placeholder="选择执行时机" />
          </SelectTrigger>
          <SelectContent>
            {EXECUTION_PHASE_OPTIONS.map((option) => (
              <SelectItem key={option.value} value={option.value}>
                <div className="flex flex-col">
                  <span>{option.label}</span>
                </div>
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          {EXECUTION_PHASE_OPTIONS.find(o => o.value === watchedExecutionPhase)?.description}
        </p>
      </div>

      {/* 超时时间和内存限制并排 */}
      <div className="grid grid-cols-2 gap-4">
        {/* 超时时间 */}
        <div className="space-y-2">
          <Label htmlFor="timeout_ms" className="text-sm font-medium text-black flex items-center gap-2">
            <Clock className="h-4 w-4 text-muted-foreground" />
            超时时间
          </Label>
          <Input
            id="timeout_ms"
            type="number"
            min={100}
            max={60000}
            {...register('timeout_ms', { valueAsNumber: true })}
            className="font-mono"
          />
          <p className="text-xs text-muted-foreground">毫秒 (100-60000)</p>
          {errors.timeout_ms && (
            <p className="text-xs text-rose-500">{errors.timeout_ms.message}</p>
          )}
        </div>

        {/* 内存限制 */}
        <div className="space-y-2">
          <Label htmlFor="memory_limit_mb" className="text-sm font-medium text-black flex items-center gap-2">
            <Database className="h-4 w-4 text-muted-foreground" />
            内存限制
          </Label>
          <Input
            id="memory_limit_mb"
            type="number"
            min={1}
            max={100}
            {...register('memory_limit_mb', { valueAsNumber: true })}
            className="font-mono"
          />
          <p className="text-xs text-muted-foreground">MB (1-100)</p>
          {errors.memory_limit_mb && (
            <p className="text-xs text-rose-500">{errors.memory_limit_mb.message}</p>
          )}
        </div>
      </div>

      {/* 错误处理策略 */}
      <div className="space-y-2">
        <Label htmlFor="on_error" className="text-sm font-medium text-black flex items-center gap-2">
          <AlertTriangle className="h-4 w-4 text-amber-500" />
          错误处理
        </Label>
        <Select
          value={watchedOnError || 'recoverable'}
          onValueChange={(value) => {
            setValue('on_error', value as ScriptConfigFormData['on_error']);
            saveData();
          }}
        >
          <SelectTrigger id="on_error" className="font-mono">
            <SelectValue placeholder="选择错误处理策略" />
          </SelectTrigger>
          <SelectContent>
            {ON_ERROR_OPTIONS.map((option) => (
              <SelectItem key={option.value} value={option.value}>
                <div className="flex flex-col">
                  <span>{option.label}</span>
                </div>
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          {ON_ERROR_OPTIONS.find(o => o.value === watchedOnError)?.description}
        </p>
      </div>

      {/* 断言失败策略 */}
      <div className="space-y-2">
        <Label htmlFor="on_assertion_fail" className="text-sm font-medium text-black flex items-center gap-2">
          <CheckCircle className="h-4 w-4 text-muted-foreground" />
          断言失败时
        </Label>
        <Select
          value={watchedOnAssertionFail || 'route_false'}
          onValueChange={(value) => {
            setValue('on_assertion_fail', value as ScriptConfigFormData['on_assertion_fail']);
            saveData();
          }}
        >
          <SelectTrigger id="on_assertion_fail" className="font-mono">
            <SelectValue placeholder="选择断言失败策略" />
          </SelectTrigger>
          <SelectContent>
            {ON_ASSERTION_FAIL_OPTIONS.map((option) => (
              <SelectItem key={option.value} value={option.value}>
                <div className="flex flex-col">
                  <span>{option.label}</span>
                </div>
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          {ON_ASSERTION_FAIL_OPTIONS.find(o => o.value === watchedOnAssertionFail)?.description}
        </p>
      </div>
    </form>
  );
}
