/**
 * getViewerComponent / viewerRegistry 测试
 *
 * 零回归契约：内置 70+ 节点类型的解析路径必须与 PR-4 之前完全一致。
 * 插件分支仅对「未注册类型 + report.plugin_data 存在」生效。
 */
import { describe, expect, it } from 'vitest';
import { getViewerComponent, viewerRegistry } from '@/lib/viewerRegistry';
import { PluginIframeViewer } from '@/components/execution/viewers/PluginIframeViewer';
import { GenericViewer } from '@/components/execution/viewers/GenericViewer';
import { HttpNodeViewer } from '@/components/execution/viewers/HttpNodeViewer';
import type { NodeExecutionReport } from '@/types';

/** 构造最小合法报告（默认 node_type 为未注册的插件类型） */
function makeReport(overrides: Partial<NodeExecutionReport> = {}): NodeExecutionReport {
  return {
    node_uuid: 'n1',
    node_type: 'vendor:widget',
    node_name: 'Widget',
    start_time: 0,
    end_time: 1,
    duration_ms: 1,
    status: 'success',
    ...overrides,
  };
}

describe('getViewerComponent', () => {
  it('getViewerComponent_known_builtin_resolves_exact_viewer', () => {
    expect(getViewerComponent('http:request')).toBe(HttpNodeViewer);
  });

  it('getViewerComponent_known_builtin_with_plugin_data_not_hijacked', () => {
    // 即使报告携带 plugin_data，已注册的内置类型仍解析到精确 viewer
    const report = makeReport({ node_type: 'http:request', plugin_data: { anything: true } });
    expect(getViewerComponent('http:request', report)).toBe(HttpNodeViewer);
  });

  it('getViewerComponent_unknown_type_without_report_falls_back_to_generic', () => {
    // 无 report 参数（旧调用方）→ 行为与 PR-4 之前完全一致
    expect(getViewerComponent('vendor:widget')).toBe(GenericViewer);
  });

  it('getViewerComponent_unknown_type_no_plugin_data_falls_back_to_generic', () => {
    expect(getViewerComponent('vendor:widget', makeReport())).toBe(GenericViewer);
  });

  it('getViewerComponent_unknown_type_with_plugin_data_returns_plugin_viewer', () => {
    const report = makeReport({ plugin_data: { output: 'ok' } });
    expect(getViewerComponent('vendor:widget', report)).toBe(PluginIframeViewer);
  });

  it('getViewerComponent_all_registered_types_preserve_exact_mapping', () => {
    // 零回归扫描：注册表内每一个类型都必须解析回它注册的 viewer
    for (const [type, viewer] of Object.entries(viewerRegistry)) {
      expect(getViewerComponent(type), `type "${type}" must resolve to its registered viewer`).toBe(viewer);
    }
  });
});
