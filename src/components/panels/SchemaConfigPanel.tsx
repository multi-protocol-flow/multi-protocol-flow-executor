/**
 * SchemaConfigPanel 通用配置面板组件
 * 从 JSON Schema 动态渲染表单，支持 string、integer、boolean、enum 字段类型
 */
import { useState, useCallback, useEffect, useRef } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

interface SchemaConfigPanelProps {
  /** JSON Schema 对象，描述配置结构 */
  schema: Record<string, unknown>;
  /** 当前配置数据 */
  data: Record<string, unknown>;
  /** 配置变更回调 */
  onSave: (data: Record<string, unknown>) => void;
}

/** JSON Schema 中单个属性的定义 */
interface SchemaProperty {
  type?: string;
  title?: string;
  description?: string;
  enum?: string[];
  default?: unknown;
  minimum?: number;
  maximum?: number;
}

/**
 * 获取字段的当前值，优先使用 data 中的值，其次使用 schema 中的 default
 */
function getFieldValue(key: string, data: Record<string, unknown>, prop: SchemaProperty): unknown {
  if (key in data && data[key] !== undefined) {
    return data[key];
  }
  return prop.default;
}

export function SchemaConfigPanel({ schema, data, onSave }: SchemaConfigPanelProps) {
  const properties = (schema.properties || {}) as Record<string, SchemaProperty>;
  const requiredFields = new Set(
    Array.isArray(schema.required) ? (schema.required as string[]) : []
  );
  const propertyEntries = Object.entries(properties);

  // 用受控状态跟踪表单值
  const [formValues, setFormValues] = useState<Record<string, unknown>>(() => {
    const initial: Record<string, unknown> = {};
    for (const [key, prop] of propertyEntries) {
      initial[key] = getFieldValue(key, data, prop);
    }
    return initial;
  });

  // 追踪待执行的 setTimeout，组件卸载时清除
  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  useEffect(() => {
    return () => {
      pendingTimersRef.current.forEach((id) => clearTimeout(id));
      pendingTimersRef.current.clear();
    };
  }, []);

  // 当外部 data 变化时（例如切换节点），同步 formValues
  useEffect(() => {
    const updated: Record<string, unknown> = {};
    for (const [key, prop] of propertyEntries) {
      updated[key] = getFieldValue(key, data, prop);
    }
    setFormValues(updated);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data]);

  /**
   * 安全的延迟执行包装器
   */
  const safeSetTimeout = useCallback((fn: () => void, ms: number) => {
    const id = setTimeout(() => {
      pendingTimersRef.current.delete(id);
      fn();
    }, ms);
    pendingTimersRef.current.add(id);
  }, []);

  /**
   * 更新字段值并延迟触发 onSave
   */
  const handleFieldChange = useCallback((key: string, value: unknown) => {
    setFormValues((prev) => {
      const next = { ...prev, [key]: value };
      // 延迟保存，合并多次快速变更
      safeSetTimeout(() => {
        onSave(next);
      }, 150);
      return next;
    });
  }, [onSave, safeSetTimeout]);

  // 监听 Ctrl+S 刷新事件
  useEffect(() => {
    const handleFlush = () => { onSave(formValues); };
    window.addEventListener('flush-config-panel', handleFlush);
    return () => { window.removeEventListener('flush-config-panel', handleFlush); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [formValues, onSave]);

  /**
   * 渲染单个字段的标签
   */
  const renderLabel = (key: string, prop: SchemaProperty) => (
    <Label htmlFor={`schema-field-${key}`} className="text-sm font-medium text-foreground">
      {prop.title || key}
      {requiredFields.has(key) && <span className="text-rose-500 ml-0.5">*</span>}
    </Label>
  );

  /**
   * 根据 schema 属性类型渲染对应的输入组件
   */
  const renderField = (key: string, prop: SchemaProperty) => {
    const value = formValues[key];
    const isEnum = Array.isArray(prop.enum) && prop.enum.length > 0;

    // string + enum → Select 下拉框
    if (prop.type === 'string' && isEnum) {
      return (
        <div key={key} className="space-y-2">
          {renderLabel(key, prop)}
          <Select
            value={(value as string) ?? ''}
            onValueChange={(v) => handleFieldChange(key, v)}
          >
            <SelectTrigger id={`schema-field-${key}`}>
              <SelectValue placeholder={`选择${prop.title || key}`} />
            </SelectTrigger>
            <SelectContent>
              {prop.enum!.map((opt) => (
                <SelectItem key={opt} value={opt}>
                  {opt}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          {prop.description && (
            <p className="text-xs text-muted-foreground">{prop.description}</p>
          )}
        </div>
      );
    }

    // string → 文本输入
    if (prop.type === 'string') {
      return (
        <div key={key} className="space-y-2">
          {renderLabel(key, prop)}
          <Input
            id={`schema-field-${key}`}
            type="text"
            value={(value as string) ?? ''}
            onChange={(e) => handleFieldChange(key, e.target.value)}
            placeholder={prop.description || ''}
          />
          {prop.description && (
            <p className="text-xs text-muted-foreground">{prop.description}</p>
          )}
        </div>
      );
    }

    // integer → 数字输入
    if (prop.type === 'integer' || prop.type === 'number') {
      return (
        <div key={key} className="space-y-2">
          {renderLabel(key, prop)}
          <Input
            id={`schema-field-${key}`}
            type="number"
            value={value !== undefined && value !== null ? String(value) : ''}
            onChange={(e) => {
              const raw = e.target.value;
              handleFieldChange(key, raw === '' ? undefined : Number(raw));
            }}
            min={prop.minimum}
            max={prop.maximum}
            className="font-mono"
          />
          {prop.description && (
            <p className="text-xs text-muted-foreground">
              {prop.description}
              {prop.minimum !== undefined && prop.maximum !== undefined && (
                <span> ({prop.minimum}-{prop.maximum})</span>
              )}
            </p>
          )}
        </div>
      );
    }

    // boolean → 开关
    if (prop.type === 'boolean') {
      return (
        <div key={key} className="space-y-2">
          <div className="flex items-center justify-between">
            {renderLabel(key, prop)}
            <Switch
              checked={value === true}
              onCheckedChange={(checked) => handleFieldChange(key, checked)}
            />
          </div>
          {prop.description && (
            <p className="text-xs text-muted-foreground">{prop.description}</p>
          )}
        </div>
      );
    }

    // 未知类型 → 回退为文本输入
    return (
      <div key={key} className="space-y-2">
        {renderLabel(key, prop)}
        <Input
          id={`schema-field-${key}`}
          type="text"
          value={value !== undefined && value !== null ? String(value) : ''}
          onChange={(e) => handleFieldChange(key, e.target.value)}
        />
        {prop.description && (
          <p className="text-xs text-muted-foreground">{prop.description}</p>
        )}
      </div>
    );
  };

  if (propertyEntries.length === 0) {
    return (
      <div className="w-full rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-4 text-center">
        <p className="text-xs text-muted-foreground">无可配置字段</p>
      </div>
    );
  }

  return (
    <div className="w-full space-y-4">
      {propertyEntries.map(([key, prop]) => renderField(key, prop))}
    </div>
  );
}
