/**
 * workspaceTools 单元测试
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  workspaceToolDefinitions,
  workspaceToolHandlers,
} from '@/services/ai/tools/workspaceTools';
import { useAIStore } from '@/stores/ai';

// Mock @tauri-apps/api/core
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe('workspaceTools', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    useAIStore.getState().reset();
  });

  it('导出了 4 个符合标准的工具定义', () => {
    expect(workspaceToolDefinitions).toHaveLength(4);
    const names = workspaceToolDefinitions.map((d) => d.function.name);
    expect(names).toEqual([
      'scan_project_structure',
      'search_project_code',
      'read_source_code',
      'inspect_api_spec',
    ]);
  });

  it('scan_project_structure 调用 ai_scan_workspace 并回填工作区技术栈', async () => {
    useAIStore.getState().setSessionWorkspace({
      path: '/mock/app',
      name: 'mock-app',
    });

    mockInvoke.mockResolvedValueOnce({
      tech_stack: 'Node.js (TypeScript / Express)',
      config_files: ['package.json', 'tsconfig.json'],
      spec_files: ['openapi.yaml'],
      key_directories: ['src/routes'],
    });

    const res = await workspaceToolHandlers.scan_project_structure({});
    expect(mockInvoke).toHaveBeenCalledWith('ai_scan_workspace', {
      workspaceDir: '/mock/app',
    });
    expect(res.tech_stack).toBe('Node.js (TypeScript / Express)');
    expect(res.spec_files).toEqual(['openapi.yaml']);

    // 验证状态自动更新回填
    const currentWs = useAIStore.getState().activeWorkspace;
    expect(currentWs?.techStack).toBe('Node.js (TypeScript / Express)');
    expect(currentWs?.apiSpecPath).toBe('openapi.yaml');
  });

  it('search_project_code 正确向后端派发正则检索命令', async () => {
    useAIStore.getState().setSessionWorkspace({
      path: '/mock/app',
      name: 'mock-app',
    });

    mockInvoke.mockResolvedValueOnce([
      { file: 'src/routes.ts', line_number: 15, line_content: 'router.post("/login", handler);' },
    ]);

    const res = await workspaceToolHandlers.search_project_code({
      pattern: 'router\\.post',
      ext: 'ts',
      max_results: 10,
    });

    expect(mockInvoke).toHaveBeenCalledWith('ai_search_code', {
      workspaceDir: '/mock/app',
      pattern: 'router\\.post',
      ext: 'ts',
      maxResults: 10,
    });
    expect(res.matches_count).toBe(1);
    expect(res.matches[0].line_number).toBe(15);
  });

  it('read_source_code 正确解析相对路径并调用切片读取', async () => {
    useAIStore.getState().setSessionWorkspace({
      path: '/mock/app',
      name: 'mock-app',
    });

    mockInvoke.mockResolvedValueOnce('1 | const a = 1;\n2 | const b = 2;');

    const res = await workspaceToolHandlers.read_source_code({
      file_path: 'src/index.ts',
      start_line: 1,
      max_lines: 50,
    });

    expect(mockInvoke).toHaveBeenCalledWith('ai_read_code_slice', {
      filePath: '/mock/app/src/index.ts',
      startLine: 1,
      maxLines: 50,
    });
    expect(res.content).toContain('const a = 1;');
  });

  it('inspect_api_spec 正确调用后端规范解析', async () => {
    useAIStore.getState().setSessionWorkspace({
      path: '/mock/app',
      name: 'mock-app',
    });

    mockInvoke.mockResolvedValueOnce({
      format: 'openapi',
      endpoints: [{ path: '/api/v1/users', method: 'get', summary: 'List Users' }],
    });

    const res = await workspaceToolHandlers.inspect_api_spec({
      file_path: 'openapi.yaml',
    });

    expect(mockInvoke).toHaveBeenCalledWith('ai_inspect_api_spec', {
      filePath: '/mock/app/openapi.yaml',
    });
    expect(res.spec).toEqual({
      format: 'openapi',
      endpoints: [{ path: '/api/v1/users', method: 'get', summary: 'List Users' }],
    });
  });
});
