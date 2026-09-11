import { describe, it, expect } from 'vitest';
import { act, render, screen } from '@testing-library/react';
import { SubFlowNode } from '@/components/canvas/nodes/SubFlowNode';
import { useExecutionStore } from '@/stores';
import type { SubFlowConfig } from '@/types';
import { ReactFlowProvider } from '@xyflow/react';

describe('SubFlowNode', () => {
  it('正确渲染子流程节点与名称', () => {
    const config: SubFlowConfig = {
      type: 'sub_flow',
      target_flow_id: 'flow-123',
      inherit_all_variables: true,
      export_all_variables: true,
    };

    render(
      <ReactFlowProvider>
        <SubFlowNode
          id="subflow-node-1"
          data={{ label: '用户登录流程', config }}
          selected={false}
          type="sub_flow"
          zIndex={1}
          isConnectable={true}
          positionAbsoluteX={0}
          positionAbsoluteY={0}
          dragging={false}
          draggable={false}
          selectable={true}
          deletable={true}
        />
      </ReactFlowProvider>
    );

    expect(screen.getByText('用户登录流程')).toBeInTheDocument();
  });

  it('执行中在节点上展示嵌套进度与当前子节点', () => {
    const config: SubFlowConfig = {
      type: 'sub_flow',
      target_flow_id: 'flow-123',
      inherit_all_variables: true,
      export_all_variables: true,
    };
    // 顶层流程把子流程当作一个节点，其内部进度通过 parentNodeUuid 归属到本节点
    act(() => {
      const store = useExecutionStore.getState();
      store.reset();
      store.startExecution();
      store.handleExecutionEvent({
        flowStarted: { flowUuid: 'parent-flow', flowName: 'Parent', totalNodes: 2 },
      });
      store.handleExecutionEvent({
        nodeStarted: { nodeUuid: 'subflow-node-1', nodeName: '用户登录流程', nodeType: 'sub_flow' },
      });
      store.handleExecutionEvent({
        flowStarted: {
          flowUuid: 'child-flow',
          flowName: 'Auth',
          totalNodes: 3,
          parentNodeUuid: 'subflow-node-1',
        },
      });
      store.handleExecutionEvent({
        nodeStarted: {
          nodeUuid: 'child-http',
          nodeName: '登录请求',
          nodeType: 'http',
          parentNodeUuid: 'subflow-node-1',
        },
      });
      store.handleExecutionEvent({
        nodeCompleted: {
          report: {
            node_uuid: 'child-entry',
            node_type: 'entry',
            node_name: 'Child Entry',
            start_time: 0,
            end_time: 1,
            duration_ms: 1,
            status: 'success',
          },
          parentNodeUuid: 'subflow-node-1',
        },
      });
    });

    render(
      <ReactFlowProvider>
        <SubFlowNode
          id="subflow-node-1"
          data={{ label: '用户登录流程', config }}
          selected={false}
          type="sub_flow"
          zIndex={1}
          isConnectable={true}
          positionAbsoluteX={0}
          positionAbsoluteY={0}
          dragging={false}
          draggable={false}
          selectable={true}
          deletable={true}
        />
      </ReactFlowProvider>
    );

    expect(screen.getByText('子流程执行中 1/3')).toBeInTheDocument();
    expect(screen.getByText('当前节点：登录请求')).toBeInTheDocument();

    act(() => {
      useExecutionStore.getState().reset();
    });
  });
});
