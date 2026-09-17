/**
 * FlowConfigPanel 组件测试
 */
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { FlowConfigPanel } from '@/components/panels/FlowConfigPanel';
import { useFlowStore, useUIStore } from '@/stores';
import { switchLocale } from '@/i18n';
import type { Flow } from '@/types';

const mockFlow: Flow = {
  uuid: 'flow-test-1234',
  name: '测试主流程',
  description: '# 流程说明\n这是一个用于自动化测试的流程',
  initial_variables: {
    base_url: 'https://api.example.com',
    timeout: 5000,
  },
  nodes: [
    {
      uuid: 'node-entry',
      name: '入口节点',
      type: 'entry',
      position: { x: 0, y: 0 },
      data: { type: 'entry' },
    },
    {
      uuid: 'node-http',
      name: 'HTTP 请求',
      type: 'http:request',
      position: { x: 100, y: 100 },
      data: { type: 'http:request', url: 'https://api.example.com/test', method: 'get' },
    },
  ],
  connections: [
    {
      id: 'conn-1',
      source_node_uuid: 'node-entry',
      source_port_id: 'output',
      target_node_uuid: 'node-http',
      target_port_id: 'input',
    },
  ],
};

describe('FlowConfigPanel', () => {
  const mockUpdateFlowName = vi.fn();
  const mockUpdateFlowDescription = vi.fn();
  const mockUpdateInitialVariables = vi.fn();
  const mockOpenEnvDialog = vi.fn();
  beforeEach(() => {
    vi.clearAllMocks();
    switchLocale('zh-CN');
    useFlowStore.setState({
      flow: mockFlow,
      updateFlowName: mockUpdateFlowName,
      updateFlowDescription: mockUpdateFlowDescription,
      updateInitialVariables: mockUpdateInitialVariables,
    });
    useUIStore.setState({
      openEnvironmentManagerDialog: mockOpenEnvDialog,
    });
  });

  it('未打开流程时显示空状态', () => {
    useFlowStore.setState({ flow: null });
    render(<FlowConfigPanel />);
    expect(screen.getByText('暂无流程')).toBeInTheDocument();
  });

  it('渲染流程属性标题、名称与节点数量徽章', () => {
    render(<FlowConfigPanel />);
    expect(screen.getByText('流程属性')).toBeInTheDocument();
    expect(screen.getByDisplayValue('测试主流程')).toBeInTheDocument();
    expect(screen.getByText('2 节点数')).toBeInTheDocument();
  });

  it('编辑流程名称并在失焦时保存', () => {
    render(<FlowConfigPanel />);
    const nameInput = screen.getByDisplayValue('测试主流程');
    fireEvent.change(nameInput, { target: { value: '重命名流程' } });
    fireEvent.blur(nameInput);
    expect(mockUpdateFlowName).toHaveBeenCalledWith('重命名流程');
  });
  it('支持切换流程描述的编辑与预览模式', () => {
    render(<FlowConfigPanel />);
    // 默认是编辑模式，显示包含流程说明的 textarea
    const textareas = screen.getAllByRole('textbox');
    const descTextarea = textareas.find((t) => (t as HTMLTextAreaElement).value.includes('流程说明'));
    expect(descTextarea).toBeDefined();

    // 点击预览按钮
    const previewBtn = screen.getByText('预览');
    fireEvent.click(previewBtn);

    // 应该显示预览并出现编辑按钮
    expect(screen.getByText('编辑')).toBeInTheDocument();

    // 再次点击切回编辑模式
    fireEvent.click(screen.getByText('编辑'));
    expect(screen.getByText('预览')).toBeInTheDocument();
  });

  it('渲染初始变量列表并展示键与值', () => {
    render(<FlowConfigPanel />);
    expect(screen.getByText('base_url:')).toBeInTheDocument();
    expect(screen.getByText('"https://api.example.com"')).toBeInTheDocument();
    expect(screen.getByText('timeout:')).toBeInTheDocument();
    expect(screen.getByText('5000')).toBeInTheDocument();
  });
  it('可以添加新的流程初始变量', () => {
    render(<FlowConfigPanel />);
    const openFormBtn = screen.getByTitle('添加变量');
    fireEvent.click(openFormBtn);

    // 出现新增输入框
    const keyInput = screen.getByPlaceholderText('变量名');
    const valueInput = screen.getByPlaceholderText('值 (支持 JSON 或字符串)');

    fireEvent.change(keyInput, { target: { value: 'retry_count' } });
    fireEvent.change(valueInput, { target: { value: '3' } });

    // 提交添加按钮是内联表单里的确定按钮
    const submitBtns = screen.getAllByRole('button', { name: '添加变量' });
    const formSubmitBtn = submitBtns[submitBtns.length - 1];
    fireEvent.click(formSubmitBtn);

    expect(mockUpdateInitialVariables).toHaveBeenCalledWith({
      base_url: 'https://api.example.com',
      timeout: 5000,
      retry_count: 3,
    });
  });

  it('可以删除已有的初始变量', () => {
    render(<FlowConfigPanel />);
    const deleteButtons = screen.getAllByTitle('删除');
    expect(deleteButtons.length).toBeGreaterThan(0);
    fireEvent.click(deleteButtons[0]);

    expect(mockUpdateInitialVariables).toHaveBeenCalled();
  });

  it('展示流程概况并支持点击复制流程 ID', async () => {
    const writeTextMock = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, {
      clipboard: {
        writeText: writeTextMock,
      },
    });

    render(<FlowConfigPanel />);
    expect(screen.getByText('flow-test-1234')).toBeInTheDocument();

    const copyBtn = screen.getByTitle('复制详情');
    fireEvent.click(copyBtn);
    expect(writeTextMock).toHaveBeenCalledWith('flow-test-1234');
  });

  it('readOnly 模式下禁用编辑并隐藏添加/删除按钮', () => {
    render(<FlowConfigPanel readOnly={true} />);
    // 流程名称为文本而非输入框
    expect(screen.queryByRole('textbox', { name: '流程名称' })).not.toBeInTheDocument();
    expect(screen.getByText('测试主流程')).toBeInTheDocument();

    // 隐藏添加变量按钮和删除按钮
    expect(screen.queryByTitle('添加变量')).not.toBeInTheDocument();
    expect(screen.queryByTitle('删除')).not.toBeInTheDocument();
  });
});
