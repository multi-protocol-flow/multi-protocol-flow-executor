/**
 * DbResultPreview - 数据库查询结果预览组件
 * 支持 JSON 格式化显示和表格视图，显示执行时间、行数、截断警告
 */
import { useState, useCallback } from 'react';
import { Table, Braces, Clock, Rows, AlertTriangle, Copy, Check } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { Badge } from '@/components/ui/badge';
import { RawJsonViewer } from '@/components/execution/viewers/RawJsonViewer';
import { cn } from '@/lib/utils';

/**
 * 数据库查询结果数据结构
 */
export interface DbQueryResult {
  /** 查询返回的行 */
  rows?: Record<string, unknown>[];
  /** 影响的行数（INSERT/UPDATE/DELETE） */
  affected?: number;
  /** 结果是否被截断 */
  truncated?: boolean;
  /** 执行时间（毫秒） */
  timing_ms?: number;
  /** 错误信息 */
  error?: string;
}

interface DbResultPreviewProps {
  /** 查询结果数据 */
  data: DbQueryResult | null | undefined;
  /** 视图模式 */
  mode?: 'json' | 'table';
  /** 自定义类名 */
  className?: string;
}

/**
 * 数据库查询结果预览组件
 */
export function DbResultPreview({ 
  data, 
  mode = 'table',
  className 
}: DbResultPreviewProps) {
  const [currentMode, setCurrentMode] = useState<'json' | 'table'>(mode);
  const [copied, setCopied] = useState(false);

  const rows = data?.rows ?? [];
  const hasRows = rows.length > 0;
  const hasAffected = data?.affected !== undefined && data.affected > 0;
  const hasError = !!data?.error;

  // 复制结果到剪贴板
  const handleCopy = useCallback(async () => {
    if (!data) return;
    
    try {
      const text = JSON.stringify(data, null, 2);
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // 静默处理复制失败
    }
  }, [data]);

  // 获取表格列名（从所有行中提取所有可能的键）
  const getColumns = (): string[] => {
    if (!hasRows) return [];
    const columnSet = new Set<string>();
    rows.forEach(row => {
      Object.keys(row).forEach(key => columnSet.add(key));
    });
    return Array.from(columnSet);
  };

  const columns = getColumns();

  // 渲染单元格值
  const renderCellValue = (value: unknown): React.ReactNode => {
    if (value === null) {
      return <span className="text-muted-foreground italic">NULL</span>;
    }
    if (value === undefined) {
      return <span className="text-muted-foreground italic">undefined</span>;
    }
    if (typeof value === 'boolean') {
      return (
        <Badge variant="outline" className="font-mono text-xs">
          {value ? 'true' : 'false'}
        </Badge>
      );
    }
    if (typeof value === 'number') {
      return <span className="font-mono text-purple-600">{value}</span>;
    }
    if (typeof value === 'object') {
      return (
        <span className="font-mono text-xs bg-muted px-1 rounded">
          {JSON.stringify(value)}
        </span>
      );
    }
    return <span>{String(value)}</span>;
  };

  // 空状态
  if (!data) {
    return (
      <div className={cn('flex items-center justify-center py-8 text-muted-foreground', className)}>
        <span className="text-sm">暂无查询结果</span>
      </div>
    );
  }

  // 错误状态
  if (hasError) {
    return (
      <div className={cn('space-y-3', className)}>
        {/* 元信息栏 */}
        <div className="flex items-center justify-between border-b border-border pb-2">
          <div className="flex items-center gap-2">
            <Badge variant="destructive" className="text-xs">
              查询错误
            </Badge>
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleCopy}
            className="h-7 px-2"
          >
            {copied ? (
              <>
                <Check className="w-3.5 h-3.5 mr-1" />
                已复制
              </>
            ) : (
              <>
                <Copy className="w-3.5 h-3.5 mr-1" />
                复制
              </>
            )}
          </Button>
        </div>
        
        {/* 错误信息 */}
        <div className="bg-destructive/10 border border-destructive/30 rounded-md p-3">
          <pre className="text-sm text-destructive font-mono whitespace-pre-wrap break-all">
            {data.error}
          </pre>
        </div>

        {/* 执行时间 */}
        {data.timing_ms !== undefined && (
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <Clock className="w-3.5 h-3.5" />
            <span>执行时间: {data.timing_ms}ms</span>
          </div>
        )}
      </div>
    );
  }

  return (
    <div className={cn('space-y-3', className)}>
      {/* 元信息栏 */}
      <div className="flex items-center justify-between border-b border-border pb-2">
        <div className="flex items-center gap-3 flex-wrap">
          {/* 行数 */}
          {hasRows && (
            <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
              <Rows className="w-3.5 h-3.5" />
              <span>{rows.length} 行</span>
            </div>
          )}
          
          {/* 影响行数 */}
          {hasAffected && (
            <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
              <span className="font-medium">{data.affected}</span>
              <span>行受影响</span>
            </div>
          )}
          
          {/* 执行时间 */}
          {data.timing_ms !== undefined && (
            <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
              <Clock className="w-3.5 h-3.5" />
              <span>{data.timing_ms}ms</span>
            </div>
          )}
          
          {/* 截断警告 */}
          {data.truncated && (
            <div className="flex items-center gap-1.5 text-xs text-amber-600">
              <AlertTriangle className="w-3.5 h-3.5" />
              <span>结果已截断</span>
            </div>
          )}
        </div>
        
        {/* 复制按钮 */}
        <Button
          variant="ghost"
          size="sm"
          onClick={handleCopy}
          className="h-7 px-2"
        >
          {copied ? (
            <>
              <Check className="w-3.5 h-3.5 mr-1" />
              已复制
            </>
          ) : (
            <>
              <Copy className="w-3.5 h-3.5 mr-1" />
              复制
            </>
          )}
        </Button>
      </div>

      {/* 视图切换 */}
      {hasRows && (
        <Tabs 
          value={currentMode} 
          onValueChange={(v) => setCurrentMode(v as 'json' | 'table')}
          className="w-full"
        >
          <TabsList className="h-8">
            <TabsTrigger value="table" className="h-6 px-2.5 text-xs">
              <Table className="w-3.5 h-3.5 mr-1" />
              表格
            </TabsTrigger>
            <TabsTrigger value="json" className="h-6 px-2.5 text-xs">
              <Braces className="w-3.5 h-3.5 mr-1" />
              JSON
            </TabsTrigger>
          </TabsList>

          {/* 表格视图 */}
          <TabsContent value="table" className="mt-2">
            <div className="border border-border rounded-md overflow-hidden">
              <div className="overflow-x-auto scrollbar-thin">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="bg-muted/50 border-b border-border">
                      {columns.map(col => (
                        <th
                          key={col}
                          className="text-left px-3 py-2 font-medium text-foreground whitespace-nowrap"
                        >
                          {col}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border">
                    {rows.map((row, rowIndex) => (
                      <tr 
                        key={rowIndex} 
                        className="hover:bg-muted/30 transition-colors"
                      >
                        {columns.map(col => (
                          <td 
                            key={col} 
                            className="px-3 py-2 text-foreground"
                          >
                            {renderCellValue(row[col])}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </TabsContent>

          {/* JSON 视图 */}
          <TabsContent value="json" className="mt-2">
            <div className="border border-border rounded-md p-3 bg-muted overflow-auto max-h-96 scrollbar-thin">
              <RawJsonViewer data={rows} />
            </div>
          </TabsContent>
        </Tabs>
      )}

      {/* 无行结果（仅 affected 或空结果） */}
      {!hasRows && hasAffected && (
        <div className="flex items-center justify-center py-4 text-muted-foreground">
          <span className="text-sm">
            查询成功，{data.affected} 行受影响
          </span>
        </div>
      )}

      {!hasRows && !hasAffected && (
        <div className="flex items-center justify-center py-4 text-muted-foreground">
          <span className="text-sm">查询返回空结果</span>
        </div>
      )}
    </div>
  );
}
