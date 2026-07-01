//! 二进制编码工具模块
//!
//! 提供 Base64 编解码、UTF-8 文本预览、标准二进制输出 JSON 构建。

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

/// Base64 编码
pub fn encode_to_base64(data: &[u8]) -> String {
    BASE64.encode(data)
}

/// Base64 解码
pub fn decode_from_base64(encoded: &str) -> Result<Vec<u8>, String> {
    BASE64
        .decode(encoded)
        .map_err(|e| format!("Base64 decoding failed: {}", e))
}

/// 尝试 UTF-8 解码（用于 `text_preview`）
///
/// 如果数据超过 1024 字节，截断并添加 `...[truncated]` 后缀。
pub fn try_decode_text(data: &[u8]) -> Option<String> {
    std::str::from_utf8(data).ok().map(|s| {
        if s.len() > 1024 {
            format!("{}...[truncated]", &s[..1024])
        } else {
            s.to_string()
        }
    })
}

/// 构建标准二进制输出 JSON
///
/// 返回格式：
/// ```json
/// {
///   "data": "<base64>",
///   "encoding": "base64",
///   "size": <byte_length>,
///   "text_preview": "<utf8_or_null>"
/// }
/// ```
pub fn build_binary_output(data: &[u8]) -> serde_json::Value {
    serde_json::json!({
        "data": encode_to_base64(data),
        "encoding": "base64",
        "size": data.len(),
        "text_preview": try_decode_text(data),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_roundtrip_empty() {
        let data: Vec<u8> = Vec::new();
        let output = build_binary_output(&data);

        assert_eq!(output["size"], 0);
        assert_eq!(output["encoding"], "base64");
        assert_eq!(output["text_preview"], "");

        let decoded = decode_from_base64(output["data"].as_str().expect("data 应为字符串"))
            .expect("解码应成功");
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_binary_roundtrip_1byte() {
        let data = vec![0x00];
        let output = build_binary_output(&data);

        assert_eq!(output["size"], 1);

        let decoded = decode_from_base64(output["data"].as_str().expect("data 应为字符串"))
            .expect("解码应成功");
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_binary_roundtrip_1kb() {
        let data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
        let output = build_binary_output(&data);

        assert_eq!(output["size"], 1024);
        // 不可打印字节，text_preview 应为 null
        assert!(output["text_preview"].is_null());

        let decoded = decode_from_base64(output["data"].as_str().expect("data 应为字符串"))
            .expect("解码应成功");
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_binary_roundtrip_all_zeros() {
        let data = vec![0x00u8; 65536];
        let output = build_binary_output(&data);

        assert_eq!(output["size"], 65536);

        let decoded = decode_from_base64(output["data"].as_str().expect("data 应为字符串"))
            .expect("解码应成功");
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_binary_roundtrip_all_ff() {
        let data = vec![0xFFu8; 65536];
        let output = build_binary_output(&data);

        assert_eq!(output["size"], 65536);

        let decoded = decode_from_base64(output["data"].as_str().expect("data 应为字符串"))
            .expect("解码应成功");
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_encode_decode_base64_basic() {
        let data = b"Hello, World!";
        let encoded = encode_to_base64(data);
        let decoded = decode_from_base64(&encoded).expect("解码应成功");
        assert_eq!(decoded, data.as_slice());
    }

    #[test]
    fn test_decode_from_base64_invalid() {
        let result = decode_from_base64("!!!invalid!!!");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Base64 decoding failed"));
    }

    #[test]
    fn test_try_decode_text_utf8() {
        let text = "你好世界";
        let data = text.as_bytes();
        let result = try_decode_text(data);
        assert_eq!(result, Some(text.to_string()));
    }

    #[test]
    fn test_try_decode_text_truncation() {
        let long_text = "a".repeat(2000);
        let data = long_text.as_bytes();
        let result = try_decode_text(data).expect("应成功解码");
        assert!(result.ends_with("...[truncated]"));
        assert_eq!(result.len(), 1024 + "...[truncated]".len());
    }

    #[test]
    fn test_try_decode_text_non_utf8() {
        let data = vec![0xFF, 0xFE, 0x00, 0x01];
        let result = try_decode_text(&data);
        assert!(result.is_none());
    }

    #[test]
    fn test_build_binary_output_with_text() {
        let text = "Hello TCP";
        let data = text.as_bytes();
        let output = build_binary_output(data);

        assert_eq!(output["size"], data.len());
        assert_eq!(output["encoding"], "base64");
        assert_eq!(output["text_preview"], text);

        let decoded = decode_from_base64(output["data"].as_str().expect("data 应为字符串"))
            .expect("解码应成功");
        assert_eq!(decoded, data);
    }
}
