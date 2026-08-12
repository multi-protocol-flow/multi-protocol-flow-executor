/**
 * SchemaConfigPanel 测试
 *
 * 回归覆盖：非必填的 `x-node-selector` 字段渲染「清空选择」项时，
 * 不得使用空字符串作为 Radix Select.Item 的 value（Radix 明确禁止：
 * 空串是清除选择、显示 placeholder 的保留值，会在 SelectContent 挂载时
 * 直接 throw）。此前 db:close 的 connection_uuid（required = []）命中该
 * 路径，画布点击节点打开面板即崩溃。
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import type { Mock } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { SchemaConfigPanel } from '@/components/panels/SchemaConfigPanel';
import { useFlowStore } from '@/stores/flow';
import type { AnyNode } from '@/types';

vi.mock('@/stores/flow', () => ({
  useFlowStore: vi.fn(),
  selectNodes: vi.fn((state: MockFlowState) => state.flow?.nodes ?? []),
}));

// Radix Select 在打开下拉时对高亮项调用 scrollIntoView，jsdom 未实现
if (typeof Element.prototype.scrollIntoView !== 'function') {
  Element.prototype.scrollIntoView = () => {};
}

/** useFlowStore 的 mock 状态：只包含组件实际用到的 flow.nodes */
type MockFlowState = { flow: { nodes: AnyNode[] } | null };

const CONNECT_NODE: AnyNode = {
  uuid: 'db-connect-001',
  type: 'db:connect',
  name: 'Connect Database',
  position: { x: 0, y: 0 },
  data: { type: 'db:connect', driver: 'sqlite' },
};

/** db:close 的 schema 形状：connection_uuid 非必填 + x-node-selector */
const CLOSE_SCHEMA = {
  type: 'object',
  properties: {
    connection_uuid: {
      type: 'string',
      title: 'Connection node',
      'x-node-selector': { node_type: 'db:connect' },
    },
  },
  required: [] as string[],
};

describe('SchemaConfigPanel', () => {
  const useFlowStoreMock = useFlowStore as unknown as Mock<
    (selector?: (state: MockFlowState) => unknown) => unknown
  >;

  beforeEach(() => {
    vi.clearAllMocks();
    useFlowStoreMock.mockImplementation((selector) => {
      const state: MockFlowState = { flow: { nodes: [CONNECT_NODE] } };
      return selector ? selector(state) : state;
    });
  });

  it('非必填 x-node-selector 字段可正常展开下拉，不触发 Radix 空值异常', async () => {
    const onSave = vi.fn();
    render(
      <SchemaConfigPanel
        schema={CLOSE_SCHEMA}
        data={{ type: 'db:close', connection_uuid: '' }}
        onSave={onSave}
      />
    );

    // 打开下拉：此前 value="" 的「清空选择」项会在 SelectContent 挂载时 throw
    const trigger = screen.getByLabelText(/Connection node/);
    fireEvent.click(trigger);

    // 正常渲染出节点项与「清空选择」项（清空项必须用哨兵值而非空串）
    const clearItem = await screen.findByText('清空选择');
    expect(clearItem).toBeDefined();
    expect(screen.getByText('Connect Database')).toBeDefined();
  });

  it('选中「清空选择」后把字段值写回空字符串', async () => {
    const onSave = vi.fn();
    render(
      <SchemaConfigPanel
        schema={CLOSE_SCHEMA}
        data={{ type: 'db:close', connection_uuid: 'db-connect-001' }}
        onSave={onSave}
      />
    );

    fireEvent.click(screen.getByLabelText(/Connection node/));
    const clearItem = await screen.findByText('清空选择');
    fireEvent.click(clearItem);

    // 值经 debounce 保存为 ''（清除选择，placeholder 态）
    await waitFor(() => {
      expect(onSave).toHaveBeenCalledWith(
        expect.objectContaining({ connection_uuid: '' })
      );
    });
  });

  it('非必填 enum 字段渲染「清空选择」项且可正常展开', async () => {
    const onSave = vi.fn();
    render(
      <SchemaConfigPanel
        schema={{
          type: 'object',
          properties: {
            fetch_mode: { type: 'string', enum: ['all', 'one', 'scalar'], title: 'Fetch mode' },
          },
          required: [],
        }}
        data={{ type: 'db:query', fetch_mode: '' }}
        onSave={onSave}
      />
    );

    fireEvent.click(screen.getByLabelText(/Fetch mode/));
    const clearItem = await screen.findByText('清空选择');
    expect(clearItem).toBeDefined();
    expect(screen.getByText('all')).toBeDefined();
  });

  it('必填 enum 字段不渲染「清空选择」项', async () => {
    render(
      <SchemaConfigPanel
        schema={{
          type: 'object',
          properties: {
            fetch_mode: { type: 'string', enum: ['all', 'one'], title: 'Fetch mode' },
          },
          required: ['fetch_mode'],
        }}
        data={{ type: 'db:query', fetch_mode: 'all' }}
        onSave={vi.fn()}
      />
    );

    fireEvent.click(screen.getByLabelText(/Fetch mode/));
    await screen.findByRole('option', { name: 'all' });
    expect(screen.queryByText('清空选择')).toBeNull();
  });

  it('enum 声明空字符串选项时不崩溃（空串是 Radix 保留值，由清空项覆盖）', async () => {
    const onSave = vi.fn();
    render(
      <SchemaConfigPanel
        schema={{
          type: 'object',
          properties: {
            mode: { type: 'string', enum: ['', 'a'], title: 'Mode' },
          },
          required: [],
        }}
        data={{ type: 'db:query', mode: '' }}
        onSave={onSave}
      />
    );

    fireEvent.click(screen.getByLabelText(/Mode/));
    await screen.findByText('清空选择');
    expect(screen.getByText('a')).toBeDefined();
  });
});
