/**
 * WebSocket Send+Collect 节点配置组件
 * 支持消息发送、采集响应配置
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import {
  wsSendCollectConfigSchema,
  type WsSendCollectConfigFormData,
  type WsCollectMode,
  type WsMatchType,
  type WsMessageType,
  type WsBinaryInputMode,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { VariableInput } from '@/components/ui/VariableInput';
import { VariableValidationWarning } from '@/components/ui/VariableValidationWarning';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { Send, Info, FolderOpen } from 'lucide-react';
import type { WsSendCollectConfig as WsSendCollectConfigType } from '@/types';
import { useFlowVariables } from '@/hooks';
import { useFlowStore } from '@/stores';
import { isWsConnectConfig } from '@/types/node';

interface WsSendCollectConfigProps {
  /** 当前节点数据 */
  data: WsSendCollectConfigType;
  /** 保存回调 */
  onSave: (data: WsSendCollectConfigType) => void;
}

/**
 * 将 WsSendCollectConfigType 转换为表单数据
 */
function toFormData(data: WsSendCollectConfigType): WsSendCollectConfigFormData {
  return {
    type: 'ws_send_collect',
    connection_id: data.connection_id || '',
    message: data.message || '',
    message_type: data.message_type || 'text',
    binary_input_mode: data.binary_input_mode || 'hex',
    send_timeout_ms: data.send_timeout_ms ?? 30000,
    collect_response: data.collect_response ?? true,
    collect_mode: data.collect_mode || 'first_match',
    collect_condition: data.collect_condition,
    collect_timeout_ms: data.collect_timeout_ms ?? 30000,
  };
}

/**
 * 将普通文本转换为十六进制字符串（UTF-8 字节）
 */
function textToHex(text: string): string {
  return Array.from(new TextEncoder().encode(text))
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}

/**
 * 将 Base64 字符串转换为十六进制字符串
 */
function base64ToHex(base64: string): string {
  try {
    const binary = atob(base64.trim());
    return Array.from(binary)
      .map(c => c.charCodeAt(0).toString(16).padStart(2, '0'))
      .join('');
  } catch {
    // Base64 无效，原样返回（让后端处理错误）
    return base64;
  }
}

/**
 * 将表单数据转换为 WsSendCollectConfigType
 * 注意：binary_input_mode 为 'base64' 或 'text' 时，在此处将 message 转为 hex
 */
function fromFormData(formData: WsSendCollectConfigFormData): WsSendCollectConfigType {
  let message = formData.message;

  // 二进制模式下，将非 hex 格式转换为 hex，确保后端统一接收 hex 字符串
  if (formData.message_type === 'binary') {
    if (formData.binary_input_mode === 'text') {
      message = textToHex(message);
    } else if (formData.binary_input_mode === 'base64') {
      message = base64ToHex(message);
    }
    // hex 模式：直接使用，后端 hex_to_bytes 会解析
  }

  return {
    type: 'ws_send_collect',
    connection_id: formData.connection_id,
    message,
    message_type: formData.message_type,
    binary_input_mode: formData.message_type === 'binary' ? (formData.binary_input_mode || 'hex') : undefined,
    send_timeout_ms: formData.send_timeout_ms,
    collect_response: formData.collect_response,
    collect_mode: formData.collect_response ? formData.collect_mode : undefined,
    collect_condition: formData.collect_response && formData.collect_mode === 'condition_match' ? formData.collect_condition : undefined,
    collect_timeout_ms: formData.collect_response ? formData.collect_timeout_ms : undefined,
  };
}

export function WsSendCollectConfig({ data, onSave }: WsSendCollectConfigProps) {
  const {
    register,
    handleSubmit,
    getValues,
    watch,
    setValue,
    reset,
    formState: { errors, isDirty },
  } = useForm<WsSendCollectConfigFormData>({
    resolver: zodResolver(wsSendCollectConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 追踪表单实际修改状态（useRef 用于卸载时读取）
  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

  // 获取流程变量
  const { variables, availableVariableNames, nodeLabels } = useFlowVariables();

  // 获取 flow 用于连接提取
  const flow = useFlowStore((state) => state.flow);

  // 提取可用的 WebSocket 连接（从 ws_connect 节点）
  const availableConnectionIds = useMemo(() => {
    if (!flow) return [];
    const connectionIds: Array<{ uuid: string; name: string }> = [];
    flow.nodes.forEach(node => {
      if (isWsConnectConfig(node.data)) {
        connectionIds.push({
          uuid: node.uuid,
          name: node.name,
        });
      }
    });
    return connectionIds;
  }, [flow]);

  // 监听表单值变化
  const collectResponse = watch('collect_response');
  const collectMode = watch('collect_mode');
  const messageType = watch('message_type');
  const binaryInputMode = watch('binary_input_mode') || 'hex';

  // 追踪待执行的 setTimeout，组件卸载时清除，防止保存到错误的节点
  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  // 文件 input ref（用于非 Tauri 环境降级）
  const fileInputRef = useRef<HTMLInputElement>(null);

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

  // 当外部数据变化时，重置表单（确保所有字段同步）
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
        console.error('保存 WebSocket Send+Collect 配置失败:', error);
      }
    })();
  }, [handleSubmit, onSave]);

  /**
   * 处理失焦事件
   */
  const handleBlur = useCallback(() => {
    safeSetTimeout(() => {
      if (!isActuallyDirtyRef.current) return;
      saveData();
    }, 100);
  }, [safeSetTimeout, saveData]);

  /**
   * 将文件内容（ArrayBuffer）转换为十六进制字符串
   */
  const arrayBufferToHex = useCallback((buffer: ArrayBuffer): string => {
    return Array.from(new Uint8Array(buffer))
      .map(b => b.toString(16).padStart(2, '0'))
      .join('');
  }, []);

  /**
   * 处理文件选择 — 隐藏 input[type=file] 触发系统文件选择器
   * 使用浏览器原生 FileReader API（在 Tauri WebView 和浏览器中均可用）
   */
  const handleFileSelect = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  /**
   * 处理文件 input 变更
   */
  const handleFileChange = useCallback(async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    try {
      const buffer = await file.arrayBuffer();
      const hex = arrayBufferToHex(buffer);
      setValue('message', hex, { shouldDirty: true });
      setValue('binary_input_mode', 'hex', { shouldDirty: true });
      saveData();
    } catch (err) {
      console.error('读取文件失败:', err);
    }
    // 清空 input，允许重复选同一文件
    e.target.value = '';
  }, [arrayBufferToHex, setValue, saveData]);

  /**
   * 获取二进制输入模式的 placeholder
   */
  const binaryPlaceholder = useMemo(() => {
    switch (binaryInputMode) {
      case 'hex': return '输入十六进制字节 (如: 010203 或 0x48656c6c6f)';
      case 'base64': return '输入 Base64 编码字符串 (如: SGVsbG8=)';
      case 'text': return '输入文本，将按 UTF-8 字节编码后以二进制帧发送';
      default: return '';
    }
  }, [binaryInputMode]);

  // 监听 Ctrl+S 刷新事件，将表单数据同步到 store
  useEffect(() => {
    const handleFlush = () => { saveData(); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
  }, [saveData]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-teal-200/50 bg-gradient-to-br from-teal-50/80 to-cyan-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-teal-900 flex items-center gap-2">
            <Send className="h-4 w-4" />
            WebSocket 发送+采集节点
          </h4>
          <p className="text-sm leading-relaxed text-teal-700/90 dark:text-teal-300/90 whitespace-normal break-words">
            通过已建立的 WebSocket 连接发送消息，可选择采集响应或条件满足后继续流程。
          </p>
        </div>
      </div>

      {/* 连接 ID */}
      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">
          连接 ID <span className="text-rose-500">*</span>
        </Label>
        {availableConnectionIds.length > 0 ? (
          <Select
            value={watch('connection_id') || ''}
            onValueChange={(value) => {
              setValue('connection_id', value, { shouldDirty: true });
              saveData();
            }}
          >
            <SelectTrigger id="connection_id" className="font-mono">
              <SelectValue placeholder="选择连接" />
            </SelectTrigger>
            <SelectContent>
              {availableConnectionIds.map((conn) => (
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
          <VariableInput
            id="connection_id"
            variables={variables}
            nodeLabels={nodeLabels}
            value={watch('connection_id') || ''}
            onValueChange={(value) => setValue('connection_id', value, { shouldDirty: true })}
            placeholder="ws_connection_id"
            className="font-mono text-sm"
          />
        )}
        {errors.connection_id && (
          <p className="text-xs text-rose-500">{errors.connection_id.message}</p>
        )}
        <VariableValidationWarning
          value={watch('connection_id') || ''}
          availableVariables={availableVariableNames}
          compact
        />
        {availableConnectionIds.length === 0 && (
          <p className="text-xs text-muted-foreground">
            未检测到 WS Connect 节点，请手动输入连接 ID 或使用变量引用
          </p>
        )}
      </div>

      {/* 消息内容 */}
      <div className="space-y-2">
        <Label htmlFor="message" className="text-sm font-medium text-foreground">
          消息内容 <span className="text-rose-500">*</span>
        </Label>

        {/* 二进制模式工具栏：仅当 message_type === 'binary' 时显示 */}
        {messageType === 'binary' && (
          <div className="flex items-center gap-2 p-2.5 rounded-md border border-orange-200/60 bg-orange-50/60 dark:border-orange-800/40 dark:bg-orange-950/30">
            <span className="text-xs text-orange-700 dark:text-orange-300 font-medium shrink-0">输入格式</span>
            <Select
              value={binaryInputMode}
              onValueChange={(value) => {
                setValue('binary_input_mode', value as WsBinaryInputMode, { shouldDirty: true });
                saveData();
              }}
            >
              <SelectTrigger className="h-7 text-xs font-mono flex-1">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="hex">Hex 十六进制 (如: 010203)</SelectItem>
                <SelectItem value="base64">Base64 编码 (如: SGVsbG8=)</SelectItem>
                <SelectItem value="text">文本转二进制 (UTF-8 字节)</SelectItem>
              </SelectContent>
            </Select>
            {/* 文件上传按钮 */}
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-7 px-2 text-xs shrink-0"
              onClick={handleFileSelect}
              title="从文件读取二进制内容（自动转换为 Hex）"
            >
              <FolderOpen className="h-3 w-3 mr-1" />
              上传文件
            </Button>
            {/* 隐藏的浏览器文件 input */}
            <input
              ref={fileInputRef}
              type="file"
              className="hidden"
              onChange={handleFileChange}
            />
          </div>
        )}

        <Textarea
          id="message"
          {...register('message')}
          placeholder={messageType === 'binary' ? binaryPlaceholder : '{"type": "ping", "data": "hello"}'}
          className="min-h-[100px] resize-y font-mono text-sm"
          rows={4}
        />
        {errors.message && (
          <p className="text-xs text-rose-500">{errors.message.message}</p>
        )}
        <VariableValidationWarning
          value={watch('message') || ''}
          availableVariables={availableVariableNames}
        />
        {messageType !== 'binary' && (
          <p className="text-xs text-muted-foreground">
            支持使用 <code className="rounded bg-muted px-1">{'{{变量名}}'}</code> 引用变量
          </p>
        )}
        {messageType === 'binary' && binaryInputMode === 'text' && (
          <p className="text-xs text-orange-600 dark:text-orange-400">
            ⚠️ 文本转二进制模式：内容将按 UTF-8 字节编码，通过二进制帧（Binary Frame）发送
          </p>
        )}
      </div>

      {/* 消息类型 */}
      <div className="space-y-2">
        <Label htmlFor="message_type" className="text-sm font-medium text-foreground">
          消息类型
        </Label>
        <Select
          value={watch('message_type') || 'text'}
          onValueChange={(value) => {
            setValue('message_type', value as WsMessageType, { shouldDirty: true });
            saveData();
          }}
        >
          <SelectTrigger id="message_type" className="font-mono">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="text">文本 (Text)</SelectItem>
            <SelectItem value="binary">二进制 (Binary)</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* 发送超时 */}
      <div className="space-y-2">
        <Label htmlFor="send_timeout_ms" className="text-sm font-medium text-foreground">
          发送超时
        </Label>
        <Input
          id="send_timeout_ms"
          type="number"
          min={1000}
          max={300000}
          {...register('send_timeout_ms', { valueAsNumber: true })}
        />
        <p className="text-xs text-muted-foreground">毫秒 (ms)，范围 1000-300000</p>
      </div>

      {/* 采集响应开关 */}
      <div className="flex items-center space-x-2">
        <Switch
          id="collect_response"
          checked={collectResponse || false}
          onCheckedChange={(checked) => {
            setValue('collect_response', checked, { shouldDirty: true });
            if (!checked) {
              // 关闭采集响应时，清除所有关联字段，避免 schema 验证失败
              setValue('collect_mode', undefined as unknown as WsCollectMode, { shouldDirty: true });
              setValue('collect_condition', undefined as unknown as { match_type: WsMatchType; expression: string }, { shouldDirty: true });
              setValue('collect_timeout_ms', undefined as unknown as number, { shouldDirty: true });
            }
            saveData();
          }}
        />
        <Label htmlFor="collect_response" className="text-sm font-medium text-foreground cursor-pointer">
          发送后采集响应
        </Label>
      </div>

      {/* 采集配置（仅当 collect_response 为 true 时显示） */}
      {collectResponse && (
        <>
          {/* 采集模式 */}
          <div className="space-y-2">
            <Label htmlFor="collect_mode" className="text-sm font-medium text-foreground">
              采集模式
            </Label>
            <Select
              value={collectMode || 'first_match'}
                onValueChange={(value) => {
                  setValue('collect_mode', value as WsCollectMode, { shouldDirty: true });
                  if (value === 'condition_match') {
                    // 切换到 condition_match 模式时，初始化 collect_condition 为默认值
                    setValue('collect_condition', { match_type: 'json_path', expression: '' }, { shouldDirty: true });
                  } else {
                    // 切换到非 condition_match 模式时，清除 collect_condition，避免 schema 验证失败
                    setValue('collect_condition', undefined as unknown as { match_type: WsMatchType; expression: string }, { shouldDirty: true });
                  }
                  saveData();
                }}
            >
              <SelectTrigger id="collect_mode" className="font-mono">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="first_match">采集首条即停</SelectItem>
                <SelectItem value="condition_match">匹配条件即停</SelectItem>
                <SelectItem value="duration_collect">持续采集</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* 采集条件（仅当 collect_mode 为 'condition_match' 时显示） */}
          {collectMode === 'condition_match' && (
            <div className="space-y-2">
              <Label className="text-sm font-medium text-foreground">采集条件</Label>
              <div className="space-y-2 p-3 rounded-md border bg-muted/30">
                {/* 匹配类型 */}
                <div className="space-y-2">
                  <Label htmlFor="collect_condition.match_type" className="text-xs text-muted-foreground">
                    匹配类型
                  </Label>
                  <Select
                    value={watch('collect_condition.match_type') || 'json_path'}
                    onValueChange={(value) => {
                      setValue('collect_condition.match_type', value as WsMatchType, { shouldDirty: true });
                      saveData();
                    }}
                  >
                    <SelectTrigger id="collect_condition.match_type" className="font-mono text-xs h-8">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="json_path">JSONPath</SelectItem>
                      <SelectItem value="regex">正则表达式</SelectItem>
                      <SelectItem value="expression">表达式</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                {/* 匹配表达式 */}
                <div className="space-y-2">
                  <Label htmlFor="collect_condition.expression" className="text-xs text-muted-foreground">
                    匹配表达式 <span className="text-rose-500">*</span>
                  </Label>
                  <Textarea
                    id="collect_condition.expression"
                    {...register('collect_condition.expression', { required: '匹配表达式不能为空' })}
                    placeholder={watch('collect_condition.match_type') === 'expression' ? 'msg == "authenticated"' : watch('collect_condition.match_type') === 'regex' ? 'authenticated|success' : '$.msg'}
                    className={`min-h-[80px] resize-y font-mono text-xs ${
                      errors.collect_condition?.expression ? 'border-rose-500 focus-visible:ring-rose-500' : ''
                    }`}
                    rows={3}
                  />
                  {errors.collect_condition?.expression && (
                    <p className="text-xs text-rose-500">{errors.collect_condition.expression.message}</p>
                  )}
                  <VariableValidationWarning
                    value={watch('collect_condition.expression') || ''}
                    availableVariables={availableVariableNames}
                    compact
                  />
                  
                  {/* 匹配类型说明 */}
                  <div className="flex items-start gap-2 rounded-md bg-sky-50/60 dark:bg-sky-950/40 border border-sky-200/50 dark:border-sky-700/40 p-3 w-full">
                    <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500 dark:text-sky-400" />
                    <div className="text-xs text-sky-700/90 dark:text-sky-300/90 space-y-2 min-w-0 flex-1">
                      <p className="font-medium text-sky-800 dark:text-sky-200">匹配类型说明：</p>
                      <div className="space-y-1.5">
                        {/* JSONPath */}
                        <div className="p-2 rounded bg-white/60 dark:bg-sky-900/30 border border-sky-100 dark:border-sky-700/30 w-full">
                          <p className="font-medium text-sky-800 dark:text-sky-200">📌 JSONPath — 检查字段是否存在</p>
                          <p className="font-mono text-[11px] text-sky-600 dark:text-sky-400 mt-1">
                            <code className="bg-sky-100/50 dark:bg-sky-800/60 px-1 rounded">$.msg</code> 检查 msg 字段是否存在
                          </p>
                          <p className="font-mono text-[11px] text-sky-600 dark:text-sky-400">
                            <code className="bg-sky-100/50 dark:bg-sky-800/60 px-1 rounded">$.data.status</code> 检查嵌套字段
                          </p>
                          <p className="text-[10px] text-sky-500 dark:text-sky-400 mt-1">⚠️ 仅检查字段存在，不支持值比较</p>
                        </div>
                        {/* Regex */}
                        <div className="p-2 rounded bg-white/60 dark:bg-sky-900/30 border border-sky-100 dark:border-sky-700/30 w-full">
                          <p className="font-medium text-sky-800 dark:text-sky-200">🔍 正则表达式 — 匹配消息文本</p>
                          <p className="font-mono text-[11px] text-sky-600 dark:text-sky-400 mt-1">
                            <code className="bg-sky-100/50 dark:bg-sky-800/60 px-1 rounded">authenticated</code> 包含该文本
                          </p>
                          <p className="font-mono text-[11px] text-sky-600 dark:text-sky-400">
                            <code className="bg-sky-100/50 dark:bg-sky-800/60 px-1 rounded">success|ok</code> 多选一
                          </p>
                          <p className="text-[10px] text-sky-500 dark:text-sky-400 mt-1">⚠️ 对整个消息文本进行正则匹配</p>
                        </div>
                        {/* Expression */}
                        <div className="p-2 rounded bg-white/60 dark:bg-sky-900/30 border border-sky-100 dark:border-sky-700/30 w-full">
                          <div className="flex items-center gap-1">
                            <p className="font-medium text-sky-800 dark:text-sky-200">🧮 多条件表达式 — 值比较（推荐）</p>
                            <span className="relative group inline-flex">
                              <Info className="h-3.5 w-3.5 text-sky-500 dark:text-sky-400 cursor-help" />
                              <span className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 px-3 py-2 rounded-lg bg-gray-900 dark:bg-gray-100 text-white dark:text-gray-900 text-[11px] whitespace-nowrap opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-opacity pointer-events-none z-10 shadow-lg">
                                <div className="space-y-1">
                                  <p>支持多条件组合：</p>
                                  <p><code className="font-mono">&&</code> 与 &nbsp; <code className="font-mono">||</code> 或 &nbsp; <code className="font-mono">!</code> 非</p>
                                  <p><code className="font-mono">&gt; &lt; &gt;= &lt;= == !=</code></p>
                                  <p className="text-[10px] opacity-80">例：a == "" && b != ""</p>
                                </div>
                                <span className="absolute top-full left-1/2 -translate-x-1/2 -mt-1 border-4 border-transparent border-t-gray-900 dark:border-t-gray-100" />
                              </span>
                            </span>
                          </div>
                          <p className="font-mono text-[11px] text-sky-600 dark:text-sky-400 mt-1">
                            <code className="bg-sky-100/50 dark:bg-sky-800/60 px-1 rounded">msg == "authenticated"</code> 字符串比较
                          </p>
                          <p className="font-mono text-[11px] text-sky-600 dark:text-sky-400">
                            <code className="bg-sky-100/50 dark:bg-sky-800/60 px-1 rounded">code == 1</code> 数字比较
                          </p>
                          <p className="text-[10px] text-sky-500 dark:text-sky-400 mt-1">✅ JSON 顶层字段自动注入，无需 $. 前缀</p>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* 采集超时 */}
          <div className="space-y-2">
            <Label htmlFor="collect_timeout_ms" className="text-sm font-medium text-foreground">
              采集超时
            </Label>
            <Input
              id="collect_timeout_ms"
              type="number"
              min={1000}
              max={300000}
              {...register('collect_timeout_ms', { valueAsNumber: true })}
            />
            <p className="text-xs text-muted-foreground">毫秒 (ms)，范围 1000-300000</p>
          </div>
        </>
      )}

    </form>
  );
}
