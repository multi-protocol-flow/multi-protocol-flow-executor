/**
 * 端口类型枚举
 */
export enum PortType {
  Input = 'input',
  Output = 'output',
}

/**
 * 端口接口
 */
export interface Port {
  id: string;
  type: PortType;
  label?: string;
}

/**
 * 连接接口
 */
export interface Connection {
  id: string;
  sourceNodeId: string;
  targetNodeId: string;
  sourcePortId: string;
  targetPortId: string;
}
