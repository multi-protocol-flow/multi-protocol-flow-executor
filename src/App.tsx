import React, { useCallback, useMemo, useState } from 'react';
import type { Node, Edge, Connection, NodeChange, EdgeChange } from '@xyflow/react';
import {
  ReactFlow,
  Background,
  Controls,
  applyNodeChanges,
  applyEdgeChanges,
  BackgroundVariant,
  Handle,
  Position,
  type NodeMouseHandler,
} from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { cn } from './lib/utils';
import { ContextMenu } from './components/ContextMenu';
import ConfigModal from './components/modals/ConfigModal';
import { NodeInfoModal } from './components/modals/NodeInfoModal';
import { TaskTypeModal } from './components/modals/TaskTypeModal';
import type { TaskNodeType } from './types/workflow';
import { Zap, Save, Check, Play, List, Plus, Pin, PinOff, PanelLeftClose, PanelRightClose, CircleAlert, Info, Globe, Code, FileText, Mail, Database } from 'lucide-react';
import { motion, AnimatePresence } from 'motion/react';

/**
 * Workflow 接口
 */
interface Workflow {
  id: string;
  name: string;
  nodes: Node[];
  connections: Edge[];
  createdAt: number;
  updatedAt: number;
}

/**
 * LogEntry 接口
 */
interface LogEntry {
  id: string;
  type: "info" | "success" | "error" | "system";
  message: string;
  timestamp: number;
}

/**
 * 任务类型图标映射
 */
const TASK_TYPE_ICONS: Record<TaskNodeType, React.ReactNode> = {
  http: <Globe size={20} />,
  transform: <Code size={20} />,
  condition: <FileText size={20} />,
  email: <Mail size={20} />,
  database: <Database size={20} />
};

/**
 * 任务类型颜色映射
 */
const TASK_TYPE_COLORS: Record<TaskNodeType, string> = {
  http: "text-blue-500",
  transform: "text-purple-500",
  condition: "text-orange-500",
  email: "text-pink-500",
  database: "text-indigo-500"
};

/**
 * 获取任务类型名称
 */
const getTaskTypeName = (taskType: TaskNodeType): string => {
  const names: Record<TaskNodeType, string> = {
    http: 'HTTP 请求',
    transform: '数据转换',
    condition: '条件判断',
    email: '邮件发送',
    database: '数据库操作',
  };
  return names[taskType];
};

/**
 * 基础节点组件
 */
function BaseNode({ data, selected }: { data: any; selected: boolean }) {
  const isStartNode = data?.nodeType === 'start';
  const isTaskNode = data?.nodeType === 'task';

  let nodeColor: string;
  if (isStartNode) {
    nodeColor = 'bg-emerald-500 border-emerald-600';
  } else if (isTaskNode && data?.taskType) {
    const colorMap: Record<string, string> = {
      http: 'bg-blue-500 border-blue-600',
      transform: 'bg-purple-500 border-purple-600',
      condition: 'bg-orange-500 border-orange-600',
      email: 'bg-pink-500 border-pink-600',
      database: 'bg-indigo-500 border-indigo-600',
    };
    nodeColor = colorMap[data.taskType] || 'bg-slate-800 border-slate-700';
  } else {
    nodeColor = 'bg-slate-800 border-slate-700';
  }

  // Handle颜色映射
  const handleColor: Record<string, string> = {
    http: 'bg-blue-300',
    transform: 'bg-purple-300',
    condition: 'bg-orange-300',
    email: 'bg-pink-300',
    database: 'bg-indigo-300',
    start: 'bg-emerald-300',
  };

  return (
    <div
      data-testid={data?.dataTestId}
      className={cn(
        'px-4 py-3 rounded-full border-2 shadow-lg transition-all duration-200',
        nodeColor,
        selected && 'ring-2 ring-blue-500 ring-offset-2 ring-offset-slate-900'
      )}
    >
      {/* 输入端口 - 仅任务节点 */}
      {isTaskNode && (
        <Handle
          id="in"
          type="target"
          position={Position.Left}
          className={cn(
            'w-3 h-3',
            handleColor[data.taskType] || 'bg-slate-300',
            'border-2',
            `border-${data.taskType === 'http' ? 'blue' : data.taskType === 'transform' ? 'purple' : data.taskType === 'condition' ? 'orange' : data.taskType === 'email' ? 'pink' : data.taskType === 'database' ? 'indigo' : 'slate'}-600`,
            'transition-colors duration-200'
          )}
        />
      )}

      <div className="flex items-center gap-2">
        {data?.icon && (
          <div className={cn('text-white', data.color)}>
            {data.icon}
          </div>
        )}
        <div className="text-white font-medium text-sm" data-testid="node-label">
          {data?.label || '节点'}
        </div>
      </div>
      {data?.description && (
        <div className="text-slate-300 text-xs mt-1">
          {data.description}
        </div>
      )}

      {/* 输出端口 - 任务节点和开始节点 */}
      {(isTaskNode || isStartNode) && (
        <Handle
          id="out"
          type="source"
          position={Position.Right}
          className={cn(
            'w-3 h-3',
            isStartNode ? handleColor.start : (handleColor[data.taskType] || 'bg-slate-300'),
            'border-2',
            isStartNode ? 'border-emerald-600' : `border-${data.taskType === 'http' ? 'blue' : data.taskType === 'transform' ? 'purple' : data.taskType === 'condition' ? 'orange' : data.taskType === 'email' ? 'pink' : data.taskType === 'database' ? 'indigo' : 'slate'}-600`,
            'transition-colors duration-200'
          )}
        />
      )}
    </div>
  );
}

/**
 * App 主组件
 *
 * 功能：
 * - 顶部工具栏：Logo、标题、操作按钮
 * - React Flow 画布
 * - 自动创建开始节点（绿色、圆形、data-testid="start-node"）
 * - 支持节点拖拽
 * - 右键点击开始节点显示菜单（重命名、复制、查看信息）
 * - 可以通过右键菜单打开配置面板
 * - 可以编辑节点名称并保存
 * - 开始节点不能被删除
 */
function App() {
  // 顶部工具栏状态管理
  const [saveStatus, setSaveStatus] = useState<"idle" | "saving" | "saved">("idle");
  const [isExecuting, setIsExecuting] = useState(false);
  const [currentWorkflow, setCurrentWorkflow] = useState({ id: "workflow_1", name: "用户注册流程" });
  const [isTaskTypeOpen, setIsTaskTypeOpen] = useState(false);

  // 左侧面板状态
  const [leftPanelOpen, setLeftPanelOpen] = useState(true);
  const [leftPanelPinned, setLeftPanelPinned] = useState(false);

  // 右侧面板状态
  const [rightPanelOpen, setRightPanelOpen] = useState(true);
  const [rightPanelPinned, setRightPanelPinned] = useState(false);
  const [executionLogs, setExecutionLogs] = useState<LogEntry[]>([]);

  // 工作流状态管理
  const [workflows, setWorkflows] = useState<Workflow[]>([
    {
      id: "workflow_1",
      name: "用户注册流程",
      nodes: [],
      connections: [],
      createdAt: Date.now(),
      updatedAt: Date.now()
    },
    {
      id: "workflow_2",
      name: "订单处理流程",
      nodes: [],
      connections: [],
      createdAt: Date.now(),
      updatedAt: Date.now()
    },
    {
      id: "workflow_3",
      name: "数据同步流程",
      nodes: [],
      connections: [],
      createdAt: Date.now(),
      updatedAt: Date.now()
    }
  ]);
  const [currentWorkflowId, setCurrentWorkflowId] = useState("workflow_1");

  // 状态管理
  const [nodes, setNodes] = React.useState<Node[]>([]);
  const [edges, setEdges] = React.useState<Edge[]>([]);
  const [selectedNode, setSelectedNode] = React.useState<Node | null>(null);
  const [isConfigModalOpen, setIsConfigModalOpen] = React.useState(false);
  const [isNodeInfoModalOpen, setIsNodeInfoModalOpen] = React.useState(false);
  const [contextMenuNode, setContextMenuNode] = React.useState<Node | null>(null);
  const [contextMenuPosition, setContextMenuPosition] = React.useState<{ x: number; y: number } | null>(null);
  const [configNodeName, setConfigNodeName] = React.useState('');

  // 初始化：自动创建开始节点
  React.useEffect(() => {
    const startNodeId = 'start-node-id-' + Date.now();
    const startNode: Node = {
      id: startNodeId,
      type: 'start',
      position: { x: 250, y: 100 },
      data: {
        label: '开始',
        nodeType: 'start',
        description: '工作流的起点',
        dataTestId: 'start-node',
      },
    };
    setNodes([startNode]);
  }, []);

  /**
   * 处理保存工作流
   */
  const handleSaveWorkflow = useCallback(() => {
    setSaveStatus('saving');

    // 模拟保存操作
    setTimeout(() => {
      setSaveStatus('saved');

      // 2秒后恢复为 idle 状态
      setTimeout(() => {
        setSaveStatus('idle');
      }, 2000);
    }, 500);
  }, []);

  /**
   * 添加执行日志
   */
  const handleAddLog = useCallback((type: "info" | "success" | "error" | "system", message: string) => {
    const newLog: LogEntry = {
      id: `log-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      type,
      message,
      timestamp: Date.now(),
    };
    setExecutionLogs((prev) => [...prev, newLog]);
  }, []);

  /**
   * 处理运行流程
   */
  const handleRun = useCallback(() => {
    const startNode = nodes.find(n => n.type === 'start');
    if (!startNode) return;

    setIsExecuting(true);
    handleAddLog('system', '[System] 启动引擎...');

    const hasOutput = edges.some(e => e.source === startNode.id);

    setTimeout(() => {
      if (!hasOutput) {
        handleAddLog('error', '[Error] 错误: 开始节点未连接到任何节点');
        setNodes(prev => prev.map(n =>
          n.type === 'start' ? { ...n, data: { ...n.data, error: '未连接输出端口' } } : n
        ));
        setIsExecuting(false);
        return;
      }

      setNodes(prev => prev.map(n =>
        n.type === 'start' ? { ...n, data: { ...n.data, error: undefined, isExecuting: true } } : n
      ));
      handleAddLog('info', '[Info] 流程开始执行');
      handleAddLog('success', '[Success] 开始节点执行完成 (0ms)');

      const nextConn = edges.find(e => e.source === startNode.id);
      if (nextConn) {
        handleAddLog('info', `[Info] 控制权传递至: ${nextConn.target}`);
      }

      setTimeout(() => {
        setIsExecuting(false);
        setNodes(prev => prev.map(n => ({ ...n, data: { ...n.data, isExecuting: false } })));
        handleAddLog('system', '[System] 流程执行完毕');
      }, 1500);
    }, 800);
  }, [nodes, edges, handleAddLog]);

  /**
   * 切换工作流
   */
  const handleSwitchWorkflow = useCallback((workflowId: string) => {
    // 保存当前工作流
    setWorkflows(prev => prev.map(w =>
      w.id === currentWorkflowId
        ? { ...w, nodes, connections: edges, updatedAt: Date.now() }
        : w
    ));

    // 切换到新工作流
    setCurrentWorkflowId(workflowId);
  }, [currentWorkflowId, nodes, edges]);

  /**
   * 添加新工作流
   */
  const handleAddWorkflow = useCallback(() => {
    const newWorkflow: Workflow = {
      id: `workflow_${Date.now()}`,
      name: `新编排流程 ${workflows.length + 1}`,
      nodes: [],
      connections: [],
      createdAt: Date.now(),
      updatedAt: Date.now(),
    };

    setWorkflows(prev => [...prev, newWorkflow]);
    setCurrentWorkflowId(newWorkflow.id);
  }, [workflows.length]);

  /**
   * 处理节点变化（拖拽等）
   */
  const handleNodesChange = useCallback(
    (changes: NodeChange[]) => {
      const updatedNodes = applyNodeChanges(changes, nodes);
      setNodes(updatedNodes);

      // Track selected node
      const selected = updatedNodes.find((n) => n.selected);
      setSelectedNode(selected || null);
    },
    [nodes]
  );

  /**
   * 处理边变化（连接删除等）
   */
  const handleEdgesChange = useCallback(
    (changes: EdgeChange[]) => {
      const updatedEdges = applyEdgeChanges(changes, edges);
      setEdges(updatedEdges);
    },
    [edges]
  );

  /**
   * 处理新连接创建
   */
  const handleConnect = useCallback(
    (connection: Connection) => {
      const newEdge: Edge = {
        id: `edge-${connection.source}-${connection.target}-${Date.now()}`,
        source: connection.source || '',
        target: connection.target || '',
        sourceHandle: connection.sourceHandle || null,
        targetHandle: connection.targetHandle || null,
        type: 'smoothstep',
        animated: true,
      };
      setEdges((eds) => [...eds, newEdge]);
    },
    []
  );

  /**
   * 验证连接是否有效
   * - 防止自连接（连接到同一节点）
   * - 防止重复连接（相同的源和目标节点）
   */
  const isValidConnection = useCallback(
    (connection: Connection) => {
      // 防止自连接
      if (connection.source === connection.target) {
        return false;
      }

      // 防止重复连接
      const duplicateExists = edges.some(
        (edge) => edge.source === connection.source && edge.target === connection.target
      );

      if (duplicateExists) {
        return false;
      }

      return true;
    },
    [edges]
  );

  /**
   * 处理节点删除（阻止删除开始节点）
   */
  const handleNodesDelete = useCallback(
    (nodesToDelete: Node[]) => {
      const hasStartNode = nodesToDelete.some(
        (node) => node.data?.nodeType === 'start'
      );

      if (hasStartNode) {
        alert('错误：不可删除唯一的开始节点');
        return;
      }

      setNodes((nds) => nds.filter((n) => !nodesToDelete.includes(n)));
    },
    []
  );

  /**
   * 处理节点右键点击
   */
  const handleNodeContextMenu: NodeMouseHandler = useCallback(
    (event, node) => {
      event.preventDefault();
      setContextMenuNode(node);
      setContextMenuPosition({ x: event.clientX, y: event.clientY });
    },
    []
  );

  /**
   * 处理画布点击：关闭右键菜单
   */
  const handlePaneClick = useCallback(() => {
    setContextMenuNode(null);
    setContextMenuPosition(null);
  }, []);

  /**
   * 处理重命名：打开配置模态框
   */
  const handleRename = useCallback(() => {
    if (contextMenuNode) {
      setSelectedNode(contextMenuNode);
      setConfigNodeName((contextMenuNode.data?.label as string) || '');
      setIsConfigModalOpen(true);
      setContextMenuNode(null);
      setContextMenuPosition(null);
    }
  }, [contextMenuNode]);

  /**
   * 处理更改节点类型
   */
  const handleChangeType = useCallback(() => {
    if (!contextMenuNode) return;
    setSelectedNode(contextMenuNode);
    setIsTaskTypeOpen(true);
    setContextMenuNode(null);
    setContextMenuPosition(null);
  }, [contextMenuNode]);

  /**
   * 处理复制节点
   */
  const handleCopy = useCallback(() => {
    if (!contextMenuNode) return;

    // Check if trying to copy start node
    if (contextMenuNode.data?.nodeType === 'start') {
      alert('错误：不能创建第二个开始节点');
      return;
    }

    const newNode: Node = {
      ...contextMenuNode,
      id: 'node-' + Date.now(),
      position: {
        x: contextMenuNode.position.x + 50,
        y: contextMenuNode.position.y + 50,
      },
      // Reset nodeType for copied nodes (except start nodes)
      data: {
        ...contextMenuNode.data,
        nodeType: contextMenuNode.data?.nodeType === 'start' ? undefined : contextMenuNode.data?.nodeType,
      },
    };

    setNodes((nds) => [...nds, newNode]);
    setContextMenuNode(null);
    setContextMenuPosition(null);
  }, [contextMenuNode]);

  /**
   * 处理查看信息
   */
  const handleViewInfo = useCallback(() => {
    if (!contextMenuNode) return;
    setSelectedNode(contextMenuNode);
    setIsNodeInfoModalOpen(true);
    setContextMenuNode(null);
    setContextMenuPosition(null);
  }, [contextMenuNode]);

  /**
   * 处理配置保存：更新节点名称
   */
  const handleConfigSaveName = useCallback((name: string) => {
    if (!selectedNode) return;

    setNodes((nds) =>
      nds.map((node) =>
        node.id === selectedNode.id
          ? {
              ...node,
              data: {
                ...node.data,
                label: name,
                name: name,
              },
            }
          : node
      )
    );
  }, [selectedNode]);

  /**
   * 清除执行日志
   */
  const handleClearLogs = useCallback(() => {
    setExecutionLogs([]);
  }, []);

  /**
   * 处理选择任务类型：创建新节点或更改现有节点类型
   */
  const handleSelectTaskType = useCallback((taskType: TaskNodeType) => {
    // If there's a selectedNode (from "Change Type" menu), update its type
    if (selectedNode && selectedNode.data?.nodeType === 'task') {
      setNodes((nds) =>
        nds.map((node) =>
          node.id === selectedNode.id
            ? {
                ...node,
                type: taskType,
                data: {
                  ...node.data,
                  taskType: taskType,
                  icon: TASK_TYPE_ICONS[taskType],
                  color: TASK_TYPE_COLORS[taskType],
                  label: getTaskTypeName(taskType),
                },
              }
            : node
        )
      );
    } else {
      // Otherwise, create a new node
      const nodeId = `task-node-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
      const newNode: Node = {
        id: nodeId,
        type: taskType,
        position: {
          x: 100 + Math.random() * 200,
          y: 100 + Math.random() * 200,
        },
        data: {
          label: getTaskTypeName(taskType),
          nodeType: 'task',
          taskType: taskType,
          icon: TASK_TYPE_ICONS[taskType],
          color: TASK_TYPE_COLORS[taskType],
        },
      };
      setNodes((nds) => [...nds, newNode]);
    }
    setSelectedNode(null);
    setIsTaskTypeOpen(false);
  }, [selectedNode]);

  /**
   * 节点类型配置
   */
  const nodeTypes = useMemo(
    () => ({
      start: BaseNode,
      http: BaseNode,
      condition: BaseNode,
      transform: BaseNode,
      email: BaseNode,
      database: BaseNode,
    }),
    []
  );

  return (
    <div className="w-screen h-screen bg-slate-950 flex flex-col">
      {/* 顶部工具栏 */}
      <header className="h-16 border-b border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 flex items-center justify-between px-6 z-40 shrink-0">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 bg-emerald-600 rounded-lg flex items-center justify-center text-white">
            <Zap size={18} fill="currentColor" />
          </div>
          <div>
            <h1 className="text-lg font-bold tracking-tight">流程编排引擎 <span className="text-xs font-normal text-zinc-400 ml-2 italic">MVP v1.0</span></h1>
            <p className="text-xs text-zinc-500">当前编排：{currentWorkflow.name}</p>
          </div>
        </div>

        <div className="flex items-center gap-4">
          <div className="flex bg-zinc-100 dark:bg-zinc-800 p-1 rounded-lg gap-1">
            <button
              onClick={() => setIsTaskTypeOpen(true)}
              className="px-3 py-1.5 text-xs font-medium hover:bg-white dark:hover:bg-zinc-700 rounded-md transition-all flex items-center gap-1.5 shadow-sm"
            >
              + 任务节点
            </button>
          </div>

          <button
            onClick={handleSaveWorkflow}
            disabled={saveStatus === "saving"}
            className={`flex items-center gap-2 px-4 py-2 rounded-lg font-medium text-sm transition-all ${
              saveStatus === "saved"
                ? "bg-green-500 text-white"
                : saveStatus === "saving"
                ? "bg-zinc-200 text-zinc-400 cursor-not-allowed"
                : "bg-zinc-900 dark:bg-zinc-700 hover:bg-zinc-800 dark:hover:bg-zinc-600 text-white"
            }`}
          >
            {saveStatus === "saved" ? (
              <>
                <Check size={16} /> 已保存
              </>
            ) : (
              <>
                <Save size={16} /> 保存编排
              </>
            )}
          </button>

          <button
            onClick={handleRun}
            disabled={isExecuting}
            className={`flex items-center gap-2 px-5 py-2 rounded-full font-semibold text-sm transition-all shadow-lg ${
              isExecuting
              ? "bg-zinc-200 text-zinc-400 cursor-not-allowed"
              : "bg-emerald-600 hover:bg-emerald-500 text-white shadow-emerald-500/20 active:scale-95"
            }`}
          >
            {isExecuting ? (
              <div className="animate-spin h-4 w-4 border-2 border-white border-t-transparent rounded-full" />
            ) : (
              <Play size={16} fill="currentColor" />
            )}
            {isExecuting ? "运行中..." : "运行流程"}
          </button>
        </div>
      </header>

      {/* 画布区域 */}
      <div className="flex-1 relative">
        {/* 左侧工作流面板 */}
        <AnimatePresence>
          {(leftPanelOpen || !leftPanelPinned) && (
            <motion.aside
              initial={{ x: -320 }}
              animate={{ x: 0 }}
              exit={{ x: -320 }}
              transition={{ type: "spring", damping: 25, stiffness: 200 }}
              className={`w-80 border-r border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 flex flex-col shadow-lg ${
                leftPanelPinned ? 'relative z-40' : 'absolute left-0 top-0 bottom-0 z-50'
              }`}
              onMouseLeave={() => {
                if (!leftPanelPinned) {
                  setLeftPanelOpen(false);
                }
              }}
            >
              <div className="p-4 border-b border-zinc-100 dark:border-zinc-800 flex items-center justify-between">
                <h2 className="text-sm font-semibold flex items-center gap-2">
                  <List size={14} className="text-emerald-500" /> 编排列表
                </h2>
                <div className="flex items-center gap-1">
                  <button
                    onClick={handleAddWorkflow}
                    className="text-xs text-emerald-600 hover:text-emerald-700 dark:text-emerald-400 dark:hover:text-emerald-300 flex items-center gap-1 p-1.5 hover:bg-emerald-50 dark:hover:bg-emerald-950 rounded"
                  >
                    <Plus size={12} />
                  </button>
                  <button
                    onClick={() => setLeftPanelPinned(!leftPanelPinned)}
                    className="p-1.5 hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300"
                    title={leftPanelPinned ? "取消固定" : "固定面板"}
                  >
                    {leftPanelPinned ? <Pin size={14} /> : <PinOff size={14} />}
                  </button>
                  {leftPanelPinned && (
                    <button
                      onClick={() => setLeftPanelOpen(false)}
                      className="p-1.5 hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300"
                      title="折叠面板"
                    >
                      <PanelLeftClose size={14} />
                    </button>
                  )}
                </div>
              </div>
              <div className="flex-1 overflow-y-auto p-3 space-y-2">
                {workflows.map(workflow => (
                  <button
                    key={workflow.id}
                    onClick={() => handleSwitchWorkflow(workflow.id)}
                    className={`w-full text-left p-3 rounded-lg transition-all ${
                      workflow.id === currentWorkflowId
                        ? "bg-emerald-50 dark:bg-emerald-950 border-2 border-emerald-500"
                        : "bg-zinc-50 dark:bg-zinc-800 hover:bg-zinc-100 dark:hover:bg-zinc-700 border-2 border-transparent"
                    }`}
                  >
                    <div className="flex items-start justify-between mb-1">
                      <h3 className="font-medium text-sm truncate">{workflow.name}</h3>
                      {workflow.id === currentWorkflowId && (
                        <div className="w-2 h-2 bg-emerald-500 rounded-full shrink-0 mt-1" />
                      )}
                    </div>
                    <div className="flex items-center gap-2 text-[10px] text-zinc-500">
                      <span>{workflow.nodes.length} 节点</span>
                      <span>•</span>
                      <span>{workflow.connections.length} 连接</span>
                    </div>
                    <p className="text-[10px] text-zinc-400 mt-1">
                      更新于 {new Date(workflow.updatedAt).toLocaleDateString()}
                    </p>
                  </button>
                ))}
              </div>
            </motion.aside>
          )}
        </AnimatePresence>

        {/* 左侧展开按钮 */}
        {!leftPanelOpen && (
          <button
            onClick={() => setLeftPanelOpen(true)}
            className="absolute left-0 top-1/2 -translate-y-1/2 z-50 bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 rounded-r-lg p-2 shadow-lg hover:bg-zinc-50 dark:hover:bg-zinc-800 transition-colors"
            title="展开编排列表"
          >
            <List size={16} className="text-emerald-500" />
          </button>
        )}

        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={handleNodesChange}
          onEdgesChange={handleEdgesChange}
          onConnect={handleConnect}
          isValidConnection={isValidConnection}
          onNodesDelete={handleNodesDelete}
          onNodeContextMenu={handleNodeContextMenu}
          onPaneClick={handlePaneClick}
          nodeTypes={nodeTypes}
          fitView
          attributionPosition="bottom-left"
          defaultEdgeOptions={{
            type: 'smoothstep',
            animated: true,
            style: { stroke: '#64748b', strokeWidth: 2 },
          }}
        >
          <Background
            variant={BackgroundVariant.Dots}
            gap={16}
            size={1}
            color="#334155"
          />
          <Controls
            className="bg-slate-800 border-slate-700 text-slate-200"
            showZoom
            showFitView
            showInteractive
          />
        </ReactFlow>

        {/* 右键菜单 - 使用自定义定位 */}
        {contextMenuNode && contextMenuPosition && (
          <div
            style={{
              position: 'absolute',
              left: contextMenuPosition.x,
              top: contextMenuPosition.y,
              zIndex: 9999,
            }}
          >
            <ContextMenu
              open={!!contextMenuNode}
              onOpenChange={(isOpen) => {
                if (!isOpen) {
                  setContextMenuNode(null);
                  setContextMenuPosition(null);
                }
              }}
              onRename={handleRename}
              onCopy={handleCopy}
              onViewInfo={handleViewInfo}
              onChangeType={contextMenuNode.data?.nodeType === 'task' ? handleChangeType : undefined}
            >
              <div className="w-0 h-0" />
            </ContextMenu>
          </div>
        )}

        <ConfigModal
          isOpen={isConfigModalOpen}
          onClose={() => setIsConfigModalOpen(false)}
          name={configNodeName}
          setName={handleConfigSaveName}
        />

        <TaskTypeModal
          isOpen={isTaskTypeOpen}
          onClose={() => setIsTaskTypeOpen(false)}
          onSelectType={handleSelectTaskType}
        />

        <NodeInfoModal
          node={selectedNode ? {
            ...selectedNode.data,
            id: selectedNode.id,
            type: selectedNode.type as any,
            name: (selectedNode.data?.label as string) || '',
            createdAt: Date.now(),
            position: selectedNode.position,
          } : null}
          isOpen={isNodeInfoModalOpen}
          onClose={() => setIsNodeInfoModalOpen(false)}
        />

        {/* 右侧执行日志面板 */}
        <AnimatePresence>
          {(rightPanelOpen || !rightPanelPinned) && (
            <motion.aside
              initial={{ x: 320 }}
              animate={{ x: 0 }}
              exit={{ x: 320 }}
              transition={{ type: "spring", damping: 25, stiffness: 200 }}
              className={`w-80 border-l border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-900 flex flex-col shadow-lg ${
                rightPanelPinned ? 'relative z-40' : 'absolute right-0 top-0 bottom-0 z-50'
              }`}
              onMouseLeave={() => {
                if (!rightPanelPinned) {
                  setRightPanelOpen(false);
                }
              }}
            >
              <div className="flex-1 flex flex-col">
                <div className="p-4 border-b border-zinc-100 dark:border-zinc-800 flex items-center justify-between">
                  <h2 className="text-sm font-semibold flex items-center gap-2">
                    <Zap size={14} className="text-emerald-500" /> 执行日志
                  </h2>
                  <div className="flex items-center gap-1">
                    <button
                      onClick={handleClearLogs}
                      className="text-xs text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-200 p-1.5 hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded"
                    >
                      清除
                    </button>
                    <button
                      onClick={() => setRightPanelPinned(!rightPanelPinned)}
                      className="p-1.5 hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300"
                      title={rightPanelPinned ? "取消固定" : "固定面板"}
                    >
                      {rightPanelPinned ? <Pin size={14} /> : <PinOff size={14} />}
                    </button>
                    {rightPanelPinned && (
                      <button
                        onClick={() => setRightPanelOpen(false)}
                        className="p-1.5 hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300"
                        title="折叠面板"
                      >
                        <PanelRightClose size={14} />
                      </button>
                    )}
                  </div>
                </div>
                <div className="flex-1 overflow-y-auto p-4 space-y-2 font-mono text-[11px]">
                  {executionLogs.length === 0 && (
                    <div className="h-full flex flex-col items-center justify-center text-zinc-400 text-center px-8">
                      <CircleAlert size={32} className="mb-2 opacity-20" />
                      <p>准备就绪。点击"运行流程"开始从开始节点执行。</p>
                    </div>
                  )}
                  {executionLogs.map((log) => (
                    <div
                      key={log.id}
                      className={`${
                        log.type === 'info'
                          ? 'text-zinc-600'
                          : log.type === 'success'
                          ? 'text-emerald-600'
                          : log.type === 'error'
                          ? 'text-red-600'
                          : 'text-zinc-500 italic'
                      }`}
                    >
                      {log.message}
                    </div>
                  ))}
                </div>

                <div className="p-4 bg-zinc-50 dark:bg-zinc-800/30 border-t border-zinc-100 dark:border-zinc-800">
                  <h3 className="text-[10px] font-bold text-zinc-400 uppercase tracking-widest mb-3">使用指南</h3>
                  <ul className="text-xs space-y-2 text-zinc-600 dark:text-zinc-400">
                    <li className="flex items-start gap-2">
                      <div className="w-1 h-1 rounded-full bg-emerald-500 mt-1.5 shrink-0" />
                      <span>右键节点进行重命名、更改类型或查看详情。</span>
                    </li>
                    <li className="flex items-start gap-2">
                      <div className="w-1 h-1 rounded-full bg-emerald-500 mt-1.5 shrink-0" />
                      <span>从节点底部端口拖拽连接线。</span>
                    </li>
                    <li className="flex items-start gap-2">
                      <div className="w-1 h-1 rounded-full bg-emerald-500 mt-1.5 shrink-0" />
                      <span>每个流程有且仅有一个绿色开始节点，不可删除。</span>
                    </li>
                  </ul>
                </div>
              </div>
            </motion.aside>
          )}
        </AnimatePresence>

        {/* 右侧展开按钮 */}
        {!rightPanelOpen && (
          <button
            onClick={() => setRightPanelOpen(true)}
            className="absolute right-0 top-1/2 -translate-y-1/2 z-50 bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 rounded-l-lg p-2 shadow-lg hover:bg-zinc-50 dark:hover:bg-zinc-800 transition-colors"
            title="展开执行日志"
          >
            <Zap size={16} className="text-emerald-500" />
          </button>
        )}
      </div>
    </div>
  );
}

export default App;
