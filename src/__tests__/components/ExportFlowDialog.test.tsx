/**
 * ExportFlowDialog 单元测试
 * 验证：
 * 1. 弹窗正常展示目标流程信息与固化环境参数选项
 * 2. 勾选 Bake 时，敏感凭据（如 secret/key/token）被扫描并支持清空
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ExportFlowDialog, buildExportedFlow } from '@/components/dialogs/ExportFlowDialog';
import { useUIStore } from '@/stores/ui';
import { useEnvironmentStore } from '@/stores/environment';
import { switchLocale } from '@/i18n';
import type { Flow } from '@/types/flow';
import type { EnvironmentSet } from '@/types/environment';

describe('ExportFlowDialog', () => {
  const mockFlow: Flow = {
    uuid: 'flow-export-test',
    name: '待导出流程',
    initial_variables: {
      API_HOST: 'https://fallback.com',
      API_SECRET: 'super-secret-key',
    },
    nodes: [],
    connections: [],
  };

  beforeEach(() => {
    switchLocale('zh-CN');
    useEnvironmentStore.getState().reset();
    useUIStore.setState({
      showExportFlowDialog: true,
      exportFlowTarget: mockFlow,
    });
  });

  it('展示流程导出标题与目标流程名称', async () => {
    render(<ExportFlowDialog />);

    expect(await screen.findByText('导出流程')).toBeInTheDocument();
    expect(await screen.findByText(/待导出流程/)).toBeInTheDocument();
    expect(await screen.findByText(/固化当前环境参数到流程中/)).toBeInTheDocument();
  });
  it('检测敏感凭据变量并展示安全扫描警告与复选框', async () => {
    await useEnvironmentStore.getState().createEnvironment('dev', {
      API_HOST: 'https://dev.api',
      ACCESS_TOKEN: 'token-abc-123',
    });
    await useEnvironmentStore.getState().setActiveEnvironment('dev');

    render(<ExportFlowDialog />);

    // 应该扫描出 API_SECRET 与 ACCESS_TOKEN
    expect(await screen.findByText('凭据安全扫描 (Credential Sanitization)')).toBeInTheDocument();
    expect(await screen.findByText('API_SECRET')).toBeInTheDocument();
    expect(await screen.findByText('ACCESS_TOKEN')).toBeInTheDocument();
  });
});

describe('buildExportedFlow', () => {
  const flowWithSecret: Flow = {
    uuid: 'flow-export-pure',
    name: '待导出流程',
    initial_variables: {
      API_HOST: 'https://fallback.api',
      API_SECRET: 'flow-embedded-secret',
    },
    nodes: [],
    connections: [],
  };

  const devEnv: EnvironmentSet = {
    name: 'dev',
    variables: {
      API_HOST: 'https://dev.api',
      ACCESS_TOKEN: 'env-token',
    },
  };

  it('未激活环境时也必须清空勾选的敏感变量（防止凭据泄漏）', () => {
    const exported = buildExportedFlow({
      targetFlow: flowWithSecret,
      bakeEnvironment: true,
      activeEnvironmentData: null,
      sanitizeKeys: new Set(['API_SECRET']),
    });

    expect(exported.initial_variables?.API_SECRET).toBe('');
    expect(exported.initial_variables?.API_HOST).toBe('https://fallback.api');
  });

  it('激活环境时按「流程兜底 → 环境」合并并清空环境侧敏感变量', () => {
    const exported = buildExportedFlow({
      targetFlow: flowWithSecret,
      bakeEnvironment: true,
      activeEnvironmentData: devEnv,
      sanitizeKeys: new Set(['API_SECRET', 'ACCESS_TOKEN']),
    });

    expect(exported.initial_variables?.API_HOST).toBe('https://dev.api');
    expect(exported.initial_variables?.API_SECRET).toBe('');
    expect(exported.initial_variables?.ACCESS_TOKEN).toBe('');
  });

  it('未勾选 Bake 时原样导出，且不污染原流程对象', () => {
    const exported = buildExportedFlow({
      targetFlow: flowWithSecret,
      bakeEnvironment: false,
      activeEnvironmentData: devEnv,
      sanitizeKeys: new Set(['API_SECRET']),
    });

    expect(exported.initial_variables).toEqual(flowWithSecret.initial_variables);
    expect(exported.uuid).toBe(flowWithSecret.uuid);
    expect(flowWithSecret.initial_variables?.API_SECRET).toBe('flow-embedded-secret');
  });
});
