//! TCP 消息分帧引擎
//!
//! 支持 5 种分帧模式：
//! - `LengthPrefix`: N 字节长度前缀（大端序）
//! - Delimiter: 分隔符边界
//! - `FixedLength`: 固定长度
//! - Timeout: 超时读取
//! - Manual: 直通（不添加帧）

use flow_engine_core::traits::{FramingConfig, TcpFramingMode};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// TCP 分帧引擎
///
/// 负责在发送时添加帧、在接收时解析帧。
pub struct TcpFramingEngine;

impl TcpFramingEngine {
    /// 为数据添加帧并发送到写入端
    ///
    /// 根据配置的分帧模式，将 `data` 封装为带帧的数据并写入 `writer`。
    pub async fn frame_send<W: AsyncWriteExt + Unpin>(
        writer: &mut W,
        data: &[u8],
        config: &FramingConfig,
    ) -> Result<(), String> {
        match &config.mode {
            TcpFramingMode::LengthPrefix { prefix_bytes } => {
                let payload_len = data.len();
                let prefix = encode_length_prefix(payload_len, *prefix_bytes)?;

                writer
                    .write_all(&prefix)
                    .await
                    .map_err(|e| format!("发送长度前缀失败: {}", e))?;
                writer
                    .write_all(data)
                    .await
                    .map_err(|e| format!("发送数据失败: {}", e))?;
            }
            TcpFramingMode::Delimiter { delimiter } => {
                writer
                    .write_all(data)
                    .await
                    .map_err(|e| format!("发送数据失败: {}", e))?;
                writer
                    .write_all(delimiter.as_bytes())
                    .await
                    .map_err(|e| format!("发送分隔符失败: {}", e))?;
            }
            TcpFramingMode::FixedLength { length } => {
                let payload = if data.len() >= *length {
                    // 截断到固定长度
                    &data[..*length]
                } else {
                    // 不足则补零
                    return Err(format!(
                        "数据长度 {} 小于固定帧长度 {}",
                        data.len(),
                        length
                    ));
                };
                writer
                    .write_all(payload)
                    .await
                    .map_err(|e| format!("发送固定长度数据失败: {}", e))?;
            }
            TcpFramingMode::Timeout { .. } | TcpFramingMode::Manual => {
                // 超时模式和手动模式：直接发送原始数据
                writer
                    .write_all(data)
                    .await
                    .map_err(|e| format!("发送数据失败: {}", e))?;
            }
        }

        writer
            .flush()
            .await
            .map_err(|e| format!("刷新写入缓冲区失败: {}", e))?;
        Ok(())
    }

    /// 仅生成带帧的数据（不发送），用于测试和预计算
    ///
    /// 返回包含帧头的完整字节数据。
    pub fn frame_data(data: &[u8], config: &FramingConfig) -> Result<Vec<u8>, String> {
        match &config.mode {
            TcpFramingMode::LengthPrefix { prefix_bytes } => {
                let prefix = encode_length_prefix(data.len(), *prefix_bytes)?;
                let mut framed = Vec::with_capacity(prefix.len() + data.len());
                framed.extend_from_slice(&prefix);
                framed.extend_from_slice(data);
                Ok(framed)
            }
            TcpFramingMode::Delimiter { delimiter } => {
                let mut framed = Vec::with_capacity(data.len() + delimiter.len());
                framed.extend_from_slice(data);
                framed.extend_from_slice(delimiter.as_bytes());
                Ok(framed)
            }
            TcpFramingMode::FixedLength { length } => {
                if data.len() < *length {
                    return Err(format!(
                        "数据长度 {} 小于固定帧长度 {}",
                        data.len(),
                        length
                    ));
                }
                Ok(data[..*length].to_vec())
            }
            TcpFramingMode::Timeout { .. } | TcpFramingMode::Manual => Ok(data.to_vec()),
        }
    }

    /// 从读取端解析帧
    ///
    /// 根据配置的分帧模式，从 `reader` 读取并解析一条完整消息。
    pub async fn deframe_receive<R: AsyncReadExt + Unpin>(
        reader: &mut R,
        config: &FramingConfig,
        timeout_ms: u64,
    ) -> Result<Vec<u8>, String> {
        match &config.mode {
            TcpFramingMode::LengthPrefix { prefix_bytes } => {
                read_length_prefixed(reader, *prefix_bytes, config.max_frame_size).await
            }
            TcpFramingMode::Delimiter { delimiter } => {
                read_delimited(reader, delimiter.as_bytes(), config.max_frame_size).await
            }
            TcpFramingMode::FixedLength { length } => {
                read_fixed_length(reader, *length, config.max_frame_size).await
            }
            TcpFramingMode::Timeout { timeout_ms: frame_timeout } => {
                let effective_timeout = if *frame_timeout > 0 {
                    *frame_timeout
                } else {
                    timeout_ms
                };
                read_until_timeout(reader, effective_timeout, config.max_frame_size).await
            }
            TcpFramingMode::Manual => {
                read_until_timeout(reader, timeout_ms, config.max_frame_size).await
            }
        }
    }
}

/// 编码长度前缀（大端序）
fn encode_length_prefix(len: usize, prefix_bytes: u8) -> Result<Vec<u8>, String> {
    match prefix_bytes {
        1 => {
            if len > u8::MAX as usize {
                return Err(format!(
                    "数据长度 {} 超出 1 字节前缀最大值 {}",
                    len,
                    u8::MAX
                ));
            }
            Ok(vec![len as u8])
        }
        2 => {
            if len > u16::MAX as usize {
                return Err(format!(
                    "数据长度 {} 超出 2 字节前缀最大值 {}",
                    len,
                    u16::MAX
                ));
            }
            Ok((len as u16).to_be_bytes().to_vec())
        }
        4 => {
            if len > u32::MAX as usize {
                return Err(format!(
                    "数据长度 {} 超出 4 字节前缀最大值 {}",
                    len,
                    u32::MAX
                ));
            }
            Ok((len as u32).to_be_bytes().to_vec())
        }
        8 => Ok((len as u64).to_be_bytes().to_vec()),
        _ => Err(format!("不支持的前缀字节数: {}（支持 1/2/4/8）", prefix_bytes)),
    }
}

/// 读取长度前缀消息
async fn read_length_prefixed<R: AsyncReadExt + Unpin>(
    reader: &mut R,
    prefix_bytes: u8,
    max_frame_size: usize,
) -> Result<Vec<u8>, String> {
    let mut prefix_buf = vec![0u8; prefix_bytes as usize];
    reader
        .read_exact(&mut prefix_buf)
        .await
        .map_err(|e| format!("读取长度前缀失败: {}", e))?;

    let payload_len = decode_length_prefix(&prefix_buf)?;
    if payload_len > max_frame_size {
        return Err(format!(
            "消息长度 {} 超出最大帧大小 {}",
            payload_len, max_frame_size
        ));
    }

    let mut payload = vec![0u8; payload_len];
    reader
        .read_exact(&mut payload)
        .await
        .map_err(|e| format!("读取消息体失败: {}", e))?;

    Ok(payload)
}

/// 解码长度前缀（大端序）
fn decode_length_prefix(prefix: &[u8]) -> Result<usize, String> {
    match prefix.len() {
        1 => Ok(prefix[0] as usize),
        2 => Ok(u16::from_be_bytes(
            prefix
                .try_into()
                .map_err(|_| "2 字节前缀解析失败".to_string())?,
        ) as usize),
        4 => Ok(u32::from_be_bytes(
            prefix
                .try_into()
                .map_err(|_| "4 字节前缀解析失败".to_string())?,
        ) as usize),
        8 => Ok(u64::from_be_bytes(
            prefix
                .try_into()
                .map_err(|_| "8 字节前缀解析失败".to_string())?,
        ) as usize),
        _ => Err(format!("不支持的前缀长度: {}", prefix.len())),
    }
}

/// 读取分隔符结尾的消息
async fn read_delimited<R: AsyncReadExt + Unpin>(
    reader: &mut R,
    delimiter: &[u8],
    max_frame_size: usize,
) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];

    loop {
        match reader.read(&mut byte).await {
            Ok(0) => {
                // EOF
                break;
            }
            Ok(_) => {
                buf.push(byte[0]);

                if buf.len() > max_frame_size {
                    return Err(format!(
                        "消息大小 {} 超出最大帧大小 {}",
                        buf.len(),
                        max_frame_size
                    ));
                }

                // 检查是否匹配分隔符
                if buf.len() >= delimiter.len() {
                    let tail = &buf[buf.len() - delimiter.len()..];
                    if tail == delimiter {
                        // 移除分隔符
                        buf.truncate(buf.len() - delimiter.len());
                        break;
                    }
                }
            }
            Err(e) => return Err(format!("读取数据失败: {}", e)),
        }
    }

    Ok(buf)
}

/// 读取固定长度消息
async fn read_fixed_length<R: AsyncReadExt + Unpin>(
    reader: &mut R,
    length: usize,
    max_frame_size: usize,
) -> Result<Vec<u8>, String> {
    if length > max_frame_size {
        return Err(format!(
            "固定帧长度 {} 超出最大帧大小 {}",
            length, max_frame_size
        ));
    }

    let mut buf = vec![0u8; length];
    reader
        .read_exact(&mut buf)
        .await
        .map_err(|e| format!("读取固定长度数据失败: {}", e))?;

    Ok(buf)
}

/// 超时读取（在超时前读取所有可用数据）
async fn read_until_timeout<R: AsyncReadExt + Unpin>(
    reader: &mut R,
    timeout_ms: u64,
    max_frame_size: usize,
) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];

    let result = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), async {
        loop {
            match reader.read(&mut tmp).await {
                Ok(0) => break, // EOF
                Ok(n) => {
                    buf.extend_from_slice(&tmp[..n]);
                    if buf.len() > max_frame_size {
                        return Err(format!(
                            "消息大小 {} 超出最大帧大小 {}",
                            buf.len(),
                            max_frame_size
                        ));
                    }
                }
                Err(e) => return Err(format!("读取数据失败: {}", e)),
            }
        }
        Ok(())
    })
    .await;

    match result {
        Ok(inner) => inner?,
        Err(_) => {
            // 超时，返回已读取的数据（可能为空）
        }
    }

    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // === 辅助函数 ===

    fn make_config(mode: TcpFramingMode) -> FramingConfig {
        FramingConfig {
            mode,
            max_frame_size: 1024 * 1024,
            include_framing_in_data: false,
        }
    }

    /// 辅助：frame_data + deframe_receive 往返验证
    async fn roundtrip(data: &[u8], config: &FramingConfig) -> Vec<u8> {
        let framed = TcpFramingEngine::frame_data(data, config).expect("frame_data 应成功");
        let mut cursor = Cursor::new(framed);
        TcpFramingEngine::deframe_receive(&mut cursor, config, 1000)
            .await
            .expect("deframe_receive 应成功")
    }

    // === LengthPrefix 测试 ===

    #[tokio::test]
    async fn test_framing_length_prefix_2bytes() {
        let config = make_config(TcpFramingMode::LengthPrefix { prefix_bytes: 2 });
        let data = b"Hello, TCP!".to_vec();
        let result = roundtrip(&data, &config).await;
        assert_eq!(result, data);
    }

    #[tokio::test]
    async fn test_framing_length_prefix_4bytes() {
        let config = make_config(TcpFramingMode::LengthPrefix { prefix_bytes: 4 });
        let data = (0..256).map(|i| (i % 256) as u8).collect::<Vec<u8>>();
        let result = roundtrip(&data, &config).await;
        assert_eq!(result, data);
    }

    #[tokio::test]
    async fn test_framing_length_prefix_1byte() {
        let config = make_config(TcpFramingMode::LengthPrefix { prefix_bytes: 1 });
        let data = b"short".to_vec();
        let result = roundtrip(&data, &config).await;
        assert_eq!(result, data);
    }

    #[tokio::test]
    async fn test_framing_length_prefix_8bytes() {
        let config = make_config(TcpFramingMode::LengthPrefix { prefix_bytes: 8 });
        let data = b"8-byte prefix test".to_vec();
        let result = roundtrip(&data, &config).await;
        assert_eq!(result, data);
    }

    // === Delimiter 测试 ===

    #[tokio::test]
    async fn test_framing_delimiter() {
        let config = make_config(TcpFramingMode::Delimiter {
            delimiter: "\r\n".to_string(),
        });
        let data = b"line one".to_vec();
        let result = roundtrip(&data, &config).await;
        assert_eq!(result, data);
    }

    #[tokio::test]
    async fn test_framing_delimiter_custom() {
        let config = make_config(TcpFramingMode::Delimiter {
            delimiter: "<EOF>".to_string(),
        });
        let data = b"custom delimited message".to_vec();
        let result = roundtrip(&data, &config).await;
        assert_eq!(result, data);
    }

    // === FixedLength 测试 ===

    #[tokio::test]
    async fn test_framing_fixed_length() {
        let config = make_config(TcpFramingMode::FixedLength { length: 8 });
        let data = b"12345678".to_vec();
        let result = roundtrip(&data, &config).await;
        assert_eq!(result, data);
    }

    // === Manual 测试 ===

    #[tokio::test]
    async fn test_framing_manual() {
        let config = make_config(TcpFramingMode::Manual);
        let data = b"raw data".to_vec();
        // Manual 模式使用 timeout 读取，但 roundtrip 会立即返回数据
        let framed = TcpFramingEngine::frame_data(&data, &config).expect("frame_data 应成功");
        assert_eq!(framed, data); // Manual 不添加帧
    }

    // === 边界情况 ===

    #[tokio::test]
    async fn test_framing_empty_payload() {
        let config = make_config(TcpFramingMode::LengthPrefix { prefix_bytes: 4 });
        let data: Vec<u8> = Vec::new();
        let result = roundtrip(&data, &config).await;
        assert_eq!(result, data);
    }

    #[tokio::test]
    async fn test_framing_delimiter_in_payload() {
        let config = make_config(TcpFramingMode::Delimiter {
            delimiter: "\r\n".to_string(),
        });
        // 载荷中包含分隔符
        let data = b"line one\r\nline two".to_vec();
        let framed =
            TcpFramingEngine::frame_data(&data, &config).expect("frame_data 应成功");
        // framed = "line one\r\nline two\r\n"
        let mut cursor = Cursor::new(framed);
        // 第一次读取应该只读到第一个 \r\n
        let first = TcpFramingEngine::deframe_receive(&mut cursor, &config, 1000)
            .await
            .expect("deframe 应成功");
        assert_eq!(first, b"line one");

        // 第二次读取到 "line two"
        let second = TcpFramingEngine::deframe_receive(&mut cursor, &config, 1000)
            .await
            .expect("deframe 应成功");
        assert_eq!(second, b"line two");
    }

    // === encode_length_prefix 单元测试 ===

    #[test]
    fn test_encode_length_prefix_1byte() {
        let result = encode_length_prefix(255, 1).expect("应成功");
        assert_eq!(result, vec![0xFF]);
    }

    #[test]
    fn test_encode_length_prefix_2byte() {
        let result = encode_length_prefix(256, 2).expect("应成功");
        assert_eq!(result, vec![0x01, 0x00]);
    }

    #[test]
    fn test_encode_length_prefix_4byte() {
        let result = encode_length_prefix(1024, 4).expect("应成功");
        assert_eq!(result, vec![0x00, 0x00, 0x04, 0x00]);
    }

    #[test]
    fn test_encode_length_prefix_overflow() {
        let result = encode_length_prefix(256, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_length_prefix_unsupported() {
        let result = encode_length_prefix(10, 3);
        assert!(result.is_err());
    }

    // === frame_send 测试 ===

    #[tokio::test]
    async fn test_frame_send_length_prefix() {
        let config = make_config(TcpFramingMode::LengthPrefix { prefix_bytes: 4 });
        let data = b"hello";
        let mut buf = Vec::new();
        TcpFramingEngine::frame_send(&mut buf, data, &config)
            .await
            .expect("frame_send 应成功");

        // 验证: 4 字节前缀 + 5 字节数据
        assert_eq!(buf.len(), 9);
        // 大端序长度前缀 = 5
        assert_eq!(&buf[0..4], &[0, 0, 0, 5]);
        assert_eq!(&buf[4..9], b"hello");
    }

    #[tokio::test]
    async fn test_frame_send_delimiter() {
        let config = make_config(TcpFramingMode::Delimiter {
            delimiter: "\n".to_string(),
        });
        let data = b"hello";
        let mut buf = Vec::new();
        TcpFramingEngine::frame_send(&mut buf, data, &config)
            .await
            .expect("frame_send 应成功");

        assert_eq!(buf, b"hello\n".to_vec());
    }

    #[tokio::test]
    async fn test_frame_send_fixed_length_exact() {
        let config = make_config(TcpFramingMode::FixedLength { length: 5 });
        let data = b"hello";
        let mut buf = Vec::new();
        TcpFramingEngine::frame_send(&mut buf, data, &config)
            .await
            .expect("frame_send 应成功");
        assert_eq!(buf, b"hello".to_vec());
    }

    #[tokio::test]
    async fn test_frame_send_fixed_length_truncate() {
        let config = make_config(TcpFramingMode::FixedLength { length: 3 });
        let data = b"hello";
        let mut buf = Vec::new();
        TcpFramingEngine::frame_send(&mut buf, data, &config)
            .await
            .expect("frame_send 应成功");
        assert_eq!(buf, b"hel".to_vec());
    }

    #[tokio::test]
    async fn test_frame_send_fixed_length_too_short() {
        let config = make_config(TcpFramingMode::FixedLength { length: 10 });
        let data = b"hi";
        let mut buf = Vec::new();
        let result = TcpFramingEngine::frame_send(&mut buf, data, &config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_frame_send_manual() {
        let config = make_config(TcpFramingMode::Manual);
        let data = b"raw";
        let mut buf = Vec::new();
        TcpFramingEngine::frame_send(&mut buf, data, &config)
            .await
            .expect("frame_send 应成功");
        assert_eq!(buf, b"raw".to_vec());
    }

    #[tokio::test]
    async fn test_frame_send_timeout_mode() {
        let config = make_config(TcpFramingMode::Timeout { timeout_ms: 100 });
        let data = b"timed data";
        let mut buf = Vec::new();
        TcpFramingEngine::frame_send(&mut buf, data, &config)
            .await
            .expect("frame_send 应成功");
        assert_eq!(buf, b"timed data".to_vec());
    }
}
