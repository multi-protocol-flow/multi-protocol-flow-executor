/**
 * EnvironmentManagerDialog 单元测试
 * 覆盖：
 * 1. sidebar 模式：标题为全局环境管理，绝对不展示流程内置变量
 * 2. canvas 模式：标题为环境与变量管理 (当前流程)，展示流程内置变量与级联对比矩阵
 * 3. 弹窗内可直接切换激活环境
 * 4. 删除环境需二次确认，并支持「仅删除本地私有覆盖」
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, act, fireEvent, waitFor } from '@testing-library/react';
import { EnvironmentManagerDialog } from '@/components/panels/EnvironmentManager';
import { useFlowStore } from '@/stores/flow';
import { useUIStore } from '@/stores/ui';
import { useEnvironmentStore } from '@/stores/environment';
import { environmentStorageMock } from '@/services/storage/environmentStorage.mock';
import { switchLocale } from '@/i18n';
import type { Flow } from '@/types/flow';

describe('EnvironmentManagerDialog', () => {
  const mockFlow: Flow = {
    uuid: 'flow-dialog-test',
    name: '测试流程',
    initial_variables: {
      API_HOST: 'https://default.api',
      TIMEOUT: 3000,
    },
    nodes: [],
    connections: [],
  };

  beforeEach(() => {
    switchLocale('zh-CN');
    environmentStorageMock.reset();
    useEnvironmentStore.getState().reset();
    useFlowStore.setState({ flow: mockFlow });
    useUIStore.setState({
      showEnvironmentManagerDialog: true,
      environmentManagerPendingDelete: null,
    });
  });

  it('sidebar 模式：标题为全局环境管理，绝对不展示流程内置变量', async () => {
    await useEnvironmentStore.getState().createEnvironment('dev', { API_HOST: 'https://dev.api' });
    useUIStore.setState({ environmentManagerContext: 'sidebar' });

    await act(async () => {
      render(<EnvironmentManagerDialog />);
    });

    expect(screen.getByText('全局环境管理')).toBeInTheDocument();
    expect(screen.queryByText('流程内置变量 (Fallback)')).not.toBeInTheDocument();
  });

  it('canvas 模式：标题为环境与变量管理 (当前流程)，展示流程内置变量与级联矩阵', async () => {
    await useEnvironmentStore.getState().createEnvironment('dev', {
      API_HOST: 'https://dev.api',
      DEBUG: true,
    });
    await useEnvironmentStore.getState().setActiveEnvironment('dev');

    useUIStore.setState({ environmentManagerContext: 'canvas' });

    await act(async () => {
      render(<EnvironmentManagerDialog />);
    });

    expect(screen.getByText('环境与变量管理 (当前流程)')).toBeInTheDocument();
    expect(screen.getByText('流程内置变量 (Fallback)')).toBeInTheDocument();

    // 级联对比矩阵 Tab 与列头（列头带固定宽度，不再使用「状态 / 来源」合并表头）
    expect(screen.getByText('级联对比矩阵')).toBeInTheDocument();
    expect(screen.getByText('最终生效值')).toBeInTheDocument();
    expect(screen.getByText('变量来源')).toBeInTheDocument();
    expect(screen.getByText('操作')).toBeInTheDocument();

    // 环境覆盖流程兜底值时应标记为「已覆盖」
    expect(screen.getByText('已覆盖')).toBeInTheDocument();
  });

  it('侧边栏环境列表可直接切换激活环境', async () => {
    await useEnvironmentStore.getState().createEnvironment('dev', { A: 1 });
    useUIStore.setState({ environmentManagerContext: 'sidebar' });

    await act(async () => {
      render(<EnvironmentManagerDialog />);
    });

    expect(useEnvironmentStore.getState().activeEnvironmentName).toBeNull();

    await act(async () => {
      fireEvent.click(screen.getByTitle('点击激活'));
    });

    await waitFor(() => {
      expect(useEnvironmentStore.getState().activeEnvironmentName).toBe('dev');
    });
    expect(useEnvironmentStore.getState().activeEnvironmentData?.variables.A).toBe(1);
  });

  it('删除环境需二次确认，且可只删除本地私有覆盖', async () => {
    await useEnvironmentStore.getState().createEnvironment('dev', { A: 1 });
    await useEnvironmentStore.getState().saveEnvironment({ name: 'dev', variables: { A: 2 } }, true);

    useUIStore.setState({ environmentManagerContext: 'sidebar' });

    await act(async () => {
      render(<EnvironmentManagerDialog />);
    });

    // 进入删除确认视图
    await act(async () => {
      fireEvent.click(screen.getAllByTitle('删除环境')[0]);
    });

    expect(screen.getByText('删除环境')).toBeInTheDocument();

    // 选择「仅删除本地私有覆盖」→ 主环境文件保留
    await act(async () => {
      fireEvent.click(screen.getByText('仅删除本地私有覆盖'));
      fireEvent.click(screen.getByText('确认删除'));
    });

    await waitFor(() => {
      const meta = useEnvironmentStore.getState().environments;
      expect(meta).toHaveLength(1);
      expect(meta[0].hasLocalOverride).toBe(false);
    });
  });

  /**
   * 列表尚未返回时不能决定默认选中项：此时「列表为空」并不代表
   * 「激活环境不存在」，否则 canvas 模式会被永久锁死在流程内置变量视图。
   */
  it('canvas 模式：列表加载完成后再选定激活环境，不会锁死在兜底视图', async () => {
    await environmentStorageMock.saveEnvironment({
      name: 'dev',
      variables: { A: 1, API_HOST: 'https://dev.api' },
    });
    // 直接注入激活名，且不预加载列表（列表在弹窗挂载时才由弹窗拉取）
    useEnvironmentStore.setState({ activeEnvironmentName: 'dev' });
    useUIStore.setState({ environmentManagerContext: 'canvas' });

    await act(async () => {
      render(<EnvironmentManagerDialog />);
    });

    await waitFor(() => {
      expect(screen.getByText('级联对比矩阵')).toBeInTheDocument();
    });
    // 矩阵以 dev 的真实数据计算：API_HOST 覆盖了流程兜底值
    expect(screen.getByText('已覆盖')).toBeInTheDocument();
  });

  /** 删除最后一个环境后，canvas 模式应回到始终存在的「流程内置变量」视图 */
  it('canvas 模式：删除最后一个环境后回到流程内置变量视图', async () => {
    await useEnvironmentStore.getState().createEnvironment('dev', { A: 1 });
    await useEnvironmentStore.getState().setActiveEnvironment('dev');
    useUIStore.setState({ environmentManagerContext: 'canvas' });

    await act(async () => {
      render(<EnvironmentManagerDialog />);
    });

    await waitFor(() => {
      expect(screen.getByText('级联对比矩阵')).toBeInTheDocument();
    });

    await act(async () => {
      fireEvent.click(screen.getAllByTitle('删除环境')[0]);
    });
    await act(async () => {
      fireEvent.click(screen.getByText('确认删除'));
    });

    await waitFor(() => {
      expect(useEnvironmentStore.getState().environments).toHaveLength(0);
    });
    // 回到兜底视图（h3 标题只出现在右侧内容区）
    expect(screen.getByRole('heading', { name: /流程内置变量/ })).toBeInTheDocument();
    expect(screen.queryByText('级联对比矩阵')).not.toBeInTheDocument();
  });

  /** 变量编辑必须失焦提交：逐键写盘会造成「每敲一个字符一次 IPC + 一次 toast」 */
  it('变量编辑在失焦时才写盘，输入过程不产生存储调用', async () => {
    await useEnvironmentStore.getState().createEnvironment('dev', { A: 'x' });
    useUIStore.setState({ environmentManagerContext: 'sidebar' });

    const saveSpy = vi.spyOn(environmentStorageMock, 'saveEnvironment');

    await act(async () => {
      render(<EnvironmentManagerDialog />);
    });

    const input = await screen.findByDisplayValue('x');
    fireEvent.change(input, { target: { value: 'y' } });

    expect(saveSpy).not.toHaveBeenCalled();
    expect((input as HTMLInputElement).value).toBe('y');

    fireEvent.blur(input);

    await waitFor(() => {
      expect(saveSpy).toHaveBeenCalledTimes(1);
    });
    expect(saveSpy.mock.calls[0][0]).toMatchObject({ name: 'dev', variables: { A: 'y' } });

    // 值未变化时再次失焦不应重复写盘
    fireEvent.blur(input);
    expect(saveSpy).toHaveBeenCalledTimes(1);

    saveSpy.mockRestore();
  });

  it('没有本地私有覆盖时，删除范围中的该选项不可用', async () => {
    await useEnvironmentStore.getState().createEnvironment('dev', { A: 1 });
    useUIStore.setState({ environmentManagerContext: 'sidebar' });

    await act(async () => {
      render(<EnvironmentManagerDialog />);
    });

    await act(async () => {
      fireEvent.click(screen.getAllByTitle('删除环境')[0]);
    });

    const radios = screen.getAllByRole('radio') as HTMLInputElement[];
    // 左栏激活开关 + 删除范围的两个 radio
    const scopeRadios = radios.filter((r) => r.name === 'env-delete-scope');
    expect(scopeRadios).toHaveLength(2);
    expect(scopeRadios[1].disabled).toBe(true);
  });
});
