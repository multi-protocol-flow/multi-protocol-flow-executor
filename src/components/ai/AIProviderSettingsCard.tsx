/**
 * AI Provider 设置卡片 - 响应式优化版
 */

import { useState } from 'react';
import {
  Sparkles,
  Plus,
  Trash2,
  Edit2,
  Check,
  Eye,
  EyeOff,
  Activity,
  Loader2,
  RefreshCw,
  Server,
  Zap,
  ChevronDown,
  ChevronUp,
  ShieldCheck,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Switch } from '@/components/ui/switch';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import { useSettingsStore } from '@/stores/settings';
import { testConnection, fetchModels } from '@/services/ai/openaiClient';
import { useAIStore } from '@/stores/ai';
import { getLL } from '@/i18n/i18n-lazy';
import type { AIProviderConfig } from '@/types/ai';
const PRESETS: Array<{
  name: string;
  baseUrl: string;
  model: string;
  description: string;
}> = [
  {
    name: 'DeepSeek Official',
    baseUrl: 'https://api.deepseek.com/v1',
    model: 'deepseek-chat',
    description: 'DeepSeek V3 高性能模型',
  },
  {
    name: 'DeepSeek Reasoner',
    baseUrl: 'https://api.deepseek.com/v1',
    model: 'deepseek-reasoner',
    description: 'DeepSeek R1 深度思考推理模型',
  },
  {
    name: 'Local Ollama',
    baseUrl: 'http://localhost:11434/v1',
    model: 'qwen2.5:latest',
    description: '本地 Ollama 实例 (无需 API Key)',
  },
  {
    name: 'OpenAI Official',
    baseUrl: 'https://api.openai.com/v1',
    model: 'gpt-4o',
    description: 'OpenAI GPT-4o 旗舰模型',
  },
];

export function AIProviderSettingsCard({ className }: { className?: string } = {}) {
  const ll = getLL();
  const settings = useSettingsStore();
  const { alwaysTrustHighRisk, setAlwaysTrustHighRisk } = useAIStore();

  const [editingProvider, setEditingProvider] = useState<AIProviderConfig | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  const [showApiKey, setShowApiKey] = useState(false);
  const [showSystemPrompt, setShowSystemPrompt] = useState(false);

  // 测试连接状态
  const [testingId, setTestingId] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<{
    id: string;
    success: boolean;
    latencyMs?: number;
    error?: string;
  } | null>(null);

  // 抓取模型列表状态
  const [fetchingModels, setFetchingModels] = useState(false);
  const [availableModels, setAvailableModels] = useState<string[]>([]);

  const handleCancel = () => {
    setEditingProvider(null);
    setIsCreating(false);
    setTestResult(null);
    setAvailableModels([]);
    setShowApiKey(false);
  };

  const handleStartCreate = (presetIndex?: number) => {
    const preset = presetIndex !== undefined ? PRESETS[presetIndex] : null;
    const newConfig: AIProviderConfig = {
      id: crypto.randomUUID(),
      name: preset ? preset.name : 'Custom OpenAI Compatible',
      baseUrl: preset ? preset.baseUrl : 'https://api.openai.com/v1',
      apiKey: '',
      model: preset ? preset.model : 'gpt-4o',
      temperature: 0.7,
    };
    setEditingProvider(newConfig);
    setIsCreating(true);
    setTestResult(null);
    setAvailableModels([]);
    setShowApiKey(false);
  };

  const handleToggleEdit = (provider: AIProviderConfig) => {
    if (!isCreating && editingProvider?.id === provider.id) {
      handleCancel();
    } else {
      setEditingProvider({ ...provider });
      setIsCreating(false);
      setTestResult(null);
      setAvailableModels([]);
      setShowApiKey(false);
    }
  };

  const handleSaveProvider = async () => {
    if (!editingProvider) return;
    if (!editingProvider.name.trim() || !editingProvider.baseUrl.trim() || !editingProvider.model.trim()) {
      return;
    }

    const currentProviders = [...settings.aiProviders];
    const index = currentProviders.findIndex((p) => p.id === editingProvider.id);

    if (index >= 0) {
      currentProviders[index] = editingProvider;
    } else {
      currentProviders.push(editingProvider);
    }

    await settings.setAiProviders(currentProviders);

    // 如果还没有激活的 Provider，自动设为激活
    if (!settings.activeAiProviderId || settings.activeAiProviderId === editingProvider.id) {
      await settings.setActiveAiProviderId(editingProvider.id);
    }

    handleCancel();
  };

  const handleDeleteProvider = async (id: string) => {
    if (editingProvider?.id === id) {
      handleCancel();
    }
    const currentProviders = settings.aiProviders.filter((p) => p.id !== id);
    await settings.setAiProviders(currentProviders);

    if (settings.activeAiProviderId === id) {
      const nextId = currentProviders.length > 0 ? currentProviders[0].id : null;
      await settings.setActiveAiProviderId(nextId);
    }
  };

  const handleTestConnection = async (config: AIProviderConfig) => {
    setTestingId(config.id);
    setTestResult(null);
    try {
      const res = await testConnection(config);
      setTestResult({
        id: config.id,
        success: res.success,
        latencyMs: res.latencyMs,
        error: res.error,
      });
    } finally {
      setTestingId(null);
    }
  };

  const handleFetchModels = async () => {
    if (!editingProvider) return;
    setFetchingModels(true);
    try {
      const models = await fetchModels(editingProvider);
      setAvailableModels(models);
    } finally {
      setFetchingModels(false);
    }
  };

  const renderProviderForm = (isNew: boolean) => {
    if (!editingProvider) return null;

    return (
      <div
        className={cn(
          'space-y-3.5',
          isNew
            ? 'rounded-lg border border-primary/40 bg-muted/20 p-3.5 shadow-xs'
            : 'border-t border-border/50 bg-muted/20 p-3.5'
        )}
      >
        <div className="flex items-center justify-between pb-2 border-b border-border/50">
          <span className="font-semibold text-xs text-foreground">
            {isNew ? ll.ai.settings.form_create_title() : ll.ai.settings.form_edit_title()}
          </span>
          <div className="flex items-center gap-1.5">
            <Button
              size="sm"
              variant="ghost"
              className="h-6 px-2 text-xs"
              onClick={handleCancel}
            >
              {ll.ai.settings.cancel()}
            </Button>
            <Button
              size="sm"
              className="h-6 px-2.5 text-xs bg-primary text-primary-foreground hover:bg-primary/90 hover:text-primary-foreground"
              onClick={handleSaveProvider}
            >
              {ll.ai.settings.save()}
            </Button>
          </div>
        </div>

        {/* 预设快速填充 */}
        {isNew && (
          <div className="space-y-1">
            <span className="text-[11px] text-muted-foreground">{ll.ai.settings.quick_fill_presets()}</span>
            <div className="flex flex-wrap gap-1">
              {PRESETS.map((p) => (
                <button
                  key={p.name}
                  type="button"
                  className="text-[10px] px-1.5 py-0.5 rounded bg-muted hover:bg-primary/20 text-muted-foreground hover:text-primary transition-colors border border-border/50"
                  onClick={() => {
                    setEditingProvider({
                      ...editingProvider,
                      name: p.name,
                      baseUrl: p.baseUrl,
                      model: p.model,
                    });
                  }}
                >
                  {p.name}
                </button>
              ))}
            </div>
          </div>
        )}

        <div className="space-y-2.5">
          {/* 名称 */}
          <div className="space-y-1">
            <Label className="text-xs font-medium text-foreground">{ll.ai.settings.name_label()}</Label>
            <Input
              value={editingProvider.name}
              onChange={(e) => setEditingProvider({ ...editingProvider, name: e.target.value })}
              placeholder="DeepSeek / OpenAI"
              className="h-8 text-xs w-full"
            />
          </div>

          {/* Base URL */}
          <div className="space-y-1">
            <Label className="text-xs font-medium text-foreground">{ll.ai.settings.base_url_label()}</Label>
            <Input
              value={editingProvider.baseUrl}
              onChange={(e) => setEditingProvider({ ...editingProvider, baseUrl: e.target.value })}
              placeholder="https://api.deepseek.com/v1"
              className="h-8 text-xs font-mono w-full"
            />
          </div>

          {/* API Key */}
          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <Label className="text-xs font-medium text-foreground">{ll.ai.settings.api_key_label()}</Label>
              <button
                type="button"
                className="text-muted-foreground hover:text-foreground text-[10px] flex items-center gap-1"
                onClick={() => setShowApiKey(!showApiKey)}
              >
                {showApiKey ? <EyeOff className="h-3 w-3" /> : <Eye className="h-3 w-3" />}
                {showApiKey ? ll.ai.settings.hide() : ll.ai.settings.show()}
              </button>
            </div>
            <Input
              type={showApiKey ? 'text' : 'password'}
              value={editingProvider.apiKey}
              onChange={(e) => setEditingProvider({ ...editingProvider, apiKey: e.target.value })}
              placeholder="sk-..."
              className="h-8 text-xs font-mono w-full"
            />
          </div>

          {/* 模型名称 */}
          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <Label className="text-xs font-medium text-foreground">{ll.ai.settings.model_label()}</Label>
              <button
                type="button"
                className="text-primary hover:underline text-[10px] flex items-center gap-1"
                onClick={handleFetchModels}
                disabled={fetchingModels || !editingProvider.baseUrl}
              >
                <RefreshCw className={`h-2.5 w-2.5 ${fetchingModels ? 'animate-spin' : ''}`} />
                {ll.ai.settings.fetch_models()}
              </button>
            </div>
            <Input
              value={editingProvider.model}
              onChange={(e) => setEditingProvider({ ...editingProvider, model: e.target.value })}
              placeholder="deepseek-chat / gpt-4o"
              className="h-8 text-xs font-mono w-full"
            />
            {availableModels.length > 0 && (
              <div className="flex flex-wrap gap-1 mt-1 max-h-24 overflow-y-auto p-1 bg-background rounded border border-border/60 text-[10px]">
                {availableModels.map((m) => (
                  <button
                    key={m}
                    type="button"
                    className="px-1.5 py-0.5 rounded bg-muted hover:bg-primary/20 text-muted-foreground hover:text-primary transition-colors"
                    onClick={() => setEditingProvider({ ...editingProvider, model: m })}
                  >
                    {m}
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* 表单内测试连接按钮 */}
        <div className="flex flex-col gap-1.5 pt-1">
          <Button
            type="button"
            size="sm"
            variant="outline"
            className="h-7 text-xs w-full gap-1.5"
            onClick={() => handleTestConnection(editingProvider)}
            disabled={testingId === editingProvider.id}
          >
            {testingId === editingProvider.id ? (
              <Loader2 className="h-3 w-3 animate-spin text-primary" />
            ) : (
              <Activity className="h-3 w-3" />
            )}
            {ll.ai.settings.test_connection()}
          </Button>

          {testResult?.id === editingProvider.id && (
            <span
              className={`text-[11px] px-2 py-1 rounded font-mono break-all ${
                testResult.success
                  ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
                  : 'bg-destructive/10 text-destructive'
              }`}
            >
              {testResult.success
                ? `${ll.ai.settings.test_success()} (${testResult.latencyMs}ms)`
                : `${ll.ai.settings.test_failed()}: ${testResult.error}`}
            </span>
          )}
        </div>
      </div>
    );
  };
  return (
    <div className={cn('space-y-4', className)}>
      {/* 模块头部 */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Sparkles className="h-4 w-4 text-primary shrink-0" />
          <span className="text-sm font-semibold text-foreground">
            {ll.ai.settings.title()}
          </span>
        </div>
        <div
          role="button"
          tabIndex={0}
          onClick={() => settings.setAiEnabled(!settings.aiEnabled)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              settings.setAiEnabled(!settings.aiEnabled);
            }
          }}
          className={cn(
            'flex items-center gap-1.5 h-7 px-2.5 rounded-md text-xs font-medium border transition-colors cursor-pointer select-none',
            settings.aiEnabled
              ? 'bg-emerald-500/10 border-emerald-500/35 text-emerald-600 dark:bg-emerald-950/40 dark:border-emerald-600/40 dark:text-emerald-300'
              : 'bg-muted/40 border-border/80 text-muted-foreground hover:bg-muted/60 dark:bg-zinc-900/60 dark:border-zinc-800 hover:text-foreground'
          )}
          title={settings.aiEnabled ? ll.ai.settings.enabled() : ll.ai.settings.disabled()}
        >
          <Zap
            className={cn(
              'h-3.5 w-3.5 shrink-0 transition-transform',
              settings.aiEnabled
                ? 'fill-emerald-500 text-emerald-500 dark:fill-emerald-400 dark:text-emerald-400 scale-105'
                : 'text-muted-foreground/70'
            )}
          />
          <span
            className={cn(
              'transition-colors',
              settings.aiEnabled
                ? 'text-emerald-700 dark:text-emerald-300 font-semibold'
                : 'text-muted-foreground'
            )}
          >
            {settings.aiEnabled ? ll.ai.settings.enabled() : ll.ai.settings.disabled()}
          </span>
          <Switch
            id="ai-enabled-toggle"
            checked={settings.aiEnabled}
            onCheckedChange={(val) => settings.setAiEnabled(val)}
            variant="emerald"
            className="scale-75 origin-center pointer-events-none"
            aria-label={settings.aiEnabled ? ll.ai.settings.enabled() : ll.ai.settings.disabled()}
          />
        </div>
      </div>

      <p className="text-xs text-muted-foreground leading-relaxed">
        {ll.ai.settings.description()}
      </p>

      {/* Provider 列表区域 */}
      <div className="space-y-2.5">
        <div className="flex items-center justify-between">
          <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
            {ll.ai.settings.providers_list()}
          </span>
          {!editingProvider && (
            <Button
              size="sm"
              variant="outline"
              className="h-6 px-2 text-xs gap-1"
              onClick={() => handleStartCreate()}
            >
              <Plus className="h-3 w-3" />
              {ll.ai.settings.add_provider()}
            </Button>
          )}
        </div>
        {/* 新建 Provider 表单（置于列表顶部，点击添加后立即展开） */}
        {isCreating && editingProvider && renderProviderForm(true)}

        {/* 无 Provider 时的空状态 */}
        {settings.aiProviders.length === 0 && !editingProvider && (
          <div className="rounded-lg border border-dashed border-border/80 p-4 text-center space-y-2.5 bg-muted/10">
            <Server className="mx-auto h-6 w-6 text-muted-foreground/50" />
            <div className="space-y-0.5">
              <p className="text-xs font-medium text-foreground">{ll.ai.settings.no_providers()}</p>
              <p className="text-[11px] text-muted-foreground">
                {ll.ai.settings.no_providers_desc()}
              </p>
            </div>
            <div className="grid grid-cols-2 gap-1.5 pt-1">
              {PRESETS.map((preset, idx) => (
                <Button
                  key={preset.name}
                  variant="outline"
                  size="sm"
                  className="text-xs h-7 justify-start px-2 truncate"
                  onClick={() => handleStartCreate(idx)}
                >
                  <Zap className="mr-1 h-3 w-3 text-amber-500 shrink-0" />
                  <span className="truncate flex-1 min-w-0 text-left">{preset.name}</span>
                </Button>
              ))}
            </div>
          </div>
        )}

        {/* Provider 列表卡片 */}
        {settings.aiProviders.map((provider) => {
          const isActive = settings.activeAiProviderId === provider.id;
          const isEditing = !isCreating && editingProvider?.id === provider.id;

          return (
            <div
              key={provider.id}
              className={cn(
                'rounded-lg border transition-all overflow-hidden',
                isEditing
                  ? 'border-primary/60 bg-muted/10 shadow-xs ring-1 ring-primary/20'
                  : isActive
                    ? 'border-primary/50 bg-primary/5 shadow-xs'
                    : 'border-border/60 bg-card hover:border-border'
              )}
            >
              {/* 卡片头部行 */}
              <div className="p-3 space-y-2">
                {/* 卡片第一行：激活单选 + 名称 + 徽章 + 操作按钮 */}
                <div className="flex items-center justify-between gap-2 min-w-0">
                  <div className="flex items-center gap-2 min-w-0">
                    <button
                      type="button"
                      className={cn(
                        'flex h-4 w-4 shrink-0 rounded-full border items-center justify-center transition-colors',
                        isActive
                          ? 'border-primary bg-primary text-primary-foreground'
                          : 'border-muted-foreground/40 hover:border-primary'
                      )}
                      onClick={() => settings.setActiveAiProviderId(provider.id)}
                      title={isActive ? ll.ai.settings.active_provider() : ll.ai.settings.set_active()}
                    >
                      {isActive && <Check className="h-2.5 w-2.5" />}
                    </button>

                    <span className="font-medium text-xs text-foreground truncate">
                      {provider.name}
                    </span>

                    {isActive && (
                      <Badge variant="secondary" className="text-[10px] h-4 px-1 bg-primary/10 text-primary border-0 shrink-0">
                        {ll.ai.settings.active_badge()}
                      </Badge>
                    )}
                  </div>

                  {/* 右侧操作按钮组 */}
                  <div className="flex items-center gap-1 shrink-0">
                    <Button
                      size="icon"
                      variant="ghost"
                      className="h-6 w-6 text-muted-foreground hover:text-foreground"
                      onClick={() => handleTestConnection(isEditing ? editingProvider! : provider)}
                      disabled={testingId === provider.id || (isEditing && testingId === editingProvider?.id)}
                      title={ll.ai.settings.test_connection()}
                    >
                      {testingId === provider.id || (isEditing && testingId === editingProvider?.id) ? (
                        <Loader2 className="h-3 w-3 animate-spin text-primary" />
                      ) : (
                        <Activity className="h-3 w-3" />
                      )}
                    </Button>

                    <Button
                      size="icon"
                      variant="ghost"
                      className={cn(
                        'h-6 w-6 text-muted-foreground hover:text-foreground',
                        isEditing && 'bg-primary/15 text-primary hover:text-primary hover:bg-primary/20'
                      )}
                      onClick={() => handleToggleEdit(provider)}
                      title={isEditing ? ll.ai.settings.cancel() : ll.ai.settings.edit()}
                    >
                      <Edit2 className="h-3 w-3" />
                    </Button>

                    <Button
                      size="icon"
                      variant="ghost"
                      className="h-6 w-6 text-muted-foreground hover:text-destructive"
                      onClick={() => handleDeleteProvider(provider.id)}
                      title={ll.ai.settings.delete()}
                    >
                      <Trash2 className="h-3 w-3" />
                    </Button>
                  </div>
                </div>

                {/* 卡片第二行：BaseURL + Model 徽章 */}
                <div className="flex flex-wrap items-center gap-1.5 text-[11px] text-muted-foreground font-mono">
                  <span className="truncate max-w-[220px] bg-muted/60 px-1.5 py-0.5 rounded border border-border/40">
                    {provider.baseUrl}
                  </span>
                  <Badge variant="outline" className="text-[10px] h-4 px-1 font-mono">
                    {provider.model}
                  </Badge>
                </div>

                {/* 测试结果提示行（非编辑状态下展示在此处，编辑状态下展示在表单底部） */}
                {!isEditing && testResult?.id === provider.id && (
                  <div className="pt-1">
                    <span
                      className={`text-[11px] px-2 py-0.5 rounded font-mono inline-block break-all ${
                        testResult.success
                          ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
                          : 'bg-destructive/10 text-destructive'
                      }`}
                    >
                      {testResult.success
                        ? `${ll.ai.settings.test_success()} (${testResult.latencyMs}ms)`
                        : `${ll.ai.settings.test_failed()}: ${testResult.error}`}
                    </span>
                  </div>
                )}
              </div>

              {/* 当处于编辑状态时，在此卡片内部展开编辑表单 */}
              {isEditing && renderProviderForm(false)}
            </div>
          );
        })}
      </div>
      {/* 自定义全局 System Prompt（可折叠式设计，节省空间） */}
      <div className="rounded-lg border border-border/60 bg-muted/10 overflow-hidden text-xs">
        <button
          type="button"
          className="flex w-full items-center justify-between p-2.5 text-left font-medium text-foreground hover:bg-muted/30 transition-colors"
          onClick={() => setShowSystemPrompt(!showSystemPrompt)}
        >
          <span>{ll.ai.settings.custom_system_prompt_label()}</span>
          {showSystemPrompt ? (
            <ChevronUp className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
          ) : (
            <ChevronDown className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
          )}
        </button>

        {showSystemPrompt && (
          <div className="p-2.5 pt-0 space-y-1.5 border-t border-border/40">
            <Textarea
              value={settings.aiCustomSystemPrompt || ''}
              onChange={(e) => settings.setAiCustomSystemPrompt(e.target.value || undefined)}
              placeholder={ll.ai.settings.custom_system_prompt_placeholder()}
              className="text-xs font-mono min-h-[70px] w-full resize-y mt-2"
            />
            <p className="text-[10px] text-muted-foreground leading-relaxed">
              {ll.ai.settings.custom_system_prompt_tip()}
            </p>
          </div>
        )}
      </div>

      {/* 高风险操作完全信任模式设置 */}
      <div className="rounded-lg border border-border/60 bg-muted/10 p-3 text-xs space-y-1.5">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <ShieldCheck className="h-4 w-4 text-amber-600 dark:text-amber-400 shrink-0" />
            <span className="font-medium text-foreground">
              {ll.ai.settings.trust_mode_label()}
            </span>
          </div>
          <Switch
            id="ai-trust-mode-toggle"
            checked={alwaysTrustHighRisk}
            onCheckedChange={(val) => setAlwaysTrustHighRisk(val)}
            variant="amber"
            aria-label={ll.ai.settings.trust_mode_label()}
          />
        </div>
        <p className="text-[11px] text-muted-foreground leading-relaxed">
          {ll.ai.settings.trust_mode_desc()}
        </p>
      </div>
    </div>
  );
}
