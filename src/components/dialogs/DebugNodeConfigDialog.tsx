/**
 * DebugNodeConfigDialog - 调试模式节点配置弹窗
 * 复用节点配置面板组件，允许在调试模式下临时修改节点配置
 */
import { useState, useEffect, useCallback, useRef } from 'react';
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { AlertTriangle, Settings2 } from 'lucide-react';
import { toast } from 'sonner';
import { getLL } from '@/i18n/i18n-lazy';

import { getPanelLoader, isSchemaDriven } from '@/lib/configPanelRegistry';
import { SchemaConfigPanel } from '@/components/panels/SchemaConfigPanel';
import { NODE_META } from '@/lib/nodeMeta';

// 类型
import type { NodeData } from '@/types';

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

/** 调试弹窗懒加载面板组件 — 手动加载，避免 React.lazy 动态创建的身份丢失问题 */
function DebugLazyConfigPanel({ nodeType, data, onSave: onConfigSave }: { nodeType: string; data: NodeData; onSave: (data: NodeData) => void }) {
  const loader = getPanelLoader(nodeType);
  const [Component, setComponent] = useState<React.ComponentType<any> | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;
    if (!loader) {
      setComponent(null);
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    loader().then(comp => {
      if (!cancelled) {
        setComponent(() => comp);
        setIsLoading(false);
      }
    });

    return () => { cancelled = true; };
  }, [nodeType, loader]);

  if (!loader) return <div className="text-muted-foreground">不支持的节点类型</div>;

  if (isLoading || !Component) {
    return <div className="space-y-4 p-4 animate-pulse"><div className="h-24 bg-muted rounded-md" /></div>;
  }

  return <Component data={data} onSave={onConfigSave} />;
}

export function DebugNodeConfigDialog({
  open,
  nodeData,
  nodeName,
  onSave,
  onCancel,
}: DebugNodeConfigDialogProps) {
  const ll = getLL();
  // 内部编辑状态（用于跟踪修改）
  const [editedData, setEditedData] = useState<NodeData | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  // 使用 ref 持久化最新 editedData，避免异步闭包读取到旧值
  const editedDataRef = useRef<NodeData | null>(null);

  // 当 nodeData 变化时，重置编辑状态
  useEffect(() => {
    setEditedData(nodeData);
    editedDataRef.current = nodeData;
  }, [nodeData]);

  // 处理配置组件的保存回调
  const handleConfigSave = useCallback((data: NodeData) => {
    setEditedData(data);
    editedDataRef.current = data;
  }, []);

  // 处理保存：先通过 flush-config-panel 事件强制当前 config 组件提交表单，
  // 等待 onSave 回调更新 editedData 后再发送到后端
  const handleSave = async () => {
    if (!editedDataRef.current) return;

    setIsSaving(true);
    try {
      // 触发 config 组件内部的 form 提交，确保表单数据同步
      // 所有 config 组件都监听此事件并在回调中调用 onSave
      window.dispatchEvent(new CustomEvent('flush-config-panel'));

      // 等待一个微任务周期，让 config 组件的 onSave 回调更新 editedDataRef
      await new Promise((resolve) => setTimeout(resolve, 150));

      // 通过 ref 读取最新数据（flush 回调可能已更新）
      const latestData = editedDataRef.current;
      if (!latestData) return;

      await onSave(latestData);
      toast.success(ll.flow.debug_config_save_success());
      onCancel(); // 关闭弹窗
    } catch (error) {
      toast.error(ll.flow.debug_config_save_fail({ msg: (error as Error).message }));
    } finally {
      setIsSaving(false);
    }
  };

  // 处理弹窗关闭
  const handleOpenChange = (isOpen: boolean) => {
    if (!isOpen) {
      onCancel();
    }
  };

  // 获取节点类型标签
  const nodeTypeLabel = nodeData ? (NODE_META[nodeData.type as keyof typeof NODE_META]?.name ?? nodeData.type) : '';

  // 渲染配置组件（懒加载）
  const renderConfigComponent = () => {
    if (!editedData) return null;

    const nodeType = editedData.type;

    // 未知节点类型 → SchemaConfigPanel fallback（同步渲染）
    if (isSchemaDriven(nodeType)) {
      return (
        <SchemaConfigPanel
          schema={{}}
          data={editedData as unknown as Record<string, unknown>}
          onSave={(data) => handleConfigSave(data as unknown as NodeData)}
        />
      );
    }

    // 已知节点类型 → 懒加载对应的配置面板
    return (
      <DebugLazyConfigPanel nodeType={nodeType} data={editedData} onSave={handleConfigSave} />
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
