/**
 * 配置面板注册表（懒加载版）
 * 将节点类型映射到对应的 Config 组件加载器，按需动态导入
 * 未知节点类型通过 isSchemaDriven() 标记，由消费者使用 SchemaConfigPanel 渲染
 */
import type { ComponentType } from 'react';

/** 面板组件加载器类型 — 直接返回组件而非模块 */
type PanelLoader = () => Promise<ComponentType<any>>;

/**
 * 面板加载器注册表
 * 使用 () => import() 实现按需加载，只有用户首次编辑该类型节点时才加载对应面板
 * .then(m => m.Xxx) 将模块的命名导出提取出来
 */
const panelLoaders: Record<string, PanelLoader> = {
  entry: () => import('@/components/panels/EntryConfig').then(m => m.EntryConfig),
  end: () => import('@/components/panels/EndConfigPanel').then(m => m.EndConfigPanel),
  'flow:sub': () => import('@/components/panels/SubFlowConfig').then(m => m.SubFlowConfig),
  'http:request': () => import('@/components/panels/HttpConfig').then(m => m.HttpConfig),
  'http:ingress': () => import('@/components/panels/HttpIngressConfig').then(m => m.HttpIngressConfig),
  'http:respond': () => import('@/components/panels/HttpRespondConfig').then(m => m.HttpRespondConfig),
  conditional: () => import('@/components/panels/ConditionalConfig').then(m => m.ConditionalConfig),
  'ws:connect': () => import('@/components/panels/WsConnectConfig').then(m => m.WsConnectConfig),
  'ws:send_collect': () => import('@/components/panels/WsSendCollectConfig').then(m => m.WsSendCollectConfig),
  'ws:close': () => import('@/components/panels/WsCloseConfig').then(m => m.WsCloseConfig),
  assertion: () => import('@/components/panels/AssertionConfig').then(m => m.AssertionConfig),
  script: () => import('@/components/panels/ScriptConfig').then(m => m.ScriptConfig),
  'tcp:connect': () => import('@/components/panels/TcpConnectConfig').then(m => m.TcpConnectConfig),
  'tcp:send': () => import('@/components/panels/TcpSendConfig').then(m => m.TcpSendConfig),
  'tcp:receive': () => import('@/components/panels/TcpReceiveConfig').then(m => m.TcpReceiveConfig),
  'tcp:close': () => import('@/components/panels/TcpCloseConfig').then(m => m.TcpCloseConfig),
  'udp:bind': () => import('@/components/panels/UdpBindConfig').then(m => m.UdpBindConfig),
  'udp:send_to': () => import('@/components/panels/UdpSendToConfig').then(m => m.UdpSendToConfig),
  'udp:recv_from': () => import('@/components/panels/UdpRecvFromConfig').then(m => m.UdpRecvFromConfig),
  'udp:close': () => import('@/components/panels/UdpCloseConfig').then(m => m.UdpCloseConfig),
  'sse:connect': () => import('@/components/panels/SseConnectConfig').then(m => m.SseConnectConfig),
  'sse:listen': () => import('@/components/panels/SseListenConfig').then(m => m.SseListenConfig),
  'sse:disconnect': () => import('@/components/panels/SseDisconnectConfig').then(m => m.SseDisconnectConfig),
  'graphql:connect': () => import('@/components/panels/GraphqlConnectConfig').then(m => m.GraphqlConnectConfig),
  'graphql:query': () => import('@/components/panels/GraphqlQueryConfig').then(m => m.GraphqlQueryConfig),
  'graphql:subscribe': () => import('@/components/panels/GraphqlSubscribeConfig').then(m => m.GraphqlSubscribeConfig),
  'graphql:introspect': () => import('@/components/panels/GraphqlIntrospectConfig').then(m => m.GraphqlIntrospectConfig),
  'graphql:disconnect': () => import('@/components/panels/GraphqlDisconnectConfig').then(m => m.GraphqlDisconnectConfig),
  'variable:extractor': () => import('@/components/panels/VariableExtractorConfig').then(m => m.VariableExtractorConfig),
  proxy: () => import('@/components/panels/ProxyConfig').then(m => m.ProxyConfig),
};

/** 根据节点类型获取对应的面板加载器 */
export function getPanelLoader(type: string): PanelLoader | undefined {
  return panelLoaders[type];
}

/** 判断节点类型是否需要 Schema 驱动渲染（未知类型 → true） */
export function isSchemaDriven(type: string): boolean {
  return !panelLoaders[type];
}
