/**
 * VariableValidationWarning 组件测试
 */
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { 
  VariableValidationWarning, 
  detectVariableRefs, 
  findInvalidRefs 
} from '@/components/ui/VariableValidationWarning';

describe('VariableValidationWarning', () => {
  describe('辅助函数', () => {
    it('detectVariableRefs 应该正确提取变量引用', () => {
      expect(detectVariableRefs('{{status}}')).toEqual(['status']);
      expect(detectVariableRefs('{{body.data}}')).toEqual(['body.data']);
      expect(detectVariableRefs('prefix{{status}}suffix')).toEqual(['status']);
      expect(detectVariableRefs('{{a}}{{b}}')).toEqual(['a', 'b']);
      expect(detectVariableRefs('no variables')).toEqual([]);
      expect(detectVariableRefs('')).toEqual([]);
    });

    it('findInvalidRefs 应该正确识别失效变量', () => {
      const available = ['status', 'body', 'headers'];
      expect(findInvalidRefs(['status'], available)).toEqual([]);
      expect(findInvalidRefs(['unknown'], available)).toEqual(['unknown']);
      expect(findInvalidRefs(['status', 'unknown'], available)).toEqual(['unknown']);
    });
  });

  describe('组件渲染', () => {
    it('没有失效变量时不应该渲染', () => {
      const { container } = render(
        <VariableValidationWarning
          value="{{status}}"
          availableVariables={['status', 'body']}
        />
      );
      expect(container.firstChild).toBeNull();
    });

    it('有失效变量时应该渲染警告', () => {
      render(
        <VariableValidationWarning
          value="{{unknown_var}}"
          availableVariables={['status', 'body']}
        />
      );
      expect(screen.getByText(/变量引用失效:/)).toBeDefined();
      expect(screen.getByText('{{unknown_var}}')).toBeDefined();
    });

    it('icon 和文字应该在同一行（使用 items-center）', () => {
      const { container } = render(
        <VariableValidationWarning
          value="{{unknown_var}}"
          availableVariables={['status']}
        />
      );

      // 获取外层容器
      const outerDiv = container.firstChild as HTMLElement;
      expect(outerDiv).toBeDefined();
      
      // 验证使用 items-center 而不是 items-start
      expect(outerDiv.className).toContain('items-center');
      expect(outerDiv.className).not.toContain('items-start');
    });

    it('icon 和 "变量引用失效:" 文字应该在同一行显示', () => {
      const { container } = render(
        <VariableValidationWarning
          value="{{unknown_var}}"
          availableVariables={['status']}
        />
      );

      // 获取外层容器
      const outerDiv = container.firstChild as HTMLElement;
      
      // 验证 icon 存在
      const icon = outerDiv.querySelector('.lucide-alert-circle');
      expect(icon).toBeDefined();
      
      // 验证文字存在
      const textSpan = screen.getByText(/变量引用失效:/);
      expect(textSpan).toBeDefined();

      // 验证外层容器使用 items-center（确保 icon 和文字垂直居中对齐，在同一行）
      expect(outerDiv.className).toContain('items-center');
    });

    it('支持 compact 模式', () => {
      const { container } = render(
        <VariableValidationWarning
          value="{{unknown_var}}"
          availableVariables={['status']}
          compact
        />
      );

      const outerDiv = container.firstChild as HTMLElement;
      expect(outerDiv.className).toContain('px-2 py-1');
    });

    it('支持隐藏图标', () => {
      const { container } = render(
        <VariableValidationWarning
          value="{{unknown_var}}"
          availableVariables={['status']}
          showIcon={false}
        />
      );

      const icon = container.querySelector('.lucide-alert-circle');
      expect(icon).toBeNull();
      
      // 即使没有图标，文字仍然应该显示
      expect(screen.getByText(/变量引用失效:/)).toBeDefined();
    });
  });
});
