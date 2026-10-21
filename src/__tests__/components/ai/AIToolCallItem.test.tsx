import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { AIToolCallItem } from '@/components/ai/AIToolCallItem';
import type { AIToolCall } from '@/types/ai';
import { getLL } from '@/i18n/i18n-lazy';

describe('AIToolCallItem', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  const sampleToolCall: AIToolCall = {
    id: 'call_123',
    type: 'function',
    function: {
      name: 'batch_create_pipeline_long_function_name_for_testing_overflow',
      arguments: JSON.stringify({
        pipeline_name: 'Auth Workflow',
        steps_count: 5,
      }),
    },
  };

  it('1. 渲染长函数名时具有截断与不溢出属性 (truncate, min-w-0, shrink-0)', () => {
    const { container } = render(
      <AIToolCallItem toolCall={sampleToolCall} isExecuted={true} />,
    );

    // 外层容器应具有 overflow-hidden 和 max-w-full
    const outerContainer = container.firstElementChild;
    expect(outerContainer).toHaveClass('overflow-hidden');
    expect(outerContainer).toHaveClass('max-w-full');

    // 函数名应具有 truncate, flex-1, min-w-0 并在 title 中展示全名
    const nameEl = screen.getByText(sampleToolCall.function.name);
    expect(nameEl).toHaveClass('truncate');
    expect(nameEl).toHaveClass('min-w-0');
    expect(nameEl).toHaveClass('flex-1');
    expect(nameEl).toHaveAttribute('title', sampleToolCall.function.name);

    // 状态徽章应有 shrink-0 和 whitespace-nowrap 避免被挤压溢出
    const ll = getLL();
    const doneEl = screen.getByText(ll.ai.tool_call.done());
    const badgeEl = doneEl.parentElement;
    expect(badgeEl).toHaveClass('shrink-0');
    expect(badgeEl).toHaveClass('whitespace-nowrap');
  });

  it('2. isExecuted 为 false 时渲染运行中状态', () => {
    render(<AIToolCallItem toolCall={sampleToolCall} isExecuted={false} />);

    const ll = getLL();
    const runningEl = screen.getByText(ll.ai.tool_call.running());
    expect(runningEl).toBeInTheDocument();
    expect(runningEl).toHaveClass('shrink-0');
    expect(runningEl).toHaveClass('whitespace-nowrap');
  });
  it('2.1 status 为 pending 时渲染等待执行状态', () => {
    render(<AIToolCallItem toolCall={sampleToolCall} status="pending" />);

    const ll = getLL();
    const pendingEl = screen.getByText(ll.ai.tool_call.pending());
    expect(pendingEl).toBeInTheDocument();
    expect(pendingEl).toHaveClass('shrink-0');
    expect(pendingEl).toHaveClass('whitespace-nowrap');
  });


  it('3. 点击展开参数并支持一键复制参数', async () => {
    // Mock navigator.clipboard
    const writeTextMock = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, {
      clipboard: {
        writeText: writeTextMock,
      },
    });

    render(<AIToolCallItem toolCall={sampleToolCall} isExecuted={true} />);

    // 初始状态下参数区域不显示
    expect(screen.queryByText('"Auth Workflow"')).not.toBeInTheDocument();

    // 点击按钮展开
    const toggleButton = screen.getByRole('button');
    fireEvent.click(toggleButton);

    // 展开后应显示格式化的参数内容
    await waitFor(() => {
      expect(screen.getByText(/"Auth Workflow"/)).toBeInTheDocument();
    });

    // 点击复制按钮
    const ll = getLL();
    const copyBtn = screen.getByTitle(ll.ai.tool_call.copy_arguments());
    expect(copyBtn).toBeInTheDocument();
    fireEvent.click(copyBtn);

    expect(writeTextMock).toHaveBeenCalledTimes(1);
    expect(writeTextMock).toHaveBeenCalledWith(
      JSON.stringify(
        {
          pipeline_name: 'Auth Workflow',
          steps_count: 5,
        },
        null,
        2,
      ),
    );

    // 复制后应显示复制成功文案
    await waitFor(() => {
      expect(screen.getByText(ll.ai.tool_call.copied())).toBeInTheDocument();
    });
  });
  it('4. 传入 result 时展开默认展示执行结果与摘要', async () => {
    const sampleResult = JSON.stringify({
      success: true,
      node_id: 'node_456',
      message: 'Node created successfully with ID node_456',
    });

    render(
      <AIToolCallItem
        toolCall={sampleToolCall}
        result={sampleResult}
        isExecuted={true}
      />,
    );

    // 点击展开
    const toggleButton = screen.getByRole('button', { name: new RegExp(sampleToolCall.function.name) });
    fireEvent.click(toggleButton);

    // 展开后应显示结果内容和摘要信息
    await waitFor(() => {
      expect(screen.getByText('Node created successfully with ID node_456')).toBeInTheDocument();
      expect(screen.getByText(/"node_456"/)).toBeInTheDocument();
    });
  });

  it('5. 支持在入参与执行结果 Tab 之间无缝切换', async () => {
    const sampleResult = JSON.stringify({
      success: true,
      created_count: 3,
    });

    render(
      <AIToolCallItem
        toolCall={sampleToolCall}
        result={sampleResult}
        isExecuted={true}
      />,
    );

    const toggleButton = screen.getByRole('button', { name: new RegExp(sampleToolCall.function.name) });
    fireEvent.click(toggleButton);

    const ll = getLL();
    // 切换到参数 Tab
    const argsTabBtn = screen.getByRole('button', { name: ll.ai.tool_call.call_args() });
    fireEvent.click(argsTabBtn);

    await waitFor(() => {
      expect(screen.getByText(/"Auth Workflow"/)).toBeInTheDocument();
    });

    // 切回结果 Tab
    const resultTabBtn = screen.getByRole('button', { name: new RegExp(ll.ai.tool_call.execution_result()) });
    fireEvent.click(resultTabBtn);

    await waitFor(() => {
      expect(screen.getByText(/"created_count": 3/)).toBeInTheDocument();
    });
  });

  it('6. 执行失败时展示失败徽章与错误信息横幅', async () => {
    const failedResult = JSON.stringify({
      success: false,
      error: 'Target node not found',
    });

    render(
      <AIToolCallItem
        toolCall={sampleToolCall}
        result={failedResult}
        isExecuted={true}
      />,
    );

    const ll = getLL();
    expect(screen.getByText(ll.ai.tool_call.failed())).toBeInTheDocument();

    const toggleButton = screen.getByRole('button', { name: new RegExp(sampleToolCall.function.name) });
    fireEvent.click(toggleButton);

    await waitFor(() => {
      expect(screen.getByText('Target node not found')).toBeInTheDocument();
    });
  });
  it('7. 渲染语义化操作与关键节点目标信息 (如修改节点)', () => {
    const updateToolCall: AIToolCall = {
      id: 'call_update_1',
      type: 'function',
      function: {
        name: 'update_node_config',
        arguments: JSON.stringify({ node_name: '用户认证服务' }),
      },
    };

    render(<AIToolCallItem toolCall={updateToolCall} isExecuted={true} />);

    // 包含动作与具体节点名
    expect(screen.getByText('修改节点')).toBeInTheDocument();
    expect(screen.getByText('"用户认证服务"')).toBeInTheDocument();
  });

  it('8. 渲染数据源查询关键信息 (如 apifox / orders)', () => {
    const datasourceToolCall: AIToolCall = {
      id: 'call_ds_1',
      type: 'function',
      function: {
        name: 'browse_datasource',
        arguments: JSON.stringify({ datasource_id: 'apifox', resource_id: 'order_center' }),
      },
    };

    render(<AIToolCallItem toolCall={datasourceToolCall} isExecuted={true} />);
    expect(screen.getByText('查询数据源')).toBeInTheDocument();
    expect(screen.getByText('apifox · order_center')).toBeInTheDocument();
  });
});
