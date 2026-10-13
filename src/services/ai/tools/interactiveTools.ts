/**
 * AI 人工交互工具 (Human-in-the-Loop)
 * 当大模型需要人类输入内容、做选择题或提供敏感凭据 (如 sudo 密码) 时调用。
 */

import type { AIToolDefinition } from '@/types/ai';

export const interactiveToolDefinitions: AIToolDefinition[] = [
  {
    type: 'function',
    function: {
      name: 'ask_user',
      description:
        'Ask the user a question to obtain necessary human input, choices, passwords, or decisions. Use this when sudo credentials, user decisions, or custom inputs are needed so the autonomous process closes the loop cleanly without hanging or guessing.',
      parameters: {
        type: 'object',
        properties: {
          question: {
            type: 'string',
            description: 'The question or prompt to present to the user.',
          },
          options: {
            type: 'array',
            items: { type: 'string' },
            description:
              'Optional list of choices for the user to select from (e.g. ["dev", "prod"]).',
          },
          input_type: {
            type: 'string',
            enum: ['text', 'password'],
            description:
              'Expected input type. Use "password" if asking for sensitive data like a sudo password or secret token, or "text" for normal input. Default is "text".',
          },
          placeholder: {
            type: 'string',
            description: 'Optional placeholder text for the input box.',
          },
        },
        required: ['question'],
      },
    },
  },
];
