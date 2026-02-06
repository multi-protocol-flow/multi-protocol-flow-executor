import React from "react";
import { motion } from "motion/react";
import { X, Globe, Code, FileText, Mail, Database } from "lucide-react";
import type { TaskNodeType } from "../../types/workflow";

interface TaskTypeModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSelectType: (type: TaskNodeType) => void;
}

export const TaskTypeModal: React.FC<TaskTypeModalProps> = ({
  isOpen,
  onClose,
  onSelectType,
}) => {
  if (!isOpen) return null;

  const taskTypes: { type: TaskNodeType; name: string; icon: React.ReactNode; color: string }[] = [
    { type: "http", name: "HTTP 请求", icon: <Globe size={20} />, color: "bg-blue-500" },
    { type: "transform", name: "数据转换", icon: <Code size={20} />, color: "bg-purple-500" },
    { type: "condition", name: "条件判断", icon: <FileText size={20} />, color: "bg-orange-500" },
    { type: "email", name: "邮件发送", icon: <Mail size={20} />, color: "bg-pink-500" },
    { type: "database", name: "数据库操作", icon: <Database size={20} />, color: "bg-indigo-500" },
  ];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4">
      <motion.div
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        className="bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 rounded-xl shadow-2xl w-full max-w-md overflow-hidden"
      >
        <div className="px-6 py-4 border-b border-zinc-100 dark:border-zinc-800 flex justify-between items-center bg-zinc-50 dark:bg-zinc-800/50">
          <h3 className="font-semibold text-zinc-900 dark:text-zinc-100">选择任务类型</h3>
          <button onClick={onClose} className="text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-200">
            <X size={18} />
          </button>
        </div>
        <div className="p-6 grid grid-cols-2 gap-3">
          {taskTypes.map(({ type, name, icon, color }) => (
            <button
              key={type}
              onClick={() => {
                onSelectType(type);
                onClose();
              }}
              className="flex flex-col items-center gap-2 p-4 border-2 border-zinc-200 dark:border-zinc-700 rounded-lg hover:border-emerald-500 dark:hover:border-emerald-500 transition-all group"
            >
              <div className={`${color} text-white p-3 rounded-lg group-hover:scale-110 transition-transform`}>
                {icon}
              </div>
              <span className="text-sm font-medium text-zinc-700 dark:text-zinc-300">{name}</span>
            </button>
          ))}
        </div>
      </motion.div>
    </div>
  );
};
