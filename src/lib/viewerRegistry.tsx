/**
 * 执行结果查看器注册表
 * 将节点类型映射到对应的 Viewer 组件，替代 switch/case 分发
 */
import type { ComponentType } from 'react';
import type { NodeExecutionReport } from '@/types';
import { getLL } from '@/i18n/i18n-lazy';
import { HttpNodeViewer } from '@/components/execution/viewers/HttpNodeViewer';
import { WsConnectViewer } from '@/components/execution/viewers/WsConnectViewer';
import { WsSendCollectViewer } from '@/components/execution/viewers/WsSendCollectViewer';
import { WsCloseViewer } from '@/components/execution/viewers/WsCloseViewer';
import { ConditionalViewer } from '@/components/execution/viewers/ConditionalViewer';
import { AssertionViewer } from '@/components/execution/viewers/AssertionViewer';
import { GenericViewer } from '@/components/execution/viewers/GenericViewer';
import { TcpUdpViewer } from '@/components/execution/viewers/TcpUdpViewer';
import { SseViewer } from '@/components/execution/viewers/SseViewer';
import { GraphqlViewer } from '@/components/execution/viewers/GraphqlViewer';
import { PluginIframeViewer } from '@/components/execution/viewers/PluginIframeViewer';
import { getCachedNodeMeta } from '@/lib/NodeRegistry';

interface ViewerProps {
  report: NodeExecutionReport;
}

/** End 节点查看器 — 显示流程结束信息 */
function EndNodeViewer({ report }: ViewerProps) {
  const ll = getLL();
  return (
    <div className="p-4 text-center">
      <div className="text-green-600 text-lg font-semibold mb-2">
        {ll.execution.report.end_node_reached()}
      </div>
      <p className="text-sm text-muted-foreground">
        {report.node_name}
      </p>
    </div>
  );
}

/**
 * 节点类型 → 专用 Viewer 组件映射
 * 未注册的类型将使用 GenericViewer 兜底
 */
export const viewerRegistry: Record<string, ComponentType<ViewerProps>> = {
  end: EndNodeViewer,
  http: HttpNodeViewer,
  ws_connect: WsConnectViewer,
  ws_send_collect: WsSendCollectViewer,
  ws_close: WsCloseViewer,
  conditional: ConditionalViewer,
  assertion: AssertionViewer,
  tcp_connect: TcpUdpViewer,
  tcp_send: TcpUdpViewer,
  tcp_receive: TcpUdpViewer,
  tcp_close: TcpUdpViewer,
  udp_bind: TcpUdpViewer,
  udp_send_to: TcpUdpViewer,
  udp_recv_from: TcpUdpViewer,
  udp_close: TcpUdpViewer,
  sse_connect: SseViewer,
  sse_listen: SseViewer,
  sse_disconnect: SseViewer,
  graphql_connect: GraphqlViewer,
  graphql_query: GraphqlViewer,
  graphql_subscribe: GraphqlViewer,
  graphql_introspect: GraphqlViewer,
  graphql_disconnect: GraphqlViewer,
  variable_extractor: GenericViewer,
  proxy: GenericViewer,
};

/**
 * 根据节点类型获取对应的 Viewer 组件
 * - 已注册类型 → 精确 viewer（与 PR-4 之前完全一致，零回归）
 * - 未注册类型 + 插件执行载荷（report.plugin_data 存在）→ PluginIframeViewer
 * - 其余未注册类型 → GenericViewer（兜底）
 *
 * 插件检测规则：以「未注册类型 + plugin_data 存在」为准（而非类型名含 `:`），
 * 因为 plugin_data 是插件执行载荷的权威信号；无 plugin_data 的插件节点
 * （如校验失败未执行）继续走 GenericViewer 展示错误，体验更好。
 * `report` 可选：旧调用方不传时行为与 PR-4 之前完全一致。
 */
export function getViewerComponent(
  nodeType: string,
  report?: NodeExecutionReport
): ComponentType<ViewerProps> {
  const registered = viewerRegistry[nodeType];
  if (registered) return registered;
  // 插件分支：如果插件声明了 viewer 前端，或者 report 携带 plugin_data，使用 PluginIframeViewer
  const meta = getCachedNodeMeta(nodeType) || (report?.node_type ? getCachedNodeMeta(report.node_type) : null);
  if (meta?.viewer) {
    return PluginIframeViewer;
  }
  if (report && report.plugin_data !== undefined) {
    return PluginIframeViewer;
  }
  return GenericViewer;
}
