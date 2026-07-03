/**
 * MessageSchemaView 组件测试
 * - Recursive read-only protobuf schema tree viewer
 * - 空消息 / WKT / 标量字段 / 枚举 / Map / Oneof / 嵌套展开 / 深度限制 / 循环引用 / 保留字段
 */
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MessageSchemaView } from '@/components/panels/grpc/MessageSchemaView';
import type { GrpcMessageInfo } from '@/types';

// ─── Test Data ────────────────────────────────────────────────────────────────

const emptyMessage: GrpcMessageInfo = {
  full_name: 'test.Empty',
  fields: [],
  oneof_groups: [],
  is_wkt: false,
};

const wktMessage: GrpcMessageInfo = {
  full_name: 'google.protobuf.Timestamp',
  fields: [],
  oneof_groups: [],
  is_wkt: true,
};

const simpleMessage: GrpcMessageInfo = {
  full_name: 'test.Simple',
  fields: [
    { name: 'name', number: 1, type_kind: 'string', type_display: 'string', label: 'singular', is_map: false, enum_values: [] },
    { name: 'age', number: 2, type_kind: 'int32', type_display: 'int32', label: 'singular', is_map: false, enum_values: [] },
    { name: 'active', number: 3, type_kind: 'bool', type_display: 'bool', label: 'singular', is_map: false, enum_values: [] },
  ],
  oneof_groups: [],
  is_wkt: false,
};

const enumMessage: GrpcMessageInfo = {
  full_name: 'test.WithEnum',
  fields: [
    {
      name: 'color', number: 1, type_kind: 'enum', type_display: 'Color',
      label: 'singular', is_map: false,
      enum_values: [
        { name: 'RED', number: 0 },
        { name: 'GREEN', number: 1 },
        { name: 'BLUE', number: 2 },
      ],
    },
  ],
  oneof_groups: [],
  is_wkt: false,
};

const mapMessage: GrpcMessageInfo = {
  full_name: 'test.WithMap',
  fields: [
    {
      name: 'labels', number: 1, type_kind: 'message', type_display: 'map<string, int32>',
      label: 'singular', is_map: true, map_key_type: 'string', map_value_type: 'int32', enum_values: [],
    },
  ],
  oneof_groups: [],
  is_wkt: false,
};

const oneofMessage: GrpcMessageInfo = {
  full_name: 'test.WithOneof',
  fields: [
    { name: 'name', number: 1, type_kind: 'string', type_display: 'string', label: 'singular', is_map: false, enum_values: [] },
    { name: 'alt_name', number: 2, type_kind: 'string', type_display: 'string', label: 'singular', is_map: false, enum_values: [] },
    { name: 'value', number: 3, type_kind: 'int32', type_display: 'int32', label: 'singular', is_map: false, enum_values: [] },
  ],
  oneof_groups: [
    { name: 'test_oneof', field_numbers: [1, 2] },
  ],
  is_wkt: false,
};

const nestedInner: GrpcMessageInfo = {
  full_name: 'test.Inner',
  fields: [
    { name: 'nested_name', number: 1, type_kind: 'string', type_display: 'string', label: 'singular', is_map: false, enum_values: [] },
  ],
  oneof_groups: [],
  is_wkt: false,
};

const nestedMessage: GrpcMessageInfo = {
  full_name: 'test.WithNested',
  fields: [
    { name: 'name', number: 1, type_kind: 'string', type_display: 'string', label: 'singular', is_map: false, enum_values: [] },
    {
      name: 'inner', number: 2, type_kind: 'message', type_display: 'Inner',
      type_full_name: 'test.Inner', label: 'singular', is_map: false, enum_values: [],
      nested_message: nestedInner,
    },
  ],
  oneof_groups: [],
  is_wkt: false,
};

const circularMessage: GrpcMessageInfo = {
  full_name: 'test.WithCircular',
  fields: [
    {
      name: 'self_ref', number: 1, type_kind: 'circular_ref', type_display: 'SelfRef',
      label: 'singular', is_map: false, enum_values: [],
    },
  ],
  oneof_groups: [],
  is_wkt: false,
};

const reservedMessage: GrpcMessageInfo = {
  full_name: 'test.WithReserved',
  fields: [
    { name: 'name', number: 5, type_kind: 'string', type_display: 'string', label: 'singular', is_map: false, enum_values: [] },
  ],
  oneof_groups: [],
  is_wkt: false,
  reserved_ranges: [[1, 3], [10, 10]],
};

// Shared empty definitions map
const emptyDefinitions: Record<string, GrpcMessageInfo> = {};

// ─── Tests ────────────────────────────────────────────────────────────────────

describe('MessageSchemaView', () => {
  describe('Empty & WKT', () => {
    it('renders empty message with indicator', () => {
      render(
        <MessageSchemaView
          message={emptyMessage}
          messageDefinitions={emptyDefinitions}
        />,
      );

      expect(screen.getByText('test.Empty')).toBeDefined();
      expect(screen.getByText('(空消息)')).toBeDefined();
    });

    it('renders WKT as collapsed with badge', () => {
      render(
        <MessageSchemaView
          message={wktMessage}
          messageDefinitions={emptyDefinitions}
        />,
      );

      expect(screen.getByText('google.protobuf.Timestamp')).toBeDefined();
      expect(screen.getByText('WKT')).toBeDefined();
    });
  });

  describe('Scalar & Enum fields', () => {
    it('renders scalar field names and field numbers', () => {
      render(
        <MessageSchemaView
          message={simpleMessage}
          messageDefinitions={emptyDefinitions}
        />,
      );

      // Message header
      expect(screen.getByText('test.Simple')).toBeDefined();

      // Field names
      expect(screen.getByText('name')).toBeDefined();
      expect(screen.getByText('age')).toBeDefined();
      expect(screen.getByText('active')).toBeDefined();

      // Field numbers
      expect(screen.getByText('= 1')).toBeDefined();
      expect(screen.getByText('= 2')).toBeDefined();
      expect(screen.getByText('= 3')).toBeDefined();
    });

    it('renders enum values inline', () => {
      render(
        <MessageSchemaView
          message={enumMessage}
          messageDefinitions={emptyDefinitions}
        />,
      );

      // Enum values rendered as // (RED=0, GREEN=1, BLUE=2)
      expect(screen.getByText(/RED=0/)).toBeDefined();
      expect(screen.getByText(/GREEN=1/)).toBeDefined();
      expect(screen.getByText(/BLUE=2/)).toBeDefined();
    });
  });

  describe('Map & Oneof', () => {
    it('renders map field with key-value type display', () => {
      render(
        <MessageSchemaView
          message={mapMessage}
          messageDefinitions={emptyDefinitions}
        />,
      );

      expect(screen.getByText('labels')).toBeDefined();
      expect(screen.getByText('map<string, int32>')).toBeDefined();
    });

    it('renders oneof group with its fields separated from regular fields', () => {
      render(
        <MessageSchemaView
          message={oneofMessage}
          messageDefinitions={emptyDefinitions}
        />,
      );

      // Oneof header — the zh-CN translation for oneofGroup is 'oneof: {group}'
      expect(screen.getByText(/oneof: test_oneof/)).toBeDefined();

      // Fields inside the oneof group should be visible
      expect(screen.getByText('alt_name')).toBeDefined();

      // The non-oneof field 'value' should be visible as a regular field
      expect(screen.getByText('value')).toBeDefined();
    });
  });

  describe('Nested & Depth', () => {
    it('click to expand nested message reveals inner fields', async () => {
      const user = userEvent.setup();
      render(
        <MessageSchemaView
          message={nestedMessage}
          messageDefinitions={emptyDefinitions}
        />,
      );

      // The "Inner" type should be visible as a clickable button
      const innerButton = screen.getByText('Inner');
      expect(innerButton).toBeDefined();

      // Nested fields should NOT be visible before expansion
      expect(screen.queryByText('nested_name')).toBeNull();

      // Click the type name to expand
      await user.click(innerButton);

      // Now the nested field name should appear
      expect(screen.getByText('nested_name')).toBeDefined();
    });

    it('shows max depth indicator when depth >= 5', () => {
      render(
        <MessageSchemaView
          message={simpleMessage}
          messageDefinitions={emptyDefinitions}
          depth={5}
        />,
      );

      // Should show the message name and max depth text
      expect(screen.getByText('test.Simple')).toBeDefined();
      // The span text is "... (最大深度)" — use substring/regex matching
      expect(screen.getByText(/\(最大深度\)/)).toBeDefined();
    });
  });

  describe('Circular & Reserved', () => {
    it('displays circular reference indicator', () => {
      render(
        <MessageSchemaView
          message={circularMessage}
          messageDefinitions={emptyDefinitions}
        />,
      );

      expect(screen.getByText('self_ref')).toBeDefined();
      // zh-CN translation for circularReference is '循环引用', wrapped in (循环引用)
      expect(screen.getByText(/循环引用/)).toBeDefined();
    });

    it('displays reserved field ranges formatted', () => {
      render(
        <MessageSchemaView
          message={reservedMessage}
          messageDefinitions={emptyDefinitions}
        />,
      );

      // reserved_ranges [[1,3],[10,10]] → formatReservedRanges → "1-3, 10"
      // zh-CN: reserved: '保留 {ranges}' → "保留 1-3, 10"
      expect(screen.getByText(/保留 1-3, 10/)).toBeDefined();
    });
  });

  describe('Label prop', () => {
    it('renders label badge when provided', () => {
      render(
        <MessageSchemaView
          message={simpleMessage}
          messageDefinitions={emptyDefinitions}
          label="请求结构"
        />,
      );

      expect(screen.getByText('请求结构')).toBeDefined();
    });
  });
});
