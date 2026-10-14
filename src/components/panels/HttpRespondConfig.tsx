/**
 * HttpRespondConfig - HTTP Respond 响应节点配置面板
 * 配置返回给外部客户端的 HTTP 状态码、响应头与响应体内容
 */
import { useState, useEffect, useCallback, useRef } from 'react';
import { Send, Plus, Trash2, Code, FileText } from 'lucide-react';
import type { HttpRespondConfig as HttpRespondConfigType } from '@/types';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';

interface HttpRespondConfigProps {
  data: HttpRespondConfigType;
  onSave: (data: HttpRespondConfigType) => void;
  disabled?: boolean;
}

const STATUS_PRESETS = [200, 201, 204, 400, 401, 404, 500];

export function HttpRespondConfig({ data, onSave, disabled = false }: HttpRespondConfigProps) {
  const ll = getLL();

  const [statusCode, setStatusCode] = useState<number>(data.status_code ?? 200);
  const [bodyType, setBodyType] = useState<string>(data.body_type ?? 'json');
  const [body, setBody] = useState<string>(data.body ?? '{"success": true}');
  const [headers, setHeaders] = useState<Array<{ key: string; value: string }>>(() => {
    return Object.entries(data.headers ?? {}).map(([key, value]) => ({ key, value }));
  });

  const onSaveRef = useRef(onSave);
  useEffect(() => {
    onSaveRef.current = onSave;
  }, [onSave]);

  const syncChanges = useCallback(
    (newStatus: number, newType: string, newBody: string, newHeaders: Array<{ key: string; value: string }>) => {
      const headersMap: Record<string, string> = {};
      for (const h of newHeaders) {
        if (h.key.trim()) {
          headersMap[h.key.trim()] = h.value;
        }
      }

      onSaveRef.current({
        type: 'http:respond',
        status_code: newStatus,
        body_type: newType,
        body: newBody,
        headers: headersMap,
      });
    },
    []
  );

  const handleStatusChange = (val: number) => {
    setStatusCode(val);
    syncChanges(val, bodyType, body, headers);
  };

  const handleBodyTypeChange = (type: string) => {
    setBodyType(type);
    syncChanges(statusCode, type, body, headers);
  };

  const handleBodyChange = (newBody: string) => {
    setBody(newBody);
    syncChanges(statusCode, bodyType, newBody, headers);
  };

  const handleHeaderChange = (idx: number, field: 'key' | 'value', val: string) => {
    const next = headers.map((h, i) => (i === idx ? { ...h, [field]: val } : h));
    setHeaders(next);
    syncChanges(statusCode, bodyType, body, next);
  };

  const addHeader = () => {
    const next = [...headers, { key: '', value: '' }];
    setHeaders(next);
    syncChanges(statusCode, bodyType, body, next);
  };

  const removeHeader = (idx: number) => {
    const next = headers.filter((_, i) => i !== idx);
    setHeaders(next);
    syncChanges(statusCode, bodyType, body, next);
  };

  return (
    <div className="w-full space-y-4 text-xs">
      {/* 状态码配置 */}
      <div className="space-y-2">
        <label className="flex items-center gap-1.5 font-medium text-foreground">
          <Send className="w-3.5 h-3.5 text-teal-500" />
          <span>{ll.panels.httpRespond.statusLabel()}</span>
        </label>
        <div className="flex items-center gap-2">
          <input
            type="number"
            min={100}
            max={599}
            value={statusCode}
            disabled={disabled}
            onChange={(e) => handleStatusChange(parseInt(e.target.value, 10) || 200)}
            className="w-24 h-8 px-2.5 rounded border border-border bg-background font-mono focus:outline-none focus:ring-1 focus:ring-primary disabled:opacity-50"
            placeholder="200"
          />

          {/* 预设状态码快捷选择 */}
          <div className="flex flex-wrap gap-1">
            {STATUS_PRESETS.map((code) => (
              <button
                key={code}
                type="button"
                disabled={disabled}
                onClick={() => handleStatusChange(code)}
                className={cn(
                  'px-2 py-1 rounded border text-[11px] font-mono font-medium transition-colors',
                  statusCode === code
                    ? 'border-teal-500 bg-teal-500/10 text-teal-700 dark:text-teal-300'
                    : 'border-border bg-muted/40 text-muted-foreground hover:text-foreground'
                )}
              >
                {code}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* 响应头配置 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <label className="font-medium text-foreground">{ll.panels.httpRespond.headersLabel()}</label>
          <button
            type="button"
            disabled={disabled}
            onClick={addHeader}
            className="text-[11px] text-primary hover:underline flex items-center gap-1 disabled:opacity-50"
          >
            <Plus className="w-3 h-3" />
            <span>{ll.panels.httpRespond.addHeader()}</span>
          </button>
        </div>

        {headers.length === 0 ? (
          <div className="text-muted-foreground/60 text-xs py-2">
            {ll.panels.httpRespond.headersEmpty()}
          </div>
        ) : (
          <div className="space-y-1.5">
            {headers.map((h, idx) => (
              <div key={idx} className="flex items-center gap-2">
                <input
                  type="text"
                  value={h.key}
                  disabled={disabled}
                  onChange={(e) => handleHeaderChange(idx, 'key', e.target.value)}
                  placeholder={ll.panels.httpRespond.headerNamePlaceholder()}
                  className="flex-1 h-7 px-2 text-xs font-mono rounded border border-border bg-background focus:outline-none focus:ring-1 focus:ring-primary"
                />
                <input
                  type="text"
                  value={h.value}
                  disabled={disabled}
                  onChange={(e) => handleHeaderChange(idx, 'value', e.target.value)}
                  placeholder={ll.panels.httpRespond.headerValuePlaceholder({ template: '{{var}}' })}
                  className="flex-1 h-7 px-2 text-xs font-mono rounded border border-border bg-background focus:outline-none focus:ring-1 focus:ring-primary"
                />
                <button
                  type="button"
                  disabled={disabled}
                  onClick={() => removeHeader(idx)}
                  className="h-7 w-7 flex items-center justify-center text-muted-foreground hover:text-destructive rounded border border-border hover:bg-muted"
                >
                  <Trash2 className="w-3 h-3" />
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* 响应体类型与内容 */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <label className="font-medium text-foreground">{ll.panels.httpRespond.bodyLabel()}</label>
          <div className="flex rounded border border-border bg-muted/50 p-0.5 shrink-0">
            <button
              type="button"
              onClick={() => handleBodyTypeChange('json')}
              className={cn(
                'px-2 py-0.5 text-[11px] rounded font-medium transition-colors flex items-center gap-1',
                bodyType === 'json'
                  ? 'bg-background shadow-xs text-foreground'
                  : 'text-muted-foreground hover:text-foreground'
              )}
            >
              <Code className="w-3 h-3" />
              <span>JSON</span>
            </button>
            <button
              type="button"
              onClick={() => handleBodyTypeChange('text')}
              className={cn(
                'px-2 py-0.5 text-[11px] rounded font-medium transition-colors flex items-center gap-1',
                bodyType === 'text'
                  ? 'bg-background shadow-xs text-foreground'
                  : 'text-muted-foreground hover:text-foreground'
              )}
            >
              <FileText className="w-3 h-3" />
              <span>Text</span>
            </button>
          </div>
        </div>

        <textarea
          value={body}
          disabled={disabled}
          onChange={(e) => handleBodyChange(e.target.value)}
          rows={6}
          placeholder='{"message": "success", "order_id": "{{body.order_id}}"}'
          className="w-full px-2.5 py-2 text-xs font-mono rounded border border-border bg-background focus:outline-none focus:ring-1 focus:ring-primary disabled:opacity-50"
        />
        <p className="text-[11px] text-muted-foreground">
          {ll.panels.httpRespond.bodyVarHintBefore()}
          <code>{"{{variable_name}}"}</code>
          {ll.panels.httpRespond.bodyVarHintAfter()}
        </p>
      </div>
    </div>
  );
}
