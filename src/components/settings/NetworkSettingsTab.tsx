import { useState, useEffect } from 'react';
import { Radio, Globe, Copy, Check, RotateCcw } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { useSettingsStore } from '@/stores/settings';
import { getLL } from '@/i18n/i18n-lazy';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';

export function NetworkSettingsTab() {
  const { defaultIngressPort, setDefaultIngressPort } = useSettingsStore();
  const ll = getLL();

  const [portInput, setPortInput] = useState<string>(String(defaultIngressPort ?? 9527));
  const [copied, setCopied] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  useEffect(() => {
    setPortInput(String(defaultIngressPort ?? 9527));
  }, [defaultIngressPort]);

  const recommendedTag = ll.settings.preset_recommended();
  const portPresets = [
    { label: `9527 (${recommendedTag})`, value: 9527 },
    { label: '8080', value: 8080 },
    { label: '8000', value: 8000 },
    { label: '3000', value: 3000 },
    { label: '5000', value: 5000 },
  ];

  const handleApplyPort = (portNum: number) => {
    if (isNaN(portNum) || portNum < 1024 || portNum > 65535) {
      setErrorMsg(ll.settings.ingress_port_invalid());
      return;
    }
    setErrorMsg(null);
    setPortInput(String(portNum));
    setDefaultIngressPort(portNum);
    toast.success(ll.settings.saved_badge());
  };

  const currentPort = parseInt(portInput, 10) || defaultIngressPort || 9527;
  const previewUrl = `http://127.0.0.1:${currentPort}/{flow-slug}/*`;

  return (
    <div className="space-y-6">
      {/* 默认网关端口配置 Card */}
      <div className="rounded-xl border border-border/70 bg-card p-4 space-y-4">
        <div className="flex items-start justify-between gap-4">
          <div className="flex items-start gap-3 min-w-0 flex-1">
            <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-indigo-500/10 text-indigo-500 mt-0.5">
              <Radio className="h-4 w-4" />
            </div>
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2 flex-wrap">
                <span className="text-sm font-semibold text-foreground">
                  {ll.settings.ingress_gateway_port()}
                </span>
                <Badge variant="outline" className="text-[10px] font-mono border-indigo-500/30 text-indigo-500">
                  :{currentPort}
                </Badge>
              </div>
              <p className="text-xs text-muted-foreground mt-0.5 leading-relaxed">
                {ll.settings.ingress_gateway_port_desc()}
              </p>
            </div>
          </div>

          {currentPort !== 9527 && (
            <Button
              variant="outline"
              size="sm"
              onClick={() => handleApplyPort(9527)}
              className="text-xs h-7 gap-1.5 shrink-0 cursor-pointer"
            >
              <RotateCcw className="w-3.5 h-3.5" />
              <span>{ll.settings.ingress_reset_default()}</span>
            </Button>
          )}
        </div>

        {/* 端口输入框与快捷预设 */}
        <div className="pt-1 space-y-3">
          <div className="flex items-center gap-3">
            <div className="relative w-48 max-w-full">
              <Input
                type="number"
                min={1024}
                max={65535}
                value={portInput}
                onChange={(e) => {
                  setPortInput(e.target.value);
                  setErrorMsg(null);
                }}
                onBlur={() => handleApplyPort(parseInt(portInput.trim(), 10))}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    handleApplyPort(parseInt(portInput.trim(), 10));
                  }
                }}
                className={cn(
                  'h-9 font-mono text-sm pr-12',
                  errorMsg && 'border-destructive focus-visible:ring-destructive'
                )}
                placeholder="9527"
              />
              <span className="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-muted-foreground font-mono select-none">
                TCP
              </span>
            </div>
            {errorMsg && (
              <span className="text-xs text-destructive">{errorMsg}</span>
            )}
          </div>

          {/* 预设按钮组 */}
          <div className="flex items-center gap-1.5 flex-wrap pt-0.5">
            <span className="text-xs text-muted-foreground mr-1 select-none shrink-0">
              {ll.settings.ingress_port_presets()}:
            </span>
            {portPresets.map((preset) => {
              const isSelected = currentPort === preset.value;
              return (
                <Button
                  key={preset.value}
                  type="button"
                  variant={isSelected ? 'secondary' : 'outline'}
                  size="sm"
                  className={cn(
                    'h-7 px-2.5 text-xs font-mono cursor-pointer',
                    isSelected && 'font-semibold border-indigo-500/50 bg-indigo-500/10 text-indigo-500'
                  )}
                  onClick={() => handleApplyPort(preset.value)}
                >
                  {preset.label}
                </Button>
              );
            })}
          </div>
        </div>
      </div>

      {/* 实时 URL 预览 Card */}
      <div className="rounded-xl border border-border/70 bg-card p-4 space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-xs font-semibold text-foreground">
            <Globe className="w-4 h-4 text-indigo-500" />
            <span>{ll.settings.ingress_url_preview()}</span>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              navigator.clipboard.writeText(previewUrl);
              setCopied(true);
              setTimeout(() => setCopied(false), 2000);
            }}
            className="h-7 px-2 text-xs text-muted-foreground hover:text-foreground gap-1.5 cursor-pointer"
          >
            {copied ? (
              <>
                <Check className="w-3.5 h-3.5 text-emerald-500" />
                <span className="text-emerald-500 font-medium">{ll.settings.path_copied()}</span>
              </>
            ) : (
              <>
                <Copy className="w-3.5 h-3.5" />
                <span>{ll.settings.copy_path()}</span>
              </>
            )}
          </Button>
        </div>

        <div className="flex items-center justify-between p-3 rounded-lg bg-muted/40 border border-border/80 font-mono text-xs">
          <div className="flex items-center gap-1 truncate">
            <span className="text-muted-foreground">http://127.0.0.1:{currentPort}/</span>
            <span className="text-indigo-500 font-semibold">{'{flow-slug}'}</span>
            <span className="text-muted-foreground">/*</span>
          </div>
          <Badge variant="secondary" className="text-[10px] shrink-0 font-normal ml-3">
            HTTP
          </Badge>
        </div>
      </div>

      {/* 模式对照说明 */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="flex flex-col justify-between p-4 rounded-xl border border-indigo-500/30 bg-indigo-500/5 space-y-3">
          <div className="space-y-1.5">
            <div className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 rounded-full bg-indigo-500" />
                <h4 className="text-xs font-semibold text-foreground">
                  {ll.settings.ingress_shared_mode_title()}
                </h4>
              </div>
              <Badge variant="outline" className="text-[10px] bg-indigo-500/10 text-indigo-500 border-indigo-500/30 font-normal">
                Port :{currentPort}
              </Badge>
            </div>
            <p className="text-xs text-muted-foreground leading-relaxed">
              {ll.settings.ingress_shared_mode_desc()}
            </p>
          </div>
          <div className="pt-2 border-t border-indigo-500/10">
            <div className="flex items-center justify-between text-[11px]">
              <span className="text-muted-foreground">{ll.settings.ingress_route_format_label()}:</span>
              <code className="font-mono text-indigo-500 font-medium">/{'{flow-slug}'}/*</code>
            </div>
          </div>
        </div>

        <div className="flex flex-col justify-between p-4 rounded-xl border border-border/70 bg-card space-y-3">
          <div className="space-y-1.5">
            <div className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 rounded-full bg-emerald-500" />
                <h4 className="text-xs font-semibold text-foreground">
                  {ll.settings.ingress_custom_mode_title()}
                </h4>
              </div>
              <Badge variant="outline" className="text-[10px] bg-emerald-500/10 text-emerald-500 border-emerald-500/30 font-normal">
                Port :Custom
              </Badge>
            </div>
            <p className="text-xs text-muted-foreground leading-relaxed">
              {ll.settings.ingress_custom_mode_desc()}
            </p>
          </div>
          <div className="pt-2 border-t border-border/50">
            <div className="flex items-center justify-between text-[11px]">
              <span className="text-muted-foreground">{ll.settings.ingress_route_format_label()}:</span>
              <code className="font-mono text-emerald-600 dark:text-emerald-400 font-medium">/*</code>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
