import { memo, useMemo } from 'react';
import CodeMirror from '@uiw/react-codemirror';
import { javascript } from '@codemirror/lang-javascript';
import { oneDark } from '@codemirror/theme-one-dark';
import { useTheme } from '@/hooks/useTheme';
import { cn } from '@/lib/utils';

interface CodeMirrorViewerProps {
  /** 待展示的文本内容 */
  value: string;
  /** 语法模式：json 语法高亮或纯文本 raw */
  mode?: 'json' | 'raw';
  /** 容器自定义 class */
  className?: string;
  /** 高度，默认 auto */
  height?: string;
  /** 最大高度，默认 500px */
  maxHeight?: string;
}

export const CodeMirrorViewer = memo(function CodeMirrorViewer({
  value,
  mode = 'json',
  className,
  height = 'auto',
  maxHeight = '500px',
}: CodeMirrorViewerProps) {
  const { resolvedTheme } = useTheme();

  const extensions = useMemo(() => {
    if (mode === 'json') {
      return [javascript()];
    }
    return [];
  }, [mode]);

  return (
    <div
      data-testid={mode === 'json' ? 'codemirror-json-viewer' : 'codemirror-raw-viewer'}
      className={cn(
        'rounded border border-border/80 overflow-hidden font-mono text-xs select-text cursor-text',
        className
      )}
    >
      <CodeMirror
        value={value}
        height={height}
        maxHeight={maxHeight}
        theme={resolvedTheme === 'dark' ? oneDark : undefined}
        extensions={extensions}
        editable={false}
        readOnly={true}
        basicSetup={{
          lineNumbers: true,
          foldGutter: mode === 'json',
          highlightActiveLine: false,
          dropCursor: false,
          allowMultipleSelections: false,
          indentOnInput: false,
          bracketMatching: mode === 'json',
          tabSize: 2,
        }}
      />
    </div>
  );
});
