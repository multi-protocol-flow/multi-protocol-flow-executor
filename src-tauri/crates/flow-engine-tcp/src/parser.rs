//! 协议解析引擎
//!
//! 支持 3 种内置解析器：
//! - `LengthPrefix`: [length][data] 格式
//! - Delimiter: 分隔符边界
//! - TLV: [Tag][Length][Value] 格式
//!
//! 所有解析器支持 parse → re-encode → 字节精确匹配往返。

use serde::{Deserialize, Serialize};

/// 解析后的字段
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedField {
    /// 字段名称
    pub name: String,
    /// 数据中的偏移量（字节）
    pub offset: usize,
    /// 字段长度（字节）
    pub length: usize,
    /// 原始字节值
    pub value_bytes: Vec<u8>,
    /// 显示用字符串（hex 或 UTF-8）
    pub value_display: String,
}

/// 解析后的消息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedMessage {
    /// 解析出的字段列表
    pub fields: Vec<ParsedField>,
    /// 未解析的剩余数据
    pub remaining: Vec<u8>,
    /// 解析器名称
    pub parser_name: String,
}

impl ParsedMessage {
    /// 从解析结果重新编码为原始字节
    ///
    /// 按字段顺序拼接所有 `value_bytes`。
    pub fn re_encode(&self) -> Vec<u8> {
        let mut result = Vec::new();
        for field in &self.fields {
            result.extend_from_slice(&field.value_bytes);
        }
        result
    }
}

/// 解析器配置（序列化为 JSON 保存）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ParserConfig {
    /// 长度前缀解析器配置
    LengthPrefix {
        /// 长度头字节数（1/2/4/8）
        prefix_bytes: u8,
        /// 字节序："big" | "little"
        byte_order: String,
        /// 长度是否包含自身
        includes_self: bool,
    },
    /// 分隔符解析器配置
    Delimiter {
        /// 分隔符（hex 编码）
        delimiter_hex: String,
        /// 是否保留分隔符
        keep_delimiter: bool,
    },
    /// TLV（Tag-Length-Value）解析器配置
    Tlv {
        /// Tag 字节数（1/2/4）
        tag_bytes: u8,
        /// Length 字节数（1/2/4）
        length_bytes: u8,
        /// 字节序："big" | "little"
        byte_order: String,
    },
}

/// 协议解析器 trait
pub trait ProtocolParser: Send + Sync {
    /// 解析数据，返回解析结果
    fn parse(&self, data: &[u8]) -> Result<ParsedMessage, String>;

    /// 解析器名称
    fn name(&self) -> &str;

    /// 从配置创建解析器
    fn from_config(config: &ParserConfig) -> Result<Box<dyn ProtocolParser>, String>
    where
        Self: Sized,
    {
        create_parser(config)
    }
}

/// 从配置创建解析器实例
pub fn create_parser(config: &ParserConfig) -> Result<Box<dyn ProtocolParser>, String> {
    match config {
        ParserConfig::LengthPrefix {
            prefix_bytes,
            byte_order,
            includes_self,
        } => Ok(Box::new(LengthPrefixParser {
            prefix_bytes: *prefix_bytes,
            byte_order: byte_order.clone(),
            includes_self: *includes_self,
        })),
        ParserConfig::Delimiter {
            delimiter_hex,
            keep_delimiter,
        } => {
            let delimiter = hex::decode(delimiter_hex)
                .map_err(|e| format!("分隔符 hex 解码失败: {}", e))?;
            if delimiter.is_empty() {
                return Err("分隔符不能为空".to_string());
            }
            Ok(Box::new(DelimiterParser {
                delimiter,
                keep_delimiter: *keep_delimiter,
            }))
        }
        ParserConfig::Tlv {
            tag_bytes,
            length_bytes,
            byte_order,
        } => Ok(Box::new(TlvParser {
            tag_bytes: *tag_bytes,
            length_bytes: *length_bytes,
            byte_order: byte_order.clone(),
        })),
    }
}

// === 辅助函数 ===

/// 字节显示：优先 UTF-8，否则 hex
fn display_bytes(data: &[u8]) -> String {
    match std::str::from_utf8(data) {
        Ok(s) => s.to_string(),
        Err(_) => hex::encode(data),
    }
}

/// 读取多字节整数（支持大端/小端）
fn read_uint(bytes: &[u8], byte_order: &str) -> Result<u64, String> {
    match byte_order {
        "big" => match bytes.len() {
            1 => Ok(bytes[0] as u64),
            2 => Ok(u16::from_be_bytes(
                bytes
                    .try_into()
                    .map_err(|_| "2 字节读取失败".to_string())?,
            ) as u64),
            4 => Ok(u32::from_be_bytes(
                bytes
                    .try_into()
                    .map_err(|_| "4 字节读取失败".to_string())?,
            ) as u64),
            8 => Ok(u64::from_be_bytes(
                bytes
                    .try_into()
                    .map_err(|_| "8 字节读取失败".to_string())?,
            )),
            _ => Err(format!("不支持的字节数: {}", bytes.len())),
        },
        "little" => match bytes.len() {
            1 => Ok(bytes[0] as u64),
            2 => Ok(u16::from_le_bytes(
                bytes
                    .try_into()
                    .map_err(|_| "2 字节读取失败".to_string())?,
            ) as u64),
            4 => Ok(u32::from_le_bytes(
                bytes
                    .try_into()
                    .map_err(|_| "4 字节读取失败".to_string())?,
            ) as u64),
            8 => Ok(u64::from_le_bytes(
                bytes
                    .try_into()
                    .map_err(|_| "8 字节读取失败".to_string())?,
            )),
            _ => Err(format!("不支持的字节数: {}", bytes.len())),
        },
        _ => Err(format!(
            "不支持的字节序: {}（支持 'big' 或 'little'）",
            byte_order
        )),
    }
}

/// 将整数编码为多字节（支持大端/小端）
#[cfg(test)]
fn write_uint(value: u64, num_bytes: u8, byte_order: &str) -> Result<Vec<u8>, String> {
    match byte_order {
        "big" => match num_bytes {
            1 => {
                if value > u8::MAX as u64 {
                    return Err(format!("值 {} 超出 1 字节范围", value));
                }
                Ok(vec![value as u8])
            }
            2 => {
                if value > u16::MAX as u64 {
                    return Err(format!("值 {} 超出 2 字节范围", value));
                }
                Ok((value as u16).to_be_bytes().to_vec())
            }
            4 => {
                if value > u32::MAX as u64 {
                    return Err(format!("值 {} 超出 4 字节范围", value));
                }
                Ok((value as u32).to_be_bytes().to_vec())
            }
            8 => Ok(value.to_be_bytes().to_vec()),
            _ => Err(format!("不支持的字节数: {}", num_bytes)),
        },
        "little" => match num_bytes {
            1 => {
                if value > u8::MAX as u64 {
                    return Err(format!("值 {} 超出 1 字节范围", value));
                }
                Ok(vec![value as u8])
            }
            2 => {
                if value > u16::MAX as u64 {
                    return Err(format!("值 {} 超出 2 字节范围", value));
                }
                Ok((value as u16).to_le_bytes().to_vec())
            }
            4 => {
                if value > u32::MAX as u64 {
                    return Err(format!("值 {} 超出 4 字节范围", value));
                }
                Ok((value as u32).to_le_bytes().to_vec())
            }
            8 => Ok(value.to_le_bytes().to_vec()),
            _ => Err(format!("不支持的字节数: {}", num_bytes)),
        },
        _ => Err(format!(
            "不支持的字节序: {}（支持 'big' 或 'little'）",
            byte_order
        )),
    }
}

// ============================================================================
// LengthPrefixParser
// ============================================================================

/// 长度前缀解析器
///
/// 解析 [length][payload] 格式，支持 1/2/4/8 字节长度前缀和大端/小端字节序。
pub struct LengthPrefixParser {
    prefix_bytes: u8,
    byte_order: String,
    includes_self: bool,
}

impl ProtocolParser for LengthPrefixParser {
    fn parse(&self, data: &[u8]) -> Result<ParsedMessage, String> {
        if data.is_empty() {
            return Err("数据为空，无法解析长度前缀".to_string());
        }

        let prefix_len = self.prefix_bytes as usize;
        if data.len() < prefix_len {
            return Err(format!(
                "数据长度 {} 小于前缀字节数 {}",
                data.len(),
                prefix_len
            ));
        }

        let prefix_data = &data[..prefix_len];
        let raw_length = read_uint(prefix_data, &self.byte_order)?;

        // 计算实际载荷长度
        let payload_len = if self.includes_self {
            if raw_length < prefix_len as u64 {
                return Err(format!(
                    "长度值 {} 小于前缀自身长度 {}（includes_self=true 时长度应包含前缀）",
                    raw_length, prefix_len
                ));
            }
            (raw_length - prefix_len as u64) as usize
        } else {
            raw_length as usize
        };

        let total_needed = prefix_len + payload_len;
        if data.len() < total_needed {
            return Err(format!(
                "数据截断：需要 {} 字节（前缀 {} + 载荷 {}），实际 {} 字节",
                total_needed,
                prefix_len,
                payload_len,
                data.len()
            ));
        }

        let payload_data = &data[prefix_len..total_needed];
        let remaining = data[total_needed..].to_vec();

        let mut fields = Vec::new();

        // 字段 1: length（前缀字节）
        fields.push(ParsedField {
            name: "length".to_string(),
            offset: 0,
            length: prefix_len,
            value_bytes: prefix_data.to_vec(),
            value_display: raw_length.to_string(),
        });

        // 字段 2: payload（数据字节）
        if !payload_data.is_empty() {
            fields.push(ParsedField {
                name: "payload".to_string(),
                offset: prefix_len,
                length: payload_len,
                value_bytes: payload_data.to_vec(),
                value_display: display_bytes(payload_data),
            });
        }

        Ok(ParsedMessage {
            fields,
            remaining,
            parser_name: self.name().to_string(),
        })
    }

    fn name(&self) -> &str {
        "length_prefix"
    }
}

// ============================================================================
// DelimiterParser
// ============================================================================

/// 分隔符解析器
///
/// 解析 delimiter 分隔的消息，支持 hex 编码的分隔符。
pub struct DelimiterParser {
    delimiter: Vec<u8>,
    keep_delimiter: bool,
}

impl ProtocolParser for DelimiterParser {
    fn parse(&self, data: &[u8]) -> Result<ParsedMessage, String> {
        if data.is_empty() {
            return Err("数据为空，无法查找分隔符".to_string());
        }

        // 查找第一个分隔符位置
        let pos = find_delimiter(data, &self.delimiter);
        if pos.is_none() {
            return Err(format!(
                "未找到分隔符 {}",
                hex::encode(&self.delimiter)
            ));
        }

        let pos = pos.expect("已验证");
        let msg_end = pos; // 消息结束位置（分隔符开始位置）
        let delim_end = pos + self.delimiter.len(); // 分隔符结束位置

        let message_data = &data[..msg_end];
        let mut fields = Vec::new();

        // 字段 1: message（分隔符前的数据）
        fields.push(ParsedField {
            name: "message".to_string(),
            offset: 0,
            length: msg_end,
            value_bytes: message_data.to_vec(),
            value_display: display_bytes(message_data),
        });

        // 字段 2: delimiter（如果保留）
        if self.keep_delimiter {
            fields.push(ParsedField {
                name: "delimiter".to_string(),
                offset: msg_end,
                length: self.delimiter.len(),
                value_bytes: self.delimiter.clone(),
                value_display: hex::encode(&self.delimiter),
            });
        }

        let remaining = data[delim_end..].to_vec();

        Ok(ParsedMessage {
            fields,
            remaining,
            parser_name: self.name().to_string(),
        })
    }

    fn name(&self) -> &str {
        "delimiter"
    }
}

/// 在数据中查找分隔符位置
fn find_delimiter(data: &[u8], delimiter: &[u8]) -> Option<usize> {
    if delimiter.len() > data.len() {
        return None;
    }
    data.windows(delimiter.len())
        .position(|window| window == delimiter)
}

// ============================================================================
// TlvParser
// ============================================================================

/// TLV（Tag-Length-Value）解析器
///
/// 解析连续的 [Tag][Length][Value] 结构。
pub struct TlvParser {
    tag_bytes: u8,
    length_bytes: u8,
    byte_order: String,
}

impl ProtocolParser for TlvParser {
    fn parse(&self, data: &[u8]) -> Result<ParsedMessage, String> {
        if data.is_empty() {
            return Err("数据为空，无法解析 TLV".to_string());
        }

        let tag_len = self.tag_bytes as usize;
        let length_len = self.length_bytes as usize;
        let header_len = tag_len + length_len;

        let mut fields = Vec::new();
        let mut offset = 0usize;
        let mut index = 0usize;

        while offset < data.len() {
            // 检查是否有足够字节读取 header
            if offset + header_len > data.len() {
                // 剩余数据不足一个完整 TLV header
                let remaining = data[offset..].to_vec();
                return Ok(ParsedMessage {
                    fields,
                    remaining,
                    parser_name: self.name().to_string(),
                });
            }

            // 读取 Tag
            let tag_data = &data[offset..offset + tag_len];
            let tag_value = read_uint(tag_data, &self.byte_order)?;

            // 读取 Length
            let length_offset = offset + tag_len;
            let length_data = &data[length_offset..length_offset + length_len];
            let value_len = read_uint(length_data, &self.byte_order)? as usize;

            // 检查是否有足够字节读取 Value
            let value_offset = offset + header_len;
            if value_offset + value_len > data.len() {
                return Err(format!(
                    "TLV 条目 {} 数据截断：tag={} 需要值长度 {}，剩余 {} 字节",
                    index,
                    tag_value,
                    value_len,
                    data.len() - value_offset
                ));
            }

            let value_data = &data[value_offset..value_offset + value_len];

            // Tag 字段
            fields.push(ParsedField {
                name: format!("tag_{}", index),
                offset,
                length: tag_len,
                value_bytes: tag_data.to_vec(),
                value_display: tag_value.to_string(),
            });

            // Length 字段
            fields.push(ParsedField {
                name: format!("length_{}", index),
                offset: length_offset,
                length: length_len,
                value_bytes: length_data.to_vec(),
                value_display: value_len.to_string(),
            });

            // Value 字段
            if value_len > 0 {
                fields.push(ParsedField {
                    name: format!("value_{}", index),
                    offset: value_offset,
                    length: value_len,
                    value_bytes: value_data.to_vec(),
                    value_display: display_bytes(value_data),
                });
            }

            offset = value_offset + value_len;
            index += 1;
        }

        Ok(ParsedMessage {
            fields,
            remaining: Vec::new(),
            parser_name: self.name().to_string(),
        })
    }

    fn name(&self) -> &str {
        "tlv"
    }
}

// ============================================================================
// 重新编码辅助
// ============================================================================

/// 从 `ParsedMessage` 重新编码（使用 `ParsedMessage::re_encode`）
///
/// 这是便捷函数，等价于 `message.re_encode()`。
pub fn re_encode_message(message: &ParsedMessage) -> Vec<u8> {
    message.re_encode()
}

/// 从 `ParserConfig` 和原始数据执行完整的 parse → re-encode → 比较
pub fn roundtrip_verify(data: &[u8], config: &ParserConfig) -> Result<bool, String> {
    let parser = create_parser(config)?;
    let message = parser.parse(data)?;
    let re_encoded = re_encode_message(&message);

    // 比较原始数据和重新编码的数据（不含 remaining）
    let expected_len = data.len() - message.remaining.len();
    let original = &data[..expected_len];

    Ok(re_encoded == original)
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // === LengthPrefixParser 测试 ===

    #[test]
    fn test_parser_length_prefix_basic() {
        // [0x00, 0x05, 'H', 'E', 'L', 'L', 'O']
        let mut data = vec![0x00u8, 0x05];
        data.extend_from_slice(b"HELLO");

        let parser = LengthPrefixParser {
            prefix_bytes: 2,
            byte_order: "big".to_string(),
            includes_self: false,
        };

        let result = parser.parse(&data).expect("解析应成功");
        assert_eq!(result.fields.len(), 2);

        // 验证 length 字段
        assert_eq!(result.fields[0].name, "length");
        assert_eq!(result.fields[0].value_display, "5");

        // 验证 payload 字段
        assert_eq!(result.fields[1].name, "payload");
        assert_eq!(result.fields[1].value_bytes, b"HELLO");
        assert_eq!(result.fields[1].value_display, "HELLO");

        assert!(result.remaining.is_empty());
    }

    #[test]
    fn test_parser_length_prefix_with_remaining() {
        // [0x00, 0x03, 'A', 'B', 'C', 'D', 'E'] — 剩余 DE
        let mut data = vec![0x00u8, 0x03];
        data.extend_from_slice(b"ABCDE");

        let parser = LengthPrefixParser {
            prefix_bytes: 2,
            byte_order: "big".to_string(),
            includes_self: false,
        };

        let result = parser.parse(&data).expect("解析应成功");
        assert_eq!(result.fields[1].value_bytes, b"ABC");
        assert_eq!(result.remaining, b"DE");
    }

    #[test]
    fn test_parser_length_prefix_1byte() {
        let mut data = vec![0x03];
        data.extend_from_slice(b"ABC");

        let parser = LengthPrefixParser {
            prefix_bytes: 1,
            byte_order: "big".to_string(),
            includes_self: false,
        };

        let result = parser.parse(&data).expect("解析应成功");
        assert_eq!(result.fields[1].value_bytes, b"ABC");
    }

    #[test]
    fn test_parser_length_prefix_4byte() {
        let mut data = vec![0x00, 0x00, 0x00, 0x04];
        data.extend_from_slice(b"TEST");

        let parser = LengthPrefixParser {
            prefix_bytes: 4,
            byte_order: "big".to_string(),
            includes_self: false,
        };

        let result = parser.parse(&data).expect("解析应成功");
        assert_eq!(result.fields[1].value_bytes, b"TEST");
    }

    #[test]
    fn test_parser_length_prefix_8byte() {
        let mut data = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02];
        data.extend_from_slice(b"OK");

        let parser = LengthPrefixParser {
            prefix_bytes: 8,
            byte_order: "big".to_string(),
            includes_self: false,
        };

        let result = parser.parse(&data).expect("解析应成功");
        assert_eq!(result.fields[1].value_bytes, b"OK");
    }

    #[test]
    fn test_parser_length_prefix_little_endian() {
        // 小端序: 0x0500 = 5
        let mut data = vec![0x05, 0x00];
        data.extend_from_slice(b"HELLO");

        let parser = LengthPrefixParser {
            prefix_bytes: 2,
            byte_order: "little".to_string(),
            includes_self: false,
        };

        let result = parser.parse(&data).expect("解析应成功");
        assert_eq!(result.fields[1].value_bytes, b"HELLO");
    }

    #[test]
    fn test_parser_length_prefix_includes_self() {
        // includes_self=true: 长度值包含前缀本身
        // 总长 7 = 前缀 2 + 载荷 5
        let mut data = vec![0x00, 0x07]; // 长度值 = 7（包含前缀 2 字节）
        data.extend_from_slice(b"HELLO");

        let parser = LengthPrefixParser {
            prefix_bytes: 2,
            byte_order: "big".to_string(),
            includes_self: true,
        };

        let result = parser.parse(&data).expect("解析应成功");
        assert_eq!(result.fields[0].value_display, "7");
        assert_eq!(result.fields[1].value_bytes, b"HELLO");
    }

    #[test]
    fn test_parser_length_prefix_zero_payload() {
        let data = vec![0x00, 0x00]; // 长度 = 0

        let parser = LengthPrefixParser {
            prefix_bytes: 2,
            byte_order: "big".to_string(),
            includes_self: false,
        };

        let result = parser.parse(&data).expect("解析应成功");
        // 只有 length 字段，无 payload
        assert_eq!(result.fields.len(), 1);
        assert_eq!(result.fields[0].name, "length");
        assert_eq!(result.fields[0].value_display, "0");
    }

    #[test]
    fn test_parser_length_prefix_truncated() {
        // 长度声明 10，但只有 3 字节数据
        let mut data = vec![0x00, 0x0A]; // 长度 = 10
        data.extend_from_slice(b"ABC");

        let parser = LengthPrefixParser {
            prefix_bytes: 2,
            byte_order: "big".to_string(),
            includes_self: false,
        };

        let result = parser.parse(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("数据截断"));
    }

    #[test]
    fn test_parser_length_prefix_empty_data() {
        let parser = LengthPrefixParser {
            prefix_bytes: 2,
            byte_order: "big".to_string(),
            includes_self: false,
        };

        let result = parser.parse(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("数据为空"));
    }

    #[test]
    fn test_parser_length_prefix_data_shorter_than_prefix() {
        let parser = LengthPrefixParser {
            prefix_bytes: 4,
            byte_order: "big".to_string(),
            includes_self: false,
        };

        let result = parser.parse(&[0x00, 0x01]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("小于前缀字节数"));
    }

    // === DelimiterParser 测试 ===

    #[test]
    fn test_parser_delimiter_basic() {
        let data = b"hello\r\n".to_vec();
        let parser = DelimiterParser {
            delimiter: b"\r\n".to_vec(),
            keep_delimiter: false,
        };

        let result = parser.parse(&data).expect("解析应成功");
        assert_eq!(result.fields.len(), 1);
        assert_eq!(result.fields[0].name, "message");
        assert_eq!(result.fields[0].value_bytes, b"hello");
        assert_eq!(result.fields[0].value_display, "hello");
        assert!(result.remaining.is_empty());
    }

    #[test]
    fn test_parser_delimiter_with_remaining() {
        let data = b"hello\r\nworld\r\n".to_vec();
        let parser = DelimiterParser {
            delimiter: b"\r\n".to_vec(),
            keep_delimiter: false,
        };

        let result = parser.parse(&data).expect("解析应成功");
        assert_eq!(result.fields[0].value_bytes, b"hello");
        assert_eq!(result.remaining, b"world\r\n");
    }

    #[test]
    fn test_parser_delimiter_keep_delimiter() {
        let data = b"hello\r\n".to_vec();
        let parser = DelimiterParser {
            delimiter: b"\r\n".to_vec(),
            keep_delimiter: true,
        };

        let result = parser.parse(&data).expect("解析应成功");
        assert_eq!(result.fields.len(), 2);
        assert_eq!(result.fields[0].name, "message");
        assert_eq!(result.fields[1].name, "delimiter");
        assert_eq!(result.fields[1].value_bytes, b"\r\n");
    }

    #[test]
    fn test_parser_delimiter_hex_config() {
        // 使用 hex 编码的分隔符配置
        let config = ParserConfig::Delimiter {
            delimiter_hex: "0d0a".to_string(), // \r\n
            keep_delimiter: false,
        };

        let parser = create_parser(&config).expect("创建解析器应成功");
        let data = b"hello\r\n".to_vec();
        let result = parser.parse(&data).expect("解析应成功");
        assert_eq!(result.fields[0].value_bytes, b"hello");
    }

    #[test]
    fn test_parser_delimiter_no_delimiter_found() {
        let data = b"hello world".to_vec();
        let parser = DelimiterParser {
            delimiter: b"\r\n".to_vec(),
            keep_delimiter: false,
        };

        let result = parser.parse(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未找到分隔符"));
    }

    #[test]
    fn test_parser_delimiter_empty_data() {
        let parser = DelimiterParser {
            delimiter: b"\r\n".to_vec(),
            keep_delimiter: false,
        };

        let result = parser.parse(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_parser_delimiter_empty_message() {
        // 连续分隔符 → 空消息
        let data = b"\r\n".to_vec();
        let parser = DelimiterParser {
            delimiter: b"\r\n".to_vec(),
            keep_delimiter: false,
        };

        let result = parser.parse(&data).expect("解析应成功");
        assert_eq!(result.fields[0].value_bytes, b"");
        assert_eq!(result.fields[0].length, 0);
    }

    // === TlvParser 测试 ===

    #[test]
    fn test_parser_tlv_basic() {
        // [tag=0x01][len=0x03][ABC][tag=0x02][len=0x02][DE]
        let data: Vec<u8> = vec![0x01, 0x03, b'A', b'B', b'C', 0x02, 0x02, b'D', b'E'];

        let parser = TlvParser {
            tag_bytes: 1,
            length_bytes: 1,
            byte_order: "big".to_string(),
        };

        let result = parser.parse(&data).expect("解析应成功");
        // 2 个 TLV 条目，每个 3 个字段
        assert_eq!(result.fields.len(), 6);

        // 第一个 TLV
        assert_eq!(result.fields[0].name, "tag_0");
        assert_eq!(result.fields[0].value_display, "1");
        assert_eq!(result.fields[1].name, "length_0");
        assert_eq!(result.fields[1].value_display, "3");
        assert_eq!(result.fields[2].name, "value_0");
        assert_eq!(result.fields[2].value_bytes, b"ABC");

        // 第二个 TLV
        assert_eq!(result.fields[3].name, "tag_1");
        assert_eq!(result.fields[3].value_display, "2");
        assert_eq!(result.fields[4].name, "length_1");
        assert_eq!(result.fields[4].value_display, "2");
        assert_eq!(result.fields[5].name, "value_1");
        assert_eq!(result.fields[5].value_bytes, b"DE");

        assert!(result.remaining.is_empty());
    }

    #[test]
    fn test_parser_tlv_2byte_tags() {
        // [tag=0x0001][len=0x02][AB]
        let mut data = vec![0x00, 0x01, 0x02];
        data.extend_from_slice(b"AB");

        let parser = TlvParser {
            tag_bytes: 2,
            length_bytes: 1,
            byte_order: "big".to_string(),
        };

        let result = parser.parse(&data).expect("解析应成功");
        assert_eq!(result.fields[0].name, "tag_0");
        assert_eq!(result.fields[0].value_display, "1");
        assert_eq!(result.fields[2].value_bytes, b"AB");
    }

    #[test]
    fn test_parser_tlv_4byte_tags_and_length() {
        // [tag=0x00000001][len=0x00000002][AB]
        let mut data = vec![0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02];
        data.extend_from_slice(b"AB");

        let parser = TlvParser {
            tag_bytes: 4,
            length_bytes: 4,
            byte_order: "big".to_string(),
        };

        let result = parser.parse(&data).expect("解析应成功");
        assert_eq!(result.fields[0].value_display, "1");
        assert_eq!(result.fields[2].value_bytes, b"AB");
    }

    #[test]
    fn test_parser_tlv_zero_value() {
        // [tag=0x01][len=0x00]
        let data: Vec<u8> = vec![0x01, 0x00];

        let parser = TlvParser {
            tag_bytes: 1,
            length_bytes: 1,
            byte_order: "big".to_string(),
        };

        let result = parser.parse(&data).expect("解析应成功");
        // tag + length 字段，无 value（长度为 0）
        assert_eq!(result.fields.len(), 2);
        assert_eq!(result.fields[0].name, "tag_0");
        assert_eq!(result.fields[1].name, "length_0");
        assert_eq!(result.fields[1].value_display, "0");
    }

    #[test]
    fn test_parser_tlv_truncated_header() {
        // 不完整的 header
        let data: Vec<u8> = vec![0x01]; // 只有 tag，缺少 length

        let parser = TlvParser {
            tag_bytes: 1,
            length_bytes: 1,
            byte_order: "big".to_string(),
        };

        let result = parser.parse(&data).expect("解析应成功");
        // 剩余数据不足一个完整 header，放入 remaining
        assert_eq!(result.fields.len(), 0);
        assert_eq!(result.remaining, vec![0x01]);
    }

    #[test]
    fn test_parser_tlv_truncated_value() {
        // [tag=0x01][len=0x05][ABC] — 声明 5 字节但只有 3 字节
        let data: Vec<u8> = vec![0x01, 0x05, b'A', b'B', b'C'];

        let parser = TlvParser {
            tag_bytes: 1,
            length_bytes: 1,
            byte_order: "big".to_string(),
        };

        let result = parser.parse(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("数据截断"));
    }

    #[test]
    fn test_parser_tlv_empty_data() {
        let parser = TlvParser {
            tag_bytes: 1,
            length_bytes: 1,
            byte_order: "big".to_string(),
        };

        let result = parser.parse(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("数据为空"));
    }

    #[test]
    fn test_parser_tlv_little_endian() {
        // 小端序: tag=0x0100(=1 in little-endian reading), len=0x0200(=2)
        // Actually: [0x01, 0x00] as tag → little-endian reads as 1
        // [0x02, 0x00] as length → little-endian reads as 2
        let mut data = vec![0x01, 0x00, 0x02, 0x00]; // tag=1, len=2 (2-byte each, LE)
        data.extend_from_slice(b"AB");

        let parser = TlvParser {
            tag_bytes: 2,
            length_bytes: 2,
            byte_order: "little".to_string(),
        };

        let result = parser.parse(&data).expect("解析应成功");
        assert_eq!(result.fields[0].value_display, "1");
        assert_eq!(result.fields[1].value_display, "2");
        assert_eq!(result.fields[2].value_bytes, b"AB");
    }

    // === 往返测试 (parse → re-encode → byte-exact match) ===

    #[test]
    fn test_parser_roundtrip_length_prefix() {
        let mut data = vec![0x00, 0x05];
        data.extend_from_slice(b"HELLO");

        let parser = LengthPrefixParser {
            prefix_bytes: 2,
            byte_order: "big".to_string(),
            includes_self: false,
        };

        let message = parser.parse(&data).expect("解析应成功");
        let re_encoded = re_encode_message(&message);
        assert_eq!(re_encoded, data);
    }

    #[test]
    fn test_parser_roundtrip_length_prefix_4byte() {
        let mut data = vec![0x00, 0x00, 0x00, 0x0A];
        data.extend_from_slice(b"1234567890");

        let parser = LengthPrefixParser {
            prefix_bytes: 4,
            byte_order: "big".to_string(),
            includes_self: false,
        };

        let message = parser.parse(&data).expect("解析应成功");
        let re_encoded = re_encode_message(&message);
        assert_eq!(re_encoded, data);
    }

    #[test]
    fn test_parser_roundtrip_delimiter() {
        let data = b"hello\r\n".to_vec();
        let parser = DelimiterParser {
            delimiter: b"\r\n".to_vec(),
            keep_delimiter: true, // 保留分隔符才能精确匹配
        };

        let message = parser.parse(&data).expect("解析应成功");
        let re_encoded = re_encode_message(&message);
        assert_eq!(re_encoded, data);
    }

    #[test]
    fn test_parser_roundtrip_delimiter_without_keep() {
        let data = b"hello\r\n".to_vec();
        let parser = DelimiterParser {
            delimiter: b"\r\n".to_vec(),
            keep_delimiter: false,
        };

        let message = parser.parse(&data).expect("解析应成功");
        let re_encoded = re_encode_message(&message);
        // 不保留分隔符时，re-encode 不含分隔符
        assert_eq!(re_encoded, b"hello");
    }

    #[test]
    fn test_parser_roundtrip_tlv() {
        let data: Vec<u8> = vec![0x01, 0x03, b'A', b'B', b'C', 0x02, 0x02, b'D', b'E'];

        let parser = TlvParser {
            tag_bytes: 1,
            length_bytes: 1,
            byte_order: "big".to_string(),
        };

        let message = parser.parse(&data).expect("解析应成功");
        let re_encoded = re_encode_message(&message);
        assert_eq!(re_encoded, data);
    }

    #[test]
    fn test_parser_roundtrip_tlv_multiple_entries() {
        // 3 个 TLV 条目
        let data = vec![0x01, 0x02, b'A', b'B', 0x02, 0x01, b'C', 0x03, 0x00];
        let parser = TlvParser {
            tag_bytes: 1,
            length_bytes: 1,
            byte_order: "big".to_string(),
        };

        let message = parser.parse(&data).expect("解析应成功");
        let re_encoded = re_encode_message(&message);
        assert_eq!(re_encoded, data);
    }

    // === roundtrip_verify 便捷函数测试 ===

    #[test]
    fn test_parser_roundtrip_verify_length_prefix() {
        let mut data = vec![0x00, 0x05];
        data.extend_from_slice(b"HELLO");

        let config = ParserConfig::LengthPrefix {
            prefix_bytes: 2,
            byte_order: "big".to_string(),
            includes_self: false,
        };

        assert!(roundtrip_verify(&data, &config).expect("往返验证应成功"));
    }

    #[test]
    fn test_parser_roundtrip_verify_tlv() {
        let data: Vec<u8> = vec![0x01, 0x03, b'A', b'B', b'C', 0x02, 0x02, b'D', b'E'];
        let config = ParserConfig::Tlv {
            tag_bytes: 1,
            length_bytes: 1,
            byte_order: "big".to_string(),
        };

        assert!(roundtrip_verify(&data, &config).expect("往返验证应成功"));
    }

    // === create_parser 工厂测试 ===

    #[test]
    fn test_parser_create_length_prefix() {
        let config = ParserConfig::LengthPrefix {
            prefix_bytes: 4,
            byte_order: "big".to_string(),
            includes_self: false,
        };

        let parser = create_parser(&config).expect("创建解析器应成功");
        assert_eq!(parser.name(), "length_prefix");
    }

    #[test]
    fn test_parser_create_delimiter() {
        let config = ParserConfig::Delimiter {
            delimiter_hex: "0d0a".to_string(),
            keep_delimiter: false,
        };

        let parser = create_parser(&config).expect("创建解析器应成功");
        assert_eq!(parser.name(), "delimiter");
    }

    #[test]
    fn test_parser_create_tlv() {
        let config = ParserConfig::Tlv {
            tag_bytes: 2,
            length_bytes: 2,
            byte_order: "big".to_string(),
        };

        let parser = create_parser(&config).expect("创建解析器应成功");
        assert_eq!(parser.name(), "tlv");
    }

    #[test]
    fn test_parser_create_delimiter_invalid_hex() {
        let config = ParserConfig::Delimiter {
            delimiter_hex: "ZZZZ".to_string(),
            keep_delimiter: false,
        };

        let result = create_parser(&config);
        assert!(result.is_err());
        let err = result.err().expect("应返回错误");
        assert!(err.contains("hex 解码失败"));
    }

    #[test]
    fn test_parser_create_delimiter_empty_delimiter() {
        let config = ParserConfig::Delimiter {
            delimiter_hex: "".to_string(),
            keep_delimiter: false,
        };

        let result = create_parser(&config);
        assert!(result.is_err());
        let err = result.err().expect("应返回错误");
        assert!(err.contains("分隔符不能为空"));
    }

    // === ParserConfig 序列化测试 ===

    #[test]
    fn test_parser_config_serialize_deserialize() {
        let config = ParserConfig::LengthPrefix {
            prefix_bytes: 2,
            byte_order: "big".to_string(),
            includes_self: false,
        };

        let json = serde_json::to_string(&config).expect("序列化应成功");
        let deserialized: ParserConfig =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_parser_config_tlv_serialize_deserialize() {
        let config = ParserConfig::Tlv {
            tag_bytes: 2,
            length_bytes: 1,
            byte_order: "little".to_string(),
        };

        let json = serde_json::to_string(&config).expect("序列化应成功");
        assert!(json.contains("\"type\":\"tlv\""));

        let deserialized: ParserConfig =
            serde_json::from_str(&json).expect("反序列化应成功");
        assert_eq!(config, deserialized);
    }

    // === 边界情况 ===

    #[test]
    fn test_parser_binary_display() {
        // 非法 UTF-8 数据显示为 hex
        let data = vec![0xFF, 0xFE, 0x00, 0x01];
        let display = display_bytes(&data);
        assert_eq!(display, "fffe0001");
    }

    #[test]
    fn test_parser_utf8_display() {
        let data = b"HELLO".to_vec();
        let display = display_bytes(&data);
        assert_eq!(display, "HELLO");
    }

    #[test]
    fn test_parser_write_read_uint_roundtrip() {
        for &num_bytes in &[1u8, 2, 4, 8] {
            for byte_order in &["big", "little"] {
                let value: u64 = 42;
                let encoded = write_uint(value, num_bytes, byte_order).expect("编码应成功");
                assert_eq!(encoded.len(), num_bytes as usize);
                let decoded = read_uint(&encoded, byte_order).expect("解码应成功");
                assert_eq!(decoded, value);
            }
        }
    }

    #[test]
    fn test_parser_length_prefix_includes_self_error_small() {
        // includes_self=true 但长度值小于前缀长度
        let data = vec![0x00, 0x01]; // 长度 = 1 < 前缀 2

        let parser = LengthPrefixParser {
            prefix_bytes: 2,
            byte_order: "big".to_string(),
            includes_self: true,
        };

        let result = parser.parse(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("小于前缀自身长度"));
    }
}
