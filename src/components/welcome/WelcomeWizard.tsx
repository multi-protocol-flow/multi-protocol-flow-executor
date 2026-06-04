/**
 * WelcomeWizard - 新用户欢迎引导向导
 *
 * 3 步引导流程：欢迎介绍 → 创建流程 → 配置执行。
 * 使用 GuideHighlighter 在目标元素上显示引导卡片。
 * 首次启动自动触发，完成后记录 localStorage 不再显示。
 */
import { useState, useEffect, useCallback } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardFooter, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { GuideHighlighter } from './GuideHighlighter';
import { getLL } from '@/i18n/i18n-lazy';

const WELCOME_SHOWN_KEY = 'mpe-welcome-shown';

type WelcomeStep = 0 | 1 | 2 | 3; // 0 = 未启动, 1-3 = 各步骤

interface WelcomeWizardProps {
  children: React.ReactNode;
}

export function WelcomeWizard({ children }: WelcomeWizardProps) {
  const [step, setStep] = useState<WelcomeStep>(0);
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
  }, []);

  const finishWizard = useCallback(() => {
    localStorage.setItem(WELCOME_SHOWN_KEY, 'true');
    setStep(0);
  }, []);

  if (step === 0) return <>{children}</>;

  const stepContent = () => {
    switch (step) {
      case 1:
        return (
          <GuideHighlighter targetSelector="[data-guide='app-logo']" tooltipPosition="bottom">
            <Card className="w-80 shadow-xl border-primary/20">
              <CardHeader>
                <CardTitle>{welcome.welcome_step1_title?.() || '欢迎使用 MPE'}</CardTitle>
                <CardDescription>
                  {welcome.welcome_step1_desc?.() || '多协议流程编排与执行引擎'}
                </CardDescription>
              </CardHeader>
              <CardFooter className="flex justify-between">
                <Button variant="ghost" size="sm" onClick={finishWizard}>
                  {welcome.welcome_skip?.() || '跳过引导'}
                </Button>
                <Button size="sm" onClick={() => setStep(2)}>
                  {welcome.welcome_step1_action?.() || '开始使用'}
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
                <CardTitle>{welcome.welcome_step2_title?.() || '创建你的第一个流程'}</CardTitle>
                <CardDescription>
                  {welcome.welcome_step2_desc?.() || '点击左侧「新建流程」按钮开始编排'}
                </CardDescription>
              </CardHeader>
              <CardFooter className="flex justify-between">
                <Button variant="ghost" size="sm" onClick={finishWizard}>
                  {welcome.welcome_skip?.() || '跳过引导'}
                </Button>
                <Button size="sm" onClick={finishWizard}>
                  {welcome.welcome_step2_action?.() || '我知道了'}
                </Button>
              </CardFooter>
            </Card>
          </GuideHighlighter>
        );
      case 3:
        return (
          <GuideHighlighter targetSelector="[data-guide='canvas-area']" tooltipPosition="bottom">
            <Card className="w-80 shadow-xl border-primary/20">
              <CardHeader>
                <CardTitle>{welcome.welcome_step3_title?.() || '配置并执行'}</CardTitle>
                <CardDescription>
                  {welcome.welcome_step3_desc?.() || '选中节点配置参数并执行流程'}
                </CardDescription>
              </CardHeader>
              <CardFooter className="flex justify-end">
                <Button size="sm" onClick={finishWizard}>
                  {welcome.welcome_step3_action?.() || '开始编排'}
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
