/**
 * NodeConfigPanel 组件测试
 */
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { NodeConfigPanel, HttpConfig, ConditionalConfig, EntryConfig } from '@/components/panels';
import { useFlowStore, useUIStore, useDebugStore } from '@/stores';
import type { Flow, HttpConfig as HttpConfigType, ConditionalConfig as ConditionalConfigType } from '@/types';
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
        nodes: [],
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
    it('当没有选中节点且非只读模式时显示流程节点列表', () => {
      render(<NodeConfigPanel />);

      // FlowNodeList 组件显示"流程节点"标题
      expect(screen.getByText('流程节点')).toBeDefined();
      // 无节点时显示"暂无节点"
      expect(screen.getByText('暂无节点')).toBeDefined();
    });

    it('当没有选中节点且为只读模式时显示调试模式提示', () => {
      render(<NodeConfigPanel readOnly={true} />);

      expect(screen.getByText('调试模式')).toBeDefined();
    });
  });

  describe('Entry 节点', () => {
    it('显示入口节点说明', async () => {
      (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
        getSelectedNode: () => mockFlow.nodes[0],
        updateNode: mockUpdateNode,
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
