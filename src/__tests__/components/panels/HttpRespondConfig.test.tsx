/**
 * HttpRespondConfig 单元测试
 */
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { HttpRespondConfig } from '@/components/panels/HttpRespondConfig';
import type { HttpRespondConfig as HttpRespondConfigType } from '@/types';

describe('HttpRespondConfig', () => {
  const initialData: HttpRespondConfigType = {
    type: 'http:respond',
    status_code: 200,
    body_type: 'json',
    body: '{"code": 0, "msg": "ok"}',
    headers: {
      'X-Custom': 'header-val',
    },
  };

  it('正确渲染初始数据与预设状态码', () => {
    render(<HttpRespondConfig data={initialData} onSave={vi.fn()} />);

    expect(screen.getByPlaceholderText('200')).toHaveValue(200);
    expect(screen.getByDisplayValue('X-Custom')).toBeInTheDocument();
    expect(screen.getByDisplayValue('header-val')).toBeInTheDocument();
    expect(screen.getByDisplayValue('{"code": 0, "msg": "ok"}')).toBeInTheDocument();
  });

  it('点击预设状态码调用 onSave', () => {
    const onSave = vi.fn();
    render(<HttpRespondConfig data={initialData} onSave={onSave} />);

    const code201Btn = screen.getByRole('button', { name: '201' });
    fireEvent.click(code201Btn);

    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        status_code: 201,
      })
    );
  });

  it('添加与修改响应头调用 onSave', () => {
    const onSave = vi.fn();
    render(<HttpRespondConfig data={initialData} onSave={onSave} />);

    const addBtn = screen.getByRole('button', { name: /添加响应头/i });
    fireEvent.click(addBtn);

    const inputs = screen.getAllByPlaceholderText('Header 名称');
    expect(inputs.length).toBe(2);

    fireEvent.change(inputs[1], { target: { value: 'Authorization' } });

    expect(onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        headers: expect.objectContaining({
          'X-Custom': 'header-val',
          'Authorization': '',
        }),
      })
    );
  });
});
