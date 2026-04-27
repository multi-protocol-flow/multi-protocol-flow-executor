/**
 * IMAP 共享组件和 Hook
 * 提供连接选择器等可复用的 IMAP 面盘子组件
 */
import { useMemo } from 'react';
import { useFlowStore } from '@/stores';
import { isImapConnectConfig } from '@/types/node';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';

/**
 * 获取当前流程中所有 IMAP Connect 节点
 */
export function useImapConnections() {
  const flow = useFlowStore((state) => state.flow);
  return useMemo(() => {
    if (!flow) return [];
    const connections: Array<{ uuid: string; name: string }> = [];
    flow.nodes.forEach(node => {
      if (isImapConnectConfig(node.data)) {
        connections.push({ uuid: node.uuid, name: node.name });
      }
    });
    return connections;
  }, [flow]);
}

interface ImapConnectionSelectorProps {
  value: string;
  onChange: (value: string) => void;
  connections: Array<{ uuid: string; name: string }>;
  error?: string;
  registerProps?: ReturnType<typeof Object>;
}

/**
 * IMAP 连接选择器
 * 如果流程中存在 IMAP Connect 节点则使用下拉选择，否则手动输入
 */
export function ImapConnectionSelector({ value, onChange, connections, error }: ImapConnectionSelectorProps) {
  return (
    <div className="space-y-2">
      <Label className="text-sm font-medium text-foreground">
        连接 <span className="text-rose-500">*</span>
      </Label>
      {connections.length > 0 ? (
        <Select value={value || ''} onValueChange={onChange}>
          <SelectTrigger className="font-mono">
            <SelectValue placeholder="选择 IMAP 连接" />
          </SelectTrigger>
          <SelectContent>
            {connections.map((conn) => (
              <SelectItem key={conn.uuid} value={conn.uuid}>
                <span className="font-mono">{conn.name}</span>
                <span className="ml-2 text-xs text-muted-foreground">
                  ({conn.uuid.substring(0, 8)}...)
                </span>
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      ) : (
        <>
          <Input
            value={value || ''}
            onChange={(e) => onChange(e.target.value)}
            placeholder="输入连接 ID"
            className="font-mono text-sm"
          />
          <p className="text-xs text-muted-foreground">
            未检测到 IMAP Connect 节点，请手动输入连接 ID
          </p>
        </>
      )}
      {error && <p className="text-xs text-rose-500">{error}</p>}
    </div>
  );
}
