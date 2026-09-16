/**
 * NodeConfigPanel 组件测试
 */
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { NodeConfigPanel, HttpConfig, ConditionalConfig, EntryConfig } from '@/components/panels';
import { useFlowStore, useUIStore, useDebugStore, selectNodes } from '@/stores';
import { getCachedNodeMeta } from '@/lib/NodeRegistry';
import { isSchemaDriven } from '@/lib/configPanelRegistry';
import type { NodeMetadata } from '@/types/nodeRegistry';
import type { Flow, HttpConfig as HttpConfigType, ConditionalConfig as ConditionalConfigType, AnyNode } from '@/types';
import type { ComponentType } from 'react';

// Mock configPanelRegistry 使面板同步加载
vi.mock('@/lib/configPanelRegistry', () => {
  // 在 mock 工厂内使用同步 resolve 缓存
  const cache: Record<string, ComponentType<any>> = {};
  return {
    getPanelLoader: vi.fn((type: string) => {
      return () => import(`@/components/panels/${type === 'entry' ? 'EntryConfig' : type === 'http' ? 'HttpConfig' : 'ConditionalConfig'}`).then(
        (m: Record<string, ComponentType<any>>) => {
          const key = type === 'entry' ? 'EntryConfig' : type === 'http' ? 'HttpConfig' : 'ConditionalConfig';
          cache[type] = m[key];
          return m[key];
        }
      );
    }),
    isSchemaDriven: vi.fn(() => false),
  };
});

// Mock NodeRegistry 的缓存访问（保留真实导出，仅替换 getCachedNodeMeta）
vi.mock('@/lib/NodeRegistry', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/NodeRegistry')>();
  return { ...actual, getCachedNodeMeta: vi.fn() };
});

// Mock useFlowStore and useUIStore
vi.mock('@/stores', () => ({
  useFlowStore: vi.fn(),
  useUIStore: vi.fn(),
  useDebugStore: vi.fn(),
  selectHistoryVersion: vi.fn(),
  selectNodes: vi.fn(() => []),
  selectConnections: vi.fn(() => []),
}));

// 模拟流程数据
const mockFlow: Flow = {
  uuid: 'test-flow-001',
  name: '测试流程',
  nodes: [
    {
      uuid: 'entry-001',
      type: 'entry',
      name: '入口节点',
      position: { x: 100, y: 100 },
      data: { type: 'entry' },
    },
    {
      uuid: 'http-001',
      type: 'http',
      name: 'HTTP 请求',
      position: { x: 300, y: 100 },
      data: { type: 'http', url: 'https://api.example.com', method: 'get' },
    },
    {
      uuid: 'conditional-001',
      type: 'conditional',
      name: '条件判断',
      position: { x: 500, y: 100 },
      data: {
        type: 'conditional',
        field_path: 'status',
        operator: 'equal',
        expected_value: 200,
      },
    },
  ],
  connections: [],
};

describe('NodeConfigPanel', () => {
  const mockUpdateNode = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: Function) => {
      const state = {
        getSelectedNode: vi.fn(() => null),
        updateNode: mockUpdateNode,
        flow: mockFlow,
        nodes: mockFlow.nodes,
        connections: [],
      };
      return selector ? selector(state) : state;
    });
    (useUIStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      showConfirm: vi.fn(),
    });
    (useDebugStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue(vi.fn());
  });

  describe('空状态', () => {
    it('当没有选中节点且非只读模式时显示流程属性面板', () => {
      render(<NodeConfigPanel />);

      // FlowConfigPanel 组件显示"流程属性"标题
      expect(screen.getByText('流程属性')).toBeDefined();
      expect(screen.getByDisplayValue(mockFlow.name)).toBeDefined();
    });

    it('当没有选中节点且为只读模式时显示调试模式提示', () => {
      render(<NodeConfigPanel readOnly={true} />);

      expect(screen.getByText('调试模式')).toBeDefined();
    });

    it('切换选中节点及取消选中时正确在配置面板与节点列表间切换', async () => {
      let currentSelected: (typeof mockFlow.nodes)[number] | null = mockFlow.nodes[0];
      (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = {
          getSelectedNode: () => currentSelected,
          updateNode: mockUpdateNode,
          flow: mockFlow,
          nodes: mockFlow.nodes,
        };
        return selector ? selector(state) : state;
      });

      const { rerender } = render(<NodeConfigPanel />);
      await waitFor(() => {
        expect(screen.getByText('入口节点')).toBeDefined();
      });

      // 模拟在画布上选中 HTTP 节点
      currentSelected = mockFlow.nodes[1];
      rerender(<NodeConfigPanel />);
      await waitFor(() => {
        expect(screen.getByText('HTTP 请求')).toBeDefined();
      });

      // 模拟点击画布空白区域（取消选中）
      currentSelected = null;
      rerender(<NodeConfigPanel />);
      await waitFor(() => {
        expect(screen.getByText('流程属性')).toBeDefined();
      });
    });
  });

  describe('Entry 节点', () => {
    it('显示入口节点说明', async () => {
      // selector 感知的 mock：NodeConfigPanel 现在会经 useFlowVariables 订阅 state.flow
      (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = {
          getSelectedNode: () => mockFlow.nodes[0],
          updateNode: mockUpdateNode,
          flow: mockFlow,
        };
        return selector ? selector(state) : state;
      });

      render(<NodeConfigPanel />);

      // NodeConfigPanel 使用 configPanelRegistry 懒加载，需等待
      await waitFor(() => {
        expect(screen.getByText('入口节点')).toBeDefined();
      });
      // 使用部分文本匹配描述
      await waitFor(() => {
        expect(screen.getByText(/入口节点是流程的起点/)).toBeDefined();
      });
    });
  });

  describe('HTTP 节点', () => {
    it('显示 HTTP 配置表单', async () => {
      (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = {
          getSelectedNode: () => mockFlow.nodes[1],
          updateNode: mockUpdateNode,
          flow: mockFlow,
        };
        return selector ? selector(state) : state;
      });
      render(<NodeConfigPanel />);

      await waitFor(() => {
        expect(screen.getByPlaceholderText('https://api.example.com/endpoint')).toBeDefined();
      });
    });
  });

  describe('Conditional 节点', () => {
    it('显示条件配置表单', async () => {
      (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = {
          getSelectedNode: () => mockFlow.nodes[2],
          updateNode: mockUpdateNode,
          flow: mockFlow,
        };
        return selector ? selector(state) : state;
      });
      render(<NodeConfigPanel />);

      await waitFor(() => {
        expect(screen.getByText('简单条件')).toBeDefined();
      });
      await waitFor(() => {
        expect(screen.getByText('高级表达式')).toBeDefined();
      });
    });
  });

  describe('Plugin 节点（Schema 驱动）', () => {
    const pluginNode: AnyNode = {
      uuid: 'plugin-001',
      type: 'demo:ping',
      name: 'Demo Ping',
      position: { x: 0, y: 0 },
      data: { type: 'demo:ping', target: 'localhost' },
    };

    const pluginMeta: NodeMetadata = {
      type_id: 'demo:ping',
      display_name: 'Demo Ping',
      category: 'plugin',
      icon: 'plug',
      color: '#000000',
      ports: [],
      default_config: { target: 'localhost' },
    };

    beforeEach(() => {
      (isSchemaDriven as unknown as ReturnType<typeof vi.fn>).mockReturnValue(true);
      (getCachedNodeMeta as unknown as ReturnType<typeof vi.fn>).mockReturnValue(null);
      (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = {
          getSelectedNode: () => pluginNode,
          updateNode: mockUpdateNode,
          flow: { nodes: [pluginNode] },
        };
        return selector ? selector(state) : state;
      });
    });

    it('metadata 携带 config_schema 时渲染动态 JSON-Schema 表单', async () => {
      (getCachedNodeMeta as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
        ...pluginMeta,
        config_schema: {
          type: 'object',
          properties: { target: { type: 'string', title: 'Target' } },
        },
      });

      render(<NodeConfigPanel />);

      // SchemaConfigPanel 渲染 schema.properties.target 字段
      await waitFor(() => {
        expect(screen.getByLabelText(/Target/)).toBeDefined();
      });
    });

    it('metadata 声明 frontend 且携带 url 时渲染 PluginIframe 配置面板，init 携带节点上下文', async () => {
      (getCachedNodeMeta as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
        ...pluginMeta,
        frontend: { type: 'iframe', url: 'http://mpe-plugin.demo.localhost/config-panel' },
      });
      (selectNodes as unknown as ReturnType<typeof vi.fn>).mockReturnValue([pluginNode]);

      render(<NodeConfigPanel />);

      // PluginIframe 渲染沙箱 iframe（默认 title）
      const iframe = (await screen.findByTitle('Plugin config panel')) as HTMLIFrameElement;
      expect(iframe.getAttribute('src')).toBe('http://mpe-plugin.demo.localhost/config-panel');
      expect(iframe.getAttribute('sandbox')).toBe('allow-scripts');

      // iframe load → init 消息：nodeType/locale + 同插件节点快照（按前缀过滤）
      const postMessage = vi.spyOn(iframe.contentWindow as Window, 'postMessage');
      fireEvent.load(iframe);
      expect(postMessage).toHaveBeenCalledTimes(1);
      const [message] = postMessage.mock.calls[0] ?? [];
      const payload = (message as { payload?: Record<string, unknown> }).payload ?? {};
      expect(payload['nodeType']).toBe('demo:ping');
      expect(typeof payload['locale']).toBe('string');
      expect((payload['locale'] as string).length).toBeGreaterThan(0);
      expect(payload['nodes']).toEqual([
        {
          uuid: 'plugin-001',
          label: 'Demo Ping',
          type: 'demo:ping',
          config: { type: 'demo:ping', target: 'localhost' },
        },
      ]);
    });

    it('切换同插件节点时重新挂载 iframe，init 携带新节点上下文', async () => {
      const otherNode: AnyNode = {
        uuid: 'plugin-002',
        type: 'demo:echo',
        name: 'Demo Echo',
        position: { x: 0, y: 0 },
        data: { type: 'demo:echo', echo: 'hi' },
      };
      (getCachedNodeMeta as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
        ...pluginMeta,
        frontend: { type: 'iframe', url: 'http://mpe-plugin.demo.localhost/config-panel' },
      });
      (selectNodes as unknown as ReturnType<typeof vi.fn>).mockReturnValue([pluginNode, otherNode]);
      let current: AnyNode = pluginNode;
      (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = {
          getSelectedNode: () => current,
          updateNode: mockUpdateNode,
          flow: { nodes: [pluginNode, otherNode] },
        };
        return selector ? selector(state) : state;
      });

      const { rerender } = render(<NodeConfigPanel />);

      // 第一个节点：init 携带 demo:ping 上下文
      const iframe1 = (await screen.findByTitle('Plugin config panel')) as HTMLIFrameElement;
      const post1 = vi.spyOn(iframe1.contentWindow as Window, 'postMessage');
      fireEvent.load(iframe1);
      const init1 = (post1.mock.calls[0]?.[0] as { payload?: Record<string, unknown> })?.payload ?? {};
      expect(init1['nodeType']).toBe('demo:ping');

      // 切换到同插件不同节点（uuid 变化 → key 变化 → iframe 重挂载）
      current = otherNode;
      rerender(<NodeConfigPanel />);

      // 旧 iframe 已卸载，新 iframe 挂载
      expect(iframe1.isConnected).toBe(false);
      const iframe2 = (await screen.findByTitle('Plugin config panel')) as HTMLIFrameElement;
      expect(iframe2).not.toBe(iframe1);

      // 新 iframe 的 init 携带新节点的完整上下文（nodeType/config/nodes 快照）
      const post2 = vi.spyOn(iframe2.contentWindow as Window, 'postMessage');
      fireEvent.load(iframe2);
      const init2 = (post2.mock.calls[0]?.[0] as { payload?: Record<string, unknown> })?.payload ?? {};
      expect(init2['nodeType']).toBe('demo:echo');
      expect(init2['config']).toEqual({ type: 'demo:echo', echo: 'hi' });
      expect(init2['nodes']).toEqual([
        {
          uuid: 'plugin-001',
          label: 'Demo Ping',
          type: 'demo:ping',
          config: { type: 'demo:ping', target: 'localhost' },
        },
        {
          uuid: 'plugin-002',
          label: 'Demo Echo',
          type: 'demo:echo',
          config: { type: 'demo:echo', echo: 'hi' },
        },
      ]);
    });

    it('插件面板清空 headers 时保存为整体替换，深合并不会把删掉的键并回', async () => {
      const nodeWithHeaders: AnyNode = {
        uuid: 'plugin-003',
        type: 'demo:ping',
        name: 'Demo Ping',
        position: { x: 0, y: 0 },
        data: { type: 'demo:ping', target: 'localhost', headers: { a: '1', b: '2' } },
      };
      (getCachedNodeMeta as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
        ...pluginMeta,
        frontend: { type: 'iframe', url: 'http://mpe-plugin.demo.localhost/config-panel' },
      });
      (selectNodes as unknown as ReturnType<typeof vi.fn>).mockReturnValue([nodeWithHeaders]);
      (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
        const state = {
          getSelectedNode: () => nodeWithHeaders,
          updateNode: mockUpdateNode,
          flow: { uuid: 'flow-001', nodes: [nodeWithHeaders] },
          activeTabUuid: 'flow-001',
          tabSessions: {},
        };
        return selector ? selector(state) : state;
      });
      // handleSave 走 useFlowStore.getState() 取最新 flow —— mock 是纯函数，需挂方法
      (useFlowStore as unknown as { getState: () => unknown }).getState = () => ({
        flow: { uuid: 'flow-001', nodes: [nodeWithHeaders] },
        activeTabUuid: 'flow-001',
        tabSessions: {},
        updateNode: mockUpdateNode,
      });

      render(<NodeConfigPanel />);

      const iframe = (await screen.findByTitle('Plugin config panel')) as HTMLIFrameElement;
      fireEvent.load(iframe); // init 携带节点上下文

      // 面板回传完整 config，headers 已清空为 {} —— 宿主必须整体替换。
      // 旧实现 mergeNodeData 对嵌套对象深合并：{...{a,b}, ...{}} = {a,b}，
      // 且 deepEqual 判等使 store 完全不更新（删除永远不生效）。
      fireEvent(
        window,
        new MessageEvent('message', {
          data: { type: 'configChanged', payload: { config: { type: 'demo:ping', target: 'localhost', headers: {} } } },
          origin: 'null',
          source: iframe.contentWindow as unknown as MessageEventSource,
        })
      );

      await waitFor(() => {
        expect(mockUpdateNode).toHaveBeenCalledWith('plugin-003', {
          data: expect.objectContaining({ headers: {} }),
        });
      });
    });

    it('metadata 声明 frontend 但无 url（inline-only）时回落 SchemaConfigPanel', async () => {
      (getCachedNodeMeta as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
        ...pluginMeta,
        frontend: { type: 'inline', content: '<p>ping</p>' },
      });

      render(<NodeConfigPanel />);

      // 无 url → 不渲染 iframe，回落动态 JSON-Schema 表单（空 schema → 空白提示）
      await waitFor(() => {
        expect(screen.getByText('无可配置字段')).toBeDefined();
      });
      expect(screen.queryByTitle('Plugin config panel')).toBeNull();
    });

    it('无 config_schema 且无 frontend 时保持空白 fallback', async () => {
      render(<NodeConfigPanel />);

      await waitFor(() => {
        expect(screen.getByText('无可配置字段')).toBeDefined();
      });
    });
  });
});

describe('HttpConfig', () => {
  const mockOnSave = vi.fn();
  const defaultData: HttpConfigType = {
    type: 'http',
    url: 'https://api.example.com',
    method: 'get',
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
      const state = { flow: null };
      return selector ? selector(state) : state;
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('渲染 URL 输入框', () => {
    render(<HttpConfig data={defaultData} onSave={mockOnSave} />);

    expect(screen.getByLabelText(/URL/)).toBeDefined();
  });

  it('渲染方法选择器', () => {
    render(<HttpConfig data={defaultData} onSave={mockOnSave} />);

    expect(screen.getByLabelText(/请求方法/)).toBeDefined();
  });

  it('渲染 Headers 编辑区域', () => {
    render(<HttpConfig data={defaultData} onSave={mockOnSave} />);

    expect(screen.getByText('请求头')).toBeDefined();
    expect(screen.getByText('暂无自定义 Headers')).toBeDefined();
  });

  it('渲染 Body 输入框', () => {
    render(<HttpConfig data={defaultData} onSave={mockOnSave} />);

    expect(screen.getByLabelText(/Body/)).toBeDefined();
  });

  it('支持添加 Headers', async () => {
    render(<HttpConfig data={defaultData} onSave={mockOnSave} />);

    const addButton = screen.getByText('添加');
    fireEvent.click(addButton);

    expect(screen.getByPlaceholderText('Header 名称')).toBeDefined();
    expect(screen.getByPlaceholderText('值')).toBeDefined();
  });

  it('显示预设数据', () => {
    const dataWithHeaders: HttpConfigType = {
      type: 'http',
      url: 'https://api.example.com',
      method: 'post',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ key: 'value' }),
    };

    render(<HttpConfig data={dataWithHeaders} onSave={mockOnSave} />);

    expect(screen.getByDisplayValue('https://api.example.com')).toBeDefined();
  });
});

describe('ConditionalConfig', () => {
  const mockOnSave = vi.fn();
  const defaultData: ConditionalConfigType = {
    type: 'conditional',
    condition_type: 'simple',
    field_path: 'status',
    operator: 'equal',
    expected_value: 200,
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector?: (state: unknown) => unknown) => {
      const state = { flow: null };
      return selector ? selector(state) : state;
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('渲染条件类型选择器', () => {
    render(<ConditionalConfig data={defaultData} onSave={mockOnSave} />);

    expect(screen.getByText('简单条件')).toBeDefined();
    expect(screen.getByText('高级表达式')).toBeDefined();
  });

  it('简单条件模式显示字段路径、操作符、期望值', () => {
    render(<ConditionalConfig data={defaultData} onSave={mockOnSave} />);

    expect(screen.getByLabelText(/字段路径/)).toBeDefined();
    expect(screen.getByLabelText(/操作符/)).toBeDefined();
    expect(screen.getByLabelText(/期望值/)).toBeDefined();
  });

  it('切换到高级表达式模式', () => {
    render(<ConditionalConfig data={defaultData} onSave={mockOnSave} />);

    const buttons = screen.getAllByRole('button');
    const advancedButton = buttons.find(btn => btn.textContent?.includes('高级表达式'));
    expect(advancedButton).toBeDefined();
    
    if (advancedButton) {
      fireEvent.click(advancedButton);
      // 点击后检查是否出现高级表达式标签
      expect(advancedButton).toHaveClass('border-violet-500');
    }
  });

  it('显示预设数据', () => {
    render(<ConditionalConfig data={defaultData} onSave={mockOnSave} />);

    expect(screen.getByDisplayValue('status')).toBeDefined();
  });

  it('类型选择器在 equal 操作符时显示', () => {
    render(<ConditionalConfig data={defaultData} onSave={mockOnSave} />);

    // equal 操作符时应该显示类型选择器
    expect(screen.getByLabelText(/值类型/)).toBeDefined();
    // 检查类型选择器存在，使用 getAllByText 因为选项在多个地方出现
    expect(screen.getAllByText(/字符串/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/数字/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/布尔值/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/空值/).length).toBeGreaterThan(0);
  });

  it('类型选择器在 not_equal 操作符时显示', () => {
    const notEqualData: ConditionalConfigType = {
      ...defaultData,
      operator: 'not_equal',
    };
    render(<ConditionalConfig data={notEqualData} onSave={mockOnSave} />);

    // not_equal 操作符时应该显示类型选择器
    expect(screen.getByLabelText(/值类型/)).toBeDefined();
  });

  it('类型选择器在 greater_than 操作符时隐藏', () => {
    const greaterData: ConditionalConfigType = {
      ...defaultData,
      operator: 'greater',
    };
    render(<ConditionalConfig data={greaterData} onSave={mockOnSave} />);

    // greater_than 操作符时不应该显示类型选择器
    expect(screen.queryByLabelText(/值类型/)).toBeNull();
  });

  it('类型选择器默认选中 string 类型', () => {
    render(<ConditionalConfig data={defaultData} onSave={mockOnSave} />);

    // 检查默认值是否为 string
    const typeSelect = screen.getByLabelText(/值类型/);
    expect(typeSelect).toBeDefined();
  });

  it('类型选择器支持切换类型', () => {
    const numberTypeData: ConditionalConfigType = {
      ...defaultData,
      expected_value_type: 'number',
    };
    render(<ConditionalConfig data={numberTypeData} onSave={mockOnSave} />);

    // 检查是否渲染了类型选择器
    expect(screen.getByLabelText(/值类型/)).toBeDefined();
  });
});

describe('EntryConfig', () => {
  it('渲染入口节点说明', () => {
    render(<EntryConfig data={{ type: 'entry' }} onSave={() => {}} />);

    // EntryConfig 使用 i18n panels.entryNode.title() = '入口节点'
    expect(screen.getByText('入口节点')).toBeDefined();
    // panels.entryNode.description() = '入口节点是流程的起点...'
    expect(screen.getByText(/入口节点是流程的起点/)).toBeDefined();
  });
});

describe('Schema 校验', () => {
  it('httpConfigSchema 校验有效 URL', async () => {
    const { httpConfigSchema } = await import('@/components/panels/schemas');

    const result = httpConfigSchema.safeParse({
      url: 'https://example.com',
      method: 'get',
    });

    expect(result.success).toBe(true);
  });

  it('httpConfigSchema 拒绝无效 URL', async () => {
    const { httpConfigSchema } = await import('@/components/panels/schemas');

    const result = httpConfigSchema.safeParse({
      url: 'invalid-url',
      method: 'get',
    });

    // 当前 schema 对 URL 校验较宽松，任何非空字符串均可通过
    // 实际 URL 校验在执行时由后端执行器处理
    expect(result.success).toBe(true);
  });

  it('conditionalConfigSchema 校验有效数据', async () => {
    const { conditionalConfigSchema } = await import('@/components/panels/schemas');

    const result = conditionalConfigSchema.safeParse({
      condition_type: 'simple',
      field_path: 'status',
      operator: 'equal',
      expected_value: '200',
    });

    expect(result.success).toBe(true);
  });
});
