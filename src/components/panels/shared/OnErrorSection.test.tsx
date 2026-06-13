/** @vitest-environment jsdom */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { OnErrorSection } from './OnErrorSection';

describe('OnErrorSection', () => {
  const mockOnChange = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('rendering', () => {
    it('renders the Select component with three options', () => {
      render(
        <OnErrorSection
          value="route_to_false"
          onChange={mockOnChange}
        />,
      );

      // The trigger should display the current selected label
      expect(screen.getByText('路由到 false 分支')).toBeInTheDocument();

      // Open the dropdown
      fireEvent.click(screen.getByRole('combobox'));

      // Check all three options are in the document
      expect(screen.getByText('终止流程')).toBeInTheDocument();
      expect(screen.getByText('忽略错误')).toBeInTheDocument();
    });

    it('renders the section title', () => {
      render(
        <OnErrorSection
          value="route_to_false"
          onChange={mockOnChange}
        />,
      );

      expect(screen.getByText('执行错误处理')).toBeInTheDocument();
    });

    it('shows the correct description based on the selected option', () => {
      const { rerender } = render(
        <OnErrorSection
          value="route_to_false"
          onChange={mockOnChange}
        />,
      );

      expect(
        screen.getByText('节点执行失败时路由到 false 分支，流程继续执行'),
      ).toBeInTheDocument();

      rerender(
        <OnErrorSection
          value="abort_flow"
          onChange={mockOnChange}
        />,
      );

      expect(
        screen.getByText('节点执行失败时停止流程并报错'),
      ).toBeInTheDocument();

      rerender(
        <OnErrorSection
          value="ignore"
          onChange={mockOnChange}
        />,
      );

      expect(
        screen.getByText('忽略错误，继续正常执行'),
      ).toBeInTheDocument();
    });

    it('defaults to route_to_false when value is undefined', () => {
      render(
        <OnErrorSection
          value={undefined}
          onChange={mockOnChange}
        />,
      );

      expect(
        screen.getByText('节点执行失败时路由到 false 分支，流程继续执行'),
      ).toBeInTheDocument();
    });
  });

  describe('interaction', () => {
    it('calls onChange with "route_to_false" when RouteToFalse option is selected', () => {
      render(
        <OnErrorSection
          value="abort_flow"
          onChange={mockOnChange}
        />,
      );

      // Open dropdown and click the route_to_false option
      fireEvent.click(screen.getByRole('combobox'));
      fireEvent.click(screen.getByText('路由到 false 分支'));
      expect(mockOnChange).toHaveBeenCalledWith('route_to_false');
    });

    it('calls onChange with "abort_flow" when AbortFlow option is selected', () => {
      render(
        <OnErrorSection
          value="route_to_false"
          onChange={mockOnChange}
        />,
      );

      fireEvent.click(screen.getByRole('combobox'));
      fireEvent.click(screen.getByText('终止流程'));
      expect(mockOnChange).toHaveBeenCalledWith('abort_flow');
    });

    it('calls onChange with "ignore" when Ignore option is selected', () => {
      render(
        <OnErrorSection
          value="route_to_false"
          onChange={mockOnChange}
        />,
      );

      fireEvent.click(screen.getByRole('combobox'));
      fireEvent.click(screen.getByText('忽略错误'));
      expect(mockOnChange).toHaveBeenCalledWith('ignore');
    });
  });

  describe('should not render hasFalsePort prop (removed)', () => {
    it('always renders regardless of any port state (hasFalsePort removed)', () => {
      const { container } = render(
        <OnErrorSection
          value="route_to_false"
          onChange={mockOnChange}
        />,
      );

      // The component should always render content
      expect(container.firstChild).not.toBeNull();
      expect(screen.getByText('执行错误处理')).toBeInTheDocument();
    });
  });
});
