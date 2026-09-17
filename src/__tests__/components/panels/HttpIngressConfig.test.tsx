/**
 * HttpIngressConfig 单元测试
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { HttpIngressConfig } from '@/components/panels/HttpIngressConfig';
import type { HttpIngressConfig as HttpIngressConfigType } from '@/types';

describe('HttpIngressConfig', () => {
  const initialData: HttpIngressConfigType = {
    type: 'http:ingress',
    port: 9527,
    path_pattern: '/webhook/stripe',
    methods: ['GET', 'POST'],
  };

  it('正确渲染表单字段初始值', () => {
    render(<HttpIngressConfig data={initialData} onSave={vi.fn()} />);

    expect(screen.getByPlaceholderText('9527')).toHaveValue(9527);
    expect(screen.getByPlaceholderText('/* 或 /webhook/stripe')).toHaveValue('/webhook/stripe');
    expect(screen.getByText('GET')).toHaveClass('border-indigo-500');
    expect(screen.getByText('POST')).toHaveClass('border-indigo-500');
    expect(screen.getByText('DELETE')).not.toHaveClass('border-indigo-500');
  });

  it('修改端口和路径触发 onSave', () => {
    const onSave = vi.fn();
    render(<HttpIngressConfig data={initialData} onSave={onSave} />);

    const portInput = screen.getByPlaceholderText('9527');
    fireEvent.change(portInput, { target: { value: '8080' } });

    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        port: 8080,
        path_pattern: '/webhook/stripe',
      })
    );

    const pathInput = screen.getByPlaceholderText('/* 或 /webhook/stripe');
    fireEvent.change(pathInput, { target: { value: '/pay/*' } });

    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        port: 8080,
        path_pattern: '/pay/*',
      })
    );
  });

  it('切换 HTTP 方法调用 onSave', () => {
    const onSave = vi.fn();
    render(<HttpIngressConfig data={initialData} onSave={onSave} />);

    const deleteBtn = screen.getByText('DELETE');
    fireEvent.click(deleteBtn);

    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        methods: ['GET', 'POST', 'DELETE'],
      })
    );
  });
});
