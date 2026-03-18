/**
 * SQL 编辑器组件
 * 基于 Monaco Editor 实现 SQL 语法高亮和 {{variable}} 变量高亮
 */
import * as React from 'react';
import Editor, { OnMount, OnChange } from '@monaco-editor/react';
import type { editor } from 'monaco-editor';
import { cn } from '@/lib/utils';

// 变量高亮的 Token 类型名称
const VARIABLE_TOKEN_TYPE = 'custom.variable';

/**
 * SqlEditor Props
 */
export interface SqlEditorProps {
  /** 编辑器内容值 */
  value: string;
  /** 值变化回调 */
  onChange?: (value: string) => void;
  /** 占位符文本 */
  placeholder?: string;
  /** 是否禁用 */
  disabled?: boolean;
  /** 自定义类名 */
  className?: string;
  /** 编辑器高度 */
  height?: number | string;
  /** 是否显示格式化按钮 */
  showFormatButton?: boolean;
}

/**
 * 配置 Monaco 编辑器的 SQL 语言支持
 * 添加 {{variable}} 语法高亮
 */
function configureSqlLanguage(monaco: typeof import('monaco-editor')) {
  // 定义变量 token 类型
  monaco.languages.setMonarchTokensProvider('sql', {
    // 保留 SQL 默认 token
    tokenizer: {
      root: [
        // 变量语法 {{variable}} - 必须在其他规则之前
        [/\{\{/, { token: VARIABLE_TOKEN_TYPE, bracket: '@open' }],
        [/\}\}/, { token: VARIABLE_TOKEN_TYPE, bracket: '@close' }],
        // 变量内容
        [/\{\{[^}]*\}\}/, VARIABLE_TOKEN_TYPE],
        // 字符串
        [/'/, 'string', '@string_single'],
        [/"/, 'string', '@string_double'],
        // 数字
        [/\d+/, 'number'],
        // 标识符和关键字
        [/[a-zA-Z_]\w*/, {
          cases: {
            '@keywords': 'keyword',
            '@operators': 'operator',
            '@builtinFunctions': 'predefined',
            '@default': 'identifier'
          }
        }],
        // 空白
        [/\s+/, 'white'],
        // 注释
        [/--.*$/, 'comment'],
        [/\/\*/, 'comment', '@comment'],
      ],
      string_single: [
        [/[^']+/, 'string'],
        [/'/, 'string', '@pop']
      ],
      string_double: [
        [/[^"]+/, 'string'],
        [/"/, 'string', '@pop']
      ],
      comment: [
        [/[^\/*]+/, 'comment'],
        [/\/\*/, 'comment', '@push'],
        ['\\*/', 'comment', '@pop'],
        [/[\/*]/, 'comment']
      ],
    },
    keywords: [
      'SELECT', 'FROM', 'WHERE', 'INSERT', 'UPDATE', 'DELETE', 'CREATE', 'DROP',
      'ALTER', 'TABLE', 'INDEX', 'VIEW', 'DATABASE', 'SCHEMA', 'AND', 'OR', 'NOT',
      'IN', 'EXISTS', 'BETWEEN', 'LIKE', 'IS', 'NULL', 'AS', 'ORDER', 'BY', 'ASC',
      'DESC', 'LIMIT', 'OFFSET', 'GROUP', 'HAVING', 'JOIN', 'LEFT', 'RIGHT', 'INNER',
      'OUTER', 'ON', 'UNION', 'ALL', 'DISTINCT', 'VALUES', 'SET', 'INTO', 'PRIMARY',
      'KEY', 'FOREIGN', 'REFERENCES', 'CONSTRAINT', 'UNIQUE', 'DEFAULT', 'CHECK',
      'CASCADE', 'RESTRICT', 'WITH', 'CASE', 'WHEN', 'THEN', 'ELSE', 'END', 'IF',
      'WHILE', 'FOR', 'DECLARE', 'BEGIN', 'COMMIT', 'ROLLBACK', 'TRUNCATE', 'GRANT',
      'REVOKE', 'EXEC', 'EXECUTE', 'PROCEDURE', 'FUNCTION', 'TRIGGER', 'RETURNING',
    ],
    operators: [
      '=', '>', '<', '!', '<=', '>=', '<>', '!=', '+', '-', '*', '/', '%',
    ],
    builtinFunctions: [
      'COUNT', 'SUM', 'AVG', 'MIN', 'MAX', 'COALESCE', 'NULLIF', 'CAST', 'CONVERT',
      'CONCAT', 'SUBSTRING', 'LENGTH', 'TRIM', 'UPPER', 'LOWER', 'REPLACE',
      'NOW', 'CURRENT_DATE', 'CURRENT_TIME', 'CURRENT_TIMESTAMP', 'DATE', 'YEAR',
      'MONTH', 'DAY', 'ABS', 'ROUND', 'FLOOR', 'CEILING', 'MOD', 'POWER', 'SQRT',
      'RAND', 'IFNULL', 'NVL', 'DECODE', 'ROW_NUMBER', 'RANK', 'DENSE_RANK',
      'LAG', 'LEAD', 'FIRST_VALUE', 'LAST_VALUE', 'OVER', 'PARTITION',
    ],
  });

  // 定义变量高亮样式
  monaco.editor.defineTheme('sql-editor-theme', {
    base: 'vs',
    inherit: true,
    rules: [
      // 变量 {{xxx}} 使用紫色背景高亮
      { token: VARIABLE_TOKEN_TYPE, foreground: '7c3aed', background: 'f3e8ff', fontStyle: 'bold' },
      // SQL 关键字
      { token: 'keyword', foreground: '0066cc', fontStyle: 'bold' },
      // 字符串
      { token: 'string', foreground: '008800' },
      // 数字
      { token: 'number', foreground: 'ff6600' },
      // 注释
      { token: 'comment', foreground: '666666', fontStyle: 'italic' },
      // 内置函数
      { token: 'predefined', foreground: '9966cc' },
      // 标识符
      { token: 'identifier', foreground: '333333' },
    ],
    colors: {
      'editor.background': '#ffffff',
      'editor.foreground': '#1a1a1a',
      'editor.lineHighlightBackground': '#f5f5f5',
      'editorCursor.foreground': '#000000',
      'editor.selectionBackground': '#add6ff',
    },
  });

  // 深色主题
  monaco.editor.defineTheme('sql-editor-theme-dark', {
    base: 'vs-dark',
    inherit: true,
    rules: [
      // 变量 {{xxx}} 使用紫色高亮
      { token: VARIABLE_TOKEN_TYPE, foreground: 'a78bfa', background: '3b1f5c', fontStyle: 'bold' },
      // SQL 关键字
      { token: 'keyword', foreground: '569cd6', fontStyle: 'bold' },
      // 字符串
      { token: 'string', foreground: 'ce9178' },
      // 数字
      { token: 'number', foreground: 'b5cea8' },
      // 注释
      { token: 'comment', foreground: '6a9955', fontStyle: 'italic' },
      // 内置函数
      { token: 'predefined', foreground: 'dcdcaa' },
      // 标识符
      { token: 'identifier', foreground: 'd4d4d4' },
    ],
    colors: {
      'editor.background': '#1e1e1e',
      'editor.foreground': '#d4d4d4',
      'editor.lineHighlightBackground': '#2d2d2d',
      'editorCursor.foreground': '#ffffff',
      'editor.selectionBackground': '#264f78',
    },
  });
}

/**
 * 检测系统是否使用深色模式
 */
function useSystemDarkMode(): boolean {
  const [isDark, setIsDark] = React.useState(false);

  React.useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    setIsDark(mediaQuery.matches);

    const handler = (e: MediaQueryListEvent) => setIsDark(e.matches);
    mediaQuery.addEventListener('change', handler);
    return () => mediaQuery.removeEventListener('change', handler);
  }, []);

  return isDark;
}

/**
 * SQL 编辑器组件
 * 支持 SQL 语法高亮和 {{variable}} 变量语法高亮
 */
export const SqlEditor = React.forwardRef<editor.IStandaloneCodeEditor, SqlEditorProps>(
  (
    {
      value,
      onChange,
      placeholder,
      disabled = false,
      className,
      height = 120,
      showFormatButton = false,
    },
    ref
  ) => {
    const isDark = useSystemDarkMode();
    const editorRef = React.useRef<editor.IStandaloneCodeEditor | null>(null);
    const [showPlaceholder, setShowPlaceholder] = React.useState(!value);

    // 处理编辑器挂载
    const handleEditorMount: OnMount = (editor, monaco) => {
      editorRef.current = editor;
      
      // 转发 ref
      if (typeof ref === 'function') {
        ref(editor);
      } else if (ref) {
        ref.current = editor;
      }

      // 配置 SQL 语言
      configureSqlLanguage(monaco);

      // 设置编辑器选项
      editor.updateOptions({
        minimap: { enabled: false },
        scrollBeyondLastLine: false,
        lineNumbers: 'off',
        glyphMargin: false,
        folding: false,
        lineDecorationsWidth: 0,
        lineNumbersMinChars: 0,
        renderLineHighlight: 'none',
        scrollbar: {
          vertical: 'auto',
          horizontal: 'auto',
          verticalScrollbarSize: 8,
          horizontalScrollbarSize: 8,
        },
        padding: { top: 8, bottom: 8 },
        fontSize: 12,
        fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace',
      });
    };

    // 处理值变化
    const handleChange: OnChange = (value) => {
      const newValue = value || '';
      setShowPlaceholder(newValue.length === 0);
      onChange?.(newValue);
    };

    // 格式化 SQL（简单实现）
    const formatSql = React.useCallback(() => {
      if (!editorRef.current) return;
      
      const currentValue = editorRef.current.getValue();
      // 简单格式化：关键字大写，基本缩进
      let formatted = currentValue
        .replace(/\s+/g, ' ')
        .replace(/\s*,\s*/g, ', ')
        .replace(/\s*=\s*/g, ' = ')
        .replace(/\s+/g, ' ')
        .trim();
      
      // 关键字大写
      const keywords = [
        'SELECT', 'FROM', 'WHERE', 'AND', 'OR', 'ORDER BY', 'GROUP BY',
        'HAVING', 'LIMIT', 'OFFSET', 'INSERT INTO', 'VALUES', 'UPDATE',
        'SET', 'DELETE FROM', 'JOIN', 'LEFT JOIN', 'RIGHT JOIN', 'INNER JOIN',
        'ON', 'AS', 'IN', 'NOT IN', 'EXISTS', 'BETWEEN', 'LIKE', 'IS NULL',
        'IS NOT NULL', 'ASC', 'DESC', 'DISTINCT', 'COUNT', 'SUM', 'AVG',
        'MIN', 'MAX', 'CASE', 'WHEN', 'THEN', 'ELSE', 'END',
      ];
      
      for (const kw of keywords) {
        const regex = new RegExp(`\\b${kw}\\b`, 'gi');
        formatted = formatted.replace(regex, kw);
      }
      
      // 基本换行
      formatted = formatted
        .replace(/\bSELECT\b/gi, '\nSELECT')
        .replace(/\bFROM\b/gi, '\nFROM')
        .replace(/\bWHERE\b/gi, '\nWHERE')
        .replace(/\bORDER BY\b/gi, '\nORDER BY')
        .replace(/\bGROUP BY\b/gi, '\nGROUP BY')
        .replace(/\bHAVING\b/gi, '\nHAVING')
        .replace(/\bLIMIT\b/gi, '\nLIMIT')
        .replace(/\bJOIN\b/gi, '\nJOIN')
        .replace(/\bLEFT JOIN\b/gi, '\nLEFT JOIN')
        .replace(/\bRIGHT JOIN\b/gi, '\nRIGHT JOIN')
        .replace(/\bINNER JOIN\b/gi, '\nINNER JOIN')
        .replace(/\bAND\b/gi, '\n  AND')
        .replace(/\bOR\b/gi, '\n  OR')
        .replace(/^\n/, '')
        .replace(/\n\s*\n/g, '\n');
      
      editorRef.current.setValue(formatted);
      onChange?.(formatted);
    }, [onChange]);

    // 计算高度样式
    const heightStyle = typeof height === 'number' ? `${height}px` : height;

    return (
      <div className={cn('relative', className)}>
        <div 
          className={cn(
            'overflow-hidden rounded-md border border-black bg-white',
            disabled && 'cursor-not-allowed opacity-50'
          )}
        >
          <Editor
            height={heightStyle}
            defaultLanguage="sql"
            value={value}
            onChange={handleChange}
            onMount={handleEditorMount}
            theme={isDark ? 'sql-editor-theme-dark' : 'sql-editor-theme'}
            options={{
              readOnly: disabled,
            }}
          />
        </div>
        
        {/* 占位符 */}
        {showPlaceholder && placeholder && !disabled && (
          <div 
            className="pointer-events-none absolute left-3 top-2 text-sm text-muted-foreground"
            aria-hidden="true"
          >
            {placeholder}
          </div>
        )}
        
        {/* 格式化按钮 */}
        {showFormatButton && !disabled && (
          <button
            type="button"
            onClick={formatSql}
            className="absolute right-2 top-2 rounded bg-muted px-2 py-1 text-xs text-muted-foreground hover:bg-muted/80 hover:text-foreground transition-colors"
            title="格式化 SQL"
          >
            格式化
          </button>
        )}
      </div>
    );
  }
);

SqlEditor.displayName = 'SqlEditor';
