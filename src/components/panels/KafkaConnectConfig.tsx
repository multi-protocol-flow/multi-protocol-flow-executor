/**
 * Kafka Connect 节点配置组件
 * 支持 Bootstrap Servers、SASL 认证、TLS、Schema Registry
 */
import { useForm } from 'react-hook-form';
import { useState, useCallback, useEffect, useRef } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Radio, Info, Shield, ChevronDown, Database } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';
import type { KafkaConnectConfig as KafkaConnectConfigType } from '@/types';
import { cn } from '@/lib/utils';

/** 表单数据类型 */
interface KafkaConnectFormData {
  type: 'kafka_connect';
  bootstrap_servers: string;
  client_id: string;
  security_protocol: string;
  sasl_mechanism: string;
  sasl_username: string;
  sasl_password: string;
  sasl_oauth_token: string;
  ssl_ca_cert_path: string;
  ssl_client_cert_path: string;
  ssl_client_key_path: string;
  ssl_skip_verify: boolean;
  schema_registry_url: string;
  schema_registry_username: string;
  schema_registry_password: string;
  connect_timeout_ms: number;
}

interface KafkaConnectConfigProps {
  data: KafkaConnectConfigType;
  onSave: (data: KafkaConnectConfigType) => void;
}

function toFormData(data: KafkaConnectConfigType): KafkaConnectFormData {
  return {
    type: 'kafka_connect',
    bootstrap_servers: data.bootstrap_servers || '',
    client_id: data.client_id || '',
    security_protocol: data.security_protocol || 'plaintext',
    sasl_mechanism: data.sasl_mechanism || 'PLAIN',
    sasl_username: data.sasl_username || '',
    sasl_password: data.sasl_password || '',
    sasl_oauth_token: data.sasl_oauth_token || '',
    ssl_ca_cert_path: data.ssl_ca_cert_path || '',
    ssl_client_cert_path: data.ssl_client_cert_path || '',
    ssl_client_key_path: data.ssl_client_key_path || '',
    ssl_skip_verify: data.ssl_skip_verify ?? false,
    schema_registry_url: data.schema_registry_url || '',
    schema_registry_username: data.schema_registry_username || '',
    schema_registry_password: data.schema_registry_password || '',
    connect_timeout_ms: data.connect_timeout_ms ?? 5000,
  };
}

function fromFormData(formData: KafkaConnectFormData): KafkaConnectConfigType {
  return {
    type: 'kafka_connect',
    bootstrap_servers: formData.bootstrap_servers || undefined,
    client_id: formData.client_id || undefined,
    security_protocol: (formData.security_protocol || undefined) as KafkaConnectConfigType['security_protocol'],
    sasl_mechanism: formData.sasl_mechanism || undefined,
    sasl_username: formData.sasl_username || undefined,
    sasl_password: formData.sasl_password || undefined,
    sasl_oauth_token: formData.sasl_oauth_token || undefined,
    ssl_ca_cert_path: formData.ssl_ca_cert_path || undefined,
    ssl_client_cert_path: formData.ssl_client_cert_path || undefined,
    ssl_client_key_path: formData.ssl_client_key_path || undefined,
    ssl_skip_verify: formData.ssl_skip_verify || undefined,
    schema_registry_url: formData.schema_registry_url || undefined,
    schema_registry_username: formData.schema_registry_username || undefined,
    schema_registry_password: formData.schema_registry_password || undefined,
    connect_timeout_ms: formData.connect_timeout_ms,
  };
}

export function KafkaConnectConfig({ data, onSave }: KafkaConnectConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { isDirty },
  } = useForm<KafkaConnectFormData>({
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

  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存 Kafka Connect 配置失败:', error);
      }
    })();
  }, [handleSubmit, onSave]);

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

  // 可折叠区域
  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set());

  const toggleSection = useCallback((section: string) => {
    setExpandedSections(prev => {
      const next = new Set(prev);
      if (next.has(section)) next.delete(section);
      else next.add(section);
      return next;
    });
  }, []);

  // 监听条件渲染字段
  const securityProtocol = watch('security_protocol') || 'plaintext';
  const saslMechanism = watch('sasl_mechanism') || 'PLAIN';
  const schemaRegistryUrl = watch('schema_registry_url') || '';
  const isSasl = securityProtocol === 'sasl_plaintext' || securityProtocol === 'sasl_ssl';
  const isSsl = securityProtocol === 'ssl' || securityProtocol === 'sasl_ssl';
  const isOAuth = saslMechanism === 'OAUTHBEARER';

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-orange-200/50 bg-gradient-to-br from-orange-50/80 to-amber-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-orange-900 flex items-center gap-2">
            <Radio className="h-4 w-4" />
            {getLL().panels.kafkaConnect.title()}
          </h4>
          <p className="text-sm leading-relaxed text-orange-700/90 whitespace-normal break-words">
            {getLL().panels.kafkaConnect.description()}
          </p>
        </div>
      </div>

      {/* 连接配置 */}
      <div className="space-y-3">
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider">连接配置</p>

        <div className="space-y-2">
          <Label htmlFor="bootstrap_servers" className="text-sm font-medium text-foreground">
            Bootstrap Servers <span className="text-rose-500">*</span>
          </Label>
          <div className="relative">
            <Radio className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground z-10" />
            <Input
              id="bootstrap_servers"
              {...register('bootstrap_servers')}
              placeholder="localhost:9092"
              className="pl-9 font-mono text-sm"
            />
          </div>
          <p className="text-xs text-muted-foreground">
            多个 Broker 用逗号分隔，例如 host1:9092,host2:9092
          </p>
        </div>

        <div className="space-y-2">
          <Label htmlFor="client_id" className="text-sm font-medium text-foreground">
            {getLL().panels.kafkaConnect.clientId()}
          </Label>
          <Input
            id="client_id"
            {...register('client_id')}
            placeholder="Auto-generate if empty"
            className="font-mono text-sm"
          />
        </div>

        <div className="space-y-2">
          <Label className="text-sm font-medium text-foreground">安全协议</Label>
          <Select
            value={securityProtocol}
            onValueChange={(value) => {
              setValue('security_protocol', value, { shouldDirty: true });
              saveData();
            }}
          >
            <SelectTrigger className="font-mono text-sm">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="plaintext">PLAINTEXT</SelectItem>
              <SelectItem value="ssl">SSL</SelectItem>
              <SelectItem value="sasl_plaintext">SASL_PLAINTEXT</SelectItem>
              <SelectItem value="sasl_ssl">SASL_SSL</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>

      {/* 认证配置 — 仅 SASL 模式显示 */}
      {isSasl && (
        <div className="space-y-3">
          <p className="text-xs font-semibold text-foreground uppercase tracking-wider">认证配置</p>

          <div className="space-y-2">
            <Label className="text-sm font-medium text-foreground">SASL 机制</Label>
            <Select
              value={saslMechanism}
              onValueChange={(value) => {
                setValue('sasl_mechanism', value, { shouldDirty: true });
                saveData();
              }}
            >
              <SelectTrigger className="font-mono text-sm">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="PLAIN">PLAIN</SelectItem>
                <SelectItem value="SCRAM-SHA-256">SCRAM-SHA-256</SelectItem>
                <SelectItem value="SCRAM-SHA-512">SCRAM-SHA-512</SelectItem>
                <SelectItem value="OAUTHBEARER">OAUTHBEARER</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {isOAuth ? (
            <div className="space-y-2">
              <Label htmlFor="sasl_oauth_token" className="text-sm font-medium text-foreground">
                OAuth Token
              </Label>
              <Input
                id="sasl_oauth_token"
                {...register('sasl_oauth_token')}
                placeholder="OAuth Bearer Token"
                className="font-mono text-sm"
              />
            </div>
          ) : (
            <>
              <div className="space-y-2">
                <Label htmlFor="sasl_username" className="text-sm font-medium text-foreground">
                  SASL 用户名
                </Label>
                  <Input
                    id="sasl_username"
                    {...register('sasl_username')}
                    placeholder={getLL().panels.optional()}
                    className="font-mono text-sm"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="sasl_password" className="text-sm font-medium text-foreground">
                    SASL 密码
                  </Label>
                  <Input
                    id="sasl_password"
                    type="password"
                    {...register('sasl_password')}
                    placeholder={getLL().panels.optional()}
                  className="font-mono text-sm"
                />
              </div>
            </>
          )}
        </div>
      )}

      {/* TLS 配置 */}
      {isSsl && (
        <div className="border rounded-lg overflow-hidden">
          <button
            type="button"
            onClick={() => toggleSection('tls')}
            className="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-muted/50 transition-colors"
          >
            <div className="flex items-center gap-2">
              <Shield className="h-4 w-4 text-orange-600" />
              <span className="text-sm font-medium text-foreground">TLS 配置</span>
            </div>
            <ChevronDown className={cn(
              "h-3.5 w-3.5 text-muted-foreground transition-transform",
              expandedSections.has('tls') && "rotate-180"
            )} />
          </button>
          {expandedSections.has('tls') && (
            <div className="border-t bg-muted/20 px-3 py-3 space-y-3">
              <div className="flex items-center justify-between">
                <div>
                  <Label className="text-xs font-medium text-foreground">{getLL().panels.skipCertVerify()}</Label>
                  <p className="text-[10px] text-muted-foreground">不验证服务器 TLS 证书</p>
                </div>
                <Switch
                  checked={watch('ssl_skip_verify') ?? false}
                  onCheckedChange={(checked) => {
                    setValue('ssl_skip_verify', checked, { shouldDirty: true });
                    saveData();
                  }}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="ssl_ca_cert_path" className="text-xs font-medium text-foreground">CA 证书路径</Label>
                <Input
                  id="ssl_ca_cert_path"
                  {...register('ssl_ca_cert_path')}
                  placeholder="/path/to/ca-cert.pem"
                  className="h-8 text-xs font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="ssl_client_cert_path" className="text-xs font-medium text-foreground">客户端证书路径</Label>
                <Input
                  id="ssl_client_cert_path"
                  {...register('ssl_client_cert_path')}
                  placeholder="/path/to/client-cert.pem"
                  className="h-8 text-xs font-mono"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="ssl_client_key_path" className="text-xs font-medium text-foreground">客户端密钥路径</Label>
                <Input
                  id="ssl_client_key_path"
                  {...register('ssl_client_key_path')}
                  placeholder="/path/to/client-key.pem"
                  className="h-8 text-xs font-mono"
                />
              </div>
            </div>
          )}
        </div>
      )}

      {/* Schema Registry */}
      <div className="space-y-3">
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider flex items-center gap-2">
          <Database className="h-3.5 w-3.5" />
          Schema Registry
        </p>

        <div className="space-y-2">
          <Label htmlFor="schema_registry_url" className="text-sm font-medium text-foreground">
            {getLL().panels.kafkaConnect.schemaRegistry()}
          </Label>
          <Input
            id="schema_registry_url"
            {...register('schema_registry_url')}
            placeholder="http://localhost:8081"
            className="font-mono text-sm"
          />
        </div>

        {schemaRegistryUrl.trim() && (
          <>
            <div className="space-y-2">
              <Label htmlFor="schema_registry_username" className="text-xs font-medium text-foreground">用户名</Label>
                <Input
                    id="schema_registry_username"
                    {...register('schema_registry_username')}
                    placeholder={getLL().panels.optional()}
                    className="h-8 text-xs font-mono"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="schema_registry_password" className="text-xs font-medium text-foreground">密码</Label>
                  <Input
                    id="schema_registry_password"
                    type="password"
                    {...register('schema_registry_password')}
                    placeholder={getLL().panels.optional()}
                className="h-8 text-xs font-mono"
              />
            </div>
          </>
        )}
      </div>

      {/* {getLL().panels.advancedSettings()} */}
      <div className="space-y-3">
        <p className="text-xs font-semibold text-foreground uppercase tracking-wider">{getLL().panels.advancedSettings()}</p>

        <div className="space-y-2">
          <Label htmlFor="connect_timeout_ms" className="text-sm font-medium text-foreground">
            {getLL().panels.kafkaConnect.timeout()}
          </Label>
          <Input
            id="connect_timeout_ms"
            type="number"
            {...register('connect_timeout_ms', { valueAsNumber: true })}
            placeholder="5000"
            className="font-mono text-sm"
          />
        </div>
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">Kafka 连接说明：</p>
          <p>Bootstrap Servers 支持多个地址，用逗号分隔以实现高可用</p>
          <p>SASL 认证仅在安全协议为 SASL_PLAINTEXT 或 SASL_SSL 时可用</p>
          <p>Schema Registry 用于 Avro/Protobuf 序列化格式的消息验证</p>
        </div>
      </div>
    </form>
  );
}
