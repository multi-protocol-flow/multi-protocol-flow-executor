/**
 * AI 本地项目工作区感知与代码分析工具
 */

import { invoke } from '@tauri-apps/api/core';
import { useAIStore } from '@/stores/ai';
import type { AIToolDefinition } from '@/types/ai';

export const workspaceToolDefinitions: AIToolDefinition[] = [
  {
    type: 'function',
    function: {
      name: 'scan_project_structure',
      description:
        'Scan the linked project workspace to detect technology stack, configuration files (package.json, Cargo.toml, etc.), API spec contracts (OpenAPI/Swagger/Proto), and key business directories (controllers, routes, handlers).',
      parameters: {
        type: 'object',
        properties: {
          workspace_dir: {
            type: 'string',
            description:
              'Optional absolute path of the workspace. If omitted, uses the currently linked session workspace.',
          },
        },
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'search_project_code',
      description:
        'Search the codebase using regex pattern matching. Returns matching relative file paths, 1-based line numbers, and line contents (up to 50 results).',
      parameters: {
        type: 'object',
        properties: {
          pattern: {
            type: 'string',
            description:
              'Regular expression or substring to search for (e.g. "router\\.post", "@Get\\(", "func \\(s \\*Server\\)").',
          },
          workspace_dir: {
            type: 'string',
            description:
              'Optional absolute workspace path. If omitted, uses the currently linked session workspace.',
          },
          ext: {
            type: 'string',
            description:
              'Optional file extension filter without leading dot (e.g. "ts", "js", "go", "rs", "py").',
          },
          max_results: {
            type: 'number',
            description: 'Maximum number of matches to return (default 20, max 50).',
          },
        },
        required: ['pattern'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'read_source_code',
      description:
        'Read a slice of source code by line range from the project workspace. Hard-limited to 200 lines per call to conserve context.',
      parameters: {
        type: 'object',
        properties: {
          file_path: {
            type: 'string',
            description:
              'Relative path (from workspace root) or absolute path of the file to inspect.',
          },
          start_line: {
            type: 'number',
            description: '1-based starting line number (defaults to 1).',
          },
          max_lines: {
            type: 'number',
            description:
              'Number of lines to read (defaults to 100, max capped at 200).',
          },
        },
        required: ['file_path'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'inspect_api_spec',
      description:
        'Parse and extract structured API contract endpoints and schemas from an OpenAPI (JSON/YAML) or Protobuf (.proto) spec file.',
      parameters: {
        type: 'object',
        properties: {
          file_path: {
            type: 'string',
            description:
              'Relative path (from workspace root) or absolute path of the spec file (e.g. "openapi.yaml", "api.json", "user.proto").',
          },
        },
        required: ['file_path'],
      },
    },
  },
];

function resolvePath(inputPath: string, rootDir?: string): string {
  const trimmed = inputPath.trim();
  // 如果已经是绝对路径（以 / 或 C:\ 或 \\ 开头）
  if (/^([a-zA-Z]:[/\\]|[/\\])/.test(trimmed)) {
    return trimmed;
  }
  if (!rootDir) {
    return trimmed;
  }
  const cleanRoot = rootDir.replace(/[/\\]+$/, '');
  const cleanSub = trimmed.replace(/^[/\\]+/, '');
  return `${cleanRoot}/${cleanSub}`;
}

export const workspaceToolHandlers = {
  scan_project_structure: async (args: { workspace_dir?: string }) => {
    const activeWs = useAIStore.getState().activeWorkspace;
    const targetDir = args.workspace_dir || activeWs?.path;
    if (!targetDir) {
      throw new Error(
        'No active workspace linked. Please link a local project folder first or supply "workspace_dir".',
      );
    }

    const scanResult = await invoke<{
      tech_stack: string | null;
      config_files: string[];
      spec_files: string[];
      key_directories: string[];
    }>('ai_scan_workspace', {
      workspaceDir: targetDir,
    });

    // 自动回填技术栈与检测到的 spec 文件到当前工作区上下文
    if (activeWs && (!activeWs.techStack || !activeWs.apiSpecPath)) {
      useAIStore.getState().setSessionWorkspace({
        ...activeWs,
        techStack: activeWs.techStack || scanResult.tech_stack || undefined,
        apiSpecPath: activeWs.apiSpecPath || scanResult.spec_files[0] || undefined,
      });
    }

    return {
      workspace: targetDir,
      tech_stack: scanResult.tech_stack,
      config_files: scanResult.config_files,
      spec_files: scanResult.spec_files,
      key_directories: scanResult.key_directories,
      guidance:
        scanResult.spec_files.length > 0
          ? `Detected API spec file(s): ${scanResult.spec_files.join(', ')}. Use "inspect_api_spec" to parse endpoints directly.`
          : `No API spec detected. Use "search_project_code" and "read_source_code" to inspect route definitions.`,
    };
  },

  search_project_code: async (args: {
    pattern: string;
    workspace_dir?: string;
    ext?: string;
    max_results?: number;
  }) => {
    const activeWs = useAIStore.getState().activeWorkspace;
    const targetDir = args.workspace_dir || activeWs?.path;
    if (!targetDir) {
      throw new Error(
        'No active workspace linked. Please link a local project folder first or supply "workspace_dir".',
      );
    }

    const matches = await invoke<
      Array<{
        file: string;
        line_number: number;
        line_content: string;
      }>
    >('ai_search_code', {
      workspaceDir: targetDir,
      pattern: args.pattern,
      ext: args.ext ?? null,
      maxResults: args.max_results ?? 20,
    });

    return {
      pattern: args.pattern,
      matches_count: matches.length,
      matches,
    };
  },

  read_source_code: async (args: {
    file_path: string;
    start_line?: number;
    max_lines?: number;
  }) => {
    const activeWs = useAIStore.getState().activeWorkspace;
    const fullPath = resolvePath(args.file_path, activeWs?.path);

    const slice = await invoke<string>('ai_read_code_slice', {
      filePath: fullPath,
      startLine: args.start_line ?? 1,
      maxLines: args.max_lines ?? 100,
    });

    return {
      file: args.file_path,
      full_path: fullPath,
      start_line: args.start_line ?? 1,
      content: slice,
    };
  },

  inspect_api_spec: async (args: { file_path: string }) => {
    const activeWs = useAIStore.getState().activeWorkspace;
    const fullPath = resolvePath(args.file_path, activeWs?.path);

    const specData = await invoke<Record<string, unknown>>('ai_inspect_api_spec', {
      filePath: fullPath,
    });

    return {
      file: args.file_path,
      full_path: fullPath,
      spec: specData,
    };
  },
};
