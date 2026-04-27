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
  /** TLS CA 证书文件路径 */
  tls_ca_cert?: string;
  /** TLS 客户端证书文件路径 */
  tls_client_cert?: string;
  /** TLS 客户端私钥文件路径 */
  tls_client_key?: string;
  /** TLS SNI 覆盖 */
  tls_server_name_override?: string;
  /** Reflection 请求附加 metadata（用于认证的反射服务） */
  reflection_metadata?: [string, string][];
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

/** gRPC Reflection 模式骨架生成请求参数 */
export interface GrpcGenerateRequestSkeletonFromReflectionInput {
  /** gRPC 服务器地址 */
  url: string;
  /** 是否使用 TLS */
  use_tls?: boolean;
  /** 是否跳过 TLS 证书验证 */
  tls_skip_verify?: boolean;
  /** 连接超时（毫秒） */
  connect_timeout_ms?: number;
  /** TLS CA 证书文件路径 */
  tls_ca_cert?: string;
  /** TLS 客户端证书文件路径 */
  tls_client_cert?: string;
  /** TLS 客户端私钥文件路径 */
  tls_client_key?: string;
  /** TLS SNI 覆盖 */
  tls_server_name_override?: string;
  /** Reflection 请求附加 metadata */
  reflection_metadata?: [string, string][];
  /** 服务全名（如 "package.ServiceName"） */
  service_name: string;
  /** 方法名 */
  method_name: string;
}

/** Proto 文件信息（对应 Rust ProtoFileOutput） */
export interface ProtoFileOutput {
  /** 文件名（相对路径） */
  path: string;
  /** 文件内容 */
  content: string;
}

/** 扫描 Proto 目录响应（对应 Rust GrpcScanProtoDirectoryOutput） */
export interface GrpcScanProtoDirectoryOutput {
  /** 是否成功 */
  success: boolean;
  /** 发现的 proto 文件列表 */
  files: ProtoFileOutput[];
  /** 目录根路径（用作 proto 编译的 include 路径） */
  import_path: string | null;
  /** 错误信息（失败时） */
  error: string | null;
}

/** 读取 Proto 文件列表响应（对应 Rust GrpcReadProtoFilesOutput） */
export interface GrpcReadProtoFilesOutput {
  /** 是否成功 */
  success: boolean;
  /** 文件列表 */
  files: ProtoFileOutput[];
  /** 错误信息（失败时） */
  error: string | null;
}

/** 解析 Descriptor Set 响应（对应 Rust GrpcParseDescriptorSetOutput） */
export interface GrpcParseDescriptorSetOutput {
  /** 是否成功 */
  success: boolean;
  /** 发现的服务列表 */
  services: GrpcServiceInfo[];
  /** 错误信息（失败时） */
  error: string | null;
}

// ============================================================================
// 请求 JSON 预验证
// ============================================================================

/** 验证错误条目 */
export interface ValidationError {
  /** 字段路径（如 "address.street"） */
  path: string;
  /** 错误描述 */
  message: string;
}

/** 验证结果 */
export interface ValidationResult {
  /** 是否通过验证 */
  valid: boolean;
  /** 错误列表 */
  errors: ValidationError[];
}

/** gRPC 请求 JSON 验证响应（对应 Rust GrpcValidateRequestOutput） */
export interface GrpcValidateRequestOutput {
  /** 是否成功执行验证（不代表验证通过） */
  success: boolean;
  /** 验证结果（成功执行时返回） */
  result: ValidationResult | null;
  /** 错误信息（执行失败时） */
  error: string | null;
}

/** gRPC 健康检查响应（对应 Rust GrpcHealthCheckOutput） */
export interface GrpcHealthCheckOutput {
  /** 是否成功执行检查 */
  success: boolean;
  /** 服务状态："UNKNOWN", "SERVING", "NOT_SERVING", "SERVICE_UNKNOWN", "NOT_IMPLEMENTED" */
  status: string | null;
  /** 错误信息（执行失败时） */
  error: string | null;
}

/** gRPC Channelz 内省信息（对应 Rust ChannelzInfo，EXPERIMENTAL） */
export interface ChannelzInfo {
  /** 连接状态: "READY", "IDLE", "CONNECTING", "TRANSIENT_FAILURE", "SHUTDOWN" */
  state: string;
  /** 连接建立时间（ISO 8601） */
  connected_at: string;
  /** 连接运行时长（秒） */
  uptime_secs: number;
  /** 活跃调用数 */
  active_calls: number;
  /** 最后错误（如果有） */
  last_error: string | null;
}

/** gRPC Channelz 内省响应（对应 Rust GrpcChannelzOutput） */
export interface GrpcChannelzOutput {
  /** 是否成功 */
  success: boolean;
  /** Channelz 信息（成功时返回） */
  info: ChannelzInfo | null;
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

/**
 * 通过 Server Reflection 生成 gRPC 请求消息的 JSON 骨架
 *
 * 创建临时连接，通过 Server Reflection 获取类型信息，
 * 生成指定服务和方法的输入消息骨架。
 * 适用于未提供 proto 文件的 Reflection 场景。
 *
 * @param input - Reflection 骨架生成请求参数
 * @returns 骨架生成结果
 * @throws IpcError 如果调用失败
 */
export async function generateGrpcRequestSkeletonFromReflection(
  input: GrpcGenerateRequestSkeletonFromReflectionInput
): Promise<GrpcGenerateRequestSkeletonOutput> {
  try {
    return await invoke<GrpcGenerateRequestSkeletonOutput>(
      'grpc_generate_request_skeleton_from_reflection',
      { input }
    );
  } catch (error) {
    throw formatError(error, 'grpc_generate_request_skeleton_from_reflection');
  }
}

/**
 * 扫描目录，发现所有 .proto 文件
 *
 * 递归遍历指定目录及其子目录，收集所有 `.proto` 文件。
 * 跳过以 `.` 开头的隐藏目录。
 *
 * @param dirPath - 目录路径
 * @returns 扫描结果（文件列表和 import 路径）
 * @throws IpcError 如果调用失败
 */
export async function scanProtoDirectory(
  dirPath: string
): Promise<GrpcScanProtoDirectoryOutput> {
  try {
    return await invoke<GrpcScanProtoDirectoryOutput>('grpc_scan_proto_directory', {
      dirPath,
    });
  } catch (error) {
    throw formatError(error, 'grpc_scan_proto_directory');
  }
}

/**
 * 读取多个 .proto 文件的内容
 *
 * 接收文件路径列表，读取每个文件的内容并返回。
 * 用于文件选择器选择 .proto 文件后批量读取内容。
 *
 * @param filePaths - 文件路径列表
 * @returns 文件列表（路径和内容）
 * @throws IpcError 如果调用失败
 */
export async function readProtoFiles(
  filePaths: string[]
): Promise<GrpcReadProtoFilesOutput> {
  try {
    return await invoke<GrpcReadProtoFilesOutput>('grpc_read_proto_files', {
      filePaths,
    });
  } catch (error) {
    throw formatError(error, 'grpc_read_proto_files');
  }
}

/**
 * 解析 FileDescriptorSet 二进制文件
 *
 * 读取 `.pb` / `.fdset` / `.desc` 格式的 FileDescriptorSet 文件，
 * 解析并返回其中的服务和方法信息。
 *
 * @param filePath - 文件路径
 * @returns 服务发现结果
 * @throws IpcError 如果调用失败
 */
export async function parseDescriptorSet(
  filePath: string
): Promise<GrpcParseDescriptorSetOutput> {
  try {
    return await invoke<GrpcParseDescriptorSetOutput>('grpc_parse_descriptor_set', {
      filePath,
    });
  } catch (error) {
    throw formatError(error, 'grpc_parse_descriptor_set');
  }
}

/**
 * 验证请求 JSON 是否符合 proto 消息定义
 *
 * 根据 proto schema 检查 JSON 字段名和类型是否与消息定义匹配。
 * 仅做建议性检查，不阻止发送。
 *
 * @param json - 待验证的 JSON 字符串
 * @param messageName - 消息全名（如 "test.GetUserRequest"）
 * @param protoFiles - Proto 文件列表 [path, content]
 * @returns 验证结果
 * @throws IpcError 如果调用失败
 */
export async function validateGrpcRequest(
  json: string,
  messageName: string,
  protoFiles?: [string, string][]
): Promise<GrpcValidateRequestOutput> {
  try {
    return await invoke<GrpcValidateRequestOutput>('grpc_validate_request', {
      input: {
        json,
        message_name: messageName,
        proto_files: protoFiles,
      },
    });
  } catch (error) {
    throw formatError(error, 'grpc_validate_request');
  }
}

/**
 * 执行 gRPC 健康检查（unary Check）
 *
 * 创建临时连接向 gRPC 服务器发送 Health.Check 请求，检查后立即关闭。
 * 使用 gRPC Health Checking Protocol。
 *
 * @param url - gRPC 服务器地址
 * @param options - 可选配置
 * @returns 健康检查结果
 * @throws IpcError 如果调用失败
 */
export async function grpcHealthCheck(
  url: string,
  options?: {
    useTls?: boolean;
    tlsSkipVerify?: boolean;
    connectTimeoutMs?: number;
    tlsCaCert?: string;
    tlsClientCert?: string;
    tlsClientKey?: string;
    tlsServerNameOverride?: string;
    serviceName?: string;
  }
): Promise<GrpcHealthCheckOutput> {
  try {
    return await invoke<GrpcHealthCheckOutput>('grpc_health_check', {
      url,
      useTls: options?.useTls,
      tlsSkipVerify: options?.tlsSkipVerify,
      connectTimeoutMs: options?.connectTimeoutMs,
      tlsCaCert: options?.tlsCaCert,
      tlsClientCert: options?.tlsClientCert,
      tlsClientKey: options?.tlsClientKey,
      tlsServerNameOverride: options?.tlsServerNameOverride,
      serviceName: options?.serviceName,
    });
  } catch (error) {
    throw formatError(error, 'grpc_health_check');
  }
}

/**
 * 获取 gRPC 连接的 Channelz 内省信息（EXPERIMENTAL）
 *
 * 查询活跃 gRPC 连接的轻量级诊断信息，包括连接状态、运行时长、活跃调用数等。
 * 需要在流程执行期间调用（连接存在于执行会话的 gRPC manager 中）。
 *
 * @param connectionId - gRPC 连接 ID（通常为 grpc_connect 节点的 UUID）
 * @returns Channelz 内省结果
 * @throws IpcError 如果调用失败
 */
export async function grpcChannelzInfo(
  connectionId: string
): Promise<GrpcChannelzOutput> {
  try {
    return await invoke<GrpcChannelzOutput>('grpc_channelz_info', {
      connectionId,
    });
  } catch (error) {
    throw formatError(error, 'grpc_channelz_info');
  }
}
