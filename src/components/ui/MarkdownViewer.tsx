/**
 * MarkdownViewer - 轻量级 Markdown 渲染组件
 * 基于 react-markdown + remark-gfm，内置表格横向滚动与代码块一键复制
 */
import { useState, useCallback, isValidElement, memo } from 'react';
import type { ComponentPropsWithoutRef, ReactNode } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { Copy, Check } from 'lucide-react';
import { cn } from '@/lib/utils';
import { getLL } from '@/i18n/i18n-lazy';

interface MarkdownViewerProps {
  /** Markdown 原文 */
  content: string;
  /** 是否启用紧凑模式（减少边距） */
  compact?: boolean;
  className?: string;
}

/** 从 ReactNode 中递归提取纯文本用于复制 */
function extractTextContent(node: ReactNode): string {
  if (typeof node === 'string') return node;
  if (typeof node === 'number') return String(node);
  if (Array.isArray(node)) return node.map(extractTextContent).join('');
  if (isValidElement<{ children?: ReactNode }>(node) && node.props?.children) {
    return extractTextContent(node.props.children);
  }
  return '';
}

/** 代码块复制按钮 */
function CodeBlockCopyButton({ text }: { text: string }) {
  const ll = getLL();
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // 忽略剪贴板权限错误
    }
  }, [text]);

  return (
    <button
      type="button"
      className="inline-flex items-center gap-1 text-[10px] font-sans text-muted-foreground hover:text-foreground transition-colors px-1.5 py-0.5 rounded bg-muted/60 hover:bg-muted"
      onClick={handleCopy}
      title={copied ? ll.ai.code.copied() : ll.ai.code.copy()}
    >
      {copied ? (
        <>
          <Check className="h-3 w-3 text-emerald-500" />
          <span className="text-emerald-500">{ll.ai.code.copied()}</span>
        </>
      ) : (
        <>
          <Copy className="h-3 w-3" />
          <span>{ll.ai.code.copy()}</span>
        </>
      )}
    </button>
  );
}

/** 默认的 Markdown 渲染样式 */
const proseClass = cn(
  'prose prose-sm dark:prose-invert max-w-none break-words overflow-hidden',
  'prose-headings:scroll-mt-12',
  'prose-code:rounded prose-code:bg-muted prose-code:px-1.5 prose-code:py-0.5 prose-code:text-[12px] prose-code:font-mono prose-code:break-all',
  'prose-a:text-primary prose-a:no-underline hover:prose-a:underline prose-a:break-all',
  'prose-img:rounded-lg',
  'prose-blockquote:border-l-primary prose-blockquote:text-muted-foreground',
);

const compactClass =
  'prose-p:my-1 prose-headings:mb-2 prose-headings:mt-3 prose-ul:my-1 prose-ol:my-1 prose-li:my-0';

const REMARK_PLUGINS = [remarkGfm];

const MARKDOWN_COMPONENTS = {
  pre: ({ children, ...props }: ComponentPropsWithoutRef<'pre'>) => {
    const rawCode = extractTextContent(children);
    return (
      <div className="relative my-2 overflow-hidden rounded-md border border-border/60 bg-muted/40 text-xs">
        <div className="flex items-center justify-between px-2.5 py-1 bg-muted/50 border-b border-border/40 text-[10px] text-muted-foreground font-mono">
          <span>Code</span>
          <CodeBlockCopyButton text={rawCode} />
        </div>
        <pre
          {...props}
          className="p-2.5 overflow-x-auto max-w-full text-xs font-mono leading-relaxed bg-transparent border-0 m-0"
        >
          {children}
        </pre>
      </div>
    );
  },
  table: ({ children, ...props }: ComponentPropsWithoutRef<'table'>) => (
    <div className="my-2 overflow-x-auto max-w-full rounded border border-border/50">
      <table
        {...props}
        className="w-full text-left border-collapse text-xs my-0"
      >
        {children}
      </table>
    </div>
  ),
};

export const MarkdownViewer = memo(function MarkdownViewer({
  content,
  compact = false,
  className,
}: MarkdownViewerProps) {
  if (!content || content.trim().length === 0) {
    return null;
  }

  return (
    <div className={cn(proseClass, compact && compactClass, className)}>
      <ReactMarkdown
        remarkPlugins={REMARK_PLUGINS}
        components={MARKDOWN_COMPONENTS}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
});
