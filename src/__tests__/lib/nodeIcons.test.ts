import { describe, it, expect } from 'vitest';
import {
  BACKEND_ICON_MAP,
  resolveBackendIcon,
} from '@/lib/nodeIcons';
import { Terminal, Database, Plug, Wrench } from 'lucide-react';

// ============================================================================
// 后端 icon 字符串 → lucide 组件解析
// ============================================================================

describe('resolveBackendIcon', () => {
  it('redis 插件声明的三个图标都能解析', () => {
    // plugins/redis describe 的 icon 字段：connect=database / command=terminal / close=plug
    expect(resolveBackendIcon('database')).toBe(Database);
    expect(resolveBackendIcon('terminal')).toBe(Terminal);
    expect(resolveBackendIcon('plug')).toBe(Plug);
  });

  it('未收录的 icon 名返回 undefined（由调用方回退到占位图标）', () => {
    expect(resolveBackendIcon('no-such-icon')).toBeUndefined();
    expect(resolveBackendIcon('')).toBeUndefined();
  });

  it('BACKEND_ICON_MAP 是完整 Record，所有值都是 Lucide 组件', () => {
    const entries = Object.entries(BACKEND_ICON_MAP);
    expect(entries.length).toBeGreaterThan(20);
    for (const [key, icon] of entries) {
      expect(key.length, `key ${key} 不能为空`).toBeGreaterThan(0);
      // lucide-react 图标是 memo/forwardRef 组件（typeof 可能为 object）
      expect(icon, `key ${key} 必须是组件`).toBeTruthy();
    }
  });

  it('未收录名回退链与面板一致（NodePalette 用 Wrench 兜底）', () => {
    // 面板解析：nodeMeta 无 redis:* 静态映射 → 后端 icon → Wrench
    const paletteIcon =
      resolveBackendIcon('no-such-icon') ?? Wrench;
    expect(paletteIcon).toBe(Wrench);
  });
});
