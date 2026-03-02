/**
 * NodeConfigPanel 组件测试
 */
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { NodeConfigPanel, HttpConfig, ConditionalConfig, EntryConfig } from '@/components/panels';
import { useFlowStore, useUIStore } from '@/stores';
import type { Flow, HttpConfig as HttpConfigType, ConditionalConfig as ConditionalConfigType } from '@/types';

// Mock useFlowStore and useUIStore
vi.mock('@/stores', () => ({
  useFlowStore: vi.fn(),
  useUIStore: vi.fn(),
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
    (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      getSelectedNode: vi.fn(() => null),
      updateNode: mockUpdateNode,
    });
    (useUIStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
      showConfirm: vi.fn(),
    });
  });

  describe('空状态', () => {
    it('当没有选中节点时显示提示', () => {
      render(<NodeConfigPanel />);

      expect(screen.getByText('未选中节点')).toBeDefined();
      expect(screen.getByText('请在画布上点击选择一个节点进行配置')).toBeDefined();
    });
  });

  describe('Entry 节点', () => {
    it('显示入口节点说明', () => {
      (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
        getSelectedNode: () => mockFlow.nodes[0],
        updateNode: mockUpdateNode,
      });

      render(<NodeConfigPanel />);

      // 使用 getAllByText 因为"入口节点"出现在多个地方
      const entryNodeTexts = screen.getAllByText('入口节点');
      expect(entryNodeTexts.length).toBeGreaterThan(0);
      // 使用部分文本匹配
      expect(screen.getByText(/入口节点是流程的起点/)).toBeDefined();
    });
  });

  describe('HTTP 节点', () => {
    it('显示 HTTP 配置表单', () => {
      (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
        getSelectedNode: () => mockFlow.nodes[1],
        updateNode: mockUpdateNode,
      });

      render(<NodeConfigPanel />);

      expect(screen.getByText('HTTP 请求')).toBeDefined();
      expect(screen.getByPlaceholderText('https://api.example.com/endpoint')).toBeDefined();
    });
  });

  describe('Conditional 节点', () => {
    it('显示条件配置表单', () => {
      (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockReturnValue({
        getSelectedNode: () => mockFlow.nodes[2],
        updateNode: mockUpdateNode,
      });

      render(<NodeConfigPanel />);

      expect(screen.getByText('条件判断')).toBeDefined();
      expect(screen.getByText('简单条件')).toBeDefined();
      expect(screen.getByText('高级表达式')).toBeDefined();
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

    expect(screen.getByText('Headers')).toBeDefined();
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

    expect(screen.getByPlaceholderText('Header Name')).toBeDefined();
    expect(screen.getByPlaceholderText('Value')).toBeDefined();
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
});

describe('EntryConfig', () => {
  it('渲染入口节点说明', () => {
    render(<EntryConfig />);

    expect(screen.getByText('入口节点')).toBeDefined();
    // 使用部分文本匹配，因为文本包含逗号
    expect(screen.getByText(/入口节点是流程的起点/)).toBeDefined();
  });

  it('显示功能说明', () => {
    render(<EntryConfig />);

    expect(screen.getByText('无需配置')).toBeDefined();
    expect(screen.getByText('执行流程')).toBeDefined();
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

    expect(result.success).toBe(false);
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
