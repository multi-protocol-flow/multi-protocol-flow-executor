/**
 * MarkdownViewer - 轻量级 Markdown 渲染组件
 * 基于 react-markdown + remark-gfm
 */
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { cn } from '@/lib/utils';

interface MarkdownViewerProps {
  /** Markdown 原文 */
  content: string;
  /** 是否启用紧凑模式（减少边距） */
  compact?: boolean;
  className?: string;
}

/** 默认的 Markdown 渲染样式 */
const proseClass = cn(
  'prose prose-sm dark:prose-invert max-w-none',
  'prose-headings:scroll-mt-12',
  'prose-code:rounded prose-code:bg-muted prose-code:px-1.5 prose-code:py-0.5 prose-code:text-[13px] prose-code:font-mono',
  'prose-pre:bg-muted prose-pre:border prose-pre:border-border',
  'prose-a:text-primary prose-a:no-underline hover:prose-a:underline',
  'prose-img:rounded-lg',
  'prose-table:text-sm',
  'prose-blockquote:border-l-primary prose-blockquote:text-muted-foreground',
);

const compactClass = 'prose-p:my-1 prose-headings:mb-2 prose-headings:mt-3 prose-ul:my-1 prose-ol:my-1 prose-li:my-0';

export function MarkdownViewer({ content, compact = false, className }: MarkdownViewerProps) {
  if (!content || content.trim().length === 0) {
    return null;
  }

  return (
    <div className={cn(proseClass, compact && compactClass, className)}>
      <ReactMarkdown remarkPlugins={[remarkGfm]}>
        {content}
      </ReactMarkdown>
    </div>
  );
}
