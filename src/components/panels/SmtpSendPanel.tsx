/**
 * SMTP Send 节点配置组件
 * 支持连接选择、发件人、收件人、主题、正文、附件、自定义头
 */
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useState, useCallback, useEffect, useRef, useMemo } from 'react';
import { smtpSendConfigSchema, type SmtpSendConfigFormData } from './schemas';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Send, Info, ChevronDown, Plus, Trash2 } from 'lucide-react';
import type { SmtpSendConfig as SmtpSendConfigType, EmailAttachment } from '@/types';
import { getLL } from '@/i18n/i18n-lazy';
import { useFlowStore } from '@/stores';
import { isSmtpConnectConfig } from '@/types/node';
import { cn } from '@/lib/utils';

interface SmtpSendConfigProps {
  data: SmtpSendConfigType;
  onSave: (data: SmtpSendConfigType) => void;
}

function toFormData(data: SmtpSendConfigType): SmtpSendConfigFormData {
  return {
    type: 'smtp_send',
    connection_id: data.connection_id || '',
    from: data.from || '',
    to: data.to?.join(', ') || '',
    cc: data.cc?.join(', ') || '',
    bcc: data.bcc?.join(', ') || '',
    subject: data.subject || '',
    body: data.body || '',
    body_type: data.body_type || 'text',
    reply_to: data.reply_to || '',
    attachments_json: data.attachments?.length
      ? JSON.stringify(data.attachments)
      : '',
    headers_json: data.headers && Object.keys(data.headers).length > 0
      ? JSON.stringify(Object.entries(data.headers).map(([k, v]) => [k, v]))
      : '',
  };
}

function fromFormData(formData: SmtpSendConfigFormData): SmtpSendConfigType {
  const toAddresses = formData.to
    ?.split(/[,;]/)
    .map(s => s.trim())
    .filter(Boolean) || [];
  const ccAddresses = formData.cc
    ?.split(/[,;]/)
    .map(s => s.trim())
    .filter(Boolean) || [];
  const bccAddresses = formData.bcc
    ?.split(/[,;]/)
    .map(s => s.trim())
    .filter(Boolean) || [];

  let attachments: EmailAttachment[] | undefined;
  if (formData.attachments_json?.trim()) {
    try {
      attachments = JSON.parse(formData.attachments_json);
    } catch {
      // 忽略解析错误
    }
  }

  let headers: Record<string, string> | undefined;
  if (formData.headers_json?.trim()) {
    try {
      const pairs = JSON.parse(formData.headers_json) as [string, string][];
      if (pairs.length > 0) {
        headers = Object.fromEntries(pairs.filter(([k]) => k.trim()));
      }
    } catch {
      // 忽略解析错误
    }
  }

  return {
    type: 'smtp_send',
    connection_id: formData.connection_id,
    from: formData.from,
    to: toAddresses,
    cc: ccAddresses.length > 0 ? ccAddresses : undefined,
    bcc: bccAddresses.length > 0 ? bccAddresses : undefined,
    subject: formData.subject,
    body: formData.body || '',
    body_type: formData.body_type || 'text',
    reply_to: formData.reply_to || undefined,
    attachments: attachments && attachments.length > 0 ? attachments : undefined,
    headers: headers && Object.keys(headers).length > 0 ? headers : undefined,
  };
}

export function SmtpSendConfig({ data, onSave }: SmtpSendConfigProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    reset,
    getValues,
    formState: { errors, isDirty },
  } = useForm<SmtpSendConfigFormData>({
    resolver: zodResolver(smtpSendConfigSchema),
    defaultValues: toFormData(data),
    mode: 'onBlur',
  });

  const isActuallyDirtyRef = useRef(false);
  useEffect(() => { isActuallyDirtyRef.current = isDirty; }, [isDirty]);

  const flow = useFlowStore((state) => state.flow);

  const availableConnections = useMemo(() => {
    if (!flow) return [];
    const connections: Array<{ uuid: string; name: string }> = [];
    flow.nodes.forEach(node => {
      if (isSmtpConnectConfig(node.data)) {
        connections.push({ uuid: node.uuid, name: node.name });
      }
    });
    return connections;
  }, [flow]);

  useEffect(() => {
    reset(toFormData(data));
  }, [data, reset]);

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

  const saveData = useCallback(() => {
    handleSubmit((formData) => {
      try {
        const config = fromFormData(formData);
        onSave(config);
      } catch (error) {
        console.error('保存 SMTP Send 配置失败:', error);
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

  // 自定义头本地状态
  const [customHeaders, setCustomHeaders] = useState<Array<[string, string]>>(() => {
    if (data.headers && Object.keys(data.headers).length > 0) {
      return Object.entries(data.headers);
    }
    return [];
  });

  useEffect(() => {
    if (data.headers && Object.keys(data.headers).length > 0) {
      setCustomHeaders(Object.entries(data.headers));
    }
  }, [data.headers]);

  const syncHeaders = useCallback((newHeaders: Array<[string, string]>) => {
    setCustomHeaders(newHeaders);
    const filtered = newHeaders.filter(([k]) => k.trim());
    setValue('headers_json', filtered.length > 0 ? JSON.stringify(filtered) : '', { shouldDirty: true });
  }, [setValue]);

  const handleAddHeader = useCallback(() => {
    const newHeaders = [...customHeaders, ['', ''] as [string, string]];
    syncHeaders(newHeaders);
    safeSetTimeout(saveData, 100);
  }, [customHeaders, syncHeaders, safeSetTimeout, saveData]);

  const handleRemoveHeader = useCallback((index: number) => {
    const newHeaders = customHeaders.filter((_, i) => i !== index);
    syncHeaders(newHeaders);
    safeSetTimeout(saveData, 100);
  }, [customHeaders, syncHeaders, safeSetTimeout, saveData]);

  const handleHeaderChange = useCallback((index: number, position: 0 | 1, value: string) => {
    const newHeaders = customHeaders.map((pair, i) => {
      if (i !== index) return pair;
      const updated: [string, string] = [...pair];
      updated[position] = value;
      return updated;
    });
    syncHeaders(newHeaders);
  }, [customHeaders, syncHeaders]);

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

  return (
    <form onBlur={handleBlur} onSubmit={(e) => e.preventDefault()} className="w-full space-y-5">
      {/* 主说明卡片 */}
      <div className="rounded-lg border border-orange-200/50 bg-gradient-to-br from-orange-50/80 to-amber-50/50 p-4 overflow-hidden">
        <div className="space-y-2">
          <h4 className="text-sm font-semibold text-orange-900 flex items-center gap-2">
            <Send className="h-4 w-4" />
            {getLL().panels.smtpSend.title()}
          </h4>
          <p className="text-sm leading-relaxed text-orange-700/90 whitespace-normal break-words">
            {getLL().panels.smtpSend.description()}
          </p>
        </div>
      </div>

      {/* 连接选择 */}
      <div className="space-y-2">
        <Label htmlFor="connection_id" className="text-sm font-medium text-foreground">
          {getLL().panels.connection()} <span className="text-rose-500">*</span>
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
          <p className="text-xs text-rose-500">{errors.connection_id.message}</p>
        )}
        {availableConnections.length === 0 && (
          <p className="text-xs text-muted-foreground">
            {getLL().panels.smtpSend.noConnectNode()}
          </p>
        )}
      </div>

      {/* 发件人 */}
      <div className="space-y-2">
        <Label htmlFor="from" className="text-sm font-medium text-foreground">
          {getLL().panels.smtpSend.from()} <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="from"
          {...register('from')}
          placeholder="sender@example.com"
          className="font-mono text-sm"
        />
        {errors.from && (
          <p className="text-xs text-rose-500">{errors.from.message}</p>
        )}
      </div>

      {/* 收件人 */}
      <div className="space-y-2">
        <Label htmlFor="to" className="text-sm font-medium text-foreground">
          {getLL().panels.smtpSend.to()} <span className="text-rose-500">*</span>
        </Label>
        <Textarea
          id="to"
          {...register('to')}
          placeholder="recipient1@example.com, recipient2@example.com"
          className="min-h-[60px] resize-y font-mono text-xs"
          rows={2}
        />
        <p className="text-xs text-muted-foreground">{getLL().panels.smtpSend.toAddress_desc()}</p>
        {errors.to && (
          <p className="text-xs text-rose-500">{errors.to.message}</p>
        )}
      </div>

      {/* CC / BCC */}
      <div className="border rounded-lg overflow-hidden">
        <button
          type="button"
          onClick={() => toggleSection('recipients')}
          className="w-full flex items-center justify-between px-3 py-2.5 text-left hover:bg-muted/50 transition-colors"
        >
          <div className="flex items-center gap-2">
            <Send className="h-4 w-4 text-orange-600" />
            <span className="text-sm font-medium text-foreground">{getLL().panels.smtpSend.cc()} / {getLL().panels.smtpSend.bcc()} ({getLL().panels.optional()})</span>
          </div>
          <ChevronDown className={cn(
            "h-3.5 w-3.5 text-muted-foreground transition-transform",
            expandedSections.has('recipients') && "rotate-180"
          )} />
        </button>
        {expandedSections.has('recipients') && (
          <div className="border-t bg-muted/20 px-3 py-3 space-y-3">
            <div className="space-y-2">
              <Label htmlFor="cc" className="text-xs font-medium text-foreground">{getLL().panels.smtpSend.cc()}</Label>
              <Input
                id="cc"
                {...register('cc')}
                placeholder="cc1@example.com, cc2@example.com"
                className="h-8 text-xs font-mono"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="bcc" className="text-xs font-medium text-foreground">{getLL().panels.smtpSend.bcc()}</Label>
              <Input
                id="bcc"
                {...register('bcc')}
                placeholder="bcc1@example.com, bcc2@example.com"
                className="h-8 text-xs font-mono"
              />
            </div>
          </div>
        )}
      </div>

      {/* 主题 */}
      <div className="space-y-2">
        <Label htmlFor="subject" className="text-sm font-medium text-foreground">
          {getLL().panels.smtpSend.subject()} <span className="text-rose-500">*</span>
        </Label>
        <Input
          id="subject"
          {...register('subject')}
          placeholder={getLL().panels.emailSubject()}
          className="text-sm"
        />
        {errors.subject && (
          <p className="text-xs text-rose-500">{errors.subject.message}</p>
        )}
      </div>

      {/* 正文类型 */}
      <div className="space-y-2">
        <Label className="text-sm font-medium text-foreground">{getLL().panels.smtpSend.contentType()}</Label>
        <Select
          value={watch('body_type') || 'text'}
          onValueChange={(value) => {
            setValue('body_type', value as 'text' | 'html', { shouldDirty: true });
            saveData();
          }}
        >
          <SelectTrigger className="font-mono text-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="text">{getLL().panels.smtpSend.contentTypeText()}</SelectItem>
            <SelectItem value="html">{getLL().panels.smtpSend.contentTypeHtml()}</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* 正文 */}
      <div className="space-y-2">
        <Label htmlFor="body" className="text-sm font-medium text-foreground">
          {getLL().panels.smtpSend.body()}
        </Label>
        <Textarea
          id="body"
          {...register('body')}
          placeholder={watch('body_type') === 'html' ? '<h1>Hello</h1><p>Email body...</p>' : 'Email body...'}
          className={cn(
            "min-h-[120px] resize-y text-xs",
            watch('body_type') === 'html' ? 'font-mono' : ''
          )}
          rows={6}
        />
      </div>

      {/* 回复地址 */}
      <div className="space-y-2">
        <Label htmlFor="reply_to" className="text-sm font-medium text-foreground">
          {getLL().panels.smtpSend.replyTo_label()}
        </Label>
        <Input
          id="reply_to"
          {...register('reply_to')}
          placeholder="reply@example.com"
          className="font-mono text-sm"
        />
      </div>

      {/* 自定义头 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <Label className="text-sm font-medium text-foreground">{getLL().panels.smtpSend.customHeaders_label()}</Label>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleAddHeader}
            className="h-7 gap-1 text-xs text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />
            {getLL().panels.add()}
          </Button>
        </div>
        <div className="space-y-2">
          {customHeaders.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
              <p className="text-xs text-muted-foreground">{getLL().panels.smtpSend.noCustomHeaders()}</p>
            </div>
          ) : (
            customHeaders.map((pair, index) => (
              <div key={index} className="flex items-center gap-2 p-2 rounded-md border bg-muted/30">
                <Input
                  value={pair[0]}
                  onChange={(e) => handleHeaderChange(index, 0, e.target.value)}
                  onBlur={() => safeSetTimeout(saveData, 100)}
                  placeholder="Header-Name"
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Input
                  value={pair[1]}
                  onChange={(e) => handleHeaderChange(index, 1, e.target.value)}
                  onBlur={() => safeSetTimeout(saveData, 100)}
                  placeholder="Header-Value"
                  className="h-8 flex-1 font-mono text-xs"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  onClick={() => handleRemoveHeader(index)}
                  className="h-8 w-8 shrink-0 text-muted-foreground hover:text-rose-500"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            ))
          )}
        </div>
      </div>

      {/* 提示信息 */}
      <div className="flex items-start gap-2 rounded-md bg-muted/30 p-3">
        <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-sky-500" />
        <div className="text-xs text-muted-foreground space-y-1">
          <p className="font-medium text-foreground">{getLL().panels.smtpSend.senderInfo()}</p>
          <p>{getLL().panels.smtpSend.senderInfo1()}</p>
          <p>{getLL().panels.smtpSend.senderInfo2()}</p>
          <p>{getLL().panels.smtpSend.senderInfo3()}</p>
        </div>
      </div>
    </form>
  );
}
