/**
 * gRPC 服务层 — gRPC 协议相关的 Tauri IPC 调用
 */

import { invoke } from '@tauri-apps/api/core';
import { formatError } from './flow';
import type { GrpcServiceInfo } from '@/types';

// ============================================================================
// 类型定义
// ============================================================================

/** gRPC 服务发现请求参数（对应 Rust GrpcDiscoverServicesInput） */
export interface GrpcDiscoverServicesInput {
  /** gRPC 服务器地址 */
  url: string;
  /** 是否使用 TLS */
  use_tls?: boolean;
  /** 是否跳过 TLS 证书验证 */
  tls_skip_verify?: boolean;
  /** 是否启用 Server Reflection */
  enable_reflection?: boolean;
  /** Proto 文件列表 [path, content] */
  proto_files?: [string, string][];
  /** 连接超时（毫秒） */
  connect_timeout_ms?: number;
}

/** gRPC 服务发现响应（对应 Rust GrpcDiscoverServicesOutput） */
export interface GrpcDiscoverServicesOutput {
  /** 是否成功 */
  success: boolean;
  /** 发现的服务列表 */
  services: GrpcServiceInfo[];
  /** 错误信息（失败时） */
  error: string | null;
}

/** gRPC 请求骨架生成请求参数（对应 Rust GrpcGenerateRequestSkeletonInput） */
export interface GrpcGenerateRequestSkeletonInput {
  /** Proto 文件列表 [path, content] */
  proto_files: [string, string][];
  /** 服务全名（如 "package.ServiceName"） */
  service_name: string;
  /** 方法名 */
  method_name: string;
}

/** gRPC 请求骨架生成响应（对应 Rust GrpcGenerateRequestSkeletonOutput） */
export interface GrpcGenerateRequestSkeletonOutput {
  /** 是否成功 */
  success: boolean;
  /** 生成的 JSON 骨架字符串 */
  skeleton: string | null;
  /** 错误信息（失败时） */
  error: string | null;
}

// ============================================================================
// IPC 调用
// ============================================================================

/**
 * 发现 gRPC 服务
 *
 * 通过 Server Reflection 或解析 Proto 文件发现可用服务。
 * 不创建持久连接，仅返回服务/方法列表供前端缓存。
 *
 * @param input - 服务发现请求参数
 * @returns 服务发现结果
 * @throws IpcError 如果调用失败
 */
export async function discoverGrpcServices(
  input: GrpcDiscoverServicesInput
): Promise<GrpcDiscoverServicesOutput> {
  try {
    return await invoke<GrpcDiscoverServicesOutput>('grpc_discover_services', {
      input,
    });
  } catch (error) {
    throw formatError(error, 'grpc_discover_services');
  }
}

/**
 * 生成 gRPC 请求消息的 JSON 骨架
 *
 * 根据指定服务和方法的 Input Type，解析 proto 文件并生成占位 JSON。
 * 用于帮助用户快速构造请求消息。
 *
 * @param input - 骨架生成请求参数
 * @returns 骨架生成结果
 * @throws IpcError 如果调用失败
 */
export async function generateGrpcRequestSkeleton(
  input: GrpcGenerateRequestSkeletonInput
): Promise<GrpcGenerateRequestSkeletonOutput> {
  try {
    return await invoke<GrpcGenerateRequestSkeletonOutput>('grpc_generate_request_skeleton', {
      input,
    });
  } catch (error) {
    throw formatError(error, 'grpc_generate_request_skeleton');
  }
}
