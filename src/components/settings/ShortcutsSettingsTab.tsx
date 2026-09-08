import { useState, useMemo } from 'react';
import { Search, BookOpen, Sparkles, ExternalLink, ArrowRight } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { useUIStore } from '@/stores/ui';
import { getShortcutGroups } from '@/lib/shortcuts';
import { getLL } from '@/i18n/i18n-lazy';

export function ShortcutsSettingsTab() {
  const [searchQuery, setSearchQuery] = useState('');
  const closeSettingsDialog = useUIStore((s) => s.closeSettingsDialog);
  const openScriptHelpDialog = useUIStore((s) => s.openScriptHelpDialog);
  const openDynamicVariablesHelpDialog = useUIStore((s) => s.openDynamicVariablesHelpDialog);
  const ll = getLL();

  // 搜索过滤快捷键
  const filteredGroups = useMemo(() => {
    if (!searchQuery.trim()) return getShortcutGroups();
    const q = searchQuery.toLowerCase();
    return getShortcutGroups()
      .map((group) => ({
        ...group,
        shortcuts: group.shortcuts.filter(
          (s) =>
            s.description.toLowerCase().includes(q) ||
            s.keys.some((k) => k.toLowerCase().includes(q))
        ),
      }))
      .filter((g) => g.shortcuts.length > 0);
  }, [searchQuery]);

  const handleOpenScriptHelp = () => {
    closeSettingsDialog();
    openScriptHelpDialog();
  };

  const handleOpenDynamicVarsHelp = () => {
    closeSettingsDialog();
    openDynamicVariablesHelpDialog();
  };

  return (
    <div className="space-y-6">
      {/* 内置手册与参考卡片区 */}
      <div className="space-y-3">
        <div>
          <h3 className="text-sm font-semibold text-foreground">
            {ll.settings.manuals_section_title()}
          </h3>
          <p className="text-xs text-muted-foreground mt-0.5">
            {ll.settings.manuals_section_desc()}
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          {/* 脚本函数 API 参考卡片 */}
          <button
            type="button"
            onClick={handleOpenScriptHelp}
            className="group flex flex-col justify-between p-3.5 rounded-xl border border-border/70 bg-muted/20 hover:bg-muted/50 hover:border-violet-500/50 transition-all text-left cursor-pointer"
          >
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <div className="w-8 h-8 rounded-lg bg-violet-100 dark:bg-violet-950/60 flex items-center justify-center text-violet-600 dark:text-violet-400">
                  <BookOpen className="w-4 h-4" />
                </div>
                <span className="inline-flex items-center text-xs text-violet-600 dark:text-violet-400 font-medium group-hover:translate-x-0.5 transition-transform">
                  <ArrowRight className="w-3.5 h-3.5" />
                </span>
              </div>
              <div>
                <h4 className="text-sm font-medium text-foreground group-hover:text-violet-600 dark:group-hover:text-violet-400 transition-colors">
                  {ll.settings.open_script_manual()}
                </h4>
                <p className="text-xs text-muted-foreground mt-1 line-clamp-2">
                  {ll.settings.open_script_manual_desc()}
                </p>
              </div>
            </div>
            <div className="mt-3 pt-2.5 border-t border-border/40 flex items-center justify-between text-[11px] text-muted-foreground">
              <span>fn.variables · fn.response · fn.flow</span>
              <span className="inline-flex items-center gap-1 font-medium text-foreground/80">
                {ll.settings.open_in_dialog()}
                <ExternalLink className="w-2.5 h-2.5" />
              </span>
            </div>
          </button>

          {/* 动态变量参考手册卡片 */}
          <button
            type="button"
            onClick={handleOpenDynamicVarsHelp}
            className="group flex flex-col justify-between p-3.5 rounded-xl border border-border/70 bg-muted/20 hover:bg-muted/50 hover:border-emerald-500/50 transition-all text-left cursor-pointer"
          >
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <div className="w-8 h-8 rounded-lg bg-emerald-100 dark:bg-emerald-950/60 flex items-center justify-center text-emerald-600 dark:text-emerald-400">
                  <Sparkles className="w-4 h-4" />
                </div>
                <span className="inline-flex items-center text-xs text-emerald-600 dark:text-emerald-400 font-medium group-hover:translate-x-0.5 transition-transform">
                  <ArrowRight className="w-3.5 h-3.5" />
                </span>
              </div>
              <div>
                <h4 className="text-sm font-medium text-foreground group-hover:text-emerald-600 dark:group-hover:text-emerald-400 transition-colors">
                  {ll.settings.open_dynamic_vars_manual()}
                </h4>
                <p className="text-xs text-muted-foreground mt-1 line-clamp-2">
                  {ll.settings.open_dynamic_vars_manual_desc()}
                </p>
              </div>
            </div>
            <div className="mt-3 pt-2.5 border-t border-border/40 flex items-center justify-between text-[11px] text-muted-foreground">
              <span>{`{{$guid}} · {{$timestamp}} · {{$randomInt}}`}</span>
              <span className="inline-flex items-center gap-1 font-medium text-foreground/80">
                {ll.settings.open_in_dialog()}
                <ExternalLink className="w-2.5 h-2.5" />
              </span>
            </div>
          </button>
        </div>
      </div>

      {/* 分隔线 */}
      <div className="border-t border-border/60" />

      {/* 快捷键速查区 */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-sm font-semibold text-foreground">
              {ll.shortcuts.title()}
            </h3>
            <p className="text-xs text-muted-foreground mt-0.5">
              {ll.shortcuts.hint()}
            </p>
          </div>
        </div>

        {/* 快捷键搜索框 */}
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
          <Input
            type="text"
            placeholder={ll.shortcuts.searchPlaceholder()}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-8 h-8 text-xs bg-muted/20"
          />
        </div>

        {/* 快捷键分组列表 */}
        <div className="space-y-4 pt-1">
          {filteredGroups.map((group) => (
            <div key={group.group} className="space-y-1.5">
              <div className="text-[11px] font-semibold text-muted-foreground uppercase tracking-wider px-1">
                {group.group}
              </div>
              <div className="rounded-lg border border-border/60 bg-muted/10 divide-y divide-border/40 overflow-hidden">
                {group.shortcuts.map((shortcut, i) => (
                  <div
                    key={i}
                    className="flex items-center justify-between py-2 px-3 hover:bg-muted/40 transition-colors"
                  >
                    <span className="text-xs text-foreground font-normal">
                      {shortcut.description}
                    </span>
                    <div className="flex items-center gap-1 shrink-0">
                      {shortcut.keys.map((key, j) => (
                        <span key={j} className="flex items-center">
                          {j > 0 && (
                            <span className="text-[11px] text-muted-foreground mx-1">
                              {ll.common.or()}
                            </span>
                          )}
                          <kbd className="px-1.5 py-0.5 text-[11px] font-mono rounded border border-border/80 bg-background text-foreground shadow-2xs whitespace-nowrap">
                            {key}
                          </kbd>
                        </span>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ))}

          {filteredGroups.length === 0 && (
            <div className="py-8 text-center text-xs text-muted-foreground">
              {ll.shortcuts.noResults()}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
