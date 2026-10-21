/**
 * AI Provider 供应商快捷切换下拉组件
 * 允许用户在对话侧边栏、Spotlight 弹窗或输入框工具栏直接一键切换生效的 AI 供应商/模型
 */

import { useState, useRef, useEffect, memo } from 'react';
import {
  Bot,
  Check,
  ChevronDown,
  ChevronUp,
  Settings,
  Sparkles,
} from 'lucide-react';
import { useSettingsStore } from '@/stores/settings';
import { useUIStore } from '@/stores/ui';
import { getLL } from '@/i18n/i18n-lazy';
import { toast } from 'sonner';

export interface AIProviderSelectProps {
  className?: string;
  /** 弹出菜单方向：'up' (向上弹出，适合底部工具栏) 或 'down' (向下弹出，适合顶部导航栏) */
  direction?: 'up' | 'down';
  /** 是否使用极简紧凑外观 */
  compact?: boolean;
  /** 是否禁用切换（如正在生成时） */
  disabled?: boolean;
  /** 自定义最大宽度 class，默认 max-w-[140px] */
  maxWidthClass?: string;
}

export const AIProviderSelect = memo(function AIProviderSelect({
  className = '',
  direction = 'down',
  compact = false,
  disabled = false,
  maxWidthClass = 'max-w-[140px]',
}: AIProviderSelectProps) {
  const ll = getLL();
  const { aiProviders, activeAiProviderId, setActiveAiProviderId } = useSettingsStore();
  const openSettingsDialog = useUIStore((s) => s.openSettingsDialog);

  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const activeProvider = aiProviders.find((p) => p.id === activeAiProviderId);

  // 点击外部自动收起下拉弹窗
  useEffect(() => {
    if (!isOpen) return;

    const handlePointerDown = (e: MouseEvent | TouchEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setIsOpen(false);
      }
    };

    window.addEventListener('mousedown', handlePointerDown);
    window.addEventListener('touchstart', handlePointerDown);
    window.addEventListener('keydown', handleKeyDown);

    return () => {
      window.removeEventListener('mousedown', handlePointerDown);
      window.removeEventListener('touchstart', handlePointerDown);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [isOpen]);

  const handleSelectProvider = async (providerId: string, providerName: string) => {
    if (providerId === activeAiProviderId) {
      setIsOpen(false);
      return;
    }
    await setActiveAiProviderId(providerId);
    setIsOpen(false);
    toast.success(ll.ai.copilot.provider_switched({ name: providerName }));
  };

  const handleManageProviders = () => {
    setIsOpen(false);
    openSettingsDialog('ai');
  };

  const isUp = direction === 'up';

  return (
    <div className={`relative inline-flex items-center ${className}`} ref={containerRef}>
      <button
        type="button"
        disabled={disabled}
        onClick={() => setIsOpen((prev) => !prev)}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        title={
          activeProvider
            ? `${activeProvider.name} (${activeProvider.model})`
            : ll.ai.copilot.select_provider()
        }
        className={`inline-flex items-center gap-1 rounded-md text-[11px] font-mono transition-colors cursor-pointer select-none border ${
          disabled
            ? 'opacity-50 cursor-not-allowed bg-muted/30 border-border/40 text-muted-foreground'
            : isOpen
              ? 'bg-muted border-primary/40 text-foreground ring-1 ring-primary/20'
              : 'bg-muted/50 hover:bg-muted border-border/60 text-muted-foreground hover:text-foreground'
        } ${compact ? 'px-1.5 py-0.5' : 'px-2 py-1'}`}
      >
        <Bot className="h-3 w-3 text-primary shrink-0" />
        <span className={`truncate min-w-0 font-medium ${maxWidthClass}`}>
          {activeProvider ? activeProvider.name : ll.ai.copilot.select_provider()}
        </span>
        {isUp ? (
          <ChevronUp className="h-3 w-3 opacity-60 shrink-0 ml-0.5" />
        ) : (
          <ChevronDown className="h-3 w-3 opacity-60 shrink-0 ml-0.5" />
        )}
      </button>

      {isOpen && (
        <div
          role="listbox"
          aria-label={ll.ai.copilot.switch_provider()}
          className={`absolute left-0 w-60 rounded-lg border border-border bg-popover text-popover-foreground shadow-xl z-50 p-1.5 space-y-1 animate-in fade-in zoom-in-95 duration-100 ${
            isUp ? 'bottom-full mb-1.5' : 'top-full mt-1.5'
          }`}
          onMouseDown={(e) => e.stopPropagation()}
        >
          {/* 标题栏 */}
          <div className="flex items-center justify-between px-2 py-1 border-b border-border/50 text-[10px] font-semibold text-muted-foreground select-none">
            <span className="flex items-center gap-1 uppercase tracking-wider">
              <Sparkles className="h-3 w-3 text-primary" />
              <span>{ll.ai.copilot.switch_provider()}</span>
            </span>
            <span className="font-mono text-[9px] text-muted-foreground/60">
              {aiProviders.length}
            </span>
          </div>

          {/* 供应商列表 */}
          <div className="max-h-52 overflow-y-auto space-y-0.5">
            {aiProviders.length === 0 ? (
              <div className="p-3 text-center text-xs text-muted-foreground space-y-1.5">
                <p>{ll.ai.copilot.no_active_provider()}</p>
                <button
                  type="button"
                  onClick={handleManageProviders}
                  className="text-primary text-[11px] hover:underline font-medium inline-block"
                >
                  {ll.ai.copilot.configure_provider()}
                </button>
              </div>
            ) : (
              aiProviders.map((provider) => {
                const isActive = provider.id === activeAiProviderId;
                return (
                  <button
                    key={provider.id}
                    type="button"
                    role="option"
                    aria-selected={isActive}
                    onClick={() => handleSelectProvider(provider.id, provider.name)}
                    className={`w-full flex items-center justify-between px-2 py-1.5 rounded-md text-left text-xs transition-colors cursor-pointer group ${
                      isActive
                        ? 'bg-primary/10 text-primary font-medium'
                        : 'hover:bg-muted/70 text-foreground'
                    }`}
                  >
                    <div className="flex-1 min-w-0 pr-2">
                      <div className="truncate font-medium text-[11.5px] leading-tight">
                        {provider.name}
                      </div>
                      <div className="truncate font-mono text-[10px] text-muted-foreground/75 leading-tight mt-0.5">
                        {provider.model}
                      </div>
                    </div>
                    {isActive && (
                      <Check className="h-3.5 w-3.5 text-primary shrink-0 ml-1" />
                    )}
                  </button>
                );
              })
            )}
          </div>

          {/* 底部管理配置按钮 */}
          <div className="border-t border-border/40 pt-1">
            <button
              type="button"
              onClick={handleManageProviders}
              className="w-full flex items-center gap-1.5 px-2 py-1 text-[11px] rounded-md text-muted-foreground hover:text-foreground hover:bg-muted/60 transition-colors text-left cursor-pointer"
            >
              <Settings className="h-3 w-3 shrink-0" />
              <span className="truncate">{ll.ai.copilot.manage_providers()}</span>
            </button>
          </div>
        </div>
      )}
    </div>
  );
});
