/**
 * 环境选择器组件 — 画布工具栏中的环境切换 + 管理对话框
 *
 * 功能:
 * - 下拉选择当前激活环境
 * - "管理环境" 按钮打开编辑对话框
 * - 对话框中可添加/编辑/删除环境及其变量
 */
import { useCallback, useState } from 'react';
import { Globe, Plus, Trash2, Pencil, Check, X, Settings2 } from 'lucide-react';
import { useFlowStore } from '@/stores';
import { useUIStore } from '@/stores/ui';
import { cn } from '@/lib/utils';
import type { EnvironmentSet } from '@/types';

/**
 * 环境选择器（嵌入工具栏）
 */
export function EnvironmentSelector() {
  const flow = useFlowStore((s) => s.flow);
  const setActiveEnvironment = useFlowStore((s) => s.setActiveEnvironment);
  const openEnvironmentManagerDialog = useUIStore((s) => s.openEnvironmentManagerDialog);
  const [showDropdown, setShowDropdown] = useState(false);

  const environments = flow?.environments ?? [];
  const activeEnv = flow?.active_environment;

  const handleSelect = useCallback(
    (name: string) => {
      // 点击已激活的环境则取消激活
      setActiveEnvironment(activeEnv === name ? null : name);
      setShowDropdown(false);
    },
    [activeEnv, setActiveEnvironment],
  );

  if (!flow) return null;

  return (
    <>
      <div className="relative">
        <button
          type="button"
          onClick={() => setShowDropdown(!showDropdown)}
          className={cn(
            'flex items-center gap-1.5 px-2 py-1 rounded-md text-xs',
            'transition-all duration-150 cursor-pointer',
            'hover:bg-accent',
            activeEnv ? 'text-violet-600 font-medium' : 'text-muted-foreground',
          )}
          title={activeEnv ? `当前环境: ${activeEnv}` : '选择环境'}
        >
          <Globe className="w-3.5 h-3.5" />
          <span className="max-w-[80px] truncate">
            {activeEnv ?? '环境'}
          </span>
        </button>

        {showDropdown && (
          <>
            {/* 背景遮罩 */}
            <div
              className="fixed inset-0 z-40"
              onClick={() => setShowDropdown(false)}
            />
            <div
              className={cn(
                'absolute top-full left-0 mt-1 z-50',
                'w-48 bg-background/95 backdrop-blur-xl rounded-lg',
                'shadow-xl border border-border/50',
                'py-1',
              )}
            >
              {/* 默认（无环境） */}
              <button
                type="button"
                onClick={() => { setActiveEnvironment(null); setShowDropdown(false); }}
                className={cn(
                  'w-full text-left px-3 py-1.5 text-xs hover:bg-accent transition-colors cursor-pointer',
                  !activeEnv && 'text-violet-600 font-medium',
                )}
              >
                默认变量
              </button>

              {environments.map((env) => (
                <button
                  key={env.name}
                  type="button"
                  onClick={() => handleSelect(env.name)}
                  className={cn(
                    'w-full text-left px-3 py-1.5 text-xs hover:bg-accent transition-colors cursor-pointer',
                    activeEnv === env.name && 'text-violet-600 font-medium',
                  )}
                >
                  {env.name}
                  <span className="ml-1.5 text-muted-foreground">
                    ({Object.keys(env.variables).length})
                  </span>
                </button>
              ))}

              {/* 管理按钮 */}
              <div className="border-t border-border/50 mt-1 pt-1">
                <button
                  type="button"
                  onClick={() => { setShowDropdown(false); openEnvironmentManagerDialog(); }}
                  className="w-full text-left px-3 py-1.5 text-xs text-muted-foreground hover:bg-accent transition-colors cursor-pointer flex items-center gap-1.5"
                >
                  <Settings2 className="w-3 h-3" />
                  管理环境
                </button>
              </div>
            </div>
          </>
        )}
      </div>
    </>
  );
}

/**
 * 环境管理对话框
 */
export function EnvironmentManagerDialog() {
  const flow = useFlowStore((s) => s.flow);
  const addEnvironment = useFlowStore((s) => s.addEnvironment);
  const updateEnvironment = useFlowStore((s) => s.updateEnvironment);
  const removeEnvironment = useFlowStore((s) => s.removeEnvironment);
  const closeEnvironmentManagerDialog = useUIStore((s) => s.closeEnvironmentManagerDialog);

  const environments = flow?.environments ?? [];
  const [editingEnv, setEditingEnv] = useState<string | null>(null);

  const handleAdd = useCallback(() => {
    const name = `环境 ${environments.length + 1}`;
    addEnvironment({ name, variables: {} });
    setEditingEnv(name);
  }, [environments.length, addEnvironment]);

  const handleDelete = useCallback(
    (name: string) => {
      removeEnvironment(name);
      if (editingEnv === name) setEditingEnv(null);
    },
    [removeEnvironment, editingEnv],
  );

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center">
      {/* 遮罩 */}
      <div className="absolute inset-0 bg-black/40" onClick={closeEnvironmentManagerDialog} />

      {/* 对话框 */}
      <div
        className={cn(
          'relative z-10 w-[560px] max-h-[80vh]',
          'bg-background rounded-xl shadow-2xl border border-border',
          'flex flex-col',
        )}
      >
        {/* 标题栏 */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-border">
          <div className="flex items-center gap-2">
            <Globe className="w-4 h-4 text-violet-600" />
            <span className="text-sm font-semibold">环境管理</span>
          </div>
          <button
            type="button"
            onClick={closeEnvironmentManagerDialog}
            className="p-1 rounded-md hover:bg-accent text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* 环境列表 */}
        <div className="flex-1 overflow-y-auto p-4 space-y-3">
          {environments.length === 0 ? (
            <div className="rounded-md border border-dashed border-muted-foreground/30 bg-muted/20 px-3 py-6 text-center">
              <p className="text-xs text-muted-foreground">暂无环境配置</p>
              <p className="mt-1 text-xs text-muted-foreground/60">
                点击下方「添加环境」按钮创建环境变量集
              </p>
            </div>
          ) : (
            environments.map((env) => (
              <EnvironmentCard
                key={env.name}
                env={env}
                isActive={flow?.active_environment === env.name}
                isEditing={editingEnv === env.name}
                onStartEdit={() => setEditingEnv(env.name)}
                onStopEdit={() => setEditingEnv(null)}
                onDelete={() => handleDelete(env.name)}
                onUpdate={(updated) => updateEnvironment(env.name, updated)}
              />
            ))
          )}
        </div>

        {/* 底部操作栏 */}
        <div className="px-4 py-3 border-t border-border">
          <button
            type="button"
            onClick={handleAdd}
            className="w-full h-8 rounded-md border border-dashed border-muted-foreground/30 hover:border-violet-400 text-xs text-muted-foreground hover:text-violet-600 transition-colors flex items-center justify-center gap-1.5 cursor-pointer"
          >
            <Plus className="w-3.5 h-3.5" />
            添加环境
          </button>
        </div>
      </div>
    </div>
  );
}

/**
 * 单个环境卡片
 */
function EnvironmentCard({
  env,
  isActive,
  isEditing,
  onStartEdit,
  onStopEdit,
  onDelete,
  onUpdate,
}: {
  env: EnvironmentSet;
  isActive: boolean;
  isEditing: boolean;
  onStartEdit: () => void;
  onStopEdit: () => void;
  onDelete: () => void;
  onUpdate: (env: EnvironmentSet) => void;
}) {
  const setActiveEnvironment = useFlowStore((s) => s.setActiveEnvironment);
  const [editName, setEditName] = useState(env.name);
  const [newKey, setNewKey] = useState('');
  const [newValue, setNewValue] = useState('');

  const handleSaveName = useCallback(() => {
    const trimmed = editName.trim();
    if (trimmed && trimmed !== env.name) {
      onUpdate({ ...env, name: trimmed });
    } else {
      setEditName(env.name);
    }
    onStopEdit();
  }, [editName, env, onUpdate, onStopEdit]);

  const handleAddVariable = useCallback(() => {
    const key = newKey.trim();
    if (!key) return;
    let value: unknown = newValue;
    try {
      value = JSON.parse(newValue);
    } catch {
      // 保持字符串
    }
    onUpdate({
      ...env,
      variables: { ...env.variables, [key]: value },
    });
    setNewKey('');
    setNewValue('');
  }, [env, newKey, newValue, onUpdate]);

  const handleRemoveVariable = useCallback(
    (key: string) => {
      const { [key]: _, ...rest } = env.variables;
      onUpdate({ ...env, variables: rest });
    },
    [env, onUpdate],
  );

  const handleUpdateVariableValue = useCallback(
    (key: string, rawValue: string) => {
      let value: unknown = rawValue;
      try {
        value = JSON.parse(rawValue);
      } catch {
        // 保持字符串
      }
      onUpdate({
        ...env,
        variables: { ...env.variables, [key]: value },
      });
    },
    [env, onUpdate],
  );

  const varEntries = Object.entries(env.variables);

  return (
    <div
      className={cn(
        'rounded-lg border p-3 space-y-2',
        isActive ? 'border-violet-300 bg-violet-50/50 dark:border-violet-700 dark:bg-violet-950/20' : 'border-border bg-muted/10',
      )}
    >
      {/* 环境名 + 操作 */}
      <div className="flex items-center gap-2">
        {isEditing ? (
          <div className="flex items-center gap-1 flex-1">
            <input
              type="text"
              value={editName}
              onChange={(e) => setEditName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleSaveName();
                if (e.key === 'Escape') { setEditName(env.name); onStopEdit(); }
              }}
              className="h-7 flex-1 rounded border border-violet-300 bg-background px-2 text-xs font-medium focus:outline-none focus:ring-1 focus:ring-violet-400"
              autoFocus
            />
            <button
              type="button"
              onClick={handleSaveName}
              className="p-1 rounded hover:bg-accent text-green-600 cursor-pointer"
            >
              <Check className="w-3.5 h-3.5" />
            </button>
          </div>
        ) : (
          <>
            <span className="flex-1 text-xs font-medium truncate">
              {env.name}
            </span>
            {isActive && (
              <span className="text-[10px] text-violet-600 bg-violet-100 dark:bg-violet-900/40 px-1.5 py-0.5 rounded-full">
                激活
              </span>
            )}
            {!isActive && (
              <button
                type="button"
                onClick={() => setActiveEnvironment(env.name)}
                className="text-[10px] text-muted-foreground hover:text-violet-600 transition-colors cursor-pointer px-1"
                title="激活此环境"
              >
                激活
              </button>
            )}
            <button
              type="button"
              onClick={onStartEdit}
              className="p-1 rounded hover:bg-accent text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
              title="重命名"
            >
              <Pencil className="w-3 h-3" />
            </button>
            <button
              type="button"
              onClick={onDelete}
              className="p-1 rounded hover:bg-accent text-muted-foreground hover:text-rose-500 transition-colors cursor-pointer"
              title="删除环境"
            >
              <Trash2 className="w-3 h-3" />
            </button>
          </>
        )}
      </div>

      {/* 变量列表 */}
      <div className="space-y-1">
        {varEntries.map(([key, val]) => (
          <div key={key} className="flex items-center gap-1.5 group">
            <span className="text-[11px] font-mono text-muted-foreground w-24 truncate shrink-0" title={key}>
              {key}
            </span>
            <input
              type="text"
              value={typeof val === 'string' ? val : JSON.stringify(val)}
              onChange={(e) => handleUpdateVariableValue(key, e.target.value)}
              className="h-6 flex-1 rounded border border-transparent bg-transparent px-1.5 text-[11px] font-mono hover:border-border focus:border-violet-300 focus:outline-none transition-colors"
            />
            <button
              type="button"
              onClick={() => handleRemoveVariable(key)}
              className="p-0.5 rounded opacity-0 group-hover:opacity-100 text-muted-foreground hover:text-rose-500 transition-all cursor-pointer"
            >
              <X className="w-3 h-3" />
            </button>
          </div>
        ))}

        {/* 添加变量 */}
        <div className="flex items-center gap-1.5 pt-1">
          <input
            type="text"
            value={newKey}
            onChange={(e) => setNewKey(e.target.value)}
            placeholder="键名"
            className="h-6 w-24 rounded border border-dashed border-muted-foreground/30 bg-transparent px-1.5 text-[11px] font-mono focus:border-violet-300 focus:outline-none placeholder:text-muted-foreground/40"
            onKeyDown={(e) => {
              if (e.key === 'Enter' && newKey.trim()) handleAddVariable();
            }}
          />
          <input
            type="text"
            value={newValue}
            onChange={(e) => setNewValue(e.target.value)}
            placeholder="值"
            className="h-6 flex-1 rounded border border-dashed border-muted-foreground/30 bg-transparent px-1.5 text-[11px] font-mono focus:border-violet-300 focus:outline-none placeholder:text-muted-foreground/40"
            onKeyDown={(e) => {
              if (e.key === 'Enter' && newKey.trim()) handleAddVariable();
            }}
          />
          <button
            type="button"
            onClick={handleAddVariable}
            disabled={!newKey.trim()}
            className={cn(
              'p-0.5 rounded transition-colors cursor-pointer',
              newKey.trim()
                ? 'text-violet-600 hover:text-violet-700'
                : 'text-muted-foreground/30 cursor-not-allowed',
            )}
          >
            <Plus className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </div>
  );
}
