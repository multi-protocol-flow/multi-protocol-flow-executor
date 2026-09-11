/**
 * WelcomeWizard - 新用户欢迎引导向导
 *
 * 3 步引导流程：欢迎介绍 → 创建流程 → 配置执行。
 * 使用 GuideHighlighter 在目标元素上显示引导卡片。
 * 首次启动自动触发，完成后记录 localStorage 不再显示。
 */
import { useEffect, useCallback } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardFooter, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { GuideHighlighter } from './GuideHighlighter';
import { getLL } from '@/i18n/i18n-lazy';
import { useUIStore } from '@/stores/ui';

const WELCOME_SHOWN_KEY = 'mpe-welcome-shown';


interface WelcomeWizardProps {
  children: React.ReactNode;
}

export function WelcomeWizard({ children }: WelcomeWizardProps) {
  const step = useUIStore((s) => s.welcomeStep);
  const setStep = useUIStore((s) => s.setWelcomeStep);
  const ll = getLL();
  const welcome = ll.welcome || ll.common;

  // 检查是否已显示过欢迎引导
  useEffect(() => {
    const shown = localStorage.getItem(WELCOME_SHOWN_KEY);
    if (!shown) {
      // 等待 UI 完全渲染后再显示
      const timer = setTimeout(() => setStep(1), 500);
      return () => clearTimeout(timer);
    }
  }, [setStep]);

  const finishWizard = useCallback(() => {
    localStorage.setItem(WELCOME_SHOWN_KEY, 'true');
    setStep(0);
  }, [setStep]);

  if (step === 0) return <>{children}</>;

  const stepContent = () => {
    switch (step) {
      case 1:
        return (
          <GuideHighlighter targetSelector="[data-guide='app-logo']" tooltipPosition="bottom">
            <Card className="w-80 shadow-xl border-primary/20">
              <CardHeader>
                <CardTitle>                  {welcome.welcome_step1_title?.() || 'Welcome to MPE'}</CardTitle>
                <CardDescription>
                  {welcome.welcome_step1_desc?.() || 'Multi-Protocol Flow Orchestration & Execution Engine'}
                </CardDescription>
              </CardHeader>
              <CardFooter className="flex justify-between">
                <Button variant="ghost" size="sm" onClick={finishWizard}>
                  {welcome.welcome_skip?.() || 'Skip Tutorial'}
                </Button>
                <Button size="sm" onClick={() => setStep(2)}>
                  {welcome.welcome_step1_action?.() || 'Get Started'}
                </Button>
              </CardFooter>
            </Card>
          </GuideHighlighter>
        );
      case 2:
        return (
          <GuideHighlighter targetSelector="[data-guide='new-flow-btn']" tooltipPosition="right">
            <Card className="w-80 shadow-xl border-primary/20">
              <CardHeader>
                <CardTitle>{welcome.welcome_step2_title?.() || 'Create Your First Flow'}</CardTitle>
                <CardDescription>
                  {welcome.welcome_step2_desc?.() || 'Click "New Flow" on the left to start building'}
                </CardDescription>
              </CardHeader>
              <CardFooter className="flex justify-between">
                <Button variant="ghost" size="sm" onClick={finishWizard}>
                  {welcome.welcome_skip?.() || 'Skip Tutorial'}
                </Button>
                <Button size="sm" onClick={() => setStep(3)}>
                  {welcome.welcome_step2_action?.() || 'Next'}
                </Button>
              </CardFooter>
            </Card>
          </GuideHighlighter>
        );
      case 3:
        return (
          <GuideHighlighter targetSelector="[data-guide='canvas-area']" tooltipPosition="center">
            <Card className="w-80 shadow-xl border-primary/20">
              <CardHeader>
                <CardTitle>{welcome.welcome_step3_title?.() || 'Configure & Execute'}</CardTitle>
                <CardDescription>
                  {welcome.welcome_step3_desc?.() || 'Select a node, configure parameters, and execute'}
                </CardDescription>
              </CardHeader>
              <CardFooter className="flex justify-end">
                <Button size="sm" onClick={finishWizard}>
                  {welcome.welcome_step3_action?.() || 'Start Building'}
                </Button>
              </CardFooter>
            </Card>
          </GuideHighlighter>
        );
    }
  };

  return (
    <>
      {children}
      {stepContent()}
    </>
  );
}
