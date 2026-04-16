/**
 * 本地存储服务
 * 提供流程的 CRUD 操作
 */

// 流程存储
export { flowStorage, type FlowMetadata, type FolderInfo } from './flowStorage';

// 数据迁移
export { migrateFlowsToFiles } from './migration';

// 压测报告存储
export { stressTestStorage } from './stressTestStorage';
