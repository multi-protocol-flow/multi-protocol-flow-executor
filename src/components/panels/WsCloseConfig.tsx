/**
 * WebSocket Close 节点配置组件
 * 支持关闭连接配置
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useCallback, useEffect, useMemo } from 'react';
import { wsCloseConfigSchema, type WsCloseConfigFormData } from './schemas';
import { Input } from '@/components/ui/input';
import { VariableInput } from '@/components/ui/VariableInput';
import { VariableValidationWarning } from '@/components/ui/VariableValidationWarning';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { PlugZap, Info } from 'lucide-react';
import type { WsCloseConfig as WsCloseConfigType } from '@/types';
import type { Variable } from '@/types/variable';
import { useFlowStore } from '@/stores';
import { isEntryConfig, isHttpConfig, isWsConnectConfig, isWsSendCollectConfig } from '@/types/node';

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
    type: 'ws_close',
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
    type: 'ws_close',
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
  { value: 1000, label: '1000 - 正常关闭' },
  { value: 1001, label: '1001 - 端点离开' },
  { value: 1002, label: '1002 - 协议错误' },
  { value: 1003, label: '1003 - 不支持的数据类型' },
  { value: 1005, label: '1005 - 无状态码' },
  { value: 1006, label: '1006 - 异常关闭' },
  { value: 1007, label: '1007 - 无效帧数据' },
  { value: 1008, label: '1008 - 策略违规' },
  { value: 1009, label: '1009 - 消息过大' },
  { value: 1010, label: '1010 - 缺少扩展' },
  { value: 1011, label: '1011 - 内部错误' },
  { value: 1012, label: '1012 - 服务重启' },
  { value: 1013, label: '1013 - 稍后重试' },
  { value: 1015, label: '1015 - TLS 握手失败' },
] as const;

export function WsCloseConfig({ data, onSave }: WsCloseConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    formState: { errors },
  } = useForm<WsCloseConfigFormData>({
    resolver: zodResolver(wsCloseConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  // 获取 flow 并提取变量
  const flow = useFlowStore((state) => state.flow);

  // 提取变量列表
  const variables: Variable[] = useMemo(() => {
    if (!flow) return [];
    const vars: Variable[] = [];
    flow.nodes.forEach(node => {
      // Entry 节点：提取 initial_variables
      if (isEntryConfig(node.data) && node.data.initial_variables) {
        node.data.initial_variables.forEach(iv => {
          vars.push({
            name: iv.name,
            value: iv.value,
            sourceNodeUuid: node.uuid,
            description: iv.description,
          });
        });
      }
      // HTTP 节点：提取 output_mappings
      if (isHttpConfig(node.data) && node.data.output_mappings) {
        node.data.output_mappings.forEach(om => {
          vars.push({
            name: om.target,
            value: `{{${om.source}}}`,
            sourceNodeUuid: node.uuid,
            description: om.description,
          });
        });
      }
      // WS Connect 节点：提取 output_mappings
      if (isWsConnectConfig(node.data) && node.data.output_mappings) {
        node.data.output_mappings.forEach(om => {
          vars.push({
            name: om.target,
            value: `{{${om.source}}}`,
            sourceNodeUuid: node.uuid,
            description: om.description,
          });
        });
      }
      // WS Send+Collect 节点：提取 output_mappings
      if (isWsSendCollectConfig(node.data) && node.data.output_mappings) {
        node.data.output_mappings.forEach(om => {
          vars.push({
            name: om.target,
            value: `{{${om.source}}}`,
            sourceNodeUuid: node.uuid,
            description: om.description,
          });
        });
      }
    });
    return vars;
  }, [flow]);

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

  // 提取可用变量名列表（用于验证）
  const availableVariableNames = useMemo(() => {
    return variables.map(v => v.name);
  }, [variables]);

  // 节点标签映射
  const nodeLabels = useMemo(() => {
    if (!flow) return {};
    const labels: Record<string, string> = {};
    flow.nodes.forEach(node => {
      labels[node.uuid] = node.name;
    });
    return labels;
  }, [flow]);

  // 当外部数据变化时，重置表单
  useEffect(() => {
    const formData = toFormData(data);
    setValue('connection_id', formData.connection_id);
    setValue('close_code', formData.close_code);
    setValue('close_reason', formData.close_reason);
    setValue('close_timeout_ms', formData.close_timeout_ms);
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
        console.error('保存 WebSocket Close 配置失败:', error);
      }
    })();
  }, [handleSubmit, onSave]);

  /**
   * 处理失焦事件
   */
  const handleBlur = useCallback(() => {
    setTimeout(saveData, 100);
  }, [saveData]);

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-rose-200/50 bg-gradient-to-br from-rose-50/80 to-pink-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-rose-900 flex items-center gap-2">
            <PlugZap className="h-4 w-4" />
            WebSocket 关闭连接节点
          </h4>
          <p className="text-sm leading-relaxed text-rose-700/90 whitespace-normal break-words">
            关闭指定的 WebSocket 连接，可选择发送关闭码和关闭原因。
          </p>
        </div>
      </div>

      {/* 连接 ID */}
      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-black">
          连接 ID <span className="text-rose-500">*</span>
        </Label>
        {availableConnectionIds.length > 0 ? (
          <Select
            value={watch('connection_id') || ''}
            onValueChange={(value) => {
              setValue('connection_id', value);
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
            onValueChange={(value) => setValue('connection_id', value)}
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

      {/* 关闭码 */}
      <div className="space-y-2">
        <Label htmlFor="close_code" className="text-sm font-medium text-black">
          关闭码
        </Label>
        <Select
          value={String(watch('close_code') || 1000)}
          onValueChange={(value) => {
            setValue('close_code', Number(value));
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
          选择标准的 WebSocket 关闭码
        </p>
      </div>

      {/* 关闭原因 */}
      <div className="space-y-2">
        <Label htmlFor="close_reason" className="text-sm font-medium text-black">
          关闭原因（可选）
        </Label>
        <Input
          id="close_reason"
          {...register('close_reason')}
          placeholder="正常关闭"
          className="font-mono text-sm"
        />
        <p className="text-xs text-muted-foreground">
          可选的关闭原因说明
        </p>
      </div>

      {/* 关闭超时 */}
      <div className="space-y-2">
        <Label htmlFor="close_timeout_ms" className="text-sm font-medium text-black">
          关闭超时
        </Label>
        <Input
          id="close_timeout_ms"
          type="number"
          min={1000}
          max={300000}
          {...register('close_timeout_ms', { valueAsNumber: true })}
        />
        <p className="text-xs text-muted-foreground">
          毫秒 (ms)，范围 1000-300000，等待服务器确认关闭
        </p>
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-black">关闭码说明：</p>
          <p><code className="rounded bg-muted px-1">1000</code> 正常关闭</p>
          <p><code className="rounded bg-muted px-1">1001</code> 端点离开（如页面关闭）</p>
          <p><code className="rounded bg-muted px-1">1002</code> 协议错误</p>
          <p><code className="rounded bg-muted px-1">1006</code> 异常关闭（无关闭帧）</p>
        </div>
      </div>
    </form>
  );
}
