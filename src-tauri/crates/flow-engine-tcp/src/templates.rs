//! 协议模板系统
//!
//! 提供 5 个预置协议模板，支持 JSON 序列化/反序列化、导入/导出。
//! 模板包含分帧配置、解析器配置和字段说明，便于快速创建常见协议节点。

use serde::{Deserialize, Serialize};

use crate::parser::ParserConfig;

/// 协议模板
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolTemplate {
    /// 模板唯一标识
    pub id: String,
    /// 模板名称
    pub name: String,
    /// 模板描述
    pub description: String,
    /// 协议分类
    pub category: String,
    /// 分帧配置（JSON value）
    pub framing_config: serde_json::Value,
    /// 解析器配置（可选）
    pub parser_config: Option<ParserConfig>,
    /// 示例数据（hex 编码）
    pub example_data_hex: Option<String>,
    /// 字段说明列表
    pub field_descriptions: Vec<FieldDescription>,
    /// 是否为内置模板
    pub builtin: bool,
}

/// 字段说明
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDescription {
    /// 字段名称
    pub name: String,
    /// 字段说明
    pub description: String,
    /// 数据中的字节偏移量
    pub offset: Option<usize>,
    /// 字段字节长度
    pub length: Option<usize>,
}

/// 获取所有内置模板
pub fn builtin_templates() -> Vec<ProtocolTemplate> {
    vec![
        // 1. Redis RESP 协议
        ProtocolTemplate {
            id: "redis_resp".to_string(),
            name: "Redis RESP".to_string(),
            description: "Redis 序列化协议 (REdis Serialization Protocol)，基于 \\r\\n 分隔的文本协议".to_string(),
            category: "cache".to_string(),
            framing_config: serde_json::json!({
                "mode": { "delimiter": { "delimiter": "\r\n" } },
                "max_frame_size": 1048576,
                "include_framing_in_data": false
            }),
            parser_config: Some(ParserConfig::Delimiter {
                delimiter_hex: "0d0a".to_string(),
                keep_delimiter: false,
            }),
            example_data_hex: Some("2b4f4b0d0a".to_string()), // +OK\r\n
            field_descriptions: vec![
                FieldDescription {
                    name: "type_prefix".to_string(),
                    description: "类型前缀：+ (简单字符串)、- (错误)、: (整数)、$ (批量字符串)、* (数组)".to_string(),
                    offset: Some(0),
                    length: Some(1),
                },
                FieldDescription {
                    name: "data".to_string(),
                    description: " RESP 数据内容，直至 \\r\\n 结束".to_string(),
                    offset: Some(1),
                    length: None,
                },
            ],
            builtin: true,
        },
        // 2. 自定义 TLV 协议
        ProtocolTemplate {
            id: "custom_tlv".to_string(),
            name: "Custom TLV".to_string(),
            description: "自定义 TLV (Tag-Length-Value) 二进制协议，Tag 1 字节 + Length 2 字节 + Value".to_string(),
            category: "binary".to_string(),
            framing_config: serde_json::json!({
                "mode": { "length_prefix": { "prefix_bytes": 2 } },
                "max_frame_size": 65535,
                "include_framing_in_data": false
            }),
            parser_config: Some(ParserConfig::Tlv {
                tag_bytes: 1,
                length_bytes: 2,
                byte_order: "big".to_string(),
            }),
            example_data_hex: Some("010003414243".to_string()), // [tag=0x01][len=0x0003][ABC]
            field_descriptions: vec![
                FieldDescription {
                    name: "tag".to_string(),
                    description: "类型标签，1 字节标识数据类型".to_string(),
                    offset: Some(0),
                    length: Some(1),
                },
                FieldDescription {
                    name: "length".to_string(),
                    description: "数据长度，2 字节大端序无符号整数".to_string(),
                    offset: Some(1),
                    length: Some(2),
                },
                FieldDescription {
                    name: "value".to_string(),
                    description: "实际数据内容，长度由 length 字段指定".to_string(),
                    offset: Some(3),
                    length: None,
                },
            ],
            builtin: true,
        },
        // 3. 固定头部协议
        ProtocolTemplate {
            id: "fixed_header".to_string(),
            name: "Fixed Header".to_string(),
            description: "固定魔数头 + 4 字节长度前缀的二进制协议".to_string(),
            category: "binary".to_string(),
            framing_config: serde_json::json!({
                "mode": { "length_prefix": { "prefix_bytes": 4 } },
                "max_frame_size": 1048576,
                "include_framing_in_data": false
            }),
            parser_config: Some(ParserConfig::LengthPrefix {
                prefix_bytes: 4,
                byte_order: "big".to_string(),
                includes_self: false,
            }),
            example_data_hex: Some("4d414749430002000668656c6c6f21".to_string()), // MAGIC + 0x0002 + 0x0006 + "hello!"
            field_descriptions: vec![
                FieldDescription {
                    name: "magic".to_string(),
                    description: "魔数标识，用于验证协议类型".to_string(),
                    offset: Some(0),
                    length: Some(4),
                },
                FieldDescription {
                    name: "version".to_string(),
                    description: "协议版本号，2 字节大端序".to_string(),
                    offset: Some(4),
                    length: Some(2),
                },
                FieldDescription {
                    name: "length".to_string(),
                    description: "载荷长度，4 字节大端序".to_string(),
                    offset: Some(6),
                    length: Some(4),
                },
                FieldDescription {
                    name: "payload".to_string(),
                    description: "实际载荷数据".to_string(),
                    offset: Some(10),
                    length: None,
                },
            ],
            builtin: true,
        },
        // 4. HTTP-like 文本协议
        ProtocolTemplate {
            id: "http_like".to_string(),
            name: "HTTP-like".to_string(),
            description: "类 HTTP 文本协议，使用 \\r\\n\\r\\n 分隔头部和正文".to_string(),
            category: "text".to_string(),
            framing_config: serde_json::json!({
                "mode": { "delimiter": { "delimiter": "\r\n\r\n" } },
                "max_frame_size": 1048576,
                "include_framing_in_data": false
            }),
            parser_config: Some(ParserConfig::Delimiter {
                delimiter_hex: "0d0a".to_string(),
                keep_delimiter: false,
            }),
            example_data_hex: Some("474554202f7061746820485454502f312e310d0a486f73743a206578616d706c652e636f6d0d0a0d0a".to_string()),
            // GET /path HTTP/1.1\r\nHost: example.com\r\n\r\n
            field_descriptions: vec![
                FieldDescription {
                    name: "request_line".to_string(),
                    description: "请求行：方法 + 路径 + 版本".to_string(),
                    offset: Some(0),
                    length: None,
                },
                FieldDescription {
                    name: "headers".to_string(),
                    description: "头部字段，每行一个 Key: Value 对，以 \\r\\n 结尾".to_string(),
                    offset: None,
                    length: None,
                },
                FieldDescription {
                    name: "body".to_string(),
                    description: "正文内容（如存在），位于 \\r\\n\\r\\n 之后".to_string(),
                    offset: None,
                    length: None,
                },
            ],
            builtin: true,
        },
        // 5. 空白模板
        ProtocolTemplate {
            id: "blank".to_string(),
            name: "Blank".to_string(),
            description: "空白模板，所有参数均为默认值，用户可自由定制".to_string(),
            category: "custom".to_string(),
            framing_config: serde_json::json!({
                "mode": { "length_prefix": { "prefix_bytes": 4 } },
                "max_frame_size": 1048576,
                "include_framing_in_data": false
            }),
            parser_config: None,
            example_data_hex: None,
            field_descriptions: vec![],
            builtin: true,
        },
    ]
}

/// 根据 ID 获取模板
pub fn get_template(id: &str) -> Option<ProtocolTemplate> {
    builtin_templates().into_iter().find(|t| t.id == id)
}

/// 从 JSON 加载模板
pub fn load_template(json: &str) -> Result<ProtocolTemplate, String> {
    serde_json::from_str(json).map_err(|e| format!("模板 JSON 解析失败: {}", e))
}

/// 导出模板为 JSON
pub fn export_template(template: &ProtocolTemplate) -> Result<String, String> {
    serde_json::to_string_pretty(template).map_err(|e| format!("模板 JSON 序列化失败: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    // === builtin_templates 测试 ===

    #[test]
    fn test_builtin_templates_count() {
        let templates = builtin_templates();
        assert!(templates.len() >= 3, "至少应有 3 个内置模板，实际 {}", templates.len());
        assert!(templates.len() <= 5, "至多应有 5 个内置模板，实际 {}", templates.len());
    }

    #[test]
    fn test_builtin_templates_all_have_valid_config() {
        for template in builtin_templates() {
            // 每个模板应有有效的 framing_config
            assert!(
                template.framing_config.is_object(),
                "模板 {} 的 framing_config 应为 JSON 对象",
                template.id
            );
            assert!(
                template.framing_config.get("mode").is_some(),
                "模板 {} 的 framing_config 应包含 mode 字段",
                template.id
            );
        }
    }

    #[test]
    fn test_builtin_templates_all_builtin_flag() {
        for template in builtin_templates() {
            assert!(template.builtin, "内置模板 {} 的 builtin 应为 true", template.id);
        }
    }

    #[test]
    fn test_builtin_templates_unique_ids() {
        let templates = builtin_templates();
        let mut ids = std::collections::HashSet::new();
        for t in &templates {
            assert!(ids.insert(&t.id), "模板 ID {} 重复", t.id);
        }
    }

    #[test]
    fn test_builtin_templates_have_non_empty_fields() {
        for template in builtin_templates() {
            if template.id == "blank" {
                continue; // 空白模板允许无字段说明
            }
            assert!(
                !template.id.is_empty(),
                "模板 ID 不应为空"
            );
            assert!(
                !template.name.is_empty(),
                "模板 {} 名称不应为空",
                template.id
            );
            assert!(
                !template.description.is_empty(),
                "模板 {} 描述不应为空",
                template.id
            );
            assert!(
                !template.category.is_empty(),
                "模板 {} 分类不应为空",
                template.id
            );
        }
    }

    // === get_template 测试 ===

    #[test]
    fn test_get_template_redis_resp() {
        let t = get_template("redis_resp").expect("应找到 redis_resp 模板");
        assert_eq!(t.name, "Redis RESP");
        assert_eq!(t.category, "cache");
        assert!(t.parser_config.is_some());
    }

    #[test]
    fn test_get_template_custom_tlv() {
        let t = get_template("custom_tlv").expect("应找到 custom_tlv 模板");
        assert_eq!(t.name, "Custom TLV");
        assert_eq!(t.category, "binary");
        assert!(t.parser_config.is_some());
    }

    #[test]
    fn test_get_template_fixed_header() {
        let t = get_template("fixed_header").expect("应找到 fixed_header 模板");
        assert_eq!(t.name, "Fixed Header");
        assert!(t.parser_config.is_some());
    }

    #[test]
    fn test_get_template_http_like() {
        let t = get_template("http_like").expect("应找到 http_like 模板");
        assert_eq!(t.name, "HTTP-like");
        assert!(t.parser_config.is_some());
    }

    #[test]
    fn test_get_template_blank() {
        let t = get_template("blank").expect("应找到 blank 模板");
        assert_eq!(t.name, "Blank");
        assert!(t.parser_config.is_none());
        assert!(t.example_data_hex.is_none());
        assert!(t.field_descriptions.is_empty());
    }

    #[test]
    fn test_get_template_not_found() {
        assert!(get_template("nonexistent").is_none());
    }

    // === 序列化/反序列化往返测试 ===

    #[test]
    fn test_roundtrip_redis_resp() {
        let original = get_template("redis_resp").expect("应找到模板");
        let json = export_template(&original).expect("导出应成功");
        let restored = load_template(&json).expect("加载应成功");
        assert_eq!(original, restored);
    }

    #[test]
    fn test_roundtrip_custom_tlv() {
        let original = get_template("custom_tlv").expect("应找到模板");
        let json = export_template(&original).expect("导出应成功");
        let restored = load_template(&json).expect("加载应成功");
        assert_eq!(original, restored);
    }

    #[test]
    fn test_roundtrip_fixed_header() {
        let original = get_template("fixed_header").expect("应找到模板");
        let json = export_template(&original).expect("导出应成功");
        let restored = load_template(&json).expect("加载应成功");
        assert_eq!(original, restored);
    }

    #[test]
    fn test_roundtrip_http_like() {
        let original = get_template("http_like").expect("应找到模板");
        let json = export_template(&original).expect("导出应成功");
        let restored = load_template(&json).expect("加载应成功");
        assert_eq!(original, restored);
    }

    #[test]
    fn test_roundtrip_blank() {
        let original = get_template("blank").expect("应找到模板");
        let json = export_template(&original).expect("导出应成功");
        let restored = load_template(&json).expect("加载应成功");
        assert_eq!(original, restored);
    }

    // === load_template 错误处理 ===

    #[test]
    fn test_load_template_invalid_json() {
        let result = load_template("not json at all");
        assert!(result.is_err());
        let err = result.err().expect("应返回错误");
        assert!(err.contains("解析失败"), "错误信息应包含'解析失败'，实际: {}", err);
    }

    #[test]
    fn test_load_template_missing_fields() {
        let result = load_template("{}");
        assert!(result.is_err());
    }

    // === export_template 验证 ===

    #[test]
    fn test_export_template_is_valid_json() {
        for template in builtin_templates() {
            let json = export_template(&template).expect("导出应成功");
            // 验证导出的 JSON 可以重新解析
            let reparsed: serde_json::Value =
                serde_json::from_str(&json).expect("导出结果应为有效 JSON");
            assert!(reparsed.is_object());
            assert_eq!(reparsed["id"], template.id);
        }
    }

    #[test]
    fn test_export_template_contains_expected_keys() {
        let t = get_template("redis_resp").expect("应找到模板");
        let json = export_template(&t).expect("导出应成功");
        let val: serde_json::Value = serde_json::from_str(&json).expect("应有效 JSON");

        assert!(val.get("id").is_some());
        assert!(val.get("name").is_some());
        assert!(val.get("description").is_some());
        assert!(val.get("category").is_some());
        assert!(val.get("framing_config").is_some());
        assert!(val.get("parser_config").is_some());
        assert!(val.get("example_data_hex").is_some());
        assert!(val.get("field_descriptions").is_some());
        assert!(val.get("builtin").is_some());
    }

    // === FieldDescription 测试 ===

    #[test]
    fn test_field_description_serialize_roundtrip() {
        let field = FieldDescription {
            name: "test_field".to_string(),
            description: "测试字段".to_string(),
            offset: Some(4),
            length: Some(2),
        };
        let json = serde_json::to_string(&field).expect("序列化应成功");
        let restored: FieldDescription = serde_json::from_str(&json).expect("反序列化应成功");
        assert_eq!(field, restored);
    }

    #[test]
    fn test_field_description_optional_fields() {
        // offset 和 length 可为 None
        let field = FieldDescription {
            name: "variable_len".to_string(),
            description: "变长字段".to_string(),
            offset: Some(0),
            length: None,
        };
        let json = serde_json::to_string(&field).expect("序列化应成功");
        let restored: FieldDescription = serde_json::from_str(&json).expect("反序列化应成功");
        assert_eq!(field, restored);
        assert!(restored.length.is_none());
    }
}
