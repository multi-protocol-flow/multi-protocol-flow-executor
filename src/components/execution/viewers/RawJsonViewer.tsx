import { memo } from 'react';
import { JsonView, allExpanded, defaultStyles } from 'react-json-view-lite';
import 'react-json-view-lite/dist/index.css';
import { CopyButton } from '@/components/common';
import { cn } from '@/lib/utils';

interface RawJsonViewerProps {
  /** JSON 数据 */
  data: unknown;
  /** 是否展示复制按钮 */
  showCopy?: boolean;
  /** 容器额外样式 */
  className?: string;
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

/**
 * JSON 数据展示组件
 * 使用 react-json-view-lite 提供语法高亮和折叠/展开功能
 * 支持多种数据类型：对象/数组（JSON 树）、字符串（纯文本/JSON 解析）、原始值
 */
export const RawJsonViewer = memo(function RawJsonViewer({ data, showCopy = false, className }: RawJsonViewerProps) {
  const copyButton = showCopy && data !== undefined && (
    <div className="absolute top-1.5 right-1.5 z-10 opacity-70 hover:opacity-100 transition-opacity">
      <CopyButton value={data} format="json" size="xs" />
    </div>
  );

  // null 值
  if (data === null) {
    return (
      <div className={cn("raw-json-viewer overflow-hidden relative select-text cursor-text", className)}>
        {copyButton}
        <span data-testid="primitive-value" className="text-sm font-mono text-gray-500">null</span>
      </div>
    );
  }

  // undefined 值
  if (data === undefined) {
    return (
      <div className={cn("raw-json-viewer overflow-hidden relative select-text cursor-text", className)}>
        <span data-testid="primitive-value" className="text-sm font-mono text-gray-500">undefined</span>
      </div>
    );
  }

  // 数字
  if (typeof data === 'number') {
    return (
      <div className={cn("raw-json-viewer overflow-hidden relative select-text cursor-text", className)}>
        {copyButton}
        <span data-testid="primitive-value" className="text-sm font-mono text-blue-600">{data}</span>
      </div>
    );
  }
  // 布尔值
  if (typeof data === 'boolean') {
    return (
      <div className={cn("raw-json-viewer overflow-hidden relative select-text cursor-text", className)}>
        {copyButton}
        <span data-testid="primitive-value" className="text-sm font-mono text-amber-600">{String(data)}</span>
      </div>
    );
  }

  // 对象/数组 → JSON 树
  if (typeof data === 'object') {
    return (
      <div className={cn("raw-json-viewer overflow-hidden relative select-text cursor-text", className)}>
        {copyButton}
        <JsonView
          data={data as Record<string, unknown> | unknown[]}
          shouldExpandNode={allExpanded}
          style={customStyles}
        />
      </div>
    );
  }

  // 字符串 → 尝试 JSON 解析
  if (typeof data === 'string') {
    try {
      const parsed: unknown = JSON.parse(data);
      // 解析成功且为对象/数组 → 显示 JSON 树
      if (typeof parsed === 'object' && parsed !== null) {
        return (
          <div className={cn("raw-json-viewer overflow-hidden relative select-text cursor-text", className)}>
            {copyButton}
            <JsonView
              data={parsed as Record<string, unknown> | unknown[]}
              shouldExpandNode={allExpanded}
              style={customStyles}
            />
          </div>
        );
      }
      // 解析成功但为原始值（数字/字符串/布尔）→ 显示格式化文本
      return (
        <div className={cn("raw-json-viewer overflow-hidden relative select-text cursor-text", className)}>
          {copyButton}
          <pre data-testid="plain-text-view" className="text-sm font-mono whitespace-pre-wrap break-all">{String(parsed)}</pre>
        </div>
      );
    } catch {
      // 不是有效 JSON → 显示纯文本
      return (
        <div className={cn("raw-json-viewer overflow-hidden relative select-text cursor-text", className)}>
          {copyButton}
          <pre data-testid="plain-text-view" className="text-sm font-mono whitespace-pre-wrap break-all">{data}</pre>
        </div>
      );
    }
  }

  // 其他类型（兜底）
  return (
    <div className={cn("raw-json-viewer overflow-hidden relative select-text cursor-text", className)}>
      {copyButton}
      <span data-testid="primitive-value" className="text-sm font-mono text-gray-500">{String(data)}</span>
    </div>
  );
});
