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

/**
 * fn.* API 自动补全源
 * 基于 docs/script-node.md 中的 API 参考定义
 */
function fnApiCompletions(context: CompletionContext) {
  // 匹配 fn. 及后续的点分路径（如 fn.variables.get）
  const word = context.matchBefore(/fn\.[a-zA-Z_.]*/);
  if (!word) return null;

  return {
    from: word.from,
    options: [
      // fn.variables - 变量操作
      { label: 'fn.variables.get', type: 'function', detail: '(key: string) => any', info: '获取变量值' },
      { label: 'fn.variables.set', type: 'function', detail: '(key: string, value: any) => void', info: '设置变量' },
      { label: 'fn.variables.setAll', type: 'function', detail: '(obj: object) => void', info: '批量设置变量' },
      { label: 'fn.variables.has', type: 'function', detail: '(key: string) => boolean', info: '检查变量是否存在' },
      { label: 'fn.variables.unset', type: 'function', detail: '(key: string) => void', info: '删除变量' },
      { label: 'fn.variables.clear', type: 'function', detail: '() => void', info: '清空所有变量' },
      { label: 'fn.variables.all', type: 'function', detail: '() => string', info: '获取所有变量（JSON 字符串）' },

      // fn.response - 响应数据（只读）
      { label: 'fn.response.code', type: 'property', detail: 'number', info: 'HTTP 状态码' },
      { label: 'fn.response.status', type: 'property', detail: 'string', info: '状态文本' },
      { label: 'fn.response.headers', type: 'property', detail: 'Record<string, string>', info: '响应头对象' },
      { label: 'fn.response.body', type: 'property', detail: 'string', info: '响应体（JSON 字符串）' },
      { label: 'fn.response.size', type: 'property', detail: 'number', info: '响应大小（字节）' },
      { label: 'fn.response.time', type: 'property', detail: 'number', info: '响应时间（毫秒）' },
      { label: 'fn.response.text', type: 'function', detail: '() => string', info: '获取原始文本' },
      { label: 'fn.response.json', type: 'function', detail: '() => string', info: '获取 JSON 对象（JSON 字符串）' },

      // fn.request - 请求数据（只读）
      { label: 'fn.request.url', type: 'property', detail: 'string', info: '请求 URL' },
      { label: 'fn.request.method', type: 'property', detail: 'string', info: '请求方法' },
      { label: 'fn.request.headers', type: 'property', detail: 'Record<string, string>', info: '请求头对象' },
      { label: 'fn.request.body', type: 'property', detail: 'string', info: '请求体' },

      // fn.console - 日志输出
      { label: 'fn.console.log', type: 'function', detail: '(message: any) => void', info: '普通日志' },
      { label: 'fn.console.info', type: 'function', detail: '(message: any) => void', info: '信息日志' },
      { label: 'fn.console.warn', type: 'function', detail: '(message: any) => void', info: '警告日志' },
      { label: 'fn.console.error', type: 'function', detail: '(message: any) => void', info: '错误日志' },
      { label: 'fn.console.debug', type: 'function', detail: '(message: any) => void', info: '调试日志' },
      { label: 'fn.console.table', type: 'function', detail: '(data: any) => void', info: '表格数据' },

      // fn.flow - 流程控制
      { label: 'fn.flow.skip', type: 'function', detail: '() => void', info: '跳过后续节点，流程继续' },
      { label: 'fn.flow.abort', type: 'function', detail: '(message: string) => void', info: '终止流程并报错' },
      { label: 'fn.flow.stop', type: 'function', detail: '() => void', info: '正常终止流程' },

      // fn.info - 流程信息（只读）
      { label: 'fn.info.flowName', type: 'property', detail: 'string', info: '流程名称' },
      { label: 'fn.info.flowId', type: 'property', detail: 'string', info: '流程 ID' },
      { label: 'fn.info.nodeId', type: 'property', detail: 'string', info: '当前节点 ID' },
      { label: 'fn.info.nodeName', type: 'property', detail: 'string', info: '当前节点名称' },

      // fn.util - 编码函数
      { label: 'fn.util.base64Encode', type: 'function', detail: '(str: string) => string', info: 'Base64 编码' },
      { label: 'fn.util.base64Decode', type: 'function', detail: '(str: string) => string', info: 'Base64 解码' },
      { label: 'fn.util.urlEncode', type: 'function', detail: '(str: string) => string', info: 'URL 编码' },
      { label: 'fn.util.urlDecode', type: 'function', detail: '(str: string) => string', info: 'URL 解码' },
      { label: 'fn.util.htmlEscape', type: 'function', detail: '(str: string) => string', info: 'HTML 转义' },
      { label: 'fn.util.htmlUnescape', type: 'function', detail: '(str: string) => string', info: 'HTML 反转义' },

      // fn.util - 加密函数
      { label: 'fn.util.md5', type: 'function', detail: '(str: string) => string', info: 'MD5 哈希' },
      { label: 'fn.util.sha256', type: 'function', detail: '(str: string) => string', info: 'SHA-256 哈希' },
      { label: 'fn.util.sha512', type: 'function', detail: '(str: string) => string', info: 'SHA-512 哈希' },
      { label: 'fn.util.hmacSha256', type: 'function', detail: '(key: string, data: string) => string', info: 'HMAC-SHA256' },
      { label: 'fn.util.hmacSha512', type: 'function', detail: '(key: string, data: string) => string', info: 'HMAC-SHA512' },

      // fn.util - 时间函数
      { label: 'fn.util.timestamp', type: 'function', detail: '() => number', info: '当前时间戳（毫秒）' },
      { label: 'fn.util.now', type: 'function', detail: '() => string', info: '当前 ISO 8601 时间字符串' },
      { label: 'fn.util.formatTime', type: 'function', detail: '(format: string) => string', info: '格式化当前时间' },
      { label: 'fn.util.parseTime', type: 'function', detail: '(dateStr: string, format: string) => number', info: '解析时间为时间戳' },

      // fn.util - UUID 函数
      { label: 'fn.util.uuid', type: 'function', detail: '() => string', info: '生成 UUID v4' },
      { label: 'fn.util.uuidv4', type: 'function', detail: '() => string', info: '生成 UUID v4（别名）' },

      // fn.util - JSON 函数
      { label: 'fn.util.jsonParse', type: 'function', detail: '(str: string) => any', info: '解析 JSON 字符串' },
      { label: 'fn.util.jsonStringify', type: 'function', detail: '(obj: any, indent?: number) => string', info: '对象转 JSON 字符串' },
      { label: 'fn.util.jsonPath', type: 'function', detail: '(json: string, path: string) => string', info: 'JSONPath 查询' },

      // fn.util - 类型检查函数
      { label: 'fn.util.isArray', type: 'function', detail: '(v: any) => boolean', info: '是否为数组' },
      { label: 'fn.util.isObject', type: 'function', detail: '(v: any) => boolean', info: '是否为对象' },
      { label: 'fn.util.isString', type: 'function', detail: '(v: any) => boolean', info: '是否为字符串' },
      { label: 'fn.util.isNumber', type: 'function', detail: '(v: any) => boolean', info: '是否为数字' },
      { label: 'fn.util.isBoolean', type: 'function', detail: '(v: any) => boolean', info: '是否为布尔值' },
      { label: 'fn.util.isNull', type: 'function', detail: '(v: any) => boolean', info: '是否为 null' },
      { label: 'fn.util.isUndefined', type: 'function', detail: '(v: any) => boolean', info: '是否为 undefined' },
      { label: 'fn.util.isFunction', type: 'function', detail: '(v: any) => boolean', info: '是否为函数' },
      { label: 'fn.util.isEmpty', type: 'function', detail: '(v: any) => boolean', info: '是否为空值' },
      { label: 'fn.util.typeOf', type: 'function', detail: '(v: any) => string', info: '返回类型字符串' },

      // fn.util - 对象操作函数
      { label: 'fn.util.keys', type: 'function', detail: '(obj: any) => string', info: '获取对象键数组（JSON 字符串）' },
      { label: 'fn.util.values', type: 'function', detail: '(obj: any) => string', info: '获取对象值数组（JSON 字符串）' },
      { label: 'fn.util.entries', type: 'function', detail: '(obj: any) => string', info: '获取键值对数组（JSON 字符串）' },
      { label: 'fn.util.hasKey', type: 'function', detail: '(obj: any, key: string) => boolean', info: '检查是否包含键' },
      { label: 'fn.util.pick', type: 'function', detail: '(obj: any, keys: string[]) => string', info: '选择指定键（JSON 字符串）' },
      { label: 'fn.util.omit', type: 'function', detail: '(obj: any, keys: string[]) => string', info: '排除指定键（JSON 字符串）' },
      { label: 'fn.util.merge', type: 'function', detail: '(target: any, source: any) => string', info: '合并对象（JSON 字符串）' },
      { label: 'fn.util.clone', type: 'function', detail: '(obj: any) => string', info: '深拷贝对象（JSON 字符串）' },

      // fn.expect - 链式断言
      { label: 'fn.expect', type: 'function', detail: '(value: any) => ExpectChain', info: '链式断言 API' },
    ],
  };
}

export function ScriptConfig({ data, onSave }: ScriptConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    formState: { errors },
  } = useForm<ScriptConfigFormData>({
    resolver: zodResolver(scriptConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 追踪待执行的 setTimeout，组件卸载时清除，防止保存到错误的节点
  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  useEffect(() => {
    return () => {
      pendingTimersRef.current.forEach((id) => clearTimeout(id));
      pendingTimersRef.current.clear();
    };
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
  const watchedOnError = watch('on_error');
  const watchedOnAssertionFail = watch('on_assertion_fail');

  // 当外部数据变化时，重置表单
  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

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
    safeSetTimeout(saveData, 100);
  }, [saveData]);

  /**
   * 处理脚本编辑器变更
   */
  const handleScriptChange = useCallback((value: string) => {
    setValue('script', value);
    // 不立即保存，等用户离开编辑器时保存
  }, [setValue]);

  // CodeMirror 扩展（memoized，避免不必要的重渲染）
  const editorExtensions = useMemo(() => [
    javascript(),
    autocompletion({ override: [fnApiCompletions] }),
  ], []);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 脚本编辑器 */}
      <div className="space-y-2">
        <Label htmlFor="script" className="text-sm font-medium text-foreground flex items-center gap-2">
          <span>脚本代码</span>
          <span className="text-rose-500">*</span>
        </Label>
        <div className="rounded-md border overflow-hidden">
          <CodeMirror
            value={watchedScript || ''}
            height="200px"
            theme={oneDark}
            extensions={editorExtensions}
            onChange={handleScriptChange}
            onBlur={saveData}
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
        {/* 脚本 API 提示 */}
        <div className="flex items-start gap-2 rounded-md bg-muted/30 p-2">
          <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
          <div className="text-xs text-muted-foreground space-y-1">
            <p className="font-medium text-foreground">可用 API：</p>
            <p><code className="rounded bg-muted px-1">variables.get(name)</code> 获取变量</p>
            <p><code className="rounded bg-muted px-1">variables.set(name, value)</code> 设置变量</p>
            <p><code className="rounded bg-muted px-1">assert(condition, name)</code> 断言</p>
            <p><code className="rounded bg-muted px-1">log(msg)</code> / <code className="rounded bg-muted px-1">console.log(msg)</code> 输出日志</p>
          </div>
        </div>
      </div>

      {/* 执行时机 */}
      <div className="space-y-2">
        <Label htmlFor="execution_phase" className="text-sm font-medium text-foreground flex items-center gap-2">
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
          <Label htmlFor="timeout_ms" className="text-sm font-medium text-foreground flex items-center gap-2">
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
          <Label htmlFor="memory_limit_mb" className="text-sm font-medium text-foreground flex items-center gap-2">
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
        <Label htmlFor="on_error" className="text-sm font-medium text-foreground flex items-center gap-2">
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
        <Label htmlFor="on_assertion_fail" className="text-sm font-medium text-foreground flex items-center gap-2">
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
