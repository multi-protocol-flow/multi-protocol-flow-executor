/**
 * Script 节点配置组件
 * 支持脚本编辑、超时配置、内存限制、错误处理策略等
 * 使用 CodeMirror 6 编辑器，内置 fn.* API 自动补全
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import CodeMirror from '@uiw/react-codemirror';
import { javascript } from '@codemirror/lang-javascript';
import { oneDark } from '@codemirror/theme-one-dark';
import { autocompletion, type CompletionContext } from '@codemirror/autocomplete';
import { scriptConfigSchema, type ScriptConfigFormData } from './schemas';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Input } from '@/components/ui/input';
import { Clock, Database, BookOpen } from 'lucide-react';
import type { ScriptNodeConfig } from '@/types/node';
import { useUIStore } from '@/stores/ui';
import { getLL } from '@/i18n/i18n-lazy';
import { DYNAMIC_VARIABLES } from '@/types/dynamicVariable';

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
  };
}

/** 执行时机选项（使用函数以便支持 i18n） */
function getExecutionPhaseOptions() {
  const ll = getLL().panels.script;
  return [
    { value: 'post_request', label: ll.phasePostRequest(), description: ll.phasePostRequestDesc() },
    { value: 'pre_request', label: ll.phasePreRequest(), description: ll.phasePreRequestDesc() },
    { value: 'standalone', label: ll.phaseStandalone(), description: ll.phaseStandaloneDesc() },
  ] as const;
}

/**
 * fn.* API 自动补全源
 * 基于 docs/script-node.md 中的 API 参考定义
 */
function fnApiCompletions(context: CompletionContext) {
  const ll = getLL().panels.script.apiCompletion;
  // 匹配 fn. 及后续的点分路径（如 fn.variables.get）
  const word = context.matchBefore(/fn\.[a-zA-Z_.]*/);
  if (!word) return null;

  return {
    from: word.from,
    options: [
      // fn.variables - 变量操作
      { label: 'fn.variables.get', type: 'function', detail: '(key: string) => any', info: ll.variablesGet() },
      { label: 'fn.variables.set', type: 'function', detail: '(key: string, value: any) => void', info: ll.variablesSet() },
      { label: 'fn.variables.setAll', type: 'function', detail: '(obj: object) => void', info: ll.variablesSetAll() },
      { label: 'fn.variables.has', type: 'function', detail: '(key: string) => boolean', info: ll.variablesHas() },
      { label: 'fn.variables.unset', type: 'function', detail: '(key: string) => void', info: ll.variablesUnset() },
      { label: 'fn.variables.clear', type: 'function', detail: '() => void', info: ll.variablesClear() },
      { label: 'fn.variables.all', type: 'function', detail: '() => string', info: ll.variablesAll() },
      { label: 'fn.variables.replaceIn', type: 'function', detail: '(template: string) => string', info: getLL().flow.fn_replace_in() },

      // fn.response - 响应数据（只读）
      { label: 'fn.response.code', type: 'property', detail: 'number', info: ll.responseCode() },
      { label: 'fn.response.status', type: 'property', detail: 'string', info: ll.responseStatus() },
      { label: 'fn.response.headers', type: 'property', detail: 'Record<string, string>', info: ll.responseHeaders() },
      { label: 'fn.response.body', type: 'property', detail: 'string', info: ll.responseBody() },
      { label: 'fn.response.size', type: 'property', detail: 'number', info: ll.responseSize() },
      { label: 'fn.response.time', type: 'property', detail: 'number', info: ll.responseTime() },
      { label: 'fn.response.text', type: 'function', detail: '() => string', info: ll.responseText() },
      { label: 'fn.response.json', type: 'function', detail: '() => string', info: ll.responseJson() },

      // fn.request - 请求数据（只读）
      { label: 'fn.request.url', type: 'property', detail: 'string', info: ll.requestUrl() },
      { label: 'fn.request.method', type: 'property', detail: 'string', info: ll.requestMethod() },
      { label: 'fn.request.headers', type: 'property', detail: 'Record<string, string>', info: ll.requestHeaders() },
      { label: 'fn.request.body', type: 'property', detail: 'string', info: ll.requestBody() },

      // fn.console - 日志输出
      { label: 'fn.console.log', type: 'function', detail: '(message: any) => void', info: ll.consoleLog() },
      { label: 'fn.console.info', type: 'function', detail: '(message: any) => void', info: ll.consoleInfo() },
      { label: 'fn.console.warn', type: 'function', detail: '(message: any) => void', info: ll.consoleWarn() },
      { label: 'fn.console.error', type: 'function', detail: '(message: any) => void', info: ll.consoleError() },
      { label: 'fn.console.debug', type: 'function', detail: '(message: any) => void', info: ll.consoleDebug() },
      { label: 'fn.console.table', type: 'function', detail: '(data: any) => void', info: ll.consoleTable() },

      // fn.flow - 流程控制
      { label: 'fn.flow.skip', type: 'function', detail: '() => void', info: ll.flowSkip() },
      { label: 'fn.flow.abort', type: 'function', detail: '(message: string) => void', info: ll.flowAbort() },
      { label: 'fn.flow.stop', type: 'function', detail: '() => void', info: ll.flowStop() },

      // fn.info - 流程信息（只读）
      { label: 'fn.info.flowName', type: 'property', detail: 'string', info: ll.flowInfoName() },
      { label: 'fn.info.flowId', type: 'property', detail: 'string', info: ll.flowInfoId() },
      { label: 'fn.info.nodeId', type: 'property', detail: 'string', info: ll.flowInfoNodeId() },
      { label: 'fn.info.nodeName', type: 'property', detail: 'string', info: ll.flowInfoNodeName() },

      // fn.util - 编码函数
      { label: 'fn.util.base64Encode', type: 'function', detail: '(str: string) => string', info: ll.utilBase64Encode() },
      { label: 'fn.util.base64Decode', type: 'function', detail: '(str: string) => string', info: ll.utilBase64Decode() },
      { label: 'fn.util.urlEncode', type: 'function', detail: '(str: string) => string', info: ll.utilUrlEncode() },
      { label: 'fn.util.urlDecode', type: 'function', detail: '(str: string) => string', info: ll.utilUrlDecode() },
      { label: 'fn.util.htmlEscape', type: 'function', detail: '(str: string) => string', info: ll.utilHtmlEscape() },
      { label: 'fn.util.htmlUnescape', type: 'function', detail: '(str: string) => string', info: ll.utilHtmlUnescape() },

      // fn.util - 加密函数
      { label: 'fn.util.md5', type: 'function', detail: '(str: string) => string', info: ll.utilMd5() },
      { label: 'fn.util.sha256', type: 'function', detail: '(str: string) => string', info: ll.utilSha256() },
      { label: 'fn.util.sha512', type: 'function', detail: '(str: string) => string', info: ll.utilSha512() },
      { label: 'fn.util.hmacSha256', type: 'function', detail: '(key: string, data: string) => string', info: ll.utilHmacSha256() },
      { label: 'fn.util.hmacSha512', type: 'function', detail: '(key: string, data: string) => string', info: ll.utilHmacSha512() },

      // fn.util - 时间函数
      { label: 'fn.util.timestamp', type: 'function', detail: '() => number', info: ll.utilTimestamp() },
      { label: 'fn.util.now', type: 'function', detail: '() => string', info: ll.utilNow() },
      { label: 'fn.util.formatTime', type: 'function', detail: '(format: string) => string', info: ll.utilFormatTime() },
      { label: 'fn.util.parseTime', type: 'function', detail: '(dateStr: string, format: string) => number', info: ll.utilParseTime() },

      { label: 'fn.util.sleep', type: 'function', detail: '(ms: number) => void', info: ll.utilSleep() },

      // fn.util - UUID 函数
      { label: 'fn.util.uuid', type: 'function', detail: '() => string', info: ll.utilUuid() },
      { label: 'fn.util.uuidv4', type: 'function', detail: '() => string', info: ll.utilUuidv4() },

      // fn.util - JSON 函数
      { label: 'fn.util.jsonParse', type: 'function', detail: '(str: string) => any', info: ll.utilJsonParse() },
      { label: 'fn.util.jsonStringify', type: 'function', detail: '(obj: any, indent?: number) => string', info: ll.utilJsonStringify() },
      { label: 'fn.util.jsonPath', type: 'function', detail: '(json: string, path: string) => string', info: ll.utilJsonPath() },

      // fn.util - 类型检查函数
      { label: 'fn.util.isArray', type: 'function', detail: '(v: any) => boolean', info: ll.utilIsArray() },
      { label: 'fn.util.isObject', type: 'function', detail: '(v: any) => boolean', info: ll.utilIsObject() },
      { label: 'fn.util.isString', type: 'function', detail: '(v: any) => boolean', info: ll.utilIsString() },
      { label: 'fn.util.isNumber', type: 'function', detail: '(v: any) => boolean', info: ll.utilIsNumber() },
      { label: 'fn.util.isBoolean', type: 'function', detail: '(v: any) => boolean', info: ll.utilIsBoolean() },
      { label: 'fn.util.isNull', type: 'function', detail: '(v: any) => boolean', info: ll.utilIsNull() },
      { label: 'fn.util.isUndefined', type: 'function', detail: '(v: any) => boolean', info: ll.utilIsUndefined() },
      { label: 'fn.util.isFunction', type: 'function', detail: '(v: any) => boolean', info: ll.utilIsFunction() },
      { label: 'fn.util.isEmpty', type: 'function', detail: '(v: any) => boolean', info: ll.utilIsEmpty() },
      { label: 'fn.util.typeOf', type: 'function', detail: '(v: any) => string', info: ll.utilTypeOf() },

      // fn.util - 对象操作函数
      { label: 'fn.util.keys', type: 'function', detail: '(obj: any) => string', info: ll.utilKeys() },
      { label: 'fn.util.values', type: 'function', detail: '(obj: any) => string', info: ll.utilValues() },
      { label: 'fn.util.entries', type: 'function', detail: '(obj: any) => string', info: ll.utilEntries() },
      { label: 'fn.util.hasKey', type: 'function', detail: '(obj: any, key: string) => boolean', info: ll.utilHasKey() },
      { label: 'fn.util.pick', type: 'function', detail: '(obj: any, keys: string[]) => string', info: ll.utilPick() },
      { label: 'fn.util.omit', type: 'function', detail: '(obj: any, keys: string[]) => string', info: ll.utilOmit() },
      { label: 'fn.util.merge', type: 'function', detail: '(target: any, source: any) => string', info: ll.utilMerge() },
      { label: 'fn.util.clone', type: 'function', detail: '(obj: any) => string', info: ll.utilClone() },

      // fn.expect - 链式断言
      { label: 'fn.expect', type: 'function', detail: '(value: any) => ExpectChain', info: ll.expect() },
    ],
  };
}

/**
 * 动态变量补全源（输入 $ 时补全动态变量，如 $guid, $timestamp）
 */
function dynamicVariableCompletions(context: CompletionContext) {
  const word = context.matchBefore(/(\{\{)?\$[a-zA-Z_0-9]*/);
  if (!word) return null;

  return {
    from: word.from,
    options: DYNAMIC_VARIABLES.map((v) => ({
      label: v.name,
      type: 'variable',
      detail: v.example,
      info: v.name,
    })),
  };
}

export function ScriptConfig({ data, onSave }: ScriptConfigProps) {
  const ll = getLL().panels.script;

  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<ScriptConfigFormData>({
    resolver: zodResolver(scriptConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

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

  const safeSetTimeout = useCallback((fn: () => void, ms: number) => {
    const id = setTimeout(() => {
      pendingTimersRef.current.delete(id);
      fn();
    }, ms);
    pendingTimersRef.current.add(id);
  }, []);

  // 监听表单值
  const watchedScript = watch('script');
  const watchedExecutionPhase = watch('execution_phase');

  // 当外部数据变化时，重置表单
  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

  // i18n option arrays
  const executionPhaseOptions = getExecutionPhaseOptions();

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
    // 延迟保存，确保表单校验完成且 isDirty ref 已更新
    safeSetTimeout(() => {
      if (!isActuallyDirtyRef.current) return;
      saveData();
    }, 100);
  }, [saveData]);

  // 监听 Ctrl+S 刷新事件，将表单数据同步到 store
  useEffect(() => {
    const handleFlush = () => { saveData(); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
  }, [saveData]);

  /**
   * 处理脚本编辑器变更
   */
  const handleScriptChange = useCallback((value: string) => {
    setValue('script', value, { shouldDirty: true });
    // 不立即保存，等用户离开编辑器时保存
  }, [setValue]);

  // CodeMirror 扩展（memoized，避免不必要的重渲染）
  const editorExtensions = useMemo(() => [
    javascript(),
    autocompletion({ override: [fnApiCompletions, dynamicVariableCompletions] }),
  ], []);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 脚本编辑器 */}
      <div className="space-y-2">
        <div className="flex items-center gap-2">
          <Label htmlFor="script" className="text-sm font-medium text-foreground flex items-center gap-2">
            <span>{ll.codeLabel()}</span>
            <span className="text-rose-500">*</span>
          </Label>
          <button
            type="button"
            onClick={() => useUIStore.getState().openScriptHelpDialog()}
            className="ml-auto inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
          >
            <BookOpen className="h-3.5 w-3.5" />
            <span>{ll.apiReference()}</span>
          </button>
        </div>
        <div className="rounded-md border overflow-hidden">
          <CodeMirror
            value={watchedScript || ''}
            height="200px"
            theme={oneDark}
            extensions={editorExtensions}
            onChange={handleScriptChange}
            onBlur={() => { if (isActuallyDirtyRef.current) saveData(); }}
            basicSetup={{
              lineNumbers: true,
              foldGutter: false,
              highlightActiveLine: true,
              bracketMatching: true,
              closeBrackets: true,
              indentOnInput: true,
              tabSize: 2,
            }}
          />
        </div>
        {errors.script && (
          <p className="text-xs text-rose-500">{errors.script.message}</p>
        )}

      </div>

      {/* 执行时机 */}
      <div className="space-y-2">
        <Label htmlFor="execution_phase" className="text-sm font-medium text-foreground flex items-center gap-2">
          <Clock className="h-4 w-4 text-muted-foreground" />
          {ll.executionPhase()}
        </Label>
        <Select
          value={watchedExecutionPhase || 'post_request'}
          onValueChange={(value) => {
            setValue('execution_phase', value as ScriptConfigFormData['execution_phase'], { shouldDirty: true });
            saveData();
          }}
        >
          <SelectTrigger id="execution_phase" className="font-mono">
            <SelectValue placeholder={ll.executionPhasePlaceholder()} />
          </SelectTrigger>
          <SelectContent>
            {executionPhaseOptions.map((option) => (
              <SelectItem key={option.value} value={option.value}>
                <div className="flex flex-col">
                  <span>{option.label}</span>
                </div>
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          {executionPhaseOptions.find(o => o.value === watchedExecutionPhase)?.description}
        </p>
      </div>

      {/* 超时时间和内存限制并排 */}
      <div className="grid grid-cols-2 gap-4">
        {/* 超时时间 */}
        <div className="space-y-2">
          <Label htmlFor="timeout_ms" className="text-sm font-medium text-foreground flex items-center gap-2">
            <Clock className="h-4 w-4 text-muted-foreground" />
            {ll.timeout()}
          </Label>
          <Input
            id="timeout_ms"
            type="number"
            min={100}
            max={60000}
            {...register('timeout_ms', { valueAsNumber: true })}
            className="font-mono"
          />
          <p className="text-xs text-muted-foreground">{ll.timeoutHint()}</p>
          {errors.timeout_ms && (
            <p className="text-xs text-rose-500">{errors.timeout_ms.message}</p>
          )}
        </div>

        {/* 内存限制 */}
        <div className="space-y-2">
          <Label htmlFor="memory_limit_mb" className="text-sm font-medium text-foreground flex items-center gap-2">
            <Database className="h-4 w-4 text-muted-foreground" />
            {ll.memoryLimit()}
          </Label>
          <Input
            id="memory_limit_mb"
            type="number"
            min={1}
            max={100}
            {...register('memory_limit_mb', { valueAsNumber: true })}
            className="font-mono"
          />
          <p className="text-xs text-muted-foreground">{ll.memoryLimitHint()}</p>
          {errors.memory_limit_mb && (
            <p className="text-xs text-rose-500">{errors.memory_limit_mb.message}</p>
          )}
        </div>
      </div>


    </form>
  );
}
