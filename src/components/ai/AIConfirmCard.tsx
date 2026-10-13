/**
 * AI 高风险操作待确认卡片
 */

import { useState, memo } from 'react';
import { Check, X, ShieldAlert, ShieldCheck, MessageSquare } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { useAIStore } from '@/stores/ai';
import { getLL } from '@/i18n/i18n-lazy';
import type { AIPendingAction } from '@/types/ai';

interface AIConfirmCardProps {
  action: AIPendingAction;
}

export const AIConfirmCard = memo(function AIConfirmCard({ action }: AIConfirmCardProps) {
  const ll = getLL();
  const resolvePendingAction = useAIStore((s) => s.resolvePendingAction);
  const [isProcessing, setIsProcessing] = useState(false);
  const [userInputText, setUserInputText] = useState('');
  const isUserInput = Boolean(action.isUserInput || action.toolName === 'ask_user');

  const handleTextSubmit = async () => {
    if (!userInputText.trim()) return;
    setIsProcessing(true);
    try {
      await resolvePendingAction(action.id, true, false, userInputText.trim());
    } finally {
      setIsProcessing(false);
    }
  };

  const handleOptionSelect = async (opt: string) => {
    setIsProcessing(true);
    try {
      await resolvePendingAction(action.id, true, false, opt);
    } finally {
      setIsProcessing(false);
    }
  };
  const handleApprove = async () => {
    setIsProcessing(true);
    try {
      await resolvePendingAction(action.id, true);
    } finally {
      setIsProcessing(false);
    }
  };
  const handleTrustAll = async () => {
    setIsProcessing(true);
    try {
      await resolvePendingAction(action.id, true, true);
    } finally {
      setIsProcessing(false);
    }
  };


  const handleReject = async () => {
    setIsProcessing(true);
    try {
      await resolvePendingAction(action.id, false);
    } finally {
      setIsProcessing(false);
    }
  };

  const isPending = action.status === 'pending';

  return (
    <div
      className={`my-2.5 rounded-lg p-3.5 shadow-sm text-sm ${
        isUserInput
          ? 'border border-indigo-500/40 bg-indigo-500/10'
          : 'border border-amber-500/40 bg-amber-500/10'
      }`}
    >
      <div
        className={`flex items-center gap-2 font-medium ${
          isUserInput
            ? 'text-indigo-600 dark:text-indigo-400'
            : 'text-amber-600 dark:text-amber-400'
        }`}
      >
        {isUserInput ? (
          <MessageSquare className="h-4 w-4 shrink-0 text-indigo-500" />
        ) : (
          <ShieldAlert className="h-4 w-4 shrink-0 text-amber-500" />
        )}
        <span>
          {isUserInput
            ? ll.ai.confirm_card.ask_user_title()
            : ll.ai.confirm_card.title()}
        </span>
        <span
          className={`ml-auto text-xs px-2 py-0.5 rounded font-mono font-normal truncate max-w-[140px] shrink-0 ${
            isUserInput
              ? 'bg-indigo-500/20 text-indigo-700 dark:text-indigo-300'
              : 'bg-amber-500/20 text-amber-700 dark:text-amber-300'
          }`}
          title={action.toolName}
        >
          {action.toolName}
        </span>
      </div>

      <div className="mt-2 text-foreground/90 font-medium">
        {action.description}
      </div>

      {action.diffSummary && (
        <div className="mt-1.5 rounded bg-background/60 p-2 text-xs text-muted-foreground font-mono border border-border/50 break-all whitespace-pre-wrap max-h-40 overflow-y-auto">
          {action.diffSummary}
        </div>
      )}

      {isPending ? (
        isUserInput ? (
          <div className="mt-3 space-y-2.5">
            {Array.isArray(action.args.options) && action.args.options.length > 0 && (
              <div className="flex flex-wrap gap-1.5">
                {(action.args.options as string[]).map((opt) => (
                  <Button
                    key={opt}
                    size="sm"
                    variant="outline"
                    className="h-7 text-xs border-indigo-500/30 hover:bg-indigo-500/15 text-indigo-700 dark:text-indigo-300 font-medium"
                    onClick={() => handleOptionSelect(opt)}
                    disabled={isProcessing}
                  >
                    {opt}
                  </Button>
                ))}
              </div>
            )}
            <div className="flex items-center gap-2">
              <Input
                type={action.args.input_type === 'password' ? 'password' : 'text'}
                value={userInputText}
                onChange={(e) => setUserInputText(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && !e.shiftKey && userInputText.trim()) {
                    e.preventDefault();
                    handleTextSubmit();
                  }
                }}
                placeholder={
                  typeof action.args.placeholder === 'string'
                    ? action.args.placeholder
                    : ll.ai.confirm_card.ask_user_placeholder()
                }
                className="h-7 text-xs bg-background/80"
                disabled={isProcessing}
                autoFocus
              />
              <Button
                size="sm"
                variant="outline"
                className="h-7 text-xs border-border/80 shrink-0"
                onClick={handleReject}
                disabled={isProcessing}
              >
                <X className="mr-1 h-3 w-3" />
                {ll.ai.confirm_card.ask_user_decline()}
              </Button>
              <Button
                size="sm"
                className="h-7 text-xs bg-indigo-600 hover:bg-indigo-700 text-white font-medium shrink-0"
                onClick={handleTextSubmit}
                disabled={isProcessing || !userInputText.trim()}
              >
                <Check className="mr-1 h-3 w-3" />
                {ll.ai.confirm_card.ask_user_submit()}
              </Button>
            </div>
          </div>
        ) : (
          <div className="mt-3 flex flex-wrap items-center justify-end gap-2">
            <Button
              size="sm"
              variant="outline"
              className="h-7 text-xs border-amber-500/30 hover:bg-amber-500/10"
              onClick={handleReject}
              disabled={isProcessing}
            >
              <X className="mr-1 h-3 w-3" />
              {ll.ai.confirm_card.reject()}
            </Button>
            <Button
              size="sm"
              variant="outline"
              className="h-7 text-xs border-amber-500/40 hover:bg-amber-500/15 text-amber-700 dark:text-amber-300"
              onClick={handleApprove}
              disabled={isProcessing}
            >
              <Check className="mr-1 h-3 w-3" />
              {ll.ai.confirm_card.approve()}
            </Button>
            <Button
              size="sm"
              className="h-7 text-xs bg-amber-600 hover:bg-amber-700 text-white font-medium"
              onClick={handleTrustAll}
              disabled={isProcessing}
              title={ll.ai.confirm_card.trust_all_tip()}
            >
              <ShieldCheck className="mr-1 h-3.5 w-3.5" />
              {ll.ai.confirm_card.trust_all()}
            </Button>
          </div>
        )
      ) : (
        <div className="mt-2 text-xs flex items-center gap-1.5 font-medium">
          {action.status === 'approved' ? (
            <span className="text-emerald-600 dark:text-emerald-400 flex items-center gap-1">
              <Check className="h-3.5 w-3.5" />
              {isUserInput
                ? ll.ai.confirm_card.ask_user_status_answered({
                    answer:
                      action.args.input_type === 'password'
                        ? '••••••••'
                        : String(action.userResponse || '✓'),
                  })
                : ll.ai.confirm_card.status_approved()}
            </span>
          ) : (
            <span className="text-muted-foreground flex items-center gap-1">
              <X className="h-3.5 w-3.5" />
              {isUserInput
                ? ll.ai.confirm_card.ask_user_status_declined()
                : ll.ai.confirm_card.status_rejected()}
            </span>
          )}
        </div>
      )}
    </div>
  );
});
