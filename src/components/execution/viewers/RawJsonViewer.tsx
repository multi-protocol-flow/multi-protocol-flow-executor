import { memo, useMemo, useState, useCallback } from 'react';
import { JsonView, defaultStyles } from 'react-json-view-lite';
import 'react-json-view-lite/dist/index.css';
import { CopyButton } from '@/components/common';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';
import {
  formatBytes,
  estimateDataSize,
  getAdaptiveExpandStrategy,
  downloadContentAsFile,
  JSON_VIEWER_THRESHOLDS,
} from './jsonViewerUtils';
import { CodeMirrorViewer } from './CodeMirrorViewer';
import { AlertTriangle, Download, ChevronsDown, ChevronsUp } from 'lucide-react';

export interface RawJsonViewerProps {
  /** JSON 数据（对象、数组、字符串或原始值） */
  data: unknown;
  /** 是否展示复制按钮（主要用于紧凑模式或未启用顶部工具栏时） */
  showCopy?: boolean;
  /** 容器额外样式 */
  className?: string;
  /**
   * 展示模式：
   * - 'tabs': 顶部提供 Tree / JSON / Raw 多视图切换及控制栏（默认）
   * - 'compact': 紧凑模式，仅显示自适应树，不带顶部切换栏
   */
  variant?: 'tabs' | 'compact';
  /** 默认初始激活的视图 */
  defaultView?: 'tree' | 'json' | 'raw';
}

/** 自定义样式：优化 JSON 显示效果 */
const customStyles = {
  ...defaultStyles,
  container: 'json-viewer-container',
  basicChildStyle: 'json-viewer-row',
  childFieldsContainer: 'json-viewer-nested',
  label: 'json-viewer-label',
  clickableLabel: 'json-viewer-label json-viewer-label-clickable',
  collapseIcon: 'json-viewer-icon json-viewer-collapse',
  expandIcon: 'json-viewer-icon json-viewer-expand',
  punctuation: 'json-viewer-punctuation',
  nullValue: 'json-viewer-null',
  undefinedValue: 'json-viewer-undefined',
  stringValue: 'json-viewer-string',
  numberValue: 'json-viewer-number',
  booleanValue: 'json-viewer-boolean',
  otherValue: 'json-viewer-other',
  quotesForFieldNames: true,
};

type ViewMode = 'tree' | 'json' | 'raw';

/**
 * JSON & 响应数据展示组件
 * 支持多视图切换（Tree / CodeMirror 虚拟化 JSON / Raw）、自适应展开层级与超大体积防护
 */
export const RawJsonViewer = memo(function RawJsonViewer({
  data,
  showCopy = false,
  className,
  variant = 'tabs',
  defaultView,
}: RawJsonViewerProps) {
  const ll = getLL();

  // 1. 原始值提前处理（null / undefined / number / boolean）
  const copyButton = showCopy && data !== undefined && (
    <div className="absolute top-1.5 right-1.5 z-10 opacity-70 hover:opacity-100 transition-opacity">
      <CopyButton value={data} format="json" size="xs" />
    </div>
  );

  if (data === null) {
    return (
      <div className={cn('raw-json-viewer overflow-hidden relative select-text cursor-text', className)}>
        {copyButton}
        <span data-testid="primitive-value" className="text-sm font-mono text-muted-foreground italic">null</span>
      </div>
    );
  }

  if (data === undefined) {
    return (
      <div className={cn('raw-json-viewer overflow-hidden relative select-text cursor-text', className)}>
        {copyButton}
        <span data-testid="primitive-value" className="text-sm font-mono text-muted-foreground italic">undefined</span>
      </div>
    );
  }

  if (typeof data === 'number') {
    return (
      <div className={cn('raw-json-viewer overflow-hidden relative select-text cursor-text', className)}>
        {copyButton}
        <span data-testid="primitive-value" className="text-sm font-mono text-blue-600 dark:text-blue-400">{data}</span>
      </div>
    );
  }

  if (typeof data === 'boolean') {
    return (
      <div className={cn('raw-json-viewer overflow-hidden relative select-text cursor-text', className)}>
        {copyButton}
        <span data-testid="primitive-value" className="text-sm font-mono text-purple-600 dark:text-purple-400">{String(data)}</span>
      </div>
    );
  }

  // 2. 解析与准备数据
  const { isJsonObject, parsedData, formattedText, rawText, byteSize } = useMemo(() => {
    if (typeof data === 'string') {
      try {
        const parsed = JSON.parse(data);
        if (typeof parsed === 'object' && parsed !== null) {
          const formatted = JSON.stringify(parsed, null, 2);
          return {
            isJsonObject: true,
            parsedData: parsed as Record<string, unknown> | unknown[],
            formattedText: formatted,
            rawText: data,
            byteSize: estimateDataSize(parsed, data),
          };
        }
        // 解析为非对象（如字符串字面量、数字等原始值）
        return {
          isJsonObject: false,
          parsedData: null,
          formattedText: String(parsed),
          rawText: data,
          byteSize: data.length,
        };
      } catch {
        // 非 JSON 字符串
        return {
          isJsonObject: false,
          parsedData: null,
          formattedText: data,
          rawText: data,
          byteSize: data.length,
        };
      }
    }

    if (typeof data === 'object') {
      try {
        const formatted = JSON.stringify(data, null, 2);
        return {
          isJsonObject: true,
          parsedData: data as Record<string, unknown> | unknown[],
          formattedText: formatted,
          rawText: formatted,
          byteSize: estimateDataSize(data, formatted),
        };
      } catch {
        return {
          isJsonObject: true,
          parsedData: data as Record<string, unknown> | unknown[],
          formattedText: '[Circular or Unserializable Object]',
          rawText: '[Circular or Unserializable Object]',
          byteSize: 0,
        };
      }
    }

    // 兜底
    const str = String(data);
    return {
      isJsonObject: false,
      parsedData: null,
      formattedText: str,
      rawText: str,
      byteSize: str.length,
    };
  }, [data]);

  // 非 JSON 对象时，若为紧凑模式或简单纯文本，保持向下兼容的 plain-text-view
  if (!isJsonObject) {
    if (variant === 'compact' || byteSize < JSON_VIEWER_THRESHOLDS.SMALL) {
      return (
        <div className={cn('raw-json-viewer overflow-hidden relative select-text cursor-text', className)}>
          {copyButton}
          <pre data-testid="plain-text-view" className="text-sm font-mono whitespace-pre-wrap break-all">{rawText}</pre>
        </div>
      );
    }
  }

  // 3. 确定默认选中的视图
  const initialView: ViewMode = useMemo(() => {
    if (defaultView) {
      return defaultView;
    }
    if (!isJsonObject) {
      return 'raw';
    }
    // 超过 1MB 时默认切到 JSON (CodeMirror) 视图，确保秒开
    if (byteSize >= JSON_VIEWER_THRESHOLDS.MEDIUM) {
      return 'json';
    }
    return 'tree';
  }, [defaultView, isJsonObject, byteSize]);

  const [activeTab, setActiveTab] = useState<ViewMode>(initialView);
  const [expandOverride, setExpandOverride] = useState<'all' | 'collapse' | null>(null);
  const [forceRenderTree, setForceRenderTree] = useState(false);

  // 切换全部展开 / 全部折叠
  const handleToggleExpandAll = useCallback(() => {
    setExpandOverride((prev) => (prev === 'all' ? 'collapse' : 'all'));
  }, []);

  // 下载内容到本地文件
  const handleDownload = useCallback(() => {
    const isJsonFile = activeTab !== 'raw' && isJsonObject;
    const filename = isJsonFile ? 'response.json' : 'response.txt';
    const mime = isJsonFile ? 'application/json' : 'text/plain';
    const contentToDownload = activeTab === 'raw' ? rawText : formattedText;
    downloadContentAsFile(contentToDownload, filename, mime);
  }, [activeTab, isJsonObject, rawText, formattedText]);

  // 展开层级策略
  const expandStrategy = useMemo(() => {
    return getAdaptiveExpandStrategy(byteSize, expandOverride);
  }, [byteSize, expandOverride]);

  // 若为紧凑模式直接渲染树
  if (variant === 'compact') {
    return (
      <div className={cn('raw-json-viewer overflow-hidden relative select-text cursor-text', className)}>
        {copyButton}
        {isJsonObject && parsedData ? (
          <JsonView
            data={parsedData}
            shouldExpandNode={expandStrategy}
            style={customStyles}
          />
        ) : (
          <pre data-testid="plain-text-view" className="text-sm font-mono whitespace-pre-wrap break-all">{rawText}</pre>
        )}
      </div>
    );
  }

  const sizeLabel = formatBytes(byteSize);
  const isExtremeLarge = byteSize >= JSON_VIEWER_THRESHOLDS.EXTREME;

  return (
    <div className={cn('raw-json-viewer flex flex-col rounded border border-border/70 bg-card overflow-hidden', className)}>
      {/* 顶部工具栏 */}
      <div className="flex flex-wrap items-center justify-between gap-2 px-2.5 py-1.5 bg-muted/40 border-b border-border/70 select-none">
        {/* 左侧 Tab 切换 */}
        <div className="flex items-center gap-1">
          {isJsonObject && (
            <button
              type="button"
              onClick={() => setActiveTab('tree')}
              className={cn(
                'px-2 py-0.5 text-xs rounded font-medium transition-colors',
                activeTab === 'tree'
                  ? 'bg-background text-foreground shadow-xs'
                  : 'text-muted-foreground hover:text-foreground hover:bg-muted'
              )}
            >
              {ll.execution.viewers.tab_tree()}
            </button>
          )}
          {isJsonObject && (
            <button
              type="button"
              onClick={() => setActiveTab('json')}
              className={cn(
                'px-2 py-0.5 text-xs rounded font-medium transition-colors',
                activeTab === 'json'
                  ? 'bg-background text-foreground shadow-xs'
                  : 'text-muted-foreground hover:text-foreground hover:bg-muted'
              )}
            >
              {ll.execution.viewers.tab_json()}
            </button>
          )}
          <button
            type="button"
            onClick={() => setActiveTab('raw')}
            className={cn(
              'px-2 py-0.5 text-xs rounded font-medium transition-colors',
              activeTab === 'raw'
                ? 'bg-background text-foreground shadow-xs'
                : 'text-muted-foreground hover:text-foreground hover:bg-muted'
            )}
          >
            {ll.execution.viewers.tab_raw()}
          </button>
        </div>

        {/* 右侧动作与信息 */}
        <div className="flex items-center gap-2">
          {/* Tree 视图下的展开/折叠按钮 */}
          {activeTab === 'tree' && isJsonObject && (!isExtremeLarge || forceRenderTree) && (
            <button
              type="button"
              onClick={handleToggleExpandAll}
              title={expandOverride === 'all' ? ll.execution.viewers.collapse_all() : ll.execution.viewers.expand_all()}
              className="flex items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground px-1.5 py-0.5 rounded hover:bg-muted transition-colors"
            >
              {expandOverride === 'all' ? (
                <>
                  <ChevronsUp className="w-3 h-3" />
                  <span>{ll.execution.viewers.collapse_all()}</span>
                </>
              ) : (
                <>
                  <ChevronsDown className="w-3 h-3" />
                  <span>{ll.execution.viewers.expand_all()}</span>
                </>
              )}
            </button>
          )}

          {/* 大小展示 */}
          <span className="text-[11px] font-mono text-muted-foreground">
            {ll.execution.viewers.size_label({ size: sizeLabel })}
          </span>

          {/* 复制按钮 */}
          <CopyButton
            value={activeTab === 'raw' ? rawText : (parsedData ?? rawText)}
            format={activeTab === 'raw' ? 'raw' : 'json'}
            size="xs"
          />

          {/* 下载按钮 */}
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={handleDownload}
            title={ll.execution.viewers.download_file()}
            className="h-6 w-6 text-muted-foreground hover:text-foreground"
          >
            <Download className="w-3 h-3" />
          </Button>
        </div>
      </div>

      {/* 内容区域 */}
      <div className="p-2 relative select-text cursor-text min-h-[60px] overflow-auto">
        {activeTab === 'tree' && isJsonObject && parsedData && (
          <>
            {isExtremeLarge && !forceRenderTree ? (
              <div className="p-4 rounded border border-amber-500/30 bg-amber-500/10 text-amber-900 dark:text-amber-200 flex flex-col items-center gap-2.5 my-2">
                <AlertTriangle className="w-5 h-5 text-amber-500 shrink-0" />
                <p className="text-xs text-center font-medium max-w-md">
                  {ll.execution.viewers.extreme_data_warning({ size: sizeLabel })}
                </p>
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={() => setForceRenderTree(true)}
                  className="text-xs h-7"
                >
                  {ll.execution.viewers.force_render_tree()}
                </Button>
              </div>
            ) : (
              <>
                {byteSize >= JSON_VIEWER_THRESHOLDS.SMALL && expandOverride === null && (
                  <div className="mb-2 px-2 py-1 bg-muted/60 text-muted-foreground rounded text-[11px] flex items-center gap-1.5">
                    <span className="w-1.5 h-1.5 rounded-full bg-amber-500" />
                    <span>{ll.execution.viewers.large_data_warning({ size: sizeLabel })}</span>
                  </div>
                )}
                <JsonView
                  data={parsedData}
                  shouldExpandNode={expandStrategy}
                  style={customStyles}
                />
              </>
            )}
          </>
        )}

        {activeTab === 'json' && (
          <CodeMirrorViewer value={formattedText} mode="json" />
        )}

        {activeTab === 'raw' && (
          <CodeMirrorViewer value={rawText} mode="raw" />
        )}
      </div>
    </div>
  );
});
