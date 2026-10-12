/**
 * HumanInteractionCard - 人机协同交互卡片 (HITL: Human-in-the-Loop)
 *
 * 当运行期节点（如 agent:react）触发等待用户输入、敏感工具确认或单步调试时，
 * 在执行面板与画布浮层展示此交互卡片供用户决策，打通双向人机回路。
 */
import { memo, useState, useCallback } from 'react';
import {
  Bot,
  AlertTriangle,
  Play,
  FastForward,
  Square,
  Send,
  Check,
  X,
  Loader2,
  Clock,
} from 'lucide-react';
import type { PendingInteraction, HumanResponsePayload } from '@/types/execution_event';
import { useExecutionStore } from '@/stores';
import { getLL } from '@/i18n/i18n-lazy';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';

export interface HumanInteractionCardProps {
  nodeUuid: string;
  interaction: PendingInteraction;
  className?: string;
}

export const HumanInteractionCard = memo(function HumanInteractionCard({
  nodeUuid,
  interaction,
  className,
}: HumanInteractionCardProps) {
  const ll = getLL();
  const [inputText, setInputText] = useState('');
  const [rejectReason, setRejectReason] = useState('');
  const [showRejectInput, setShowRejectInput] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const { submitHumanResponse } = useExecutionStore.getState();

  const handleSendResponse = useCallback(
    async (payload: HumanResponsePayload) => {
      setIsSubmitting(true);
      try {
        await submitHumanResponse(nodeUuid, interaction.interaction_id, payload);
      } finally {
        setIsSubmitting(false);
      }
    },
    [nodeUuid, interaction.interaction_id, submitHumanResponse]
  );

  const isAsk = interaction.interaction_type === 'ask_human';
  const isConfirm = interaction.interaction_type === 'confirmation';
  const isStep = interaction.interaction_type === 'step_pause';

  return (
    <div
      className={cn(
        'rounded-lg border p-4 shadow-sm transition-all',
        isAsk && 'border-purple-200 bg-purple-50/60 dark:border-purple-900/60 dark:bg-purple-950/20',
        isConfirm && 'border-amber-200 bg-amber-50/60 dark:border-amber-900/60 dark:bg-amber-950/20',
        isStep && 'border-blue-200 bg-blue-50/60 dark:border-blue-900/60 dark:bg-blue-950/20',
        className
      )}
    >
      {/* 头部标题与徽标 */}
      <div className="flex items-center justify-between gap-2 mb-3">
        <div className="flex items-center gap-2 min-w-0">
          {isAsk && <Bot className="w-5 h-5 text-purple-600 dark:text-purple-400 shrink-0" />}
          {isConfirm && (
            <AlertTriangle className="w-5 h-5 text-amber-600 dark:text-amber-400 shrink-0" />
          )}
          {isStep && <Clock className="w-5 h-5 text-blue-600 dark:text-blue-400 shrink-0" />}
          <span className="font-semibold text-sm truncate">
            {isAsk && ll.execution.hitl.ask_human_title()}
            {isConfirm && ll.execution.hitl.confirm_title()}
            {isStep && ll.execution.hitl.step_title()}
          </span>
        </div>
        <div className="flex items-center gap-1.5 shrink-0">
          {interaction.step_index && (
            <Badge variant="outline" className="text-xs px-1.5 py-0">
              Round {interaction.step_index}
            </Badge>
          )}
          <Badge
            variant="secondary"
            className={cn(
              'text-xs px-2 py-0.5 animate-pulse font-mono',
              isAsk && 'bg-purple-100 text-purple-700 dark:bg-purple-900/40 dark:text-purple-300',
              isConfirm && 'bg-amber-100 text-amber-700 dark:bg-amber-900/40 dark:text-amber-300',
              isStep && 'bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-300'
            )}
          >
            {interaction.interaction_type}
          </Badge>
        </div>
      </div>

      {/* 提示正文 */}
      <div className="mb-3">
        <p className="text-xs text-muted-foreground mb-1">{ll.execution.hitl.waiting_desc()}</p>
        <div className="rounded-md border border-border/80 bg-background/80 p-3 text-sm font-medium leading-relaxed break-words whitespace-pre-wrap">
          {interaction.prompt}
        </div>
      </div>

      {/* Confirmation 模式：参数预览 */}
      {isConfirm && (
        <div className="mb-3 space-y-1.5">
          <div className="flex items-center justify-between text-xs text-muted-foreground">
            <span>{ll.execution.hitl.tool_name_label()}:</span>
            <span className="font-mono font-semibold text-foreground">
              {interaction.tool_name || interaction.tool_id}
            </span>
          </div>
          {interaction.input_args && (
            <div>
              <span className="text-xs text-muted-foreground block mb-1">
                {ll.execution.hitl.tool_args_label()}:
              </span>
              <pre className="text-xs font-mono bg-muted/60 p-2 rounded border border-border overflow-x-auto max-h-32">
                {JSON.stringify(interaction.input_args, null, 2)}
              </pre>
            </div>
          )}
        </div>
      )}

      {/* 交互控件区域 */}
      {isAsk && (
        <div className="space-y-2">
          <textarea
            value={inputText}
            onChange={(e) => setInputText(e.target.value)}
            placeholder={ll.execution.hitl.input_placeholder()}
            disabled={isSubmitting}
            rows={2}
            className="w-full text-xs rounded-md border border-input bg-background p-2.5 outline-none focus:ring-1 focus:ring-purple-500 disabled:opacity-50 resize-y"
            onKeyDown={(e) => {
              if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                e.preventDefault();
                void handleSendResponse({ text: inputText.trim() || 'Confirmed' });
              }
            }}
          />
          <div className="flex justify-end gap-2">
            <Button
              type="button"
              size="sm"
              disabled={isSubmitting}
              className="bg-purple-600 hover:bg-purple-700 text-white gap-1.5"
              onClick={() => handleSendResponse({ text: inputText.trim() || 'Confirmed' })}
            >
              {isSubmitting ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <Send className="w-3.5 h-3.5" />
              )}
              {ll.execution.hitl.submit_button()}
            </Button>
          </div>
        </div>
      )}

      {isConfirm && (
        <div className="space-y-2">
          {showRejectInput && (
            <input
              type="text"
              value={rejectReason}
              onChange={(e) => setRejectReason(e.target.value)}
              placeholder={ll.execution.hitl.reject_reason_placeholder()}
              disabled={isSubmitting}
              className="w-full text-xs rounded-md border border-input bg-background px-3 py-1.5 outline-none focus:ring-1 focus:ring-red-500"
            />
          )}
          <div className="flex justify-end gap-2">
            {!showRejectInput ? (
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={isSubmitting}
                className="text-red-600 hover:bg-red-50 dark:hover:bg-red-950/30 gap-1"
                onClick={() => setShowRejectInput(true)}
              >
                <X className="w-3.5 h-3.5" />
                {ll.execution.hitl.confirm_reject()}
              </Button>
            ) : (
              <Button
                type="button"
                variant="destructive"
                size="sm"
                disabled={isSubmitting}
                className="gap-1"
                onClick={() =>
                  handleSendResponse({
                    approved: false,
                    reason: rejectReason.trim() || 'Rejected by user',
                  })
                }
              >
                <X className="w-3.5 h-3.5" />
                {ll.execution.hitl.confirm_reject()}
              </Button>
            )}

            <Button
              type="button"
              size="sm"
              disabled={isSubmitting}
              className="bg-emerald-600 hover:bg-emerald-700 text-white gap-1"
              onClick={() => handleSendResponse({ approved: true })}
            >
              {isSubmitting ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <Check className="w-3.5 h-3.5" />
              )}
              {ll.execution.hitl.confirm_approve()}
            </Button>
          </div>
        </div>
      )}

      {isStep && (
        <div className="flex justify-end gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={isSubmitting}
            className="text-red-600 hover:bg-red-50 dark:hover:bg-red-950/30 gap-1"
            onClick={() => handleSendResponse({ action: 'cancel' })}
          >
            <Square className="w-3.5 h-3.5" />
            {ll.execution.hitl.step_cancel()}
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={isSubmitting}
            className="gap-1"
            onClick={() => handleSendResponse({ action: 'run_to_end' })}
          >
            <FastForward className="w-3.5 h-3.5" />
            {ll.execution.hitl.step_continue()}
          </Button>
          <Button
            type="button"
            size="sm"
            disabled={isSubmitting}
            className="bg-blue-600 hover:bg-blue-700 text-white gap-1"
            onClick={() => handleSendResponse({ action: 'next_step' })}
          >
            {isSubmitting ? (
              <Loader2 className="w-3.5 h-3.5 animate-spin" />
            ) : (
              <Play className="w-3.5 h-3.5" />
            )}
            {ll.execution.hitl.step_next()}
          </Button>
        </div>
      )}
    </div>
  );
});
