/**
 * 本地命令审批与信任库 IPC 服务（`local:exec`）
 *
 * 后端信任库 `store/exec-trust.bin` 的读写与审批弹窗回填都经此模块；
 * 非 Tauri（浏览器 playground）环境没有宿主能力：列表返回空、审批直接放行失败。
 */
import { isTauriEnv } from '@/lib/platform';

// 说明：`@tauri-apps/api/core` 采用懒加载，因为浏览器 playground（非 Tauri）
// 没有宿主管线，静态引入会让该模块进入 web 入口的依赖图；与本目录其它
// 服务（`settings.ts` / `flow.ts`）保持一致。
async function tauriInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(command, args);
}

/** 已批准的信任记录（与后端 `ExecApprovalRecord` 对齐） */
export interface ExecApprovalRecord {
  /** 流程文件绝对路径（未保存流程为空） */
  flowPath?: string;
  /** 命令哈希 */
  commandHash: string;
  /** 批准时间（Unix 秒） */
  createdAt: number;
  /** 命令摘要（设置页展示） */
  display: string;
  /** 节点名称 */
  nodeName: string;
}

/** 审批请求（后端推送，与 `ExecApprovalRequest` 对齐） */
export interface ExecApprovalRequest {
  /** 请求 ID */
  requestId: string;
  /** 信任范围：流程绝对路径或 `session:<flow_uuid>` */
  flowScope: string;
  /** 流程名称 */
  flowName: string;
  /** 节点 UUID */
  nodeUuid: string;
  /** 节点名称 */
  nodeName: string;
  /** 完整命令行 */
  commandDisplay: string;
  /** 工作目录 */
  cwd: string;
  /** 注入的环境变量名（不含值） */
  envNames: string[];
  /** 额外继承的宿主环境变量名（不含值；会原样传给子进程） */
  inheritEnvNames: string[];
  /** 是否 shell 模式 */
  shellMode: boolean;
  /** 命令哈希 */
  commandHash: string;
}

/** 后端推送审批请求的事件名 */
export const EXEC_APPROVAL_EVENT = 'exec-approval-request';

/**
 * 回填审批结果
 *
 * 请求已失效（超时/会话结束）时后端返回错误，调用方应静默关闭弹窗。
 */
export async function respondExecApproval(requestId: string, approved: boolean): Promise<void> {
  if (!isTauriEnv()) return;
  await tauriInvoke('respond_exec_approval', { requestId, approved });
}

/** 列出已批准的信任记录 */
export async function listExecApprovals(): Promise<ExecApprovalRecord[]> {
  if (!isTauriEnv()) return [];
  return tauriInvoke<ExecApprovalRecord[]>('list_exec_approvals');
}

/** 撤销单条信任记录 */
export async function revokeExecApproval(
  flowPath: string | undefined,
  commandHash: string
): Promise<void> {
  if (!isTauriEnv()) return;
  await tauriInvoke('revoke_exec_approval', { flowPath: flowPath ?? null, commandHash });
}

/** 清空全部信任记录 */
export async function clearExecApprovals(): Promise<void> {
  if (!isTauriEnv()) return;
  await tauriInvoke('clear_exec_approvals');
}
