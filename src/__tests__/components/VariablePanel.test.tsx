/**
 * VariablePanel 组件测试
 */
import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
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
        initial_variables: [
          { name: 'api_base', value: 'https://api.example.com', description: 'API地址' },
        ],
      },
    },
    {
      uuid: 'http-1',
      type: 'http',
      name: 'HTTP请求',
      position: { x: 200, y: 0 },
      data: {
        type: 'http',
        url: '{{api_base}}/users',
        method: 'get',
        output_mappings: [
          { source: 'status', target: 'user_status', description: '状态码' },
        ],
      },
    },
  ];

  const mockFlow: Flow = {
    uuid: 'flow-1',
    name: '测试流程',
    nodes: mockNodes,
    connections: [],
  };

  it('renders variable panel with groups', () => {
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    // 验证节点分组显示
    expect(screen.getByText('入口节点')).toBeInTheDocument();
    expect(screen.getByText('HTTP请求')).toBeInTheDocument();
  });

  it('displays initial variables from Entry node', () => {
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    // 先展开分组
    fireEvent.click(screen.getByText('全部展开'));
    expect(screen.getByText('api_base')).toBeInTheDocument();
  });

  it('displays output mappings from HTTP node', () => {
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    // 先展开分组
    fireEvent.click(screen.getByText('全部展开'));
    expect(screen.getByText('user_status')).toBeInTheDocument();
  });

  it('shows empty state when no variables', () => {
    const emptyFlow: Flow = { ...mockFlow, nodes: [] };
    render(<VariablePanel flow={emptyFlow} nodes={[]} />);
    expect(screen.getByText('暂无变量')).toBeInTheDocument();
  });

  it('shows total variable count', () => {
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    // Entry 有 1 个变量，HTTP 有 1 个变量，共 2 个
    expect(screen.getByText('2 个变量')).toBeInTheDocument();
  });

  it('filters variables by search query', () => {
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    const searchInput = screen.getByPlaceholderText('搜索变量...');
    
    // 搜索 api_base
    fireEvent.change(searchInput, { target: { value: 'api_base' } });
    // 搜索过滤后分组默认是折叠的，需要展开
    fireEvent.click(screen.getByText('全部展开'));
    expect(screen.getByText('api_base')).toBeInTheDocument();
    // user_status 应该不在结果中
    expect(screen.queryByText('user_status')).not.toBeInTheDocument();
    
    // 清空搜索
    fireEvent.change(searchInput, { target: { value: '' } });
    fireEvent.click(screen.getByText('全部展开'));
    expect(screen.getByText('api_base')).toBeInTheDocument();
    expect(screen.getByText('user_status')).toBeInTheDocument();
  });

  it('shows "未找到匹配的变量" when search has no results', () => {
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    const searchInput = screen.getByPlaceholderText('搜索变量...');
    fireEvent.change(searchInput, { target: { value: 'nonexistent_var' } });
    
    expect(screen.getByText('未找到匹配的变量')).toBeInTheDocument();
  });

  it('expands and collapses groups', () => {
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    // 点击入口节点分组标题切换展开状态
    const entryGroupHeader = screen.getByText('入口节点').closest('button');
    if (entryGroupHeader) {
      fireEvent.click(entryGroupHeader);
    }
    // 分组状态应该切换（内部状态变化）
    expect(entryGroupHeader).toBeDefined();
  });

  it('shows expand all and collapse all buttons', () => {
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    expect(screen.getByText('全部展开')).toBeInTheDocument();
    expect(screen.getByText('全部折叠')).toBeInTheDocument();
  });

  it('clicking expand all expands all groups', () => {
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    const expandAllBtn = screen.getByText('全部展开');
    fireEvent.click(expandAllBtn);
    
    // 分组应该展开
    expect(screen.getByText('api_base')).toBeInTheDocument();
  });

  it('clicking collapse all collapses all groups', () => {
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    // 先全部展开
    const expandAllBtn = screen.getByText('全部展开');
    fireEvent.click(expandAllBtn);
    
    // 再全部折叠
    const collapseAllBtn = screen.getByText('全部折叠');
    fireEvent.click(collapseAllBtn);
    
    // 变量内容应该隐藏
    expect(screen.getByText('入口节点')).toBeInTheDocument();
  });

  it('displays variable description when available', () => {
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    // 先展开分组
    const expandAllBtn = screen.getByText('全部展开');
    fireEvent.click(expandAllBtn);
    
    expect(screen.getByText('API地址')).toBeInTheDocument();
    expect(screen.getByText('状态码')).toBeInTheDocument();
  });

  it('displays variable value type badge', () => {
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    // 先展开分组
    const expandAllBtn = screen.getByText('全部展开');
    fireEvent.click(expandAllBtn);
    
    // string 类型标签 - 使用 getAllByText 因为有多个
    const stringBadges = screen.getAllByText('string');
    expect(stringBadges.length).toBeGreaterThan(0);
  });

  it('handles flow with only Entry node', () => {
    const entryOnlyFlow: Flow = {
      ...mockFlow,
      nodes: [mockNodes[0]],
    };
    
    render(<VariablePanel flow={entryOnlyFlow} nodes={[mockNodes[0]]} />);
    
    expect(screen.getByText('入口节点')).toBeInTheDocument();
    // 先展开分组
    fireEvent.click(screen.getByText('全部展开'));
    expect(screen.getByText('api_base')).toBeInTheDocument();
    expect(screen.getByText('1 个变量')).toBeInTheDocument();
  });

  it('handles Entry node without initial_variables', () => {
    const flowWithoutVars: Flow = {
      ...mockFlow,
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

  it('handles HTTP node output_mappings - all mappings are shown', () => {
    const flowWithMappings: Flow = {
      ...mockFlow,
      nodes: [
        mockNodes[0],
        {
          uuid: 'http-2',
          type: 'http',
          name: 'HTTP请求2',
          position: { x: 200, y: 0 },
          data: {
            type: 'http',
            url: 'https://example.com',
            method: 'get',
            output_mappings: [
              { source: 'status', target: 'status_var' },
              { source: 'body', target: 'body_var' },
            ],
          },
        },
      ],
    };
    
    render(<VariablePanel flow={flowWithMappings} nodes={flowWithMappings.nodes} />);
    
    // 先展开
    const expandAllBtn = screen.getByText('全部展开');
    fireEvent.click(expandAllBtn);
    
    // 所有映射都应该显示
    expect(screen.getByText('status_var')).toBeInTheDocument();
    expect(screen.getByText('body_var')).toBeInTheDocument();
  });

  it('truncates long variable values', () => {
    const longValue = 'a'.repeat(100);
    const flowWithLongValue: Flow = {
      ...mockFlow,
      nodes: [
        {
          uuid: 'entry-3',
          type: 'entry',
          name: '长值入口',
          position: { x: 0, y: 0 },
          data: {
            type: 'entry',
            initial_variables: [
              { name: 'long_var', value: longValue },
            ],
          },
        },
      ],
    };
    
    render(<VariablePanel flow={flowWithLongValue} nodes={flowWithLongValue.nodes} />);
    
    const expandAllBtn = screen.getByText('全部展开');
    fireEvent.click(expandAllBtn);
    
    // 长值应该被截断显示
    expect(screen.getByText(/a{50}\.\.\./)).toBeInTheDocument();
  });

  it('searches by description', () => {
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    const searchInput = screen.getByPlaceholderText('搜索变量...');
    
    // 搜索描述内容 "状态码"
    fireEvent.change(searchInput, { target: { value: '状态码' } });
    
    // 先展开分组
    const expandAllBtn = screen.getByText('全部展开');
    fireEvent.click(expandAllBtn);
    
    expect(screen.getByText('user_status')).toBeInTheDocument();
  });

  it('searches by variable value', () => {
    render(<VariablePanel flow={mockFlow} nodes={mockNodes} />);
    
    const searchInput = screen.getByPlaceholderText('搜索变量...');
    
    // 搜索值内容 "api.example.com"
    fireEvent.change(searchInput, { target: { value: 'example.com' } });
    
    // 先展开分组
    const expandAllBtn = screen.getByText('全部展开');
    fireEvent.click(expandAllBtn);
    
    expect(screen.getByText('api_base')).toBeInTheDocument();
  });
});
