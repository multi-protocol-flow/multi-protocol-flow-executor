import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { X } from 'lucide-react';

export interface ConfigModalProps {
  isOpen: boolean;
  onClose: () => void;
  name: string;
  setName: (name: string) => void;
}

const ConfigModal = ({
  isOpen,
  onClose,
  name,
  setName
}: ConfigModalProps) => {
  const [localName, setLocalName] = useState(name);

  useEffect(() => {
    setLocalName(name);
  }, [name, isOpen]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4">
      <motion.div
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        className="bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 rounded-xl shadow-2xl w-full max-w-sm overflow-hidden"
      >
        <div className="px-6 py-4 border-b border-zinc-100 dark:border-zinc-800 flex justify-between items-center bg-zinc-50 dark:bg-zinc-800/50">
          <h3 className="font-semibold text-zinc-900 dark:text-zinc-100">节点配置</h3>
          <button onClick={onClose} className="text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-200">
            <X size={18} />
          </button>
        </div>
        <div className="p-6 space-y-4">
          <div>
            <label className="block text-xs font-medium text-zinc-500 uppercase tracking-wider mb-1.5">名称</label>
            <input
              type="text"
              maxLength={50}
              value={localName}
              onChange={(e) => setLocalName(e.target.value)}
              className="w-full px-3 py-2 bg-zinc-50 dark:bg-zinc-950 border border-zinc-200 dark:border-zinc-800 rounded-lg focus:ring-2 focus:ring-emerald-500 focus:border-emerald-500 outline-none transition-all text-zinc-900 dark:text-zinc-100"
              placeholder="节点名称"
            />
            <p className="mt-1.5 text-[10px] text-zinc-400 text-right">{localName.length}/50</p>
          </div>
        </div>
        <div className="px-6 py-4 bg-zinc-50 dark:bg-zinc-800/50 flex justify-end gap-3 border-t border-zinc-100 dark:border-zinc-800">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm font-medium text-zinc-600 dark:text-zinc-400 hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded-lg transition-colors"
          >
            取消
          </button>
          <button
            onClick={() => { setName(localName); onClose(); }}
            className="px-4 py-2 text-sm font-medium bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg transition-colors"
          >
            确定
          </button>
        </div>
      </motion.div>
    </div>
  );
};

export default ConfigModal;
