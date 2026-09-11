import { Sun, Moon, Monitor, Check } from 'lucide-react';
import { useSettingsStore } from '@/stores/settings';
import type { ThemeMode, Locale } from '@/types/settings';
import { getLL } from '@/i18n/i18n-lazy';
import { cn } from '@/lib/utils';

export function GeneralSettingsTab() {
  const { theme, locale, setTheme, setLocale } = useSettingsStore();
  const ll = getLL();

  const themeOptions: Array<{
    value: ThemeMode;
    label: string;
    icon: typeof Sun;
    previewClass: string;
  }> = [
    {
      value: 'light',
      label: ll.settings.theme_light(),
      icon: Sun,
      previewClass: 'bg-[#fafafa] border-[#e4e4e7] text-zinc-900',
    },
    {
      value: 'dark',
      label: ll.settings.theme_dark(),
      icon: Moon,
      previewClass: 'bg-[#18181b] border-[#27272a] text-zinc-100',
    },
    {
      value: 'system',
      label: ll.settings.theme_system(),
      icon: Monitor,
      previewClass: 'bg-gradient-to-r from-[#fafafa] to-[#18181b] border-[#71717a]/40 text-foreground',
    },
  ];

  const localeOptions: Array<{
    value: Locale;
    label: string;
    sublabel: string;
  }> = [
    {
      value: 'zh-CN',
      label: '简体中文',
      sublabel: 'Chinese (Simplified)',
    },
    {
      value: 'en-US',
      label: 'English',
      sublabel: '英语 (US)',
    },
  ];

  return (
    <div className="space-y-6">
      {/* 主题设置 */}
      <div className="space-y-3">
        <div>
          <h3 className="text-sm font-semibold text-foreground">
            {ll.settings.appearance()}
          </h3>
          <p className="text-xs text-muted-foreground mt-0.5">
            {ll.settings.appearance_desc()}
          </p>
        </div>

        <div className="grid grid-cols-3 gap-3">
          {themeOptions.map((opt) => {
            const isSelected = theme === opt.value;
            const Icon = opt.icon;
            return (
              <button
                key={opt.value}
                type="button"
                onClick={() => setTheme(opt.value)}
                className={cn(
                  'relative flex flex-col items-center justify-between rounded-xl border-2 p-3 text-center transition-all cursor-pointer select-none',
                  isSelected
                    ? 'border-primary bg-primary/5 shadow-xs'
                    : 'border-border/70 hover:border-border hover:bg-muted/40'
                )}
              >
                {isSelected && (
                  <span className="absolute top-2 right-2 flex h-4 w-4 items-center justify-center rounded-full bg-primary text-primary-foreground">
                    <Check className="h-2.5 w-2.5 stroke-[3]" />
                  </span>
                )}

                {/* 样式微缩示意块 */}
                <div
                  className={cn(
                    'w-full h-12 rounded-lg border flex items-center justify-center mb-2.5 transition-transform group-hover:scale-105',
                    opt.previewClass
                  )}
                >
                  <Icon className="h-5 w-5" />
                </div>

                <span className={cn('text-xs font-medium', isSelected ? 'text-primary font-semibold' : 'text-foreground')}>
                  {opt.label}
                </span>
              </button>
            );
          })}
        </div>
      </div>

      {/* 分隔线 */}
      <div className="border-t border-border/60" />

      {/* 语言设置 */}
      <div className="space-y-3">
        <div>
          <h3 className="text-sm font-semibold text-foreground">
            {ll.settings.language()}
          </h3>
          <p className="text-xs text-muted-foreground mt-0.5">
            {ll.settings.language_desc()}
          </p>
        </div>

        <div className="grid grid-cols-2 gap-3">
          {localeOptions.map((opt) => {
            const isSelected = locale === opt.value;
            return (
              <button
                key={opt.value}
                type="button"
                onClick={() => setLocale(opt.value)}
                className={cn(
                  'relative flex items-center justify-between rounded-lg border-2 p-3 text-left transition-all cursor-pointer select-none',
                  isSelected
                    ? 'border-primary bg-primary/5 shadow-xs'
                    : 'border-border/70 hover:border-border hover:bg-muted/40'
                )}
              >
                <div>
                  <div className={cn('text-sm font-medium', isSelected ? 'text-primary font-semibold' : 'text-foreground')}>
                    {opt.label}
                  </div>
                  <div className="text-xs text-muted-foreground mt-0.5">{opt.sublabel}</div>
                </div>

                {isSelected && (
                  <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground">
                    <Check className="h-3 w-3 stroke-[3]" />
                  </span>
                )}
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}
