/**
 * ShortcutsDialog - 快捷键面板
 * Ctrl+/ 触发弹出，显示所有键盘快捷键列表，支持搜索
 */
import { useState, useMemo } from 'react';
import { Search, Keyboard } from 'lucide-react';
import { Dialog, DialogContent } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { useUIStore } from '@/stores';
import { getLL } from '@/i18n/i18n-lazy';

/** 快捷键分组定义 */
interface ShortcutGroup {
  group: string;
  shortcuts: { keys: string[]; description: string }[];
}

/** 所有快捷键分组 */
function getShortcutGroups(): ShortcutGroup[] {
  const ll = getLL().shortcuts;
  return [
    {
      group: ll.group.edit(),
      shortcuts: [
        { keys: ['Ctrl+Z'], description: ll.undo() },
        { keys: ['Ctrl+Shift+Z', 'Ctrl+Y'], description: ll.redo() },
        { keys: ['Ctrl+S'], description: ll.save() },
        { keys: ['Ctrl+D'], description: ll.duplicateNode() },
        { keys: ['Delete', 'Backspace'], description: ll.delete() },
        { keys: ['Ctrl+A'], description: ll.selectAll() },
      ],
    },
    {
      group: ll.group.execute(),
      shortcuts: [
        { keys: ['Ctrl+Enter'], description: ll.executeFlow() },
        { keys: ['Ctrl+B'], description: ll.togglePanelLeft() },
        { keys: ['Ctrl+H'], description: ll.togglePanelHistory() },
        { keys: ['Ctrl+J'], description: ll.togglePanelConfig() },
        { keys: ['Ctrl+,'], description: ll.openSettings() },
      ],
    },
    {
      group: ll.group.canvas(),
      shortcuts: [
        { keys: [getLL().common.space_drag()], description: ll.panCanvas() },
        { keys: [getLL().common.ctrl_scroll()], description: ll.zoomCanvas() },
        { keys: ['Ctrl+0'], description: ll.resetZoom() },
        { keys: [getLL().common.ctrl_zoom()], description: ll.zoomInOut() },
        { keys: ['F'], description: ll.fitView() },
        { keys: ['Ctrl+L'], description: ll.focusNodeList() },
        { keys: ['Ctrl+/'], description: ll.openShortcuts() },
      ],
    },
  ];
}

export function ShortcutsDialog() {
  const showShortcutsDialog = useUIStore((s) => s.showShortcutsDialog);
  const closeShortcutsDialog = useUIStore((s) => s.closeShortcutsDialog);
  const [searchQuery, setSearchQuery] = useState('');
  const ll = getLL().shortcuts;

  // 搜索过滤
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

  // 打开时重置搜索
  const handleOpenChange = (open: boolean) => {
    if (!open) {
      setSearchQuery('');
      closeShortcutsDialog();
    }
  };

  return (
    <Dialog open={showShortcutsDialog} onOpenChange={handleOpenChange}>
      <DialogContent
        className="sm:max-w-[500px] h-[70vh] !gap-0 p-0 flex flex-col"
        hideCloseButton
      >
        {/* 标题 */}
        <div className="flex items-center gap-2 px-6 pt-5 pb-3">
          <Keyboard className="w-5 h-5 text-muted-foreground" />
          <h2 className="text-lg font-semibold text-foreground">{ll.title()}</h2>
        </div>

        {/* 搜索框 */}
        <div className="px-6 pb-3">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              type="text"
              placeholder={ll.searchPlaceholder()}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9 h-9 text-sm"
              autoFocus
            />
          </div>
        </div>

        {/* 快捷键列表 — 可滚动 */}
        <div className="flex-1 overflow-y-auto px-6 min-h-0">
          <div className="space-y-4 pb-4">
            {filteredGroups.map((group) => (
              <div key={group.group}>
                <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-2">
                  {group.group}
                </h4>
                <div className="space-y-1">
                  {group.shortcuts.map((shortcut, i) => (
                    <div
                      key={i}
                      className="flex items-center justify-between py-1.5 px-2 rounded-md hover:bg-muted/50 transition-colors"
                    >
                      <span className="text-sm text-foreground">{shortcut.description}</span>
                      <div className="flex items-center gap-1 shrink-0">
                        {shortcut.keys.map((key, j) => (
                          <span key={j}>
                            {j > 0 && <span className="text-xs text-muted-foreground mx-0.5">{getLL().common.or()}</span>}
                            <kbd className="px-1.5 py-0.5 text-xs font-mono rounded border border-border bg-muted shadow-sm whitespace-nowrap">
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
              <div className="py-8 text-center text-sm text-muted-foreground">
                {ll.noResults()}
              </div>
            )}
          </div>
        </div>

        {/* 底部提示 */}
        <div className="text-[11px] text-muted-foreground/60 text-center px-6 py-2 border-t border-border shrink-0">
          {ll.hint()}
        </div>
      </DialogContent>
    </Dialog>
  );
}

export default ShortcutsDialog;
