import { JsonView, allExpanded, defaultStyles } from 'react-json-view-lite';
import 'react-json-view-lite/dist/index.css';

interface RawJsonViewerProps {
  /** JSON 数据 */
  data: unknown;
}

/** 自定义样式：优化 JSON 显示效果 */
const customStyles = {
  ...defaultStyles,
  container: 'json-viewer-container',
  basicChildStyle: 'json-viewer-row',
  childFieldsContainer: 'json-viewer-nested',
  label: 'json-viewer-label',
  clickableLabel: 'json-viewer-label json-viewer-label-clickable',
  collapseIcon: 'json-viewer-icon json-viewer-collapse',
  expandIcon: 'json-viewer-icon json-viewer-expand',
  punctuation: 'json-viewer-punctuation',
  nullValue: 'json-viewer-null',
  undefinedValue: 'json-viewer-undefined',
  stringValue: 'json-viewer-string',
  numberValue: 'json-viewer-number',
  booleanValue: 'json-viewer-boolean',
  otherValue: 'json-viewer-other',
};

/**
 * JSON 数据展示组件
 * 使用 react-json-view-lite 提供语法高亮和折叠/展开功能
 */
export function RawJsonViewer({ data }: RawJsonViewerProps) {
  // JsonView 要求 data 为 Object | any[]，这里进行类型转换
  // unknown 类型在运行时会正常工作，只是 TypeScript 需要类型断言
  const jsonData = (data !== null && typeof data === 'object') ? data : { value: data };
  
  return (
    <div className="raw-json-viewer">
      <JsonView 
        data={jsonData as Record<string, unknown> | unknown[]} 
        shouldExpandNode={allExpanded}
        style={customStyles}
      />
    </div>
  );
}
