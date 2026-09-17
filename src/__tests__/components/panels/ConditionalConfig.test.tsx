/**
 * ConditionalConfig 组件测试
 * 测试条件节点配置面板的渲染和交互
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ConditionalConfig } from '@/components/panels/ConditionalConfig';
import type { ConditionalConfig as ConditionalConfigType } from '@/types';

// Mock useFlowStore
vi.mock('@/stores', () => ({
  useFlowStore: vi.fn(),
}));

import { useFlowStore } from '@/stores';

describe('ConditionalConfig', () => {
  const mockOnSave = vi.fn();
  const defaultData: ConditionalConfigType = {
    type: 'conditional',
    condition_type: 'simple',
    field_path: '',
    operator: 'equal',
    expected_value: undefined,
    advanced_expression: undefined,
  };

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
      {
        uuid: 'http-1',
        type: 'http:request',
        name: 'HTTP请求',
        position: { x: 200, y: 0 },
        data: {
          type: 'http:request',
          url: '{{api_base}}/users',
          method: 'get',
          output_mappings: [
            { source: 'status', target: 'status', description: 'HTTP状态码' },
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
    it('应该渲染条件类型选择按钮', () => {
      render(<ConditionalConfig data={defaultData} onSave={mockOnSave} />);

      // 验证简单条件按钮存在
      expect(screen.getByRole('button', { name: /简单条件/i })).toBeDefined();
      // 验证高级表达式按钮存在
      expect(screen.getByRole('button', { name: /高级表达式/i })).toBeDefined();
    });

    it('默认应该选中简单条件模式', () => {
      render(<ConditionalConfig data={defaultData} onSave={mockOnSave} />);

      // 简单条件按钮应该有选中样式
      const simpleButton = screen.getByRole('button', { name: /简单条件/i });
      expect(simpleButton.className).toContain('border-sky-500');
    });

    it('简单条件模式下应该显示字段路径、操作符、期望值输入', () => {
      render(<ConditionalConfig data={defaultData} onSave={mockOnSave} />);

      // 验证字段路径标签
      expect(screen.getByLabelText(/字段路径/i)).toBeDefined();
      // 验证操作符标签
      expect(screen.getByLabelText(/操作符/i)).toBeDefined();
      // 验证期望值标签
      expect(screen.getByLabelText(/期望值/i)).toBeDefined();
    });
  });

  describe('条件类型切换', () => {
    it('点击高级表达式按钮应该切换到高级模式', async () => {
      const user = userEvent.setup();
      render(<ConditionalConfig data={defaultData} onSave={mockOnSave} />);

      // 点击高级表达式按钮
      const advancedButton = screen.getByRole('button', { name: /高级表达式/i });
      await user.click(advancedButton);

      // 高级表达式按钮应该有选中样式
      expect(advancedButton.className).toContain('border-violet-500');
      // 简单条件按钮不应该有选中样式
      const simpleButton = screen.getByRole('button', { name: /简单条件/i });
      expect(simpleButton.className).not.toContain('border-sky-500');
    });

    it('切换到高级模式后应该显示高级表达式输入框', async () => {
      const user = userEvent.setup();
      render(<ConditionalConfig data={defaultData} onSave={mockOnSave} />);

      // 点击高级表达式按钮
      await user.click(screen.getByRole('button', { name: /高级表达式/i }));

      // 应该显示高级表达式文本框
      const textarea = screen.getByLabelText(/高级表达式/i);
      expect(textarea).toBeDefined();
      expect(textarea.tagName.toLowerCase()).toBe('textarea');
    });

    it('高级模式不应该显示简单条件的输入框', async () => {
      const user = userEvent.setup();
      render(<ConditionalConfig data={defaultData} onSave={mockOnSave} />);

      // 切换到高级模式
      await user.click(screen.getByRole('button', { name: /高级表达式/i }));

      // 字段路径输入框不应存在
      expect(screen.queryByLabelText(/字段路径/i)).toBeNull();
      // 期望值输入框不应存在
      expect(screen.queryByLabelText(/期望值/i)).toBeNull();
    });
  });

  describe('简单条件配置', () => {
    it('应该显示值类型选择器当操作符为 equal', () => {
      const data: ConditionalConfigType = {
        ...defaultData,
        operator: 'equal',
      };

      render(<ConditionalConfig data={data} onSave={mockOnSave} />);

      // 验证值类型选择器存在
      expect(screen.getByLabelText(/值类型/i)).toBeDefined();
    });

    it('应该显示值类型选择器当操作符为 not_equal', () => {
      const data: ConditionalConfigType = {
        ...defaultData,
        operator: 'not_equal',
      };

      render(<ConditionalConfig data={data} onSave={mockOnSave} />);

      // 验证值类型选择器存在
      expect(screen.getByLabelText(/值类型/i)).toBeDefined();
    });

    it('操作符不为 equal 或 not_equal 时不显示值类型选择器', () => {
      const data: ConditionalConfigType = {
        ...defaultData,
        operator: 'greater',
      };

      render(<ConditionalConfig data={data} onSave={mockOnSave} />);

      // 值类型选择器不应存在
      expect(screen.queryByLabelText(/值类型/i)).toBeNull();
    });

    it('应该显示操作符选项提示', () => {
      render(<ConditionalConfig data={defaultData} onSave={mockOnSave} />);

      // 点击操作符下拉框触发显示
      const operatorSelect = screen.getByLabelText(/操作符/i);
      expect(operatorSelect).toBeDefined();
    });
  });

  describe('高级表达式配置', () => {
    it('高级模式应该显示运算符说明', async () => {
      const user = userEvent.setup();
      render(<ConditionalConfig data={defaultData} onSave={mockOnSave} />);

      // 切换到高级模式
      await user.click(screen.getByRole('button', { name: /高级表达式/i }));

      // 验证运算符说明存在
      expect(screen.getByText(/比较运算符/i)).toBeDefined();
      expect(screen.getByText(/逻辑运算符/i)).toBeDefined();
    });

    it('高级模式应该显示变量引用说明', async () => {
      const user = userEvent.setup();
      render(<ConditionalConfig data={defaultData} onSave={mockOnSave} />);

      // 切换到高级模式
      await user.click(screen.getByRole('button', { name: /高级表达式/i }));

      // 验证变量引用说明存在
      expect(screen.getByText(/变量引用方式/i)).toBeDefined();
    });

    it('高级模式应该显示注意事项', async () => {
      const user = userEvent.setup();
      render(<ConditionalConfig data={defaultData} onSave={mockOnSave} />);

      // 切换到高级模式
      await user.click(screen.getByRole('button', { name: /高级表达式/i }));

      // 验证注意事项存在
      expect(screen.getByText(/注意事项/i)).toBeDefined();
    });

    it('初始化为高级模式时应该正确显示', () => {
      const advancedData: ConditionalConfigType = {
        type: 'conditional',
        condition_type: 'advanced',
        advanced_expression: 'status == 200',
      };

      render(<ConditionalConfig data={advancedData} onSave={mockOnSave} />);

      // 高级表达式按钮应该有选中样式
      const advancedButton = screen.getByRole('button', { name: /高级表达式/i });
      expect(advancedButton.className).toContain('border-violet-500');

      // 应该显示高级表达式文本框
      expect(screen.getByLabelText(/高级表达式/i)).toBeDefined();
    });
  });

  describe('表单数据初始化', () => {
    it('应该正确初始化简单条件数据', () => {
      const simpleData: ConditionalConfigType = {
        type: 'conditional',
        condition_type: 'simple',
        field_path: '{{status}}',
        operator: 'equal',
        expected_value: '200',
      };

      render(<ConditionalConfig data={simpleData} onSave={mockOnSave} />);

      // 验证简单条件按钮选中
      const simpleButton = screen.getByRole('button', { name: /简单条件/i });
      expect(simpleButton.className).toContain('border-sky-500');
    });

    it('应该正确初始化高级表达式数据', () => {
      const advancedData: ConditionalConfigType = {
        type: 'conditional',
        condition_type: 'advanced',
        advanced_expression: 'status >= 200 && status < 300',
      };

      render(<ConditionalConfig data={advancedData} onSave={mockOnSave} />);

      // 验证高级表达式按钮选中
      const advancedButton = screen.getByRole('button', { name: /高级表达式/i });
      expect(advancedButton.className).toContain('border-violet-500');
    });
  });

  describe('无 flow 数据时的处理', () => {
    it('当 flow 为 null 时组件应该正常渲染', () => {
      mockFlowStoreWithFlow(null);

      render(<ConditionalConfig data={defaultData} onSave={mockOnSave} />);

      // 组件应该正常渲染
      expect(screen.getByRole('button', { name: /简单条件/i })).toBeDefined();
      expect(screen.getByRole('button', { name: /高级表达式/i })).toBeDefined();
    });
  });
});
