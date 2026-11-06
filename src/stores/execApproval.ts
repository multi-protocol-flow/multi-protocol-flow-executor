/**
 * 本地命令审批队列（`local:exec`）
 *
 * 后端在执行未获批准的命令时推送 `exec-approval-request` 事件，
 * 弹窗回填结果后从队列移除；队列为空时不渲染任何 UI。
 */
import { create } from 'zustand';
import type { ExecApprovalRequest } from '@/services/tauri/execTrust';

interface ExecApprovalStore {
  /** 待处理审批请求（先进先出） */
  requests: ExecApprovalRequest[];
  /** 入队 */
  push: (request: ExecApprovalRequest) => void;
  /** 出队（按请求 ID） */
  remove: (requestId: string) => void;
  /** 清空（测试隔离 / 会话结束） */
  reset: () => void;
}

export const useExecApprovalStore = create<ExecApprovalStore>((set) => ({
  requests: [],
  push: (request) =>
    set((state) => {
      // 同一请求重复推送时去重（事件可能因重连重复投递）
      if (state.requests.some((item) => item.requestId === request.requestId)) {
        return state;
      }
      return { requests: [...state.requests, request] };
    }),
  remove: (requestId) =>
    set((state) => ({ requests: state.requests.filter((item) => item.requestId !== requestId) })),
  reset: () => set({ requests: [] }),
}));
