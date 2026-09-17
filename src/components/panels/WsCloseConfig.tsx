/**
 * WebSocket Close 节点配置组件
 * 支持关闭连接配置
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo, useRef } from 'react';
import { wsCloseConfigSchema, type WsCloseConfigFormData } from './schemas';
import { Input } from '@/components/ui/input';
import { VariableInput } from '@/components/ui/VariableInput';
import { VariableValidationWarning } from '@/components/ui/VariableValidationWarning';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { PlugZap, Info } from 'lucide-react';
import type { WsCloseConfig as WsCloseConfigType } from '@/types';
import { useFlowVariables } from '@/hooks';
import { useFlowStore } from '@/stores';
import { isWsConnectConfig } from '@/types/node';
import { getLL } from '@/i18n/i18n-lazy';

interface WsCloseConfigProps {
  /** 当前节点数据 */
  data: WsCloseConfigType;
  /** 保存回调 */
  onSave: (data: WsCloseConfigType) => void;
}

/**
 * 将 WsCloseConfigType 转换为表单数据
 */
function toFormData(data: WsCloseConfigType): WsCloseConfigFormData {
  return {
    type: 'ws:close',
    connection_id: data.connection_id || '',
    close_code: data.close_code ?? 1000,
    close_reason: data.close_reason || '',
    close_timeout_ms: data.close_timeout_ms ?? 5000,
  };
}

/**
 * 将表单数据转换为 WsCloseConfigType
 */
function fromFormData(formData: WsCloseConfigFormData): WsCloseConfigType {
  return {
    type: 'ws:close',
    connection_id: formData.connection_id,
    close_code: formData.close_code,
    close_reason: formData.close_reason || undefined,
    close_timeout_ms: formData.close_timeout_ms,
  };
}

/**
 * 常用的 WebSocket 关闭码
 */
const CLOSE_CODES = [
  { value: 1000, label: `1000 - ${getLL().panels.wsClose.closeCode1000()}` },
  { value: 1001, label: `1001 - ${getLL().panels.wsClose.closeCode1001()}` },
  { value: 1002, label: `1002 - ${getLL().panels.wsClose.closeCode1002()}` },
  { value: 1003, label: `1003 - ${getLL().panels.wsClose.closeCode1003()}` },
  { value: 1005, label: `1005 - ${getLL().panels.wsClose.closeCode1005()}` },
  { value: 1006, label: `1006 - ${getLL().panels.wsClose.closeCode1006()}` },
  { value: 1007, label: `1007 - ${getLL().panels.wsClose.closeCode1007()}` },
  { value: 1008, label: `1008 - ${getLL().panels.wsClose.closeCode1008()}` },
  { value: 1009, label: `1009 - ${getLL().panels.wsClose.closeCode1009()}` },
  { value: 1010, label: `1010 - ${getLL().panels.wsClose.closeCode1010()}` },
  { value: 1011, label: `1011 - ${getLL().panels.wsClose.closeCode1011()}` },
  { value: 1012, label: `1012 - ${getLL().panels.wsClose.closeCode1012()}` },
  { value: 1013, label: `1013 - ${getLL().panels.wsClose.closeCode1013()}` },
  { value: 1015, label: `1015 - ${getLL().panels.wsClose.closeCode1015()}` },
] as const;

export function WsCloseConfig({ data, onSave }: WsCloseConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<WsCloseConfigFormData>({
    resolver: zodResolver(wsCloseConfigSchema),
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

  // 提取可用的 WebSocket 连接（从 ws:connect 节点）
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

  // 当外部数据变化时，完整重置表单（防止节点切换时残留旧数据）
  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

  // 追踪待执行的 setTimeout，组件卸载时清除，防止保存到错误的节点
  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  useEffect(() => {
    return () => {
      // 组件卸载时清除所有待执行的定时器
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

  /**
   * 保存表单数据
   */
  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存 WebSocket Close 配置失败:', error);
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

  // 监听 Ctrl+S 刷新事件，将表单数据同步到 store
  useEffect(() => {
    const handleFlush = () => { saveData(); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
  }, [saveData]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-rose-200/50 bg-gradient-to-br from-rose-50/80 to-pink-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-rose-900 flex items-center gap-2">
            <PlugZap className="h-4 w-4" />
            {getLL().panels.wsClose.title()}
          </h4>
          <p className="text-sm leading-relaxed text-rose-700/90 whitespace-normal break-words">
            {getLL().panels.wsClose.description()}
          </p>
        </div>
      </div>

      {/* 连接 ID */}
      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">
          {getLL().panels.wsClose.connectionId()} <span className="text-rose-500">*</span>
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
              <SelectValue placeholder={getLL().panels.selectConnection()} />
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
            {getLL().panels.wsSendCollect.noConnectNode()}
          </p>
        )}
      </div>

      {/* 关闭码 */}
      <div className="space-y-2">
        <Label htmlFor="close_code" className="text-sm font-medium text-foreground">
          {getLL().panels.wsClose.closeCode()}
        </Label>
        <Select
          value={String(watch('close_code') || 1000)}
          onValueChange={(value) => {
            setValue('close_code', Number(value), { shouldDirty: true });
            saveData();
          }}
        >
          <SelectTrigger id="close_code" className="font-mono">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {CLOSE_CODES.map((code) => (
              <SelectItem key={code.value} value={String(code.value)}>
                {code.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          {getLL().panels.wsClose.closeCodeHelp()}
        </p>
      </div>

      {/* 关闭原因 */}
      <div className="space-y-2">
        <Label htmlFor="close_reason" className="text-sm font-medium text-foreground">
          {getLL().panels.wsClose.closeReason()}
        </Label>
        <Input
          id="close_reason"
          {...register('close_reason')}
          placeholder={getLL().panels.wsClose.closeReasonPlaceholder()}
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          {getLL().panels.wsClose.closeReasonHelp()}
        </p>
      </div>

      {/* 关闭超时 */}
      <div className="space-y-2">
        <Label htmlFor="close_timeout_ms" className="text-sm font-medium text-foreground">
          {getLL().panels.close()}{getLL().panels.timeout()}
        </Label>
        <Input
          id="close_timeout_ms"
          type="number"
          min={1000}
          max={300000}
          {...register('close_timeout_ms', { valueAsNumber: true })}
        />
        <p className="text-xs text-muted-foreground">
          {getLL().panels.wsClose.closeTimeoutHelp()}
        </p>
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.wsClose.closeCodeInfo_title()}</p>
          <p><code className="rounded bg-muted px-1">1000</code> {getLL().panels.wsClose.closeCodeInfo_1000()}</p>
          <p><code className="rounded bg-muted px-1">1001</code> {getLL().panels.wsClose.closeCodeInfo_1001()}</p>
          <p><code className="rounded bg-muted px-1">1002</code> {getLL().panels.wsClose.closeCodeInfo_1002()}</p>
          <p><code className="rounded bg-muted px-1">1006</code> {getLL().panels.wsClose.closeCodeInfo_1006()}</p>
        </div>
      </div>
    </form>
  );
}
