import { useEffect, useState } from 'react';
import { Palette, HardDrive, Sparkles, Cpu, CheckCircle2, Info, X, Keyboard } from 'lucide-react';
import { Dialog, DialogContent, DialogTitle, DialogDescription } from '@/components/ui/dialog';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { useUIStore } from '@/stores/ui';
import { useSettingsStore } from '@/stores/settings';
import { getLL } from '@/i18n/i18n-lazy';
import { cn } from '@/lib/utils';
import { useCurrentLocale } from '@/hooks/useLocale';
import { GeneralSettingsTab } from './GeneralSettingsTab';
import { StorageSettingsTab } from './StorageSettingsTab';
import { EngineSettingsTab } from './EngineSettingsTab';
import { AboutSettingsTab } from './AboutSettingsTab';
import { AIProviderSettingsCard } from '@/components/ai/AIProviderSettingsCard';
import { ShortcutsSettingsTab } from './ShortcutsSettingsTab';
import { useUpdateStore } from '@/stores/update';

export type SettingsTabId = 'general' | 'storage' | 'ai' | 'engine' | 'about' | 'shortcuts';

export function SettingsDialog() {
  useCurrentLocale();
  const showSettingsDialog = useUIStore((s) => s.showSettingsDialog);
  const closeSettingsDialog = useUIStore((s) => s.closeSettingsDialog);
  const settingsDialogTab = useUIStore((s) => s.settingsDialogTab);
  const setSettingsDialogTab = useUIStore((s) => s.setSettingsDialogTab);

  const loadSettings = useSettingsStore((s) => s.loadSettings);
  const currentVersion = useUpdateStore((s) => s.currentVersion);
  const loadCurrentVersion = useUpdateStore((s) => s.loadCurrentVersion);
  const ll = getLL();

  const [activeTab, setActiveTab] = useState<SettingsTabId>('general');

  // 当外部 store 变更 target tab 时同步（如 AICopilot 呼出）
  useEffect(() => {
    if (settingsDialogTab) {
      setActiveTab(settingsDialogTab);
    }
  }, [settingsDialogTab]);

  // 打开时重新加载设置
  useEffect(() => {
    if (showSettingsDialog) {
      void loadSettings();
      void loadCurrentVersion();
    }
  }, [showSettingsDialog, loadSettings, loadCurrentVersion]);

  const handleTabChange = (tabId: SettingsTabId) => {
    setActiveTab(tabId);
    setSettingsDialogTab?.(tabId);
  };

  const navItems: Array<{
    id: SettingsTabId;
    label: string;
    icon: typeof Palette;
    title: string;
    description: string;
  }> = [
    {
      id: 'general',
      label: ll.settings.tab_general(),
      icon: Palette,
      title: ll.settings.tab_general(),
      description: ll.settings.appearance_desc(),
    },
    {
      id: 'storage',
      label: ll.settings.tab_storage(),
      icon: HardDrive,
      title: ll.settings.tab_storage(),
      description: ll.settings.dialog_desc(),
    },
    {
      id: 'ai',
      label: ll.settings.tab_ai(),
      icon: Sparkles,
      title: ll.settings.tab_ai(),
      description: ll.ai.settings.description(),
    },
    {
      id: 'engine',
      label: ll.settings.tab_engine(),
      icon: Cpu,
      title: ll.settings.tab_engine(),
      description: ll.settings.engine_desc(),
    },
    {
      id: 'shortcuts',
      label: ll.settings.tab_shortcuts(),
      icon: Keyboard,
      title: ll.settings.tab_shortcuts(),
      description: ll.settings.shortcuts_desc(),
    },
    {
      id: 'about',
      label: ll.settings.tab_about(),
      icon: Info,
      title: ll.settings.tab_about(),
      description: ll.settings.about_desc(),
    },
  ];

  const currentNav = navItems.find((item) => item.id === activeTab) ?? navItems[0];

  return (
    <Dialog
      open={showSettingsDialog}
      onOpenChange={(open) => {
        if (!open) closeSettingsDialog();
      }}
    >
      <DialogContent
        hideCloseButton
        className="max-w-4xl w-[90vw] h-[640px] max-h-[90vh] p-0 gap-0 overflow-hidden flex flex-col bg-background sm:rounded-xl shadow-2xl border-border"
      >
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/70 shrink-0 select-none">
          <div className="flex items-center gap-3">
            <DialogTitle className="text-base font-semibold text-foreground">
              {ll.settings.dialog_title()}
            </DialogTitle>
            <Badge variant="outline" className="text-xs font-normal text-muted-foreground border-border/60">
              v{currentVersion || '0.1.1-beta.2'}
            </Badge>
          </div>

          <div className="flex items-center gap-3">
            <div className="flex items-center gap-1.5 text-xs text-muted-foreground bg-muted/50 border border-border/40 px-2.5 py-1 rounded-full">
              <CheckCircle2 className="h-3.5 w-3.5 text-green-600" />
              <span>{ll.settings.saved_badge()}</span>
            </div>

            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7 rounded-sm opacity-70 hover:opacity-100 text-muted-foreground hover:text-foreground cursor-pointer"
              onClick={closeSettingsDialog}
              title={ll.panels.close()}
            >
              <X className="h-4 w-4" />
              <span className="sr-only">{ll.panels.close()}</span>
            </Button>
          </div>
        </div>

        {/* 主体两栏布局 */}
        <div className="flex flex-1 min-h-0 overflow-hidden">
          {/* 左侧导航栏 */}
          <aside className="w-56 shrink-0 border-r border-border/70 bg-muted/20 p-3 flex flex-col justify-between select-none">
            <nav className="space-y-1">
              {navItems.map((item) => {
                const isCurrent = activeTab === item.id;
                const Icon = item.icon;
                return (
                  <button
                    key={item.id}
                    type="button"
                    onClick={() => handleTabChange(item.id)}
                    className={cn(
                      'w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-xs font-medium transition-all text-left cursor-pointer',
                      isCurrent
                        ? 'bg-primary/10 text-primary font-semibold shadow-2xs'
                        : 'text-muted-foreground hover:text-foreground hover:bg-muted/60'
                    )}
                  >
                    <Icon className={cn('h-4 w-4 shrink-0', isCurrent ? 'text-primary' : 'text-muted-foreground')} />
                    <span className="truncate flex-1 min-w-0">{item.label}</span>
                  </button>
                );
              })}
            </nav>

            <div className="px-3 py-2 text-[11px] text-muted-foreground/80 border-t border-border/40 space-y-0.5">
              <div className="font-medium text-foreground/80">Multi-Protocol Flow</div>
              <div>Workflow Engine & Executor</div>
            </div>
          </aside>

          {/* 右侧设置面板区域 */}
          <main className="flex-1 min-w-0 flex flex-col h-full bg-background overflow-hidden">
            {/* 当前 Tab 标题说明栏 */}
            <div className="px-6 py-3.5 border-b border-border/50 shrink-0 bg-muted/10">
              <h2 className="text-sm font-semibold text-foreground">{currentNav.title}</h2>
              <DialogDescription className="text-xs text-muted-foreground mt-0.5">
                {currentNav.description}
              </DialogDescription>
            </div>

            {/* 设置内容滚动区 */}
            <ScrollArea className="flex-1 min-h-0">
              <div className="p-6">
                {activeTab === 'general' && <GeneralSettingsTab />}
                {activeTab === 'storage' && <StorageSettingsTab />}
                {activeTab === 'ai' && <AIProviderSettingsCard className="border-t-0 pt-0" />}
                {activeTab === 'engine' && <EngineSettingsTab />}
                {activeTab === 'shortcuts' && <ShortcutsSettingsTab />}
                {activeTab === 'about' && <AboutSettingsTab />}
              </div>
            </ScrollArea>
          </main>
        </div>
      </DialogContent>
    </Dialog>
  );
}
