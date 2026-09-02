import { useState, useEffect } from 'react';
import { Timer, Cpu, Layers, Puzzle, ChevronRight } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { useSettingsStore } from '@/stores/settings';
import { useUIStore } from '@/stores/ui';
import { getLL } from '@/i18n/i18n-lazy';
import { cn } from '@/lib/utils';

export function EngineSettingsTab() {
  const {
    defaultExecutionTimeoutMs,
    pluginMaxConcurrency,
    pluginQueueCapacity,
    setDefaultExecutionTimeoutMs,
    setPluginMaxConcurrency,
    setPluginQueueCapacity,
  } = useSettingsStore();

  const openPluginSettingsDialog = useUIStore((s) => s.openPluginSettingsDialog);
  const ll = getLL();

  const [timeoutInput, setTimeoutInput] = useState<string>(
    defaultExecutionTimeoutMs !== undefined ? String(defaultExecutionTimeoutMs) : '30000'
  );
  const [concurrencyInput, setConcurrencyInput] = useState<string>(
    pluginMaxConcurrency !== undefined ? String(pluginMaxConcurrency) : '256'
  );
  const [queueCapInput, setQueueCapInput] = useState<string>(
    pluginQueueCapacity !== undefined ? String(pluginQueueCapacity) : '2048'
  );

  useEffect(() => {
    setTimeoutInput(defaultExecutionTimeoutMs !== undefined ? String(defaultExecutionTimeoutMs) : '30000');
  }, [defaultExecutionTimeoutMs]);

  useEffect(() => {
    setConcurrencyInput(pluginMaxConcurrency !== undefined ? String(pluginMaxConcurrency) : '256');
  }, [pluginMaxConcurrency]);

  useEffect(() => {
    setQueueCapInput(pluginQueueCapacity !== undefined ? String(pluginQueueCapacity) : '2048');
  }, [pluginQueueCapacity]);

  const timeoutPresets = [
    { label: '10s', value: 10000 },
    { label: '30s', value: 30000 },
    { label: '60s', value: 60000 },
    { label: '120s', value: 120000 },
    { label: '300s', value: 300000 },
  ];

  const concurrencyPresets = [
    { label: '64', value: 64 },
    { label: '128', value: 128 },
    { label: '256 (推荐)', value: 256 },
    { label: '512', value: 512 },
    { label: '1024', value: 1024 },
  ];

  const queuePresets = [
    { label: '512', value: 512 },
    { label: '1024', value: 1024 },
    { label: '2048 (推荐)', value: 2048 },
    { label: '4096', value: 4096 },
    { label: '8192', value: 8192 },
  ];

  const handleTimeoutPreset = (val: number) => {
    setTimeoutInput(String(val));
    setDefaultExecutionTimeoutMs(val);
  };

  const handleConcurrencyPreset = (val: number) => {
    setConcurrencyInput(String(val));
    setPluginMaxConcurrency(val);
  };

  const handleQueuePreset = (val: number) => {
    setQueueCapInput(String(val));
    setPluginQueueCapacity(val);
  };

  return (
    <div className="space-y-6">
      {/* 默认执行超时 */}
      <div className="rounded-lg border border-border/70 bg-card p-4 space-y-3">
        <div className="flex items-center gap-2">
          <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-muted text-foreground">
            <Timer className="h-4 w-4" />
          </div>
          <div>
            <div className="text-sm font-semibold text-foreground">
              {ll.settings.default_timeout?.() ?? 'Default Node Execution Timeout'}
            </div>
            <p className="text-xs text-muted-foreground mt-0.5">
              {ll.settings.default_timeout_tip?.() ?? 'Global fallback timeout for nodes.'}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <div className="relative w-48 shrink-0">
            <Input
              type="number"
              min={1000}
              max={3600000}
              step={1000}
              value={timeoutInput}
              onChange={(e) => {
                const text = e.target.value;
                setTimeoutInput(text);
                if (text !== '') {
                  const val = Number(text);
                  if (!isNaN(val) && val >= 1000 && val <= 3600000) {
                    setDefaultExecutionTimeoutMs(val);
                  }
                }
              }}
              onBlur={() => {
                const val = Number(timeoutInput);
                if (timeoutInput === '' || isNaN(val) || val < 1000) {
                  const fallback = 30000;
                  setTimeoutInput(String(fallback));
                  setDefaultExecutionTimeoutMs(fallback);
                } else if (val > 3600000) {
                  const clamped = 3600000;
                  setTimeoutInput(String(clamped));
                  setDefaultExecutionTimeoutMs(clamped);
                }
              }}
              className="pr-10 font-mono text-sm"
            />
            <span className="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-muted-foreground select-none">
              ms
            </span>
          </div>

          {/* 快速预设标签 */}
          <div className="flex items-center gap-1.5 flex-wrap">
            <span className="text-xs text-muted-foreground mr-1 select-none">
              {ll.settings.timeout_presets?.() ?? 'Presets'}:
            </span>
            {timeoutPresets.map((preset) => (
              <Button
                key={preset.value}
                type="button"
                variant={Number(timeoutInput) === preset.value ? 'secondary' : 'outline'}
                size="sm"
                className={cn(
                  'h-7 px-2 text-xs cursor-pointer',
                  Number(timeoutInput) === preset.value && 'font-semibold border-primary/50'
                )}
                onClick={() => handleTimeoutPreset(preset.value)}
              >
                {preset.label}
              </Button>
            ))}
          </div>
        </div>
      </div>

      {/* 插件最大并发数 */}
      <div className="rounded-lg border border-border/70 bg-card p-4 space-y-3">
        <div className="flex items-center gap-2">
          <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-muted text-foreground">
            <Cpu className="h-4 w-4" />
          </div>
          <div>
            <div className="text-sm font-semibold text-foreground">
              {ll.settings.plugin_max_concurrency?.() ?? 'Max Plugin Concurrency'}
            </div>
            <p className="text-xs text-muted-foreground mt-0.5">
              {ll.settings.plugin_max_concurrency_tip?.() ?? 'Max concurrent requests per plugin process.'}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <div className="w-48 shrink-0">
            <Input
              type="number"
              min={1}
              max={8192}
              step={32}
              value={concurrencyInput}
              onChange={(e) => {
                const text = e.target.value;
                setConcurrencyInput(text);
                if (text !== '') {
                  const val = Number(text);
                  if (!isNaN(val) && val >= 1 && val <= 8192) {
                    setPluginMaxConcurrency(val);
                  }
                }
              }}
              onBlur={() => {
                const val = Number(concurrencyInput);
                if (concurrencyInput === '' || isNaN(val) || val < 1) {
                  const fallback = 256;
                  setConcurrencyInput(String(fallback));
                  setPluginMaxConcurrency(fallback);
                } else if (val > 8192) {
                  const clamped = 8192;
                  setConcurrencyInput(String(clamped));
                  setPluginMaxConcurrency(clamped);
                }
              }}
              className="font-mono text-sm"
            />
          </div>

          <div className="flex items-center gap-1.5 flex-wrap">
            {concurrencyPresets.map((preset) => (
              <Button
                key={preset.value}
                type="button"
                variant={Number(concurrencyInput) === preset.value ? 'secondary' : 'outline'}
                size="sm"
                className={cn(
                  'h-7 px-2 text-xs cursor-pointer',
                  Number(concurrencyInput) === preset.value && 'font-semibold border-primary/50'
                )}
                onClick={() => handleConcurrencyPreset(preset.value)}
              >
                {preset.label}
              </Button>
            ))}
          </div>
        </div>
      </div>

      {/* 插件通信队列深度 */}
      <div className="rounded-lg border border-border/70 bg-card p-4 space-y-3">
        <div className="flex items-center gap-2">
          <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-muted text-foreground">
            <Layers className="h-4 w-4" />
          </div>
          <div>
            <div className="text-sm font-semibold text-foreground">
              {ll.settings.plugin_queue_capacity?.() ?? 'Plugin Queue Capacity'}
            </div>
            <p className="text-xs text-muted-foreground mt-0.5">
              {ll.settings.plugin_queue_capacity_tip?.() ?? 'Queue depth between host and plugin.'}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <div className="w-48 shrink-0">
            <Input
              type="number"
              min={16}
              max={65536}
              step={256}
              value={queueCapInput}
              onChange={(e) => {
                const text = e.target.value;
                setQueueCapInput(text);
                if (text !== '') {
                  const val = Number(text);
                  if (!isNaN(val) && val >= 16 && val <= 65536) {
                    setPluginQueueCapacity(val);
                  }
                }
              }}
              onBlur={() => {
                const val = Number(queueCapInput);
                if (queueCapInput === '' || isNaN(val) || val < 16) {
                  const fallback = 2048;
                  setQueueCapInput(String(fallback));
                  setPluginQueueCapacity(fallback);
                } else if (val > 65536) {
                  const clamped = 65536;
                  setQueueCapInput(String(clamped));
                  setPluginQueueCapacity(clamped);
                }
              }}
              className="font-mono text-sm"
            />
          </div>

          <div className="flex items-center gap-1.5 flex-wrap">
            {queuePresets.map((preset) => (
              <Button
                key={preset.value}
                type="button"
                variant={Number(queueCapInput) === preset.value ? 'secondary' : 'outline'}
                size="sm"
                className={cn(
                  'h-7 px-2 text-xs cursor-pointer',
                  Number(queueCapInput) === preset.value && 'font-semibold border-primary/50'
                )}
                onClick={() => handleQueuePreset(preset.value)}
              >
                {preset.label}
              </Button>
            ))}
          </div>
        </div>
      </div>

      {/* 插件管理快捷通道 */}
      <div className="rounded-lg border border-dashed border-border p-4 flex items-center justify-between gap-4 bg-muted/20">
        <div className="flex items-center gap-2.5">
          <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary">
            <Puzzle className="h-4 w-4" />
          </div>
          <div>
            <div className="text-sm font-medium text-foreground flex items-center gap-2">
              <span>{ll.plugins.settings_title?.() ?? 'Plugin Settings'}</span>
              <Badge variant="outline" className="text-[10px] px-1.5 py-0 h-4">
                Marketplace
              </Badge>
            </div>
            <p className="text-xs text-muted-foreground mt-0.5">
              {ll.plugins.source_default?.() ?? 'Manage installed plugins, custom paths, and plugin marketplace.'}
            </p>
          </div>
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={openPluginSettingsDialog}
          className="gap-1 text-xs shrink-0 cursor-pointer"
        >
          <span>{ll.settings.plugins_manager_link?.() ?? 'Open Plugins Manager'}</span>
          <ChevronRight className="h-3.5 w-3.5" />
        </Button>
      </div>
    </div>
  );
}
