/**
 * VariableExtractorConfig 组件测试
 * - 渲染输出映射与数据来源节点当前值
 * - 提取行添加/删除（动态行管理）
 * - 编辑 + 卸载保存 (onSave)
 * - 数据来源节点选择保存
 * - 重复变量名提示
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { VariableExtractorConfig } from '@/components/panels/VariableExtractorConfig';
import { useFlowStore } from '@/stores';
import type { VariableExtractorConfig as VariableExtractorConfigType, OutputMapping } from '@/types';

// jsdom 未实现 Pointer Capture / scrollIntoView（SearchableNodeSelect 下拉依赖）
if (typeof Element !== 'undefined') {
  Element.prototype.hasPointerCapture = () => false;
  Element.prototype.setPointerCapture = () => undefined;
  Element.prototype.releasePointerCapture = () => undefined;
  Element.prototype.scrollIntoView = () => undefined;
}

// Mock react-hook-form 的 zodResolver（简化 schema 验证）
vi.mock('@hookform/resolvers/zod', () => ({
  zodResolver: (_schema: unknown) => (values: unknown) => ({
    values,
    errors: {},
  }),
}));

// Mock useFlowStore（避免真实 store 依赖）
vi.mock('@/stores', () => ({
  useFlowStore: vi.fn(),
}));

/** 模拟流程：包含一个 http 上游节点 */
const mockFlow = {
  uuid: 'test-flow',
  name: '测试流程',
  nodes: [
    {
      uuid: 'node-http-1',
      type: 'http',
      name: 'HTTP 请求',
      position: { x: 0, y: 0 },
      data: { type: 'http' },
    },
  ],
  connections: [],
};

/**
 * 创建基础 VariableExtractorConfig 数据
 */
function createBaseConfig(
  overrides: Partial<VariableExtractorConfigType> = {},
): VariableExtractorConfigType {
  return {
    type: 'variable_extractor',
    ...overrides,
  };
}

describe('VariableExtractorConfig', () => {
  const mockOnSave = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    (useFlowStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (state: unknown) => unknown) => {
        return selector({ flow: mockFlow, selectedNodeId: 'node-extractor-1' });
      },
    );
  });

  describe('基础渲染', () => {
    it('渲染输出映射当前值', () => {
      const mappings: OutputMapping[] = [
        { source: '$.status', target: 'status_code', description: '状态码' },
      ];
      render(
        <VariableExtractorConfig data={createBaseConfig({ output_mappings: mappings })} onSave={mockOnSave} />,
      );

      expect(screen.getByText('输出变量映射')).toBeDefined();
      expect(screen.getByDisplayValue('status_code')).toBeDefined();
      expect(screen.getByDisplayValue('$.status')).toBeDefined();
      expect(screen.getByDisplayValue('状态码')).toBeDefined();
    });

    it('渲染当前数据来源节点', () => {
      render(
        <VariableExtractorConfig
          data={createBaseConfig({ source_node_id: 'node-http-1' })}
          onSave={mockOnSave}
        />,
      );

      expect(screen.getByText('HTTP 请求 (http)')).toBeDefined();
    });
  });

  describe('提取行管理', () => {
    it('无映射时显示空提示，点击添加映射出现新行', async () => {
      const user = userEvent.setup();
      render(<VariableExtractorConfig data={createBaseConfig()} onSave={mockOnSave} />);

      expect(screen.getByText('暂无输出映射')).toBeDefined();
      await user.click(screen.getByRole('button', { name: '添加映射' }));

      // 新行使用默认 source '$.status'
      expect(screen.getByDisplayValue('$.status')).toBeDefined();
      expect(screen.getByPlaceholderText('变量名（字母、数字、下划线）')).toBeDefined();
    });

    it('添加行并编辑后卸载保存（onSave 收到新映射）', async () => {
      const user = userEvent.setup();
      const { unmount } = render(<VariableExtractorConfig data={createBaseConfig()} onSave={mockOnSave} />);

      await user.click(screen.getByRole('button', { name: '添加映射' }));
      const targetInput = screen.getByPlaceholderText('变量名（字母、数字、下划线）') as HTMLInputElement;
      await user.type(targetInput, 'new_var');
      const sourceInput = screen.getByPlaceholderText('源字段（如 $.status、$.body.data.token）') as HTMLInputElement;
      fireEvent.change(sourceInput, { target: { value: '$.body.data' } });
      unmount();

      // 表单 onBlur（焦点进入表单）与卸载清理都可能触发保存，断言最后一次保存 payload
      expect(mockOnSave).toHaveBeenLastCalledWith(
        expect.objectContaining({
          type: 'variable_extractor',
          output_mappings: [{ source: '$.body.data', target: 'new_var', description: '' }],
        }),
      );
    });

    it('删除行后延迟保存剩余映射', async () => {
      const user = userEvent.setup();
      const { container } = render(
        <VariableExtractorConfig
          data={createBaseConfig({
            output_mappings: [
              { source: '$.status', target: 'first' },
              { source: '$.body', target: 'second' },
            ],
          })}
          onSave={mockOnSave}
        />,
      );

      // 删除按钮为纯图标按钮（无可访问文本）
      const deleteButtons = Array.from(container.querySelectorAll('button')).filter(
        (btn) => !(btn.textContent ?? '').trim(),
      );
      expect(deleteButtons).toHaveLength(2);
      await user.click(deleteButtons[0]);

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalled();
      });
      const payload = mockOnSave.mock.calls[0][0] as VariableExtractorConfigType;
      expect(payload.output_mappings).toHaveLength(1);
      expect(payload.output_mappings?.[0].target).toBe('second');
    });
  });

  describe('数据来源选择', () => {
    it('选择上游节点后延迟保存 source_node_id', async () => {
      const user = userEvent.setup();
      render(<VariableExtractorConfig data={createBaseConfig()} onSave={mockOnSave} />);

      await user.click(screen.getByRole('button', { name: /自动（最近上游节点输出）/ }));
      await user.click(screen.getByRole('button', { name: /HTTP 请求/ }));

      await waitFor(() => {
        expect(mockOnSave).toHaveBeenCalledWith(
          expect.objectContaining({ source_node_id: 'node-http-1' }),
        );
      });
    });
  });

  describe('重复变量名提示', () => {
    it('输出映射目标变量名重复时显示提示', () => {
      render(
        <VariableExtractorConfig
          data={createBaseConfig({
            output_mappings: [
              { source: '$.status', target: 'dup' },
              { source: '$.body', target: 'dup' },
            ],
          })}
          onSave={mockOnSave}
        />,
      );

      expect(screen.getAllByText('变量名已存在').length).toBeGreaterThan(0);
    });
  });
});
