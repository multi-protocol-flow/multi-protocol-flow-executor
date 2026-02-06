import React from 'react';
import { motion } from 'framer-motion';
import { X, Info } from 'lucide-react';
import type { NodeData, TaskNodeType } from '../../types/workflow';

interface NodeInfoModalProps {
  node: NodeData | null;
  isOpen: boolean;
  onClose: () => void;
}

export const NodeInfoModal: React.FC<NodeInfoModalProps> = ({
  node,
  isOpen,
  onClose
}) => {
  if (!isOpen || !node) return null;

  const getTaskTypeName = (type?: TaskNodeType) => {
    const map = {
      http: "HTTP 请求",
      transform: "数据转换",
      condition: "条件判断",
      email: "邮件发送",
      database: "数据库操作",
    };
    return type ? map[type] : "普通任务";
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4">
      <motion.div
        initial={{ opacity: 0, y: 10 }}
        animate={{ opacity: 1, y: 0 }}
        className="bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 rounded-xl shadow-2xl w-full max-w-sm overflow-hidden"
      >
        <div className="px-6 py-4 border-b border-zinc-100 dark:border-zinc-800 flex justify-between items-center">
          <h3 className="font-semibold flex items-center gap-2 text-zinc-900 dark:text-zinc-100">
            <Info size={16} className="text-blue-500" /> 节点信息
          </h3>
          <button onClick={onClose} className="text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-200">
            <X size={18} />
          </button>
        </div>
        <div className="p-6 space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <p className="text-xs text-zinc-500 uppercase">节点 ID</p>
              <p className="text-sm font-mono mt-0.5 truncate text-zinc-900 dark:text-zinc-100">{node.id}</p>
            </div>
            <div>
              <p className="text-xs text-zinc-500 uppercase">节点类型</p>
              <p className="text-sm mt-0.5 text-zinc-900 dark:text-zinc-100">
                {node.type === 'start' ? '开始节点' : getTaskTypeName(node.taskType)}
              </p>
            </div>
            <div>
              <p className="text-xs text-zinc-500 uppercase">创建时间</p>
              <p className="text-sm mt-0.5 text-zinc-900 dark:text-zinc-100">{new Date(node.createdAt).toLocaleString()}</p>
            </div>
            <div>
              <p className="text-xs text-zinc-500 uppercase">坐标位置</p>
              <p className="text-sm mt-0.5 text-zinc-900 dark:text-zinc-100">X: {Math.round(node.position.x)}, Y: {Math.round(node.position.y)}</p>
            </div>
          </div>
        </div>
        <div className="px-6 py-4 bg-zinc-50 dark:bg-zinc-800/50 flex justify-end">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm font-medium bg-zinc-900 dark:bg-zinc-700 text-white rounded-lg"
          >
            关闭
          </button>
        </div>
      </motion.div>
    </div>
  );
};
