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

  // 1. 严格拦截文件输出重定向、后台符与复合注入符
  if (/[><&;`$()]/.test(trimmed)) {
    // 允许且仅允许只读管道 '|'
    if (/[><&;`$()]/.test(trimmed.replace(/\|/g, ''))) {
      return false;
    }
  }

  // 2. 如果包含管道 '|'，则所有管道段必须全部属于只读操作
  if (trimmed.includes('|')) {
    const stages = trimmed.split('|').map((s) => s.trim());
    if (stages.some((s) => !s)) return false;
    // 检查首个命令是否安全
    if (!isSafeSingleTerminalCommand(stages[0])) return false;
    // 检查后续管道过滤器是否全部为只读工具
    const SAFE_PIPE_FILTERS: Record<string, true> = {
      grep: true,
      egrep: true,
      fgrep: true,
      head: true,
      tail: true,
      findstr: true,
      'select-string': true,
      jq: true,
      sort: true,
      uniq: true,
      wc: true,
      less: true,
      more: true,
      awk: true,
      cut: true,
      tr: true,
    };
    for (let i = 1; i < stages.length; i++) {
      const pipeTokens = stages[i].split(/\s+/);
      const pipeCmd = pipeTokens[0]?.toLowerCase();
      if (!pipeCmd || !SAFE_PIPE_FILTERS[pipeCmd]) {
        return false;
      }
    }
    return true;
  }

  return isSafeSingleTerminalCommand(trimmed);
}

/**
 * 检查单个不包含管道的命令是否为安全只读/诊断操作
 */
function isSafeSingleTerminalCommand(cmdStr: string): boolean {
  const tokens = cmdStr.split(/\s+/);
  const baseCmd = tokens[0]?.toLowerCase();
  if (!baseCmd) return false;

  // 显式高危命令前缀，命中直接拦截
  const DANGEROUS_BASE_CMDS: Record<string, true> = {
    rm: true,
    del: true,
    erase: true,
    rmdir: true,
    'remove-item': true,
    ri: true,
    mv: true,
    move: true,
    'move-item': true,
    chmod: true,
    chown: true,
    kill: true,
    pkill: true,
    killall: true,
    'stop-process': true,
    shutdown: true,
    reboot: true,
    'set-content': true,
    'add-content': true,
    'out-file': true,
  };
  if (DANGEROUS_BASE_CMDS[baseCmd]) return false;

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

  // 2. Git 只读查询操作
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
      // 如果 git branch 带 -d 或 -D 删除参数则拦截
      if (gitSubCmd === 'branch' && tokens.some((t) => t === '-d' || t === '-D')) {
        return false;
      }
      return true;
    }
    return false;
  }

  // 3. 基础目录、文件、系统与进程只读查询
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
    ps: true,
    top: true,
    htop: true,
    tasklist: true,
    'get-process': true,
    'get-service': true,
    netstat: true,
    lsof: true,
    ss: true,
    uptime: true,
    free: true,
    df: true,
    du: true,
    env: true,
    printenv: true,
    find: true,
    fd: true,
    rg: true,
    grep: true,
    wc: true,
    diff: true,
    'test-netconnection': true,
    ping: true,
    traceroute: true,
    tracert: true,
    nslookup: true,
  };
  if (READ_ONLY_CMDS[baseCmd]) {
    return true;
  }

  // 4. curl 只读网络探测（不能带 POST/DELETE 或 -d 数据写入参数）
  if (baseCmd === 'curl') {
    const hasWriteFlag = tokens.some((t) =>
      /^(-X|--request)$/i.test(t) ||
      /^(POST|PUT|DELETE|PATCH)$/i.test(t) ||
      /^(-d|--data|--data-raw|--data-binary|--data-urlencode|-F|--form)$/i.test(t),
    );
    return !hasWriteFlag;
  }

  // 5. 包管理器只读依赖查看与安全测试运行
  if (['npm', 'pnpm', 'yarn', 'bun'].includes(baseCmd)) {
    const subCmd = tokens[1]?.toLowerCase();
    // 安全只读查看与测试命令
    if (['list', 'ls', 'why', 'outdated', 'test'].includes(subCmd)) {
      return true;
    }
    // npm run test, npm run lint, npm run typecheck
    if (subCmd === 'run') {
      const scriptTarget = tokens[2]?.toLowerCase();
      if (['test', 'lint', 'typecheck', 'check'].includes(scriptTarget)) {
        return true;
      }
    }
    return false;
  }

  // 6. Cargo 只读查看与非破坏性检查测试
  if (baseCmd === 'cargo') {
    const subCmd = tokens[1]?.toLowerCase();
    const SAFE_CARGO_SUB_CMDS: Record<string, true> = {
      tree: true,
      metadata: true,
      'verify-project': true,
      check: true,
      clippy: true,
      test: true,
    };
    if (subCmd && SAFE_CARGO_SUB_CMDS[subCmd]) {
      return true;
    }
    return false;
  }

  // 7. TypeScript / 测试框架只读检查
  if (baseCmd === 'tsc' && tokens.includes('--noEmit')) {
    return true;
  }
  if (['pytest', 'vitest'].includes(baseCmd)) {
    return true;
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
          stdin: {
            type: 'string',
            description:
              'Optional standard input (stdin) string to feed into the process before closing stdin (e.g. password for "sudo -S", piped input, or answers to prompts).',
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
    stdin?: string;
  }) => {
    const activeWs = useAIStore.getState().activeWorkspace;
    const targetDir = args.cwd || activeWs?.path;

    const result = await invoke<AiCommandExecutionResult>('ai_execute_command', {
      command: args.command,
      cwd: targetDir || null,
      timeoutSecs: args.timeout_secs ?? null,
      stdin: args.stdin ?? null,
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
