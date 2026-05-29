/**
 * End 节点配置面板 — 纯展示，无需配置
 */
import { getLL } from '@/i18n/i18n-lazy';

export function EndConfigPanel() {
  const ll = getLL();
  return (
    <div className="p-4 text-sm text-muted-foreground">
      {ll.nodes.end_config_hint()}
    </div>
  );
}
