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
  http: () => import('@/components/panels/HttpConfig').then(m => m.HttpConfig),
  conditional: () => import('@/components/panels/ConditionalConfig').then(m => m.ConditionalConfig),
  ws_connect: () => import('@/components/panels/WsConnectConfig').then(m => m.WsConnectConfig),
  ws_send_collect: () => import('@/components/panels/WsSendCollectConfig').then(m => m.WsSendCollectConfig),
  ws_close: () => import('@/components/panels/WsCloseConfig').then(m => m.WsCloseConfig),
  assertion: () => import('@/components/panels/AssertionConfig').then(m => m.AssertionConfig),
  db_connect: () => import('@/components/panels/DbConnectConfig').then(m => m.DbConnectConfig),
  db_query: () => import('@/components/panels/DbQueryConfig').then(m => m.DbQueryConfig),
  db_begin: () => import('@/components/panels/DbBeginConfig').then(m => m.DbBeginConfig),
  db_commit: () => import('@/components/panels/DbCommitConfig').then(m => m.DbCommitConfig),
  db_rollback: () => import('@/components/panels/DbRollbackConfig').then(m => m.DbRollbackConfig),
  db_close: () => import('@/components/panels/DbCloseConfig').then(m => m.DbCloseConfig),
  script: () => import('@/components/panels/ScriptConfig').then(m => m.ScriptConfig),
  grpc_connect: () => import('@/components/panels/GrpcConnectConfig').then(m => m.GrpcConnectConfig),
  grpc_call: () => import('@/components/panels/GrpcCallConfig').then(m => m.GrpcCallConfig),
  grpc_close: () => import('@/components/panels/GrpcCloseConfig').then(m => m.GrpcCloseConfig),
  mcp_connect: () => import('@/components/panels/McpConnectConfig').then(m => m.McpConnectConfig),
  mcp_call_tool: () => import('@/components/panels/McpCallToolConfig').then(m => m.McpCallToolConfig),
  mcp_read_resource: () => import('@/components/panels/McpReadResourceConfig').then(m => m.McpReadResourceConfig),
  mcp_close: () => import('@/components/panels/McpCloseConfig').then(m => m.McpCloseConfig),
  mcp_get_prompt: () => import('@/components/panels/McpGetPromptConfig').then(m => m.McpGetPromptConfig),
  tcp_connect: () => import('@/components/panels/TcpConnectConfig').then(m => m.TcpConnectConfig),
  tcp_send: () => import('@/components/panels/TcpSendConfig').then(m => m.TcpSendConfig),
  tcp_receive: () => import('@/components/panels/TcpReceiveConfig').then(m => m.TcpReceiveConfig),
  tcp_close: () => import('@/components/panels/TcpCloseConfig').then(m => m.TcpCloseConfig),
  udp_bind: () => import('@/components/panels/UdpBindConfig').then(m => m.UdpBindConfig),
  udp_send_to: () => import('@/components/panels/UdpSendToConfig').then(m => m.UdpSendToConfig),
  udp_recv_from: () => import('@/components/panels/UdpRecvFromConfig').then(m => m.UdpRecvFromConfig),
  udp_close: () => import('@/components/panels/UdpCloseConfig').then(m => m.UdpCloseConfig),
  mqtt_connect: () => import('@/components/panels/MqttConnectConfig').then(m => m.MqttConnectConfig),
  mqtt_publish: () => import('@/components/panels/MqttPublishConfig').then(m => m.MqttPublishConfig),
  mqtt_subscribe: () => import('@/components/panels/MqttSubscribeConfig').then(m => m.MqttSubscribeConfig),
  mqtt_unsubscribe: () => import('@/components/panels/MqttUnsubscribeConfig').then(m => m.MqttUnsubscribeConfig),
  mqtt_disconnect: () => import('@/components/panels/MqttDisconnectConfig').then(m => m.MqttDisconnectConfig),
  mqtt_wait_message: () => import('@/components/panels/MqttWaitMessageConfig').then(m => m.MqttWaitMessageConfig),
  mqtt_request: () => import('@/components/panels/MqttRequestConfig').then(m => m.MqttRequestConfig),
  kafka_connect: () => import('@/components/panels/KafkaConnectConfig').then(m => m.KafkaConnectConfig),
  kafka_produce: () => import('@/components/panels/KafkaProduceConfig').then(m => m.KafkaProduceConfig),
  kafka_consume: () => import('@/components/panels/KafkaConsumeConfig').then(m => m.KafkaConsumeConfig),
  kafka_consume_batch: () => import('@/components/panels/KafkaConsumeBatchConfig').then(m => m.KafkaConsumeBatchConfig),
  kafka_close: () => import('@/components/panels/KafkaCloseConfig').then(m => m.KafkaCloseConfig),
  kafka_admin: () => import('@/components/panels/KafkaAdminConfig').then(m => m.KafkaAdminConfig),
  kafka_schema_registry: () => import('@/components/panels/KafkaSchemaRegistryConfig').then(m => m.KafkaSchemaRegistryConfig),
  sse_connect: () => import('@/components/panels/SseConnectConfig').then(m => m.SseConnectConfig),
  sse_listen: () => import('@/components/panels/SseListenConfig').then(m => m.SseListenConfig),
  sse_disconnect: () => import('@/components/panels/SseDisconnectConfig').then(m => m.SseDisconnectConfig),
  graphql_connect: () => import('@/components/panels/GraphqlConnectConfig').then(m => m.GraphqlConnectConfig),
  graphql_query: () => import('@/components/panels/GraphqlQueryConfig').then(m => m.GraphqlQueryConfig),
  graphql_subscribe: () => import('@/components/panels/GraphqlSubscribeConfig').then(m => m.GraphqlSubscribeConfig),
  graphql_introspect: () => import('@/components/panels/GraphqlIntrospectConfig').then(m => m.GraphqlIntrospectConfig),
  graphql_disconnect: () => import('@/components/panels/GraphqlDisconnectConfig').then(m => m.GraphqlDisconnectConfig),
  // AMQP 0-9-1
  amqp_connect: () => import('@/components/panels/AmqpConnectConfigPanel').then(m => m.AmqpConnectConfigPanel),
  amqp_close: () => import('@/components/panels/AmqpCloseConfigPanel').then(m => m.AmqpCloseConfigPanel),
  amqp_exchange_declare: () => import('@/components/panels/AmqpExchangeDeclareConfigPanel').then(m => m.AmqpExchangeDeclareConfigPanel),
  amqp_exchange_delete: () => import('@/components/panels/AmqpExchangeDeleteConfigPanel').then(m => m.AmqpExchangeDeleteConfigPanel),
  amqp_queue_declare: () => import('@/components/panels/AmqpQueueDeclareConfigPanel').then(m => m.AmqpQueueDeclareConfigPanel),
  amqp_queue_delete: () => import('@/components/panels/AmqpQueueDeleteConfigPanel').then(m => m.AmqpQueueDeleteConfigPanel),
  amqp_queue_bind: () => import('@/components/panels/AmqpQueueBindConfigPanel').then(m => m.AmqpQueueBindConfigPanel),
  amqp_publish: () => import('@/components/panels/AmqpPublishConfigPanel').then(m => m.AmqpPublishConfigPanel),
  amqp_consume: () => import('@/components/panels/AmqpConsumeConfigPanel').then(m => m.AmqpConsumeConfigPanel),
  amqp_get: () => import('@/components/panels/AmqpGetConfigPanel').then(m => m.AmqpGetConfigPanel),
  amqp_ack: () => import('@/components/panels/AmqpAckConfigPanel').then(m => m.AmqpAckConfigPanel),
  amqp_management: () => import('@/components/panels/AmqpManagementConfigPanel').then(m => m.AmqpManagementConfigPanel),
  // AMQP 1.0
  amqp10_connect: () => import('@/components/panels/Amqp10ConnectConfigPanel').then(m => m.Amqp10ConnectConfigPanel),
  amqp10_send: () => import('@/components/panels/Amqp10SendConfigPanel').then(m => m.Amqp10SendConfigPanel),
  amqp10_receive: () => import('@/components/panels/Amqp10ReceiveConfigPanel').then(m => m.Amqp10ReceiveConfigPanel),
  amqp10_accept: () => import('@/components/panels/Amqp10AcceptConfigPanel').then(m => m.Amqp10AcceptConfigPanel),
  amqp10_reject: () => import('@/components/panels/Amqp10RejectConfigPanel').then(m => m.Amqp10RejectConfigPanel),
  amqp10_disconnect: () => import('@/components/panels/Amqp10DisconnectConfigPanel').then(m => m.Amqp10DisconnectConfigPanel),
  variable_extractor: () => import('@/components/panels/VariableExtractorConfig').then(m => m.VariableExtractorConfig),
  // SMTP
  smtp_connect: () => import('@/components/panels/SmtpConnectPanel').then(m => m.SmtpConnectConfig),
  smtp_send: () => import('@/components/panels/SmtpSendPanel').then(m => m.SmtpSendConfig),
  smtp_disconnect: () => import('@/components/panels/SmtpDisconnectPanel').then(m => m.SmtpDisconnectConfig),
  // IMAP
  imap_connect: () => import('@/components/panels/ImapConnectPanel').then(m => m.ImapConnectPanel),
  imap_select: () => import('@/components/panels/ImapSelectPanel').then(m => m.ImapSelectPanel),
  imap_search: () => import('@/components/panels/ImapSearchPanel').then(m => m.ImapSearchPanel),
  imap_fetch: () => import('@/components/panels/ImapFetchPanel').then(m => m.ImapFetchPanel),
  imap_store_flags: () => import('@/components/panels/ImapStoreFlagsPanel').then(m => m.ImapStoreFlagsPanel),
  imap_copy: () => import('@/components/panels/ImapCopyPanel').then(m => m.ImapCopyPanel),
  imap_move: () => import('@/components/panels/ImapMovePanel').then(m => m.ImapMovePanel),
  imap_expunge: () => import('@/components/panels/ImapExpungePanel').then(m => m.ImapExpungePanel),
  imap_list: () => import('@/components/panels/ImapListPanel').then(m => m.ImapListPanel),
  imap_folder_create: () => import('@/components/panels/ImapFolderCreatePanel').then(m => m.ImapFolderCreatePanel),
  imap_folder_delete: () => import('@/components/panels/ImapFolderDeletePanel').then(m => m.ImapFolderDeletePanel),
  imap_idle: () => import('@/components/panels/ImapIdlePanel').then(m => m.ImapIdlePanel),
  imap_close: () => import('@/components/panels/ImapClosePanel').then(m => m.ImapClosePanel),
  proxy: () => import('@/components/panels/ProxyConfig').then(m => m.ProxyConfig),
  // Redis
  redis_connect: () => import('@/components/panels/RedisConnectConfig').then(m => m.RedisConnectConfig),
  redis_command: () => import('@/components/panels/RedisCommandConfig').then(m => m.RedisCommandConfig),
  redis_close: () => import('@/components/panels/RedisCloseConfig').then(m => m.RedisCloseConfig),
};

/** 根据节点类型获取对应的面板加载器 */
export function getPanelLoader(type: string): PanelLoader | undefined {
  return panelLoaders[type];
}

/** 判断节点类型是否需要 Schema 驱动渲染（未知类型 → true） */
export function isSchemaDriven(type: string): boolean {
  return !panelLoaders[type];
}
