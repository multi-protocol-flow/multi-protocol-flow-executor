/**
 * AI 终端命令执行工具 (PowerShell on Windows / sh on macOS & Linux)
 */

import { invoke } from '@tauri-apps/api/core';
import { useAIStore } from '@/stores/ai';
import type { AIToolDefinition } from '@/types/ai';

export interface AiCommandExecutionResult {
  exit_code: number | null;
  stdout: string;
  stderr: string;
  timed_out: boolean;
  duration_ms: number;
}

/**
 * 校验终端命令是否在只读/环境探测免确认白名单中。
 * 命中白名单的命令可自动放行，其余命令必须经由用户点击确认。
 */
export function isSafeTerminalCommand(command: string): boolean {
  const trimmed = command.trim();
  if (!trimmed) return false;

  // 包含任何重定向输出、后台运行或高危拼接符号一律要求用户确认
  if (/[><&;`$()]/.test(trimmed)) {
    return false;
  }

  // 分解基础命令与子参数
  const tokens = trimmed.split(/\s+/);
  const baseCmd = tokens[0]?.toLowerCase();
  if (!baseCmd) return false;
  // 1. 版本与环境探测（如 node -v, python --version, cargo -V 等）
  const VERSION_QUERY_CMDS: Record<string, true> = {
    node: true,
    npm: true,
    pnpm: true,
    yarn: true,
    bun: true,
    deno: true,
    python: true,
    python3: true,
    pip: true,
    pip3: true,
    cargo: true,
    rustc: true,
    rustup: true,
    go: true,
    java: true,
    javac: true,
    docker: true,
    'docker-compose': true,
    git: true,
    curl: true,
    wget: true,
    ping: true,
  };

  if (VERSION_QUERY_CMDS[baseCmd]) {
    if (tokens.length === 2 && /^(-v|--version|-V|version)$/i.test(tokens[1])) {
      return true;
    }
  }

  // 2. Git 只读状态查询
  if (baseCmd === 'git') {
    const gitSubCmd = tokens[1]?.toLowerCase();
    const SAFE_GIT_SUB_CMDS: Record<string, true> = {
      status: true,
      diff: true,
      log: true,
      branch: true,
      remote: true,
      show: true,
      tag: true,
      'rev-parse': true,
    };
    if (gitSubCmd && SAFE_GIT_SUB_CMDS[gitSubCmd]) {
      return true;
    }
  }

  // 3. 基础目录与文件只读浏览
  const READ_ONLY_CMDS: Record<string, true> = {
    ls: true,
    dir: true,
    pwd: true,
    'get-location': true,
    'get-childitem': true,
    cat: true,
    head: true,
    tail: true,
    type: true,
    'get-content': true,
    echo: true,
    whoami: true,
    hostname: true,
    date: true,
    which: true,
    where: true,
  };

  if (READ_ONLY_CMDS[baseCmd]) {
    return true;
  }

  // 4. 包管理器只读依赖查看
  if (['npm', 'pnpm', 'yarn', 'bun'].includes(baseCmd)) {
    const subCmd = tokens[1]?.toLowerCase();
    const SAFE_PKG_SUB_CMDS: Record<string, true> = {
      list: true,
      ls: true,
      why: true,
      outdated: true,
    };
    if (subCmd && SAFE_PKG_SUB_CMDS[subCmd]) {
      return true;
    }
  }

  // 5. Cargo 只读查看
  if (baseCmd === 'cargo') {
    const subCmd = tokens[1]?.toLowerCase();
    const SAFE_CARGO_SUB_CMDS: Record<string, true> = {
      tree: true,
      metadata: true,
      'verify-project': true,
    };
    if (subCmd && SAFE_CARGO_SUB_CMDS[subCmd]) {
      return true;
    }
  }

  return false;
}

export const terminalToolDefinitions: AIToolDefinition[] = [
  {
    type: 'function',
    function: {
      name: 'run_terminal_command',
      description:
        'Execute a shell command in the local system environment (PowerShell on Windows, sh on Linux/macOS). Read-only/diagnostic commands execute smoothly, while mutating actions prompt the user for confirmation. Output is capped at 32KB with a default 30s timeout.',
      parameters: {
        type: 'object',
        properties: {
          command: {
            type: 'string',
            description:
              'The exact shell command line to execute (e.g. "git status", "cargo check", "curl -I http://localhost:8080"). Persistent daemon servers (e.g. "npm run dev") should NOT be run as they will be killed by timeout.',
          },
          cwd: {
            type: 'string',
            description:
              'Optional absolute path of the working directory. If omitted, defaults to the active project workspace root.',
          },
          timeout_secs: {
            type: 'number',
            description:
              'Optional execution timeout in seconds (default 30, maximum 120).',
          },
        },
        required: ['command'],
      },
    },
  },
];

export const terminalToolHandlers = {
  run_terminal_command: async (args: {
    command: string;
    cwd?: string;
    timeout_secs?: number;
  }) => {
    const activeWs = useAIStore.getState().activeWorkspace;
    const targetDir = args.cwd || activeWs?.path;

    const result = await invoke<AiCommandExecutionResult>('ai_execute_command', {
      command: args.command,
      cwd: targetDir || null,
      timeoutSecs: args.timeout_secs ?? null,
    });

    return {
      command: args.command,
      cwd: targetDir || null,
      exit_code: result.exit_code,
      stdout: result.stdout,
      stderr: result.stderr,
      timed_out: result.timed_out,
      duration_ms: result.duration_ms,
    };
  },
};
