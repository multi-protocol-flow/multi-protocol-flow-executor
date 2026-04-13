/**
 * DebugNodeConfigDialog - 调试模式节点配置弹窗
 * 复用节点配置面板组件，允许在调试模式下临时修改节点配置
 */
import { useState, useEffect } from 'react';
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { AlertTriangle, Settings2 } from 'lucide-react';
import { toast } from 'sonner';

// 节点配置组件
import { EntryConfig } from '@/components/panels/EntryConfig';
import { HttpConfig } from '@/components/panels/HttpConfig';
import { ConditionalConfig } from '@/components/panels/ConditionalConfig';
import { WsConnectConfig } from '@/components/panels/WsConnectConfig';
import { WsSendCollectConfig } from '@/components/panels/WsSendCollectConfig';
import { WsCloseConfig } from '@/components/panels/WsCloseConfig';

// 类型和类型守卫
import type { NodeData } from '@/types';
import {
  isEntryConfig,
  isHttpConfig,
  isConditionalConfig,
  isWsConnectConfig,
  isWsSendCollectConfig,
  isWsCloseConfig,
} from '@/types';

export interface DebugNodeConfigDialogProps {
  /** 是否显示对话框 */
  open: boolean;
  /** 当前节点数据 */
  nodeData: NodeData | null;
  /** 节点名称 */
  nodeName: string;
  /** 保存回调 */
  onSave: (data: NodeData) => Promise<void>;
  /** 取消回调 */
  onCancel: () => void;
}

/**
 * 节点类型显示名称映射
 */
const NODE_TYPE_LABELS: Record<string, string> = {
  entry: '入口节点',
  http: 'HTTP 请求',
  conditional: '条件分支',
  ws_connect: 'WebSocket 连接',
  ws_send_collect: 'WebSocket 发送采集',
  ws_close: 'WebSocket 关闭',
};

export function DebugNodeConfigDialog({
  open,
  nodeData,
  nodeName,
  onSave,
  onCancel,
}: DebugNodeConfigDialogProps) {
  // 内部编辑状态（用于跟踪修改）
  const [editedData, setEditedData] = useState<NodeData | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  // 当 nodeData 变化时，重置编辑状态
  useEffect(() => {
    setEditedData(nodeData);
  }, [nodeData]);

  // 处理保存
  const handleSave = async () => {
    if (!editedData) return;

    setIsSaving(true);
    try {
      await onSave(editedData);
      toast.success('节点配置已临时修改');
      onCancel(); // 关闭弹窗
    } catch (error) {
      toast.error(`修改配置失败: ${(error as Error).message}`);
    } finally {
      setIsSaving(false);
    }
  };

  // 处理配置组件的保存回调
  const handleConfigSave = (data: NodeData) => {
    setEditedData(data);
  };

  // 处理弹窗关闭
  const handleOpenChange = (isOpen: boolean) => {
    if (!isOpen) {
      onCancel();
    }
  };

  // 获取节点类型标签
  const nodeTypeLabel = nodeData ? NODE_TYPE_LABELS[nodeData.type] || nodeData.type : '';

  // 渲染配置组件
  const renderConfigComponent = () => {
    if (!editedData) return null;

    // 保存节点类型用于错误提示
    const nodeType = editedData.type;

    // 根据节点类型渲染对应的配置组件
    if (isEntryConfig(editedData)) {
      return (
        <EntryConfig
          data={editedData}
          onSave={(data) => handleConfigSave(data as NodeData)}
        />
      );
    }

    if (isHttpConfig(editedData)) {
      return (
        <HttpConfig
          data={editedData}
          onSave={(data) => handleConfigSave(data as NodeData)}
        />
      );
    }

    if (isConditionalConfig(editedData)) {
      return (
        <ConditionalConfig
          data={editedData}
          onSave={(data) => handleConfigSave(data as NodeData)}
        />
      );
    }

    if (isWsConnectConfig(editedData)) {
      return (
        <WsConnectConfig
          data={editedData}
          onSave={(data) => handleConfigSave(data as NodeData)}
        />
      );
    }

    if (isWsSendCollectConfig(editedData)) {
      return (
        <WsSendCollectConfig
          data={editedData}
          onSave={(data) => handleConfigSave(data as NodeData)}
        />
      );
    }

    if (isWsCloseConfig(editedData)) {
      return (
        <WsCloseConfig
          data={editedData}
          onSave={(data) => handleConfigSave(data as NodeData)}
        />
      );
    }

    return (
      <div className="text-sm text-muted-foreground text-center py-8">
        不支持的节点类型: {nodeType}
      </div>
    );
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent 
        className="w-full max-w-[600px] max-h-[85vh] flex flex-col p-0"
        hideCloseButton
      >
        {/* 固定头部区域 */}
        <div className="shrink-0 px-6 pt-6 pb-2">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-full bg-amber-100 flex items-center justify-center">
              <Settings2 className="w-5 h-5 text-amber-600" />
            </div>
            <div>
              <DialogTitle className="text-lg">编辑节点配置</DialogTitle>
              <p className="text-sm text-muted-foreground mt-0.5">
                {nodeName} · {nodeTypeLabel}
              </p>
            </div>
          </div>
        </div>

        {/* 可滚动内容区域（包含 sticky 提示卡片） */}
        <div className="flex-1 min-h-0 overflow-y-auto">
          {/* Sticky 提示卡片 - 滚动时吸顶 */}
          <div className="sticky top-0 z-10 bg-background px-6 py-3 border-b border-border">
            <div className="flex items-start gap-2 px-4 py-3 bg-amber-50 border border-amber-200 rounded-lg">
              <AlertTriangle className="w-4 h-4 text-amber-600 shrink-0 mt-0.5" />
              <div className="text-sm text-amber-800">
                <strong>仅调试会话生效：</strong>此修改仅影响当前调试会话，不会保存到流程数据中。调试结束后配置将自动丢弃。
              </div>
            </div>
          </div>

          {/* 配置内容 */}
          <div className="px-6 py-4">
            {renderConfigComponent()}
          </div>
        </div>

        {/* 固定底部区域 */}
        <div className="shrink-0 px-6 py-4 border-t border-border flex justify-end gap-2">
          <Button variant="outline" onClick={onCancel} disabled={isSaving}>
            取消
          </Button>
          <Button onClick={handleSave} disabled={isSaving || !editedData}>
            {isSaving ? '保存中...' : '保存修改'}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
