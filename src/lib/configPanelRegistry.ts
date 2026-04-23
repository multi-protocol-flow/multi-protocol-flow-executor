/**
 * 配置面板注册表
 * 将 14 种节点类型映射到对应的 Config 组件，消除 if/else 分支分发
 * 未知节点类型通过 isSchemaDriven() 标记，由消费者使用 SchemaConfigPanel 渲染
 */
import type { ComponentType } from 'react';

import { EntryConfig } from '@/components/panels/EntryConfig';
import { HttpConfig } from '@/components/panels/HttpConfig';
import { ConditionalConfig } from '@/components/panels/ConditionalConfig';
import { WsConnectConfig } from '@/components/panels/WsConnectConfig';
import { WsSendCollectConfig } from '@/components/panels/WsSendCollectConfig';
import { WsCloseConfig } from '@/components/panels/WsCloseConfig';
import { AssertionConfig } from '@/components/panels/AssertionConfig';
import { DbConnectConfig } from '@/components/panels/DbConnectConfig';
import { DbQueryConfig } from '@/components/panels/DbQueryConfig';
import { DbBeginConfig } from '@/components/panels/DbBeginConfig';
import { DbCommitConfig } from '@/components/panels/DbCommitConfig';
import { DbRollbackConfig } from '@/components/panels/DbRollbackConfig';
import { DbCloseConfig } from '@/components/panels/DbCloseConfig';
import { ScriptConfig } from '@/components/panels/ScriptConfig';
import { GrpcConnectConfig } from '@/components/panels/GrpcConnectConfig';
import { GrpcCallConfig } from '@/components/panels/GrpcCallConfig';
import { GrpcCloseConfig } from '@/components/panels/GrpcCloseConfig';

export const configPanelRegistry: Record<string, ComponentType<any>> = {
  entry: EntryConfig,
  http: HttpConfig,
  conditional: ConditionalConfig,
  ws_connect: WsConnectConfig,
  ws_send_collect: WsSendCollectConfig,
  ws_close: WsCloseConfig,
  assertion: AssertionConfig,
  db_connect: DbConnectConfig,
  db_query: DbQueryConfig,
  db_begin: DbBeginConfig,
  db_commit: DbCommitConfig,
  db_rollback: DbRollbackConfig,
  db_close: DbCloseConfig,
  script: ScriptConfig,
  grpc_connect: GrpcConnectConfig,
  grpc_call: GrpcCallConfig,
  grpc_close: GrpcCloseConfig,
};

/** 根据节点类型获取对应的配置面板组件 */
export function getConfigComponent(type: string): ComponentType<any> | undefined {
  return configPanelRegistry[type];
}

/** 判断节点类型是否需要 Schema 驱动渲染（未知类型 → true） */
export function isSchemaDriven(type: string): boolean {
  return !configPanelRegistry[type];
}
