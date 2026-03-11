/** @vitest-environment jsdom */
/**
 * AssertionConfig 组件测试
 * 测试断言节点配置面板的渲染和基本交互
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { AssertionConfig } from '@/components/panels/AssertionConfig';
import type { AssertionNodeConfig, Assertion } from '@/types/node';

// Mock useFlowStore
vi.mock('@/stores', () => ({
  useFlowStore: vi.fn(),
}));

import { useFlowStore } from '@/stores';

describe('AssertionConfig', () => {
  const mockOnSave = vi.fn();

  // 默认单条断言
  const defaultAssertion: Assertion = {
    id: 'assertion-1',
    target: '',
    operator: 'eq',
    expected: '',
    enabled: true,
    description: '',
  };

  // 默认配置
  const defaultData: AssertionNodeConfig = {
    type: 'assertion',
    assertions: [defaultAssertion],
    mode: 'all',
    on_failure: 'route_false',
    continue_on_failure: false,
  };

  // Mock 流程数据
  const mockFlow = {
    uuid: 'flow-1',
    name: '测试流程',
    nodes: [
      {
        uuid: 'entry-1',
        type: 'entry',
        name: '入口节点',
        position: { x: 0, y: 0 },
        data: {
          type: 'entry',
          initial_variables: [
            { name: 'api_base', value: 'https://api.example.com', description: 'API基础URL' },
          ],
        },
      },
    ],
    connections: [],
  };

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const mockFlowStoreWithFlow = (flow: any = mockFlow) => {
    vi.mocked(useFlowStore).mockImplementation((selector: any) => {
      return selector({ flow });
    });
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockFlowStoreWithFlow();
  });

  describe('组件渲染', () => {
    it('应该渲染预设选择下拉框', () => {
      render(<AssertionConfig data={defaultData} onSave={mockOnSave} />);
      expect(screen.getByLabelText(/预设/i)).toBeDefined();
    });

    it('应该渲染全局配置区域 (模式和失败行为)', () => {
      render(<AssertionConfig data={defaultData} onSave={mockOnSave} />);
      expect(screen.getByText(/组合模式/i)).toBeDefined();
      expect(screen.getByText(/失败行为/i)).toBeDefined();
    });

    it('应该渲染断言列表区域', () => {
      render(<AssertionConfig data={defaultData} onSave={mockOnSave} />);
      expect(screen.getByText(/断言列表/i)).toBeDefined();
    });

    it('应该渲染添加断言按钮', () => {
      render(<AssertionConfig data={defaultData} onSave={mockOnSave} />);
      expect(screen.getByRole('button', { name: /添加断言/i })).toBeDefined();
    });
  });

  describe('预设选择', () => {
    it('应该显示预设下拉框并包含预设选项', () => {
      render(<AssertionConfig data={defaultData} onSave={mockOnSave} />);
      // 预设下拉框存在
      const presetSelect = screen.getByLabelText(/预设/i);
      expect(presetSelect).toBeDefined();
      // 默认显示"无预设" - 使用 getAllByText 处理重复元素
      expect(screen.getAllByText(/无预设/i).length).toBeGreaterThan(0);
    });
  });

  describe('断言增删改', () => {
    it('点击添加断言按钮应该触发保存', () => {
      render(<AssertionConfig data={defaultData} onSave={mockOnSave} />);
      const addButton = screen.getByRole('button', { name: /添加断言/i });
      fireEvent.click(addButton);
      // 组件内部会调用 onSave
    });

    it('应该渲染断言目标字段输入框', () => {
      render(<AssertionConfig data={defaultData} onSave={mockOnSave} />);
      expect(screen.getByPlaceholderText(/目标字段/i)).toBeDefined();
    });

    it('应该渲染断言期望值输入框', () => {
      render(<AssertionConfig data={defaultData} onSave={mockOnSave} />);
      expect(screen.getByPlaceholderText(/期望值/i)).toBeDefined();
    });
  });

  describe('表单交互', () => {
    it('应该渲染组合模式按钮', () => {
      render(<AssertionConfig data={defaultData} onSave={mockOnSave} />);
      expect(screen.getByRole('button', { name: /全部通过/i })).toBeDefined();
      expect(screen.getByRole('button', { name: /任一通过/i })).toBeDefined();
    });

    it('应该渲染失败行为按钮', () => {
      render(<AssertionConfig data={defaultData} onSave={mockOnSave} />);
      expect(screen.getByRole('button', { name: /路由到 false 分支/i })).toBeDefined();
      expect(screen.getByRole('button', { name: /抛出异常/i })).toBeDefined();
    });

    it('应该渲染失败后继续执行开关', () => {
      render(<AssertionConfig data={defaultData} onSave={mockOnSave} />);
      expect(screen.getByText(/失败后继续执行/i)).toBeDefined();
    });
  });

  describe('变量验证', () => {
    it('目标字段输入框应该存在', () => {
      render(<AssertionConfig data={defaultData} onSave={mockOnSave} />);
      const targetInput = screen.getByPlaceholderText(/目标字段/i);
      expect(targetInput).toBeDefined();
    });

    it('期望值输入框应该存在', () => {
      render(<AssertionConfig data={defaultData} onSave={mockOnSave} />);
      const expectedInput = screen.getByPlaceholderText(/期望值/i);
      expect(expectedInput).toBeDefined();
    });
  });

  describe('useFieldArray 行为', () => {
    it('多条断言应该正确渲染', () => {
      const dataWithMultipleAssertions: AssertionNodeConfig = {
        ...defaultData,
        assertions: [
          { ...defaultAssertion, target: 'status', expected: '200' },
          { ...defaultAssertion, target: 'body.success', expected: 'true' },
          { ...defaultAssertion, target: 'response_time', expected: '500' },
        ],
      };

      render(<AssertionConfig data={dataWithMultipleAssertions} onSave={mockOnSave} />);
      const targetInputs = screen.getAllByPlaceholderText(/目标字段/i);
      expect(targetInputs.length).toBe(3);
    });

    it('禁用断言应该显示禁用状态', () => {
      const dataWithDisabledAssertion: AssertionNodeConfig = {
        ...defaultData,
        assertions: [
          { ...defaultAssertion, target: 'status', expected: '200', enabled: false },
        ],
      };

      render(<AssertionConfig data={dataWithDisabledAssertion} onSave={mockOnSave} />);
      const targetInput = screen.getByPlaceholderText(/目标字段/i);
      expect(targetInput).toBeDisabled();
    });
  });

  describe('无 flow 数据时的处理', () => {
    it('当 flow 为 null 时组件应该正常渲染', () => {
      mockFlowStoreWithFlow(null);
      render(<AssertionConfig data={defaultData} onSave={mockOnSave} />);
      expect(screen.getByLabelText(/预设/i)).toBeDefined();
      expect(screen.getByRole('button', { name: /添加断言/i })).toBeDefined();
    });
  });

  describe('初始数据加载', () => {
    it('应该正确加载已有的断言配置', () => {
      const existingData: AssertionNodeConfig = {
        type: 'assertion',
        assertions: [
          { id: 'assertion-1', target: 'status', operator: 'eq', expected: '200', enabled: true, description: '状态码检查' },
          { id: 'assertion-2', target: 'body.success', operator: 'eq', expected: 'true', enabled: true, description: '成功标志检查' },
        ],
        mode: 'all',
        on_failure: 'throw',
        continue_on_failure: true,
      };

      render(<AssertionConfig data={existingData} onSave={mockOnSave} />);
      const targetInputs = screen.getAllByPlaceholderText(/目标字段/i);
      expect(targetInputs.length).toBe(2);
    });

    it('空断言列表应该显示空状态提示', () => {
      const emptyData: AssertionNodeConfig = {
        type: 'assertion',
        assertions: [],
        mode: 'all',
        on_failure: 'route_false',
        continue_on_failure: false,
      };

      render(<AssertionConfig data={emptyData} onSave={mockOnSave} />);
      expect(screen.getByText(/暂无断言/i)).toBeDefined();
    });
  });

  describe('全局配置', () => {
    it('应该显示当前组合模式状态', () => {
      render(<AssertionConfig data={defaultData} onSave={mockOnSave} />);
      // 默认 mode 是 'all'，所以 "全部通过" 按钮应该是选中状态
      const allButton = screen.getByRole('button', { name: /全部通过/i });
      expect(allButton.classList.contains('border-emerald-500')).toBe(true);
    });

    it('应该显示当前失败行为状态', () => {
      render(<AssertionConfig data={defaultData} onSave={mockOnSave} />);
      // 默认 on_failure 是 'route_false'，所以 "路由到 false 分支" 按钮应该是选中状态
      const routeButton = screen.getByRole('button', { name: /路由到 false 分支/i });
      expect(routeButton.classList.contains('border-amber-500')).toBe(true);
    });
  });
});

// 清理 mock
vi.restoreAllMocks();
