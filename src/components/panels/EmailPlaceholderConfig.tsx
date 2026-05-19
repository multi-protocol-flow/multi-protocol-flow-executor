/**
 * Email 节点占位配置面板
 * 临时面板，T27-T31 将创建完整的 Email 配置面板
 */
import type { NodeData } from '@/types';
import { Mail } from 'lucide-react';
import { getLL } from '@/i18n/i18n-lazy';

interface EmailPlaceholderConfigProps {
  /** 当前节点数据 */
  data: NodeData;
}

/**
 * Email 节点占位配置面板
 * 在正式面板创建前提供基本的数据透传
 */
export function EmailPlaceholderConfig({ data }: EmailPlaceholderConfigProps) {
  const ll = getLL().panels;
  return (
    <div className="flex flex-col items-center justify-center py-8 text-muted-foreground">
      <Mail className="h-8 w-8 mb-3" />
      <p className="text-sm">{ll.emailUnderDevelopment()}</p>
      <p className="text-xs mt-1">{ll.currentNodeType({ type: data.type })}</p>
    </div>
  );
}
