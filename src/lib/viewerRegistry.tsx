/**
 * 执行结果查看器注册表
 * 将节点类型映射到对应的 Viewer 组件，替代 switch/case 分发
 */
import type { ComponentType } from 'react';
import type { NodeExecutionReport } from '@/types';
import { HttpNodeViewer } from '@/components/execution/viewers/HttpNodeViewer';
import { WsConnectViewer } from '@/components/execution/viewers/WsConnectViewer';
import { WsSendCollectViewer } from '@/components/execution/viewers/WsSendCollectViewer';
import { WsCloseViewer } from '@/components/execution/viewers/WsCloseViewer';
import { ConditionalViewer } from '@/components/execution/viewers/ConditionalViewer';
import { AssertionViewer } from '@/components/execution/viewers/AssertionViewer';
import { GenericViewer } from '@/components/execution/viewers/GenericViewer';

interface ViewerProps {
  report: NodeExecutionReport;
}

/**
 * 节点类型 → 专用 Viewer 组件映射
 * 未注册的类型将使用 GenericViewer 兜底
 */
export const viewerRegistry: Record<string, ComponentType<ViewerProps>> = {
  http: HttpNodeViewer,
  ws_connect: WsConnectViewer,
  ws_send_collect: WsSendCollectViewer,
  ws_close: WsCloseViewer,
  conditional: ConditionalViewer,
  assertion: AssertionViewer,
};

/**
 * 根据节点类型获取对应的 Viewer 组件
 * 未注册的类型返回 GenericViewer
 */
export function getViewerComponent(nodeType: string): ComponentType<ViewerProps> {
  return viewerRegistry[nodeType] ?? GenericViewer;
}
