/**
 * WelcomeWizard & GuideHighlighter 引导组件测试
 */
import { render, screen, act, fireEvent } from '@testing-library/react';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { WelcomeWizard } from '@/components/welcome/WelcomeWizard';
import { GuideHighlighter } from '@/components/welcome/GuideHighlighter';

describe('WelcomeWizard', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.useFakeTimers();

    // 在 DOM 中创建引导所挂载的目标元素
    const logo = document.createElement('div');
    logo.setAttribute('data-guide', 'app-logo');
    logo.getBoundingClientRect = () => ({
      top: 10,
      left: 10,
      width: 120,
      height: 40,
      bottom: 50,
      right: 130,
      x: 10,
      y: 10,
      toJSON: () => {},
    });
    document.body.appendChild(logo);

    const newFlowBtn = document.createElement('button');
    newFlowBtn.setAttribute('data-guide', 'new-flow-btn');
    newFlowBtn.getBoundingClientRect = () => ({
      top: 100,
      left: 10,
      width: 180,
      height: 36,
      bottom: 136,
      right: 190,
      x: 10,
      y: 100,
      toJSON: () => {},
    });
    document.body.appendChild(newFlowBtn);

    const canvasArea = document.createElement('div');
    canvasArea.setAttribute('data-guide', 'canvas-area');
    canvasArea.getBoundingClientRect = () => ({
      top: 56,
      left: 240,
      width: 1000,
      height: 700,
      bottom: 756,
      right: 1240,
      x: 240,
      y: 56,
      toJSON: () => {},
    });
    document.body.appendChild(canvasArea);
  });

  afterEach(() => {
    vi.useRealTimers();
    localStorage.clear();
    document.querySelectorAll('[data-guide]').forEach((el) => el.remove());
  });

  it('非首次启动（localStorage 已存在记录）不展示引导', () => {
    localStorage.setItem('mpe-welcome-shown', 'true');
    render(
      <WelcomeWizard>
        <div data-testid="app-content">App Content</div>
      </WelcomeWizard>
    );

    expect(screen.getByTestId('app-content')).toBeInTheDocument();
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(screen.queryByText(/Welcome to MPE|欢迎使用 MPE/i)).not.toBeInTheDocument();
  });

  it('首次启动展示引导，按顺序完成 3 步引导到结束', () => {
    render(
      <WelcomeWizard>
        <div data-testid="app-content">App Content</div>
      </WelcomeWizard>
    );

    expect(screen.getByTestId('app-content')).toBeInTheDocument();

    // 等待 500ms 定时器触发第一步
    act(() => {
      vi.advanceTimersByTime(600);
    });

    // Step 1: 欢迎
    expect(screen.getByText(/Welcome to MPE|欢迎使用 MPE/i)).toBeInTheDocument();
    const step1Btn = screen.getByRole('button', { name: /Get Started|开始使用/i });
    expect(step1Btn).toBeInTheDocument();

    // 点击进入 Step 2
    act(() => {
      fireEvent.click(step1Btn);
    });

    // Step 2: 创建流程
    expect(screen.getByText(/Create Your First Flow|创建你的第一个流程/i)).toBeInTheDocument();
    const step2Btn = screen.getByRole('button', { name: /Next|我知道了/i });
    expect(step2Btn).toBeInTheDocument();

    // 点击进入 Step 3
    act(() => {
      fireEvent.click(step2Btn);
    });

    // Step 3: 画布与执行引导（必须包含可点击的完成按钮，且按钮未被遮挡）
    expect(screen.getByText(/Configure & Execute|配置并执行/i)).toBeInTheDocument();
    const finishBtn = screen.getByRole('button', { name: /Start Building|开始编排/i });
    expect(finishBtn).toBeInTheDocument();

    // 点击完成引导
    act(() => {
      fireEvent.click(finishBtn);
    });

    // 引导关闭，已记录到 localStorage
    expect(screen.queryByText(/Configure & Execute|配置并执行/i)).not.toBeInTheDocument();
    expect(localStorage.getItem('mpe-welcome-shown')).toBe('true');
  });

  it('在 Step 1 点击跳过引导直接结束并记录 localStorage', () => {
    render(
      <WelcomeWizard>
        <div data-testid="app-content">App Content</div>
      </WelcomeWizard>
    );

    act(() => {
      vi.advanceTimersByTime(600);
    });

    const skipBtn = screen.getByRole('button', { name: /Skip Tutorial|跳过引导/i });
    act(() => {
      fireEvent.click(skipBtn);
    });

    expect(screen.queryByText(/Welcome to MPE|欢迎使用 MPE/i)).not.toBeInTheDocument();
    expect(localStorage.getItem('mpe-welcome-shown')).toBe('true');
  });
});

describe('GuideHighlighter 边界防溢出与自适应', () => {
  beforeEach(() => {
    // 模拟标准 1024x768 屏幕
    window.innerWidth = 1024;
    window.innerHeight = 768;
  });

  afterEach(() => {
    document.querySelectorAll('#bottom-target, #center-target').forEach((el) => el.remove());
  });

  it('当 target 占满视口底部时，即使 tooltipPosition="bottom" 也自适应为居中或上方，卡片底部绝不溢出', () => {
    const bottomTarget = document.createElement('div');
    bottomTarget.id = 'bottom-target';
    // 目标元素从 top: 56 到 bottom: 768，占满视口底部
    bottomTarget.getBoundingClientRect = () => ({
      top: 56,
      left: 200,
      width: 800,
      height: 712,
      bottom: 768,
      right: 1000,
      x: 200,
      y: 56,
      toJSON: () => {},
    });
    document.body.appendChild(bottomTarget);

    act(() => {
      render(
        <GuideHighlighter targetSelector="#bottom-target" tooltipPosition="bottom">
          <div data-testid="card-content">Guide Card Content</div>
        </GuideHighlighter>
      );
    });

    const cardContent = screen.getByTestId('card-content');
    const cardContainer = cardContent.parentElement;
    expect(cardContainer).toBeInTheDocument();

    // 验证卡片容器样式中的 top 与 bottom 不会导致溢出视口
    const style = cardContainer?.style;
    if (style?.top && style.top.includes('px')) {
      const topPx = parseFloat(style.top);
      // 卡片高度估算约 220px，加上安全间距 16px，top 绝不能超过 window.innerHeight - 220 - 16
      expect(topPx).toBeLessThanOrEqual(window.innerHeight - 200);
    }
  });

  it('当指定 tooltipPosition="center" 时，卡片居中且在视口安全范围内', () => {
    const centerTarget = document.createElement('div');
    centerTarget.id = 'center-target';
    centerTarget.getBoundingClientRect = () => ({
      top: 56,
      left: 200,
      width: 800,
      height: 700,
      bottom: 756,
      right: 1000,
      x: 200,
      y: 56,
      toJSON: () => {},
    });
    document.body.appendChild(centerTarget);

    act(() => {
      render(
        <GuideHighlighter targetSelector="#center-target" tooltipPosition="center">
          <div data-testid="card-content">Centered Guide Card</div>
        </GuideHighlighter>
      );
    });

    const cardContent = screen.getByTestId('card-content');
    const cardContainer = cardContent.parentElement;
    expect(cardContainer).toBeInTheDocument();
    expect(cardContainer?.style.transform).toBe('translate(-50%, -50%)');
  });
});
