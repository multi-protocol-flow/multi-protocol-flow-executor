/**
 * SchemaConfigPanel 通用配置面板组件
 * 从 JSON Schema 动态渲染表单，支持 string、integer、boolean、enum 字段类型
 */
import { useState, useCallback, useEffect, useRef } from 'react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { useFlowStore, selectNodes } from '@/stores/flow';
import { getLL } from '@/i18n/i18n-lazy';

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
  /** 自定义 keyword：声明该字段从当前流程中筛选指定类型的节点生成下拉框 */
  'x-node-selector'?: { node_type?: string };
  /** JSON Schema format：'json' 多行 JSON 输入；'sql' 多行 SQL 输入；'password' 掩码输入 */
  format?: string;
}

/**
 * 校验文本是否为合法 JSON（空文本视为合法）
 */
function isValidJson(text: string): boolean {
  try {
    JSON.parse(text);
    return true;
  } catch {
    return false;
  }
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
  const ll = getLL().panels.schemaConfig;
  const properties = (schema.properties || {}) as Record<string, SchemaProperty>;
  const requiredFields = new Set(
    Array.isArray(schema.required) ? (schema.required as string[]) : []
  );
  const propertyEntries = Object.entries(properties);

  // 获取当前流程中的节点列表（用于 x-node-selector 字段筛选）
  const nodes = useFlowStore(selectNodes);

  // 用受控状态跟踪表单值
  const [formValues, setFormValues] = useState<Record<string, unknown>>(() => {
    const initial: Record<string, unknown> = {};
    for (const [key, prop] of propertyEntries) {
      initial[key] = getFieldValue(key, data, prop);
    }
    return initial;
  });

  // 追踪用户是否实际修改过字段（用于屏蔽切换节点时残留的 debounce 保存）
  const isActuallyDirtyRef = useRef(false);

  // 追踪待执行的 setTimeout，组件卸载时清除
  const pendingTimersRef = useRef<Set<ReturnType<typeof setTimeout>>>(new Set());

  useEffect(() => {
    return () => {
      pendingTimersRef.current.forEach((id) => clearTimeout(id));
      pendingTimersRef.current.clear();
    };
  }, []);

  // 当外部 data 变化时（例如切换节点）：
  // 1. 同步 formValues 到新节点的数据
  // 2. 清除待执行的延迟保存定时器，防止旧节点数据写入新节点
  // 3. 重置 dirty 标记
  useEffect(() => {
    // 清除所有待执行的延迟保存
    pendingTimersRef.current.forEach((id) => clearTimeout(id));
    pendingTimersRef.current.clear();
    isActuallyDirtyRef.current = false;

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
   * 标记 dirty 状态，debounce 回调中检查，防止节点切换后残留调用写入错误节点
   */
  const handleFieldChange = useCallback((key: string, value: unknown) => {
    isActuallyDirtyRef.current = true;
    setFormValues((prev) => {
      const next = { ...prev, [key]: value };
      // 延迟保存，合并多次快速变更
      safeSetTimeout(() => {
        if (!isActuallyDirtyRef.current) return;
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
              <SelectValue placeholder={ll.selectField({ field: prop.title || key })} />
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

    // string + x-node-selector → 节点选择下拉框（从当前流程中筛选指定类型的节点）
    if (prop.type === 'string' && prop['x-node-selector']?.node_type) {
      const targetType = prop['x-node-selector'].node_type;
      const matchedNodes = nodes.filter(
        (n) => (n.data as { type?: string }).type === targetType
      );
      return (
        <div key={key} className="space-y-2">
          {renderLabel(key, prop)}
          <Select
            value={(value as string) ?? ''}
            onValueChange={(v) => handleFieldChange(key, v)}
          >
            <SelectTrigger id={`schema-field-${key}`} className="font-mono">
              <SelectValue placeholder={ll.selectField({ field: prop.title || key })} />
            </SelectTrigger>
            <SelectContent>
              {!requiredFields.has(key) && (
                <SelectItem key="__empty__" value="">
                  {ll.clearSelection()}
                </SelectItem>
              )}
              {matchedNodes.length === 0 ? (
                <div className="px-2 py-4 text-center text-sm text-muted-foreground">
                  {ll.noConnectionNodes()}
                </div>
              ) : (
                matchedNodes.map((node) => (
                  <SelectItem key={node.uuid} value={node.uuid}>
                    {node.name}
                  </SelectItem>
                ))
              )}
            </SelectContent>
          </Select>
          {matchedNodes.length === 0 && (
            <p className="text-xs text-amber-600">{ll.addNodeFirst()}</p>
          )}
          {prop.description && (
            <p className="text-xs text-muted-foreground">{prop.description}</p>
          )}
        </div>
      );
    }

    // string + format: password → 掩码输入（值不落明文 UI）
    if (prop.type === 'string' && prop.format === 'password') {
      return (
        <div key={key} className="space-y-2">
          {renderLabel(key, prop)}
          <Input
            id={`schema-field-${key}`}
            type="password"
            value={(value as string) ?? ''}
            onChange={(e) => handleFieldChange(key, e.target.value)}
            placeholder={prop.description || ''}
            className="font-mono"
            autoComplete="off"
          />
          {prop.description && (
            <p className="text-xs text-muted-foreground">{prop.description}</p>
          )}
        </div>
      );
    }

    // string + format: sql → 多行 SQL 输入（等宽字体 textarea）
    if (prop.type === 'string' && prop.format === 'sql') {
      return (
        <div key={key} className="space-y-2">
          {renderLabel(key, prop)}
          <Textarea
            id={`schema-field-${key}`}
            value={(value as string) ?? ''}
            onChange={(e) => handleFieldChange(key, e.target.value)}
            placeholder={prop.description || ''}
            className="min-h-[120px] resize-y font-mono text-xs"
            rows={6}
          />
          {prop.description && (
            <p className="text-xs text-muted-foreground">{prop.description}</p>
          )}
        </div>
      );
    }

    // string + format: json → 多行 JSON 输入（非法 JSON 显示错误但不阻止保存）
    if (prop.type === 'string' && prop.format === 'json') {
      const rawValue =
        typeof value === 'string'
          ? value
          : value === undefined || value === null
            ? ''
            : JSON.stringify(value);
      const jsonInvalid = rawValue.trim() !== '' && !isValidJson(rawValue);
      return (
        <div key={key} className="space-y-2">
          {renderLabel(key, prop)}
          <Textarea
            id={`schema-field-${key}`}
            value={rawValue}
            onChange={(e) => handleFieldChange(key, e.target.value)}
            placeholder={ll.jsonPlaceholder()}
            className={`min-h-[120px] resize-y font-mono text-xs${jsonInvalid ? ' border-rose-500 focus-visible:border-rose-500' : ''}`}
            rows={6}
          />
          {jsonInvalid && (
            <p className="text-xs text-rose-500">{ll.jsonInvalid()}</p>
          )}
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

    // array → JSON 数组编辑（合法 JSON 数组保存为数组；否则保留原文并标记错误）
    if (prop.type === 'array') {
      const rawValue = Array.isArray(value)
        ? JSON.stringify(value)
        : typeof value === 'string'
          ? value
          : value === undefined || value === null
            ? ''
            : String(value);
      const trimmed = rawValue.trim();
      let invalid = false;
      if (trimmed !== '') {
        try {
          const parsed = JSON.parse(rawValue);
          if (!Array.isArray(parsed)) invalid = true;
        } catch {
          invalid = true;
        }
      }
      return (
        <div key={key} className="space-y-2">
          {renderLabel(key, prop)}
          <Textarea
            id={`schema-field-${key}`}
            value={rawValue}
            onChange={(e) => {
              const raw = e.target.value;
              if (raw.trim() === '') {
                handleFieldChange(key, []);
                return;
              }
              try {
                const parsed = JSON.parse(raw);
                if (Array.isArray(parsed)) {
                  handleFieldChange(key, parsed);
                  return;
                }
              } catch {
                // fallthrough: keep raw text until valid
              }
              handleFieldChange(key, raw);
            }}
            placeholder={ll.jsonPlaceholder()}
            className={`min-h-[80px] resize-y font-mono text-xs${invalid ? ' border-rose-500 focus-visible:border-rose-500' : ''}`}
            rows={4}
          />
          {invalid && (
            <p className="text-xs text-rose-500">{ll.jsonInvalid()}</p>
          )}
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
        <p className="text-xs text-muted-foreground">{ll.noFields()}</p>
      </div>
    );
  }

  return (
    <div className="w-full space-y-4">
      {propertyEntries.map(([key, prop]) => renderField(key, prop))}
    </div>
  );
}
