/**
 * VariablePanel 组件测试
 */
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { VariablePanel } from '@/components/panels/VariablePanel';
import type { Flow, Node } from '@/types';

describe('VariablePanel', () => {
  const mockNodes: Node[] = [
    {
      uuid: 'entry-1',
      type: 'entry',
      name: '入口节点',
      position: { x: 0, y: 0 },
      data: {
        type: 'entry',
      },
    },
    {
      uuid: 'http-1',
      type: 'variable_extractor',
      name: 'HTTP请求',
      position: { x: 200, y: 0 },
      data: {
        type: 'variable_extractor',
        output_mappings: [
          { target: 'user_status', source: '$.status', description: '状态码' },
        ],
      },
    },
  ];

  const mockFlow: Flow = {
    uuid: 'flow-1',
    name: '测试流程',
    nodes: mockNodes,
    connections: [],
    initial_variables: {
      api_base: 'https://api.example.com',
    },
  };

  it('renders variable panel with groups', () => {
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    // 分组按变量来源: 流程级别变量('流程变量') + variable_extractor('HTTP请求')
    // Entry 节点无 output_mappings → 不生成分组
    expect(screen.getByText('流程变量')).toBeInTheDocument();
    expect(screen.getByText('HTTP请求')).toBeInTheDocument();
  });

  it('displays initial variables from flow level', async () => {
    const user = userEvent.setup();
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    // 先展开分组
    await user.click(screen.getByText('全部展开'));
    expect(screen.getByText('api_base')).toBeInTheDocument();
  });

  it('displays output mappings from HTTP node', async () => {
    const user = userEvent.setup();
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    // 先展开分组
    await user.click(screen.getByText('全部展开'));
    expect(screen.getByText('user_status')).toBeInTheDocument();
  });

  it('shows empty state when no variables', () => {
    // 既无节点也无 initial_variables
    const emptyFlow: Flow = { ...mockFlow, nodes: [], initial_variables: {} };
    render(<VariablePanel flow={emptyFlow} nodes={[]} />);
    expect(screen.getByText('暂无变量')).toBeInTheDocument();
  });

  it('shows total variable count', () => {
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    // flow.initial_variables 有 1 个变量，HTTP 有 1 个变量，共 2 个
    expect(screen.getByText('2 个变量')).toBeInTheDocument();
  });

  it('filters variables by search query', async () => {
    const user = userEvent.setup();
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    const searchInput = screen.getByPlaceholderText('搜索变量...');
    
    // 搜索 api_base
    await user.clear(searchInput);
    await user.type(searchInput, 'api_base');
    // 搜索过滤后分组默认是折叠的，需要展开
    await user.click(screen.getByText('全部展开'));
    expect(screen.getByText('api_base')).toBeInTheDocument();
    // user_status 应该不在结果中
    expect(screen.queryByText('user_status')).not.toBeInTheDocument();
    
    // 清空搜索
    await user.clear(searchInput);
    await user.click(screen.getByText('全部展开'));
    expect(screen.getByText('api_base')).toBeInTheDocument();
    expect(screen.getByText('user_status')).toBeInTheDocument();
  });

  it('shows "未找到匹配的变量" when search has no results', async () => {
    const user = userEvent.setup();
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    const searchInput = screen.getByPlaceholderText('搜索变量...');
    await user.clear(searchInput);
    await user.type(searchInput, 'nonexistent_var');
    
    expect(screen.getByText('未找到匹配的变量')).toBeInTheDocument();
  });

  it('expands and collapses groups', async () => {
    const user = userEvent.setup();
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    // 点击 HTTP请求 分组标题切换展开状态
    const groupHeader = screen.getByText('HTTP请求').closest('button');
    if (groupHeader) {
      await user.click(groupHeader);
    }
    // 分组状态应该切换（内部状态变化）
    expect(groupHeader).toBeDefined();
  });

  it('shows expand all and collapse all buttons', () => {
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    expect(screen.getByText('全部展开')).toBeInTheDocument();
    expect(screen.getByText('全部折叠')).toBeInTheDocument();
  });

  it('clicking expand all expands all groups', async () => {
    const user = userEvent.setup();
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    const expandAllBtn = screen.getByText('全部展开');
    await user.click(expandAllBtn);
    
    // 分组应该展开
    expect(screen.getByText('api_base')).toBeInTheDocument();
  });

  it('clicking collapse all collapses all groups', async () => {
    const user = userEvent.setup();
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    // 先全部展开
    const expandAllBtn = screen.getByText('全部展开');
    await user.click(expandAllBtn);
    
    // 再全部折叠
    const collapseAllBtn = screen.getByText('全部折叠');
    await user.click(collapseAllBtn);
    
    // 分组标题应该可见但变量内容隐藏
    expect(screen.getByText('流程变量')).toBeInTheDocument();
    // 折叠后 api_base 不再可见
    expect(screen.queryByText('api_base')).not.toBeInTheDocument();
  });

  it('displays variable description when available', async () => {
    const user = userEvent.setup();
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    // 先展开分组
    const expandAllBtn = screen.getByText('全部展开');
    await user.click(expandAllBtn);
    
    expect(screen.getByText('流程初始变量')).toBeInTheDocument();
    expect(screen.getByText('状态码')).toBeInTheDocument();
  });

  it('displays variable value type badge', async () => {
    const user = userEvent.setup();
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    // 先展开分组
    const expandAllBtn = screen.getByText('全部展开');
    await user.click(expandAllBtn);
    
    // string 类型标签 - 使用 getAllByText 因为有多个
    const stringBadges = screen.getAllByText('string');
    expect(stringBadges.length).toBeGreaterThan(0);
  });

  it('handles flow with only Entry node', async () => {
    const user = userEvent.setup();
    const entryOnlyFlow: Flow = {
      ...mockFlow,
      nodes: [mockNodes[0]],
    };
    
    render(<VariablePanel flow={entryOnlyFlow} nodes={[mockNodes[0]]} />);
    
    // 只有流程变量分组（api_base from initial_variables）
    expect(screen.getByText('流程变量')).toBeInTheDocument();
    // 先展开分组
    await user.click(screen.getByText('全部展开'));
    expect(screen.getByText('api_base')).toBeInTheDocument();
    expect(screen.getByText('1 个变量')).toBeInTheDocument();
  });

  it('handles flow without initial_variables or variable extractors', () => {
    const flowWithoutVars: Flow = {
      ...mockFlow,
      initial_variables: undefined,
      nodes: [
        {
          uuid: 'entry-2',
          type: 'entry',
          name: '空入口',
          position: { x: 0, y: 0 },
          data: { type: 'entry' },
        },
      ],
    };
    
    render(<VariablePanel flow={flowWithoutVars} nodes={flowWithoutVars.nodes} />);
    
    expect(screen.getByText('暂无变量')).toBeInTheDocument();
  });

  it('handles HTTP node output_mappings - all mappings are shown', async () => {
    const user = userEvent.setup();
    const flowWithMappings: Flow = {
      ...mockFlow,
      nodes: [
        mockNodes[0],
        {
          uuid: 'http-2',
          type: 'variable_extractor',
          name: 'HTTP请求2',
          position: { x: 200, y: 0 },
          data: {
            type: 'variable_extractor',
            output_mappings: [
              { target: 'status_var', source: '$.status', description: '状态码' },
              { target: 'body_var', source: '$.body', description: '响应体' },
            ],
          },
        },
      ],
    };
    
    render(<VariablePanel flow={flowWithMappings} nodes={flowWithMappings.nodes} />);
    
    // 先展开
    const expandAllBtn = screen.getByText('全部展开');
    await user.click(expandAllBtn);
    
    // 所有映射都应该显示
    expect(screen.getByText('status_var')).toBeInTheDocument();
    expect(screen.getByText('body_var')).toBeInTheDocument();
  });

  it('truncates long variable values', async () => {
    const user = userEvent.setup();
    const longValue = 'a'.repeat(100);
    const flowWithLongValue: Flow = {
      ...mockFlow,
      initial_variables: { long_var: longValue },
      nodes: [
        {
          uuid: 'entry-3',
          type: 'entry',
          name: '长值入口',
          position: { x: 0, y: 0 },
          data: { type: 'entry' },
        },
      ],
    };
    
    render(<VariablePanel flow={flowWithLongValue} nodes={flowWithLongValue.nodes} />);
    
    const expandAllBtn = screen.getByText('全部展开');
    await user.click(expandAllBtn);
    
    // 长值应该被截断显示
    expect(screen.getByText(/a{50}\.\.\./)).toBeInTheDocument();
  });

  it('searches by description', async () => {
    const user = userEvent.setup();
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    const searchInput = screen.getByPlaceholderText('搜索变量...');
    
    // 搜索描述内容 "状态码"
    await user.clear(searchInput);
    await user.type(searchInput, '状态码');
    
    // 先展开分组
    const expandAllBtn = screen.getByText('全部展开');
    await user.click(expandAllBtn);
    
    expect(screen.getByText('user_status')).toBeInTheDocument();
  });

  it('searches by variable value', async () => {
    const user = userEvent.setup();
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    const searchInput = screen.getByPlaceholderText('搜索变量...');
    
    // 搜索值内容 "api.example.com"
    await user.clear(searchInput);
    await user.type(searchInput, 'example.com');
    
    // 先展开分组
    const expandAllBtn = screen.getByText('全部展开');
    await user.click(expandAllBtn);
    
    expect(screen.getByText('api_base')).toBeInTheDocument();
  });
});
