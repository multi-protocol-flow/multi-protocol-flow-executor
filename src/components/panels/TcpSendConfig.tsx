/**
 * TCP Send 节点配置组件
 * 支持连接选择、数据输入、编码方式、成帧配置、发送超时
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  tcpSendConfigSchema,
  type TcpSendConfigFormData,
} from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Send, Info, BookOpen } from 'lucide-react';
import type { TcpSendConfig as TcpSendConfigType, TcpFramingMode } from '@/types';
import { useFlowStore } from '@/stores';
import { isTcpConnectConfig } from '@/types/node';

/**
 * 协议模板预设
 * 包含成帧配置和解析器配置，与后端 templates.rs 保持同步
 */
interface ProtocolTemplate {
  id: string;
  name: string;
  description: string;
  category: string;
  framingMode?: TcpFramingMode;
  parserConfig?: Record<string, unknown>;
}

const PROTOCOL_TEMPLATES: ProtocolTemplate[] = [
  {
    id: 'redis_resp',
    name: 'Redis RESP',
    description: 'Redis 序列化协议',
    category: 'cache',
    framingMode: { type: 'delimiter', delimiter: '\r\n' },
    parserConfig: { type: 'delimiter', delimiter_hex: '0d0a', keep_delimiter: false },
  },
  {
    id: 'custom_tlv',
    name: 'Custom TLV',
    description: '自定义 Tag-Length-Value 协议',
    category: 'binary',
    framingMode: { type: 'length_prefix', prefix_bytes: 4 },
    parserConfig: { type: 'tlv', tag_bytes: 1, length_bytes: 2, byte_order: 'big' },
  },
  {
    id: 'fixed_header',
    name: 'Fixed Header',
    description: '固定头部长度协议',
    category: 'binary',
    framingMode: { type: 'length_prefix', prefix_bytes: 4 },
    parserConfig: { type: 'length_prefix', prefix_bytes: 4, byte_order: 'big', includes_self: false },
  },
  {
    id: 'http_like',
    name: 'HTTP-like',
    description: '类 HTTP 头部结束协议',
    category: 'text',
    framingMode: { type: 'delimiter', delimiter: '\r\n\r\n' },
    parserConfig: { type: 'delimiter', delimiter_hex: '0d0a', keep_delimiter: false },
  },
  {
    id: 'blank',
    name: 'Blank',
    description: '空白模板',
    category: 'custom',
    framingMode: undefined,
    parserConfig: undefined,
  },
];

interface TcpSendConfigProps {
  /** 当前节点数据 */
  data: TcpSendConfigType;
  /** 保存回调 */
  onSave: (data: TcpSendConfigType) => void;
}

/**
 * 成帧模式选项
 */
const FRAMING_MODE_OPTIONS = [
  { value: 'length_prefix', label: '长度前缀 (Length Prefix)' },
  { value: 'delimiter', label: '分隔符 (Delimiter)' },
  { value: 'fixed_length', label: '固定长度 (Fixed Length)' },
  { value: 'timeout', label: '超时读取 (Timeout)' },
  { value: 'manual', label: '手动控制 (Manual)' },
] as const;

/**
 * 数据编码方式选项
 */
const DATA_ENCODING_OPTIONS = [
  { value: 'text', label: '文本 (Text)' },
  { value: 'hex', label: '十六进制 (Hex)' },
  { value: 'base64', label: 'Base64' },
] as const;

/**
 * 将 TcpSendConfigType 转换为表单数据
 */
function toFormData(data: TcpSendConfigType): TcpSendConfigFormData {
  return {
    type: 'tcp_send',
    connection_id: data.connection_id || '',
    data: data.data || '',
    data_encoding: (data.data_encoding as 'text' | 'hex' | 'base64') ?? 'text',
    framing_mode: data.framing_mode,
    framing_config: data.framing_config,
    send_timeout_ms: data.send_timeout_ms ?? 30000,
    parser_config: data.parser_config,
  };
}

/**
 * 将表单数据转换为 TcpSendConfigType
 */
function fromFormData(formData: TcpSendConfigFormData): TcpSendConfigType {
  return {
    type: 'tcp_send',
    connection_id: formData.connection_id,
    data: formData.data,
    data_encoding: formData.data_encoding,
    framing_mode: formData.framing_mode,
    framing_config: formData.framing_config,
    send_timeout_ms: formData.send_timeout_ms,
    parser_config: formData.parser_config,
  };
}

/**
 * 获取成帧模式的显示名称
 */
function getFramingModeLabel(mode: TcpFramingMode | undefined): string {
  if (!mode) return '未设置';
  return FRAMING_MODE_OPTIONS.find(o => o.value === mode.type)?.label ?? mode.type;
}

export function TcpSendConfig({ data, onSave }: TcpSendConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<TcpSendConfigFormData>({
    resolver: zodResolver(tcpSendConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 获取 flow 并提取 tcp_connect 节点
  const flow = useFlowStore((state) => state.flow);

  // 提取可用的 TCP 连接（从 tcp_connect 节点）
  const availableConnections = useMemo(() => {
    if (!flow) return [];
    const connections: Array<{ uuid: string; name: string; host?: string; port?: number }> = [];
    flow.nodes.forEach(node => {
      if (isTcpConnectConfig(node.data)) {
        connections.push({
          uuid: node.uuid,
          name: node.name,
          host: node.data.host,
          port: node.data.port,
        });
      }
    });
    return connections;
  }, [flow]);

  // 监听表单值变化
  const currentFramingMode = watch('framing_mode');

  // 成帧模式选择状态
  const [selectedFramingType, setSelectedFramingType] = useState<string>(
    currentFramingMode?.type ?? 'none'
  );

  // 协议模板选择状态
  const [selectedTemplate, setSelectedTemplate] = useState<string>('custom');

  // 解析器类型选择状态
  const [selectedParserType, setSelectedParserType] = useState<string>(
    data.parser_config?.type ? String(data.parser_config.type) : 'none'
  );

  // 追踪待执行的 setTimeout，组件卸载时清除
  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  useEffect(() => {
    return () => {
      pendingTimersRef.current.forEach((id) => clearTimeout(id));
      pendingTimersRef.current.clear();
    };
  }, []);

  // 追踪表单实际修改状态
  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

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

  const safeSetTimeout = useCallback((fn: () => void, ms: number) => {
    const id = setTimeout(() => {
      pendingTimersRef.current.delete(id);
      fn();
    }, ms);
    pendingTimersRef.current.add(id);
  }, []);

  // 当外部数据变化时，重置表单
  useEffect(() => {
    reset(toFormData(data));
    setSelectedFramingType(data.framing_mode?.type ?? 'none');
    setSelectedParserType(data.parser_config?.type ? String(data.parser_config.type) : 'none');
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
        console.error('保存 TCP Send 配置失败:', error);
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

  // 监听 Ctrl+S 刷新事件
  useEffect(() => {
    const handleFlush = () => { saveData(); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
  }, [saveData]);

  /**
   * 处理成帧模式类型切换
   */
  const handleFramingTypeChange = useCallback((type: string) => {
    setSelectedFramingType(type);
    // 手动切换成帧模式时，重置模板选择为"自定义"
    setSelectedTemplate('custom');
    let newMode: TcpFramingMode | undefined;
    switch (type) {
      case 'length_prefix':
        newMode = { type: 'length_prefix', prefix_bytes: 4 };
        break;
      case 'delimiter':
        newMode = { type: 'delimiter', delimiter: '\n' };
        break;
      case 'fixed_length':
        newMode = { type: 'fixed_length', length: 1024 };
        break;
      case 'timeout':
        newMode = { type: 'timeout', timeout_ms: 5000 };
        break;
      case 'manual':
        newMode = { type: 'manual' };
        break;
      case 'none':
        newMode = undefined;
        break;
      default:
        newMode = undefined;
    }
    setValue('framing_mode', newMode as TcpFramingMode, { shouldDirty: true });
    safeSetTimeout(saveData, 100);
  }, [setValue, safeSetTimeout, saveData]);

  /**
   * 处理协议模板选择
   * 自动填充成帧配置和解析器配置
   */
  const handleTemplateChange = useCallback((templateId: string) => {
    setSelectedTemplate(templateId);
    const template = PROTOCOL_TEMPLATES.find(t => t.id === templateId);
    if (!template) return;
    // 填充成帧配置
    if (template.framingMode) {
      setSelectedFramingType(template.framingMode.type);
      setValue('framing_mode', template.framingMode, { shouldDirty: true });
    } else {
      setSelectedFramingType('');
      setValue('framing_mode', undefined as unknown as TcpFramingMode, { shouldDirty: true });
    }
    // 填充解析器配置
    if (template.parserConfig) {
      setSelectedParserType(String(template.parserConfig.type));
      setValue('parser_config', template.parserConfig, { shouldDirty: true });
    } else {
      setSelectedParserType('none');
      setValue('parser_config', undefined, { shouldDirty: true });
    }
    safeSetTimeout(saveData, 100);
  }, [setValue, safeSetTimeout, saveData]);

  /**
   * 更新成帧子字段
   */
  const updateFramingSubField = useCallback((field: string, value: number | string) => {
    const currentMode = getValues('framing_mode');
    if (!currentMode) return;
    const updated = { ...currentMode, [field]: value } as TcpFramingMode;
    setValue('framing_mode', updated, { shouldDirty: true });
    safeSetTimeout(saveData, 100);
  }, [getValues, setValue, safeSetTimeout, saveData]);

  /**
   * 处理解析器类型切换
   */
  const handleParserTypeChange = useCallback((type: string) => {
    setSelectedParserType(type);
    let newConfig: Record<string, unknown> | undefined;
    switch (type) {
      case 'length_prefix':
        newConfig = { type: 'length_prefix', prefix_bytes: 2 };
        break;
      case 'delimiter':
        newConfig = { type: 'delimiter', delimiter: '\r\n' };
        break;
      case 'tlv':
        newConfig = { type: 'tlv', tag_bytes: 1, length_bytes: 2, byte_order: 'big' };
        break;
      case 'none':
      default:
        newConfig = undefined;
        break;
    }
    setValue('parser_config', newConfig, { shouldDirty: true });
    safeSetTimeout(saveData, 100);
  }, [setValue, safeSetTimeout, saveData]);

  /**
   * 更新解析器子字段
   */
  const updateParserSubField = useCallback((field: string, value: number | string) => {
    const current = getValues('parser_config');
    if (!current) return;
    const updated = { ...current, [field]: value };
    setValue('parser_config', updated, { shouldDirty: true });
    safeSetTimeout(saveData, 100);
  }, [getValues, setValue, safeSetTimeout, saveData]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-emerald-200/50 bg-gradient-to-br from-emerald-50/80 to-teal-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-emerald-900 flex items-center gap-2">
            <Send className="h-4 w-4" />
            TCP 发送节点
          </h4>
          <p className="text-sm leading-relaxed text-emerald-700/90 whitespace-normal break-words">
            通过已建立的 TCP 连接发送数据，支持文本、十六进制、Base64 编码。
          </p>
        </div>
      </div>

      {/* 连接选择 */}
      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">
          连接 <span className="text-rose-500">*</span>
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
              <SelectValue placeholder="选择 TCP 连接" />
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
            placeholder="输入连接 ID"
            className="font-mono text-sm"
          />
        )}
        {errors.connection_id && (
          <p className="text-xs text-rose-500">{errors.connection_id.message}</p>
        )}
        {availableConnections.length === 0 && (
          <p className="text-xs text-muted-foreground">
            未检测到 TCP Connect 节点，请手动输入连接 ID
          </p>
        )}
      </div>

      {/* 数据编码方式 */}
      <div className="space-y-2">
        <Label htmlFor="data_encoding" className="text-sm font-medium text-foreground">
          编码方式
        </Label>
        <Select
          value={watch('data_encoding') || 'text'}
          onValueChange={(value) => {
            setValue('data_encoding', value as 'text' | 'hex' | 'base64', { shouldDirty: true });
            saveData();
          }}
        >
          <SelectTrigger id="data_encoding" className="font-mono text-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {DATA_ENCODING_OPTIONS.map((opt) => (
              <SelectItem key={opt.value} value={opt.value}>
                {opt.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          选择数据的编码格式，发送前会自动解码为原始字节
        </p>
      </div>

      {/* 数据输入 */}
      <div className="space-y-2">
        <Label htmlFor="data" className="text-sm font-medium text-foreground">
          发送数据
        </Label>
        <Textarea
          id="data"
          {...register('data')}
          placeholder={
            watch('data_encoding') === 'hex'
              ? '48656C6C6F20576F726C64'
              : watch('data_encoding') === 'base64'
                ? 'SGVsbG8gV29ybGQ='
                : '输入要发送的数据...'
          }
          className="min-h-[120px] resize-y font-mono text-sm"
          rows={5}
        />
        {errors.data && (
          <p className="text-xs text-rose-500">{errors.data.message}</p>
        )}
        <p className="text-xs text-muted-foreground">
          {watch('data_encoding') === 'hex'
            ? '十六进制格式，如 48656C6C6F'
            : watch('data_encoding') === 'base64'
              ? 'Base64 编码的字符串'
              : '纯文本数据，支持变量引用 {{variable}}'}
        </p>
      </div>

      {/* 协议模板选择 - 卡片网格 */}
      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground flex items-center gap-1.5">
          <BookOpen className="h-3.5 w-3.5 text-muted-foreground" />
          协议模板
        </Label>
        <div className="grid grid-cols-2 gap-2">
          {PROTOCOL_TEMPLATES.map((tpl) => {
            const isSelected = selectedTemplate === tpl.id;
            return (
              <button
                key={tpl.id}
                type="button"
                onClick={() => handleTemplateChange(tpl.id)}
                className={`group flex flex-col items-start gap-1 rounded-md border px-3 py-2.5 text-left transition-colors ${
                  isSelected
                    ? 'border-emerald-500 bg-emerald-50/80 text-emerald-900 shadow-sm'
                    : 'border-border bg-card hover:border-emerald-300 hover:bg-emerald-50/40 text-foreground'
                }`}
              >
                <span className={`text-xs font-semibold leading-tight ${
                  isSelected ? 'text-emerald-800' : 'text-foreground'
                }`}>
                  {tpl.name}
                </span>
                <span className={`text-[11px] leading-snug ${
                  isSelected ? 'text-emerald-600' : 'text-muted-foreground'
                }`}>
                  {tpl.description}
                </span>
              </button>
            );
          })}
        </div>
        <p className="text-xs text-muted-foreground">
          选择预设模板快速配置成帧与解析参数，也可手动调整
        </p>
      </div>

      {/* 成帧配置 */}
      <div className="space-y-3 rounded-md border bg-muted/30 p-3">
        <Label className="text-sm font-medium text-foreground">成帧配置（可选）</Label>
        <p className="text-xs text-muted-foreground">
          定义发送数据的帧封装方式，与连接配置配合使用
        </p>

        {/* 成帧模式选择 */}
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <Select
              value={selectedFramingType}
              onValueChange={handleFramingTypeChange}
            >
              <SelectTrigger className="font-mono text-sm">
                <SelectValue placeholder="不使用成帧" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="none">不使用成帧</SelectItem>
                {FRAMING_MODE_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>

        {/* 成帧子字段 */}
        {selectedFramingType === 'length_prefix' && (
          <div className="space-y-2 pl-2">
            <Label className="text-xs text-muted-foreground">前缀字节数</Label>
            <Select
              value={String(currentFramingMode?.type === 'length_prefix' ? (currentFramingMode as { type: 'length_prefix'; prefix_bytes: number }).prefix_bytes : 4)}
              onValueChange={(val) => updateFramingSubField('prefix_bytes', Number(val))}
            >
              <SelectTrigger className="h-8 font-mono text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {[1, 2, 4, 8].map((n) => (
                  <SelectItem key={n} value={String(n)}>{n} 字节</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        )}

        {selectedFramingType === 'delimiter' && (
          <div className="space-y-2 pl-2">
            <Label className="text-xs text-muted-foreground">分隔符</Label>
            <Input
              value={currentFramingMode?.type === 'delimiter' ? (currentFramingMode as { type: 'delimiter'; delimiter: string }).delimiter : '\n'}
              onChange={(e) => updateFramingSubField('delimiter', e.target.value)}
              className="h-8 font-mono text-xs"
              placeholder="\n"
            />
          </div>
        )}

        {selectedFramingType === 'fixed_length' && (
          <div className="space-y-2 pl-2">
            <Label className="text-xs text-muted-foreground">固定帧长度</Label>
            <Input
              type="number"
              min={1}
              value={currentFramingMode?.type === 'fixed_length' ? (currentFramingMode as { type: 'fixed_length'; length: number }).length : 1024}
              onChange={(e) => updateFramingSubField('length', Number(e.target.value))}
              className="h-8 font-mono text-xs"
            />
          </div>
        )}

        {selectedFramingType === 'timeout' && (
          <div className="space-y-2 pl-2">
            <Label className="text-xs text-muted-foreground">读取超时 (ms)</Label>
            <Input
              type="number"
              min={100}
              value={currentFramingMode?.type === 'timeout' ? (currentFramingMode as { type: 'timeout'; timeout_ms: number }).timeout_ms : 5000}
              onChange={(e) => updateFramingSubField('timeout_ms', Number(e.target.value))}
              className="h-8 font-mono text-xs"
            />
          </div>
        )}

        {selectedFramingType === 'manual' && (
          <p className="text-xs text-muted-foreground pl-2">
            手动模式下不自动成帧，直接发送原始数据
          </p>
        )}
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
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">毫秒 (ms)，范围 1000-300000</p>
      </div>

      {/* 协议解析配置 */}
      <div className="space-y-3 rounded-md border bg-muted/30 p-3">
        <Label className="text-sm font-medium text-foreground">协议解析</Label>
        <p className="text-xs text-muted-foreground">
          配置接收数据的解析方式，将原始字节流解析为结构化消息
        </p>

        {/* 解析器类型选择 */}
        <div className="space-y-2">
          <Select
            value={selectedParserType}
            onValueChange={handleParserTypeChange}
          >
            <SelectTrigger className="font-mono text-sm">
              <SelectValue placeholder="无解析" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="none">无</SelectItem>
              <SelectItem value="length_prefix">长度前缀</SelectItem>
              <SelectItem value="delimiter">分隔符</SelectItem>
              <SelectItem value="tlv">TLV</SelectItem>
            </SelectContent>
          </Select>
        </div>

        {/* 长度前缀子字段 */}
        {selectedParserType === 'length_prefix' && (
          <div className="space-y-2 pl-2">
            <Label className="text-xs text-muted-foreground">前缀字节数</Label>
            <Select
              value={String(watch('parser_config')?.prefix_bytes ?? 2)}
              onValueChange={(val) => updateParserSubField('prefix_bytes', Number(val))}
            >
              <SelectTrigger className="h-8 font-mono text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="2">2 字节</SelectItem>
                <SelectItem value="4">4 字节</SelectItem>
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">用于表示消息长度的字节数</p>
          </div>
        )}

        {/* 分隔符子字段 */}
        {selectedParserType === 'delimiter' && (
          <div className="space-y-2 pl-2">
            <Label className="text-xs text-muted-foreground">分隔符</Label>
            <Input
              value={String(watch('parser_config')?.delimiter ?? '\r\n')}
              onChange={(e) => updateParserSubField('delimiter', e.target.value)}
              className="h-8 font-mono text-xs"
              placeholder="\r\n"
            />
            <p className="text-xs text-muted-foreground">遇到此分隔符时视为一条消息结束</p>
          </div>
        )}

        {/* TLV 子字段 */}
        {selectedParserType === 'tlv' && (
          <div className="space-y-3 pl-2">
            <div className="space-y-1.5">
              <Label className="text-xs text-muted-foreground">Tag 字节数</Label>
              <Input
                type="number"
                min={1}
                max={8}
                value={Number(watch('parser_config')?.tag_bytes ?? 1)}
                onChange={(e) => updateParserSubField('tag_bytes', Number(e.target.value) || 1)}
                className="h-8 font-mono text-xs"
              />
            </div>
            <div className="space-y-1.5">
              <Label className="text-xs text-muted-foreground">Length 字节数</Label>
              <Input
                type="number"
                min={1}
                max={8}
                value={Number(watch('parser_config')?.length_bytes ?? 2)}
                onChange={(e) => updateParserSubField('length_bytes', Number(e.target.value) || 2)}
                className="h-8 font-mono text-xs"
              />
            </div>
            <div className="space-y-1.5">
              <Label className="text-xs text-muted-foreground">字节序</Label>
              <Select
                value={String(watch('parser_config')?.byte_order ?? 'big')}
                onValueChange={(val) => updateParserSubField('byte_order', val)}
              >
                <SelectTrigger className="h-8 font-mono text-xs">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="big">大端序 (Big Endian)</SelectItem>
                  <SelectItem value="little">小端序 (Little Endian)</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        )}
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">发送说明：</p>
          <p>选择已建立的 TCP 连接，输入要发送的数据</p>
          <p>数据会根据编码方式自动转换为原始字节发送</p>
          <p>成帧配置: <code className="rounded bg-muted px-1">{getFramingModeLabel(currentFramingMode)}</code></p>
        </div>
      </div>
    </form>
  );
}
