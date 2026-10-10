/**
 * AI Copilot 斜杠快捷指令注册与分发引擎
 */

import { getLL } from '@/i18n/i18n-lazy';
import { useAIStore } from '@/stores/ai';
import { useExecutionStore } from '@/stores/execution';
import { useFlowStore } from '@/stores/flow';
import type { AIMessage } from '@/types/ai';

export interface SlashCommandDefinition {
  id: string;
  name: string;
  aliases: string[];
  title: () => string;
  description: () => string;
  run: (args: string) => Promise<boolean> | boolean;
}

/**
 * 获取所有已注册的斜杠指令
 */
export function getSlashCommands(): SlashCommandDefinition[] {
  const ll = getLL();

  return [
    {
      id: 'compact',
      name: '/compact',
      aliases: ['/summarize'],
      title: () => ll.ai.copilot.command_compact(),
      description: () => ll.ai.copilot.command_compact_desc(),
      run: async () => {
        return await useAIStore.getState().compactHistory();
      },
    },
    {
      id: 'clear',
      name: '/clear',
      aliases: ['/reset'],
      title: () => ll.ai.copilot.command_clear(),
      description: () => ll.ai.copilot.command_clear_desc(),
      run: () => {
        useAIStore.getState().clearMessages();
        return true;
      },
    },
    {
      id: 'diagnose',
      name: '/diagnose',
      aliases: ['/fix'],
      title: () => ll.ai.copilot.command_diagnose(),
      description: () => ll.ai.copilot.command_diagnose_desc(),
      run: async () => {
        const report = useExecutionStore.getState().report;
        const failedNodes = report?.node_reports
          ? report.node_reports
              .filter((r) => r.status === 'failed')
              .map((r) => `${r.node_name || r.node_uuid}: ${r.error?.message || 'failed'}`)
          : [];

        const prompt = failedNodes.length > 0
          ? ll.ai.copilot.prompt_diagnose_flow_failures({
              failedNodes: failedNodes.join('; '),
            })
          : ll.ai.copilot.prompt_diagnose_flow_generic();

        await useAIStore.getState().sendMessage(prompt);
        return true;
      },
    },
    {
      id: 'review',
      name: '/review',
      aliases: ['/audit'],
      title: () => ll.ai.copilot.command_review(),
      description: () => ll.ai.copilot.command_review_desc(),
      run: async () => {
        const flow = useFlowStore.getState().flow;
        const nodeCount = flow?.nodes.length ?? 0;
        const connCount = flow?.connections.length ?? 0;
        const prompt = ll.ai.copilot.prompt_review_flow({ nodeCount, connCount });
        await useAIStore.getState().sendMessage(prompt);
        return true;
      },
    },
    {
      id: 'help',
      name: '/help',
      aliases: ['/?'],
      title: () => ll.ai.copilot.command_help(),
      description: () => ll.ai.copilot.command_help_desc(),
      run: () => {
        const helpContent = `### ${ll.ai.copilot.command_help_title()}
${ll.ai.copilot.command_help_intro()}

- ${ll.ai.copilot.command_help_compact()}
- ${ll.ai.copilot.command_help_clear()}
- ${ll.ai.copilot.command_help_diagnose()}
- ${ll.ai.copilot.command_help_review()}
- ${ll.ai.copilot.command_help_help()}`;

        const helpMsg: AIMessage = {
          id: `msg_${Date.now()}_help`,
          role: 'assistant',
          content: helpContent,
          timestamp: Date.now(),
        };

        useAIStore.setState((s) => ({
          messages: [...s.messages, helpMsg],
        }));
        return true;
      },
    },
  ];
}

/**
 * 依据用户输入的文本前缀匹配候选指令列表
 */
export function findMatchingSlashCommands(input: string): SlashCommandDefinition[] {
  const trimmed = input.trim();
  if (!trimmed.startsWith('/')) return [];

  const commandPart = trimmed.split(/\s+/)[0].toLowerCase();
  const commands = getSlashCommands();

  return commands.filter((cmd) => {
    if (cmd.name.toLowerCase().startsWith(commandPart)) return true;
    return cmd.aliases.some((alias) => alias.toLowerCase().startsWith(commandPart));
  });
}

/**
 * 检测当前输入是否为有效的斜杠指令调用
 */
export function isSlashCommand(input: string): boolean {
  const trimmed = input.trim();
  if (!trimmed.startsWith('/')) return false;
  const firstWord = trimmed.split(/\s+/)[0].toLowerCase();
  const commands = getSlashCommands();
  return commands.some(
    (cmd) => cmd.name.toLowerCase() === firstWord || cmd.aliases.some((a) => a.toLowerCase() === firstWord),
  );
}

/**
 * 执行匹配的斜杠指令
 * @returns true 表示作为指令执行成功并已被消费；false 表示未匹配到任何指令
 */
export async function executeSlashCommand(rawInput: string): Promise<boolean> {
  const trimmed = rawInput.trim();
  if (!trimmed.startsWith('/')) return false;

  const parts = trimmed.split(/\s+/);
  const trigger = parts[0].toLowerCase();
  const args = parts.slice(1).join(' ');

  const commands = getSlashCommands();
  const matched = commands.find(
    (cmd) => cmd.name.toLowerCase() === trigger || cmd.aliases.some((a) => a.toLowerCase() === trigger),
  );

  if (matched) {
    await matched.run(args);
    return true;
  }

  return false;
}
