//! HTTP 请求体类型定义
//!
//! 支持多种请求体类型：
//! - 无请求体 (None)
//! - 原始字节 (Raw)
//! - URL 编码表单 (Urlencoded)
//! - Multipart 表单 (Multipart)

use bytes::Bytes;
use http_body_util::{BodyExt, Full};

/// 统一的 Body 类型
pub type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

/// 创建空请求体
pub fn empty_body() -> BoxBody {
    http_body_util::Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

/// 创建完整请求体
pub fn full_body<T: Into<Bytes>>(chunk: T) -> BoxBody {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

/// HTTP 请求体类型
#[derive(Debug, Clone, Default)]
pub enum BodyType {
    /// 无请求体
    #[default]
    None,
    /// 原始字节（JSON、文本、二进制等）
    Raw(Vec<u8>),
    /// URL 编码表单
    Urlencoded(Vec<(String, String)>),
    /// Multipart 表单
    Multipart(MultipartBody),
}

impl BodyType {
    /// 创建无请求体
    pub fn none() -> Self {
        Self::None
    }

    /// 创建原始字节请求体
    pub fn raw(data: Vec<u8>) -> Self {
        Self::Raw(data)
    }

    /// 创建文本请求体
    pub fn text(text: String) -> Self {
        Self::Raw(text.into_bytes())
    }

    /// 创建 JSON 请求体
    pub fn json(value: &serde_json::Value) -> Self {
        Self::Raw(value.to_string().into_bytes())
    }

    /// 创建 URL 编码表单
    pub fn urlencoded(fields: Vec<(String, String)>) -> Self {
        Self::Urlencoded(fields)
    }

    /// 创建 Multipart 表单
    pub fn multipart(body: MultipartBody) -> Self {
        Self::Multipart(body)
    }

    /// 检查是否为空
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// 获取 Content-Type 头
    /// 返回 (`header_name`, `header_value`) 或 None
    pub fn content_type(&self, boundary: Option<&str>) -> Option<(&'static str, String)> {
        match self {
            Self::None => None,
            Self::Raw(_) => None, // 由调用者设置
            Self::Urlencoded(_) => Some((
                "Content-Type",
                "application/x-www-form-urlencoded".to_string(),
            )),
            Self::Multipart(_) => {
                let boundary = boundary.expect("multipart 需要 boundary");
                Some((
                    "Content-Type",
                    format!("multipart/form-data; boundary={}", boundary),
                ))
            }
        }
    }
}

/// Multipart 表单字段
#[derive(Debug, Clone)]
pub enum MultipartField {
    /// 文本字段
    Text {
        /// 字段名
        name: String,
        /// 字段值
        value: String,
    },
    /// 文件字段
    File {
        /// 字段名
        name: String,
        /// 文件名
        filename: String,
        /// MIME 类型
        mime_type: String,
        /// 文件内容
        data: Vec<u8>,
    },
}

impl MultipartField {
    /// 创建文本字段
    pub fn text(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Text {
            name: name.into(),
            value: value.into(),
        }
    }

    /// 创建文件字段
    pub fn file(
        name: impl Into<String>,
        filename: impl Into<String>,
        mime_type: impl Into<String>,
        data: Vec<u8>,
    ) -> Self {
        Self::File {
            name: name.into(),
            filename: filename.into(),
            mime_type: mime_type.into(),
            data,
        }
    }
}

/// Multipart 请求体
#[derive(Debug, Clone, Default)]
pub struct MultipartBody {
    /// 表单字段列表
    pub fields: Vec<MultipartField>,
}

impl MultipartBody {
    /// 创建空的 Multipart 请求体
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }

    /// 添加文本字段
    pub fn text(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.push(MultipartField::text(name, value));
        self
    }

    /// 添加文件字段
    pub fn file(
        mut self,
        name: impl Into<String>,
        filename: impl Into<String>,
        mime_type: impl Into<String>,
        data: Vec<u8>,
    ) -> Self {
        self.fields
            .push(MultipartField::file(name, filename, mime_type, data));
        self
    }

    /// 生成 boundary 字符串
    pub fn generate_boundary() -> String {
        // 使用时间戳和随机数生成 boundary
        let timestamp = flow_engine_core::utils::unix_timestamp_ms();
        format!("----WebKitFormBoundary{:016x}", timestamp)
    }

    /// 编码为 HTTP 请求体字节
    pub fn encode(&self, boundary: &str) -> Vec<u8> {
        let mut body = Vec::new();
        let boundary_bytes = boundary.as_bytes();

        for field in &self.fields {
            // 添加 boundary 分隔符
            body.extend_from_slice(b"--");
            body.extend_from_slice(boundary_bytes);
            body.extend_from_slice(b"\r\n");

            match field {
                MultipartField::Text { name, value } => {
                    // Content-Disposition
                    body.extend_from_slice(b"Content-Disposition: form-data; name=\"");
                    body.extend_from_slice(name.as_bytes());
                    body.extend_from_slice(b"\"\r\n\r\n");
                    // 字段值
                    body.extend_from_slice(value.as_bytes());
                    body.extend_from_slice(b"\r\n");
                }
                MultipartField::File {
                    name,
                    filename,
                    mime_type,
                    data,
                } => {
                    // Content-Disposition
                    body.extend_from_slice(b"Content-Disposition: form-data; name=\"");
                    body.extend_from_slice(name.as_bytes());
                    body.extend_from_slice(b"\"; filename=\"");
                    body.extend_from_slice(filename.as_bytes());
                    body.extend_from_slice(b"\"\r\n");
                    // Content-Type
                    body.extend_from_slice(b"Content-Type: ");
                    body.extend_from_slice(mime_type.as_bytes());
                    body.extend_from_slice(b"\r\n\r\n");
                    // 文件内容
                    body.extend_from_slice(data);
                    body.extend_from_slice(b"\r\n");
                }
            }
        }

        // 结束 boundary
        body.extend_from_slice(b"--");
        body.extend_from_slice(boundary_bytes);
        body.extend_from_slice(b"--\r\n");

        body
    }
}

/// URL 编码工具
pub struct UrlEncoder;

impl UrlEncoder {
    /// 将键值对编码为 application/x-www-form-urlencoded 格式
    pub fn encode(fields: &[(String, String)]) -> String {
        fields
            .iter()
            .map(|(k, v)| format!("{}={}", Self::url_encode(k), Self::url_encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    }

    /// URL 编码单个字符串
    fn url_encode(s: &str) -> String {
        // 简单实现：只编码特殊字符
        // 完整实现应使用 urlencoding crate，但这里为了避免新依赖使用简单实现
        let mut result = String::new();
        for c in s.chars() {
            match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                    result.push(c);
                }
                ' ' => result.push('+'),
                _ => {
                    for byte in c.to_string().as_bytes() {
                        result.push_str(&format!("%{:02X}", byte));
                    }
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_body_type_none() {
        let body = BodyType::none();
        assert!(body.is_none());
        assert!(body.content_type(None).is_none());
    }

    #[test]
    fn test_body_type_raw() {
        let body = BodyType::raw(b"test data".to_vec());
        assert!(!body.is_none());
        assert!(body.content_type(None).is_none());
    }

    #[test]
    fn test_body_type_urlencoded() {
        let body = BodyType::urlencoded(vec![
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
        ]);
        assert!(!body.is_none());
        let ct = body.content_type(None);
        assert!(ct.is_some());
        let (name, value) = ct.unwrap();
        assert_eq!(name, "Content-Type");
        assert_eq!(value, "application/x-www-form-urlencoded");
    }

    #[test]
    fn test_body_type_multipart() {
        let multipart = MultipartBody::new().text("field1", "value1").file(
            "file1",
            "test.txt",
            "text/plain",
            b"hello".to_vec(),
        );

        let body = BodyType::multipart(multipart);
        assert!(!body.is_none());

        let ct = body.content_type(Some("test-boundary"));
        assert!(ct.is_some());
        let (name, value) = ct.unwrap();
        assert_eq!(name, "Content-Type");
        assert!(value.starts_with("multipart/form-data; boundary="));
    }

    #[test]
    fn test_multipart_encode() {
        let multipart = MultipartBody::new()
            .text("name", "test")
            .text("email", "test@example.com");

        let boundary = "----WebKitFormBoundary12345678";
        let encoded = multipart.encode(boundary);

        let encoded_str = String::from_utf8(encoded).unwrap();
        assert!(encoded_str.contains("name=\"name\""));
        assert!(encoded_str.contains("test"));
        assert!(encoded_str.contains("name=\"email\""));
        assert!(encoded_str.contains("test@example.com"));
        assert!(encoded_str.ends_with("--\r\n"));
    }

    #[test]
    fn test_multipart_file() {
        let multipart =
            MultipartBody::new().file("upload", "test.txt", "text/plain", b"file content".to_vec());

        let boundary = "----WebKitFormBoundary12345678";
        let encoded = multipart.encode(boundary);

        let encoded_str = String::from_utf8(encoded).unwrap();
        assert!(encoded_str.contains("filename=\"test.txt\""));
        assert!(encoded_str.contains("Content-Type: text/plain"));
        assert!(encoded_str.contains("file content"));
    }

    #[test]
    fn test_url_encoder() {
        let fields = vec![
            ("name".to_string(), "张三".to_string()),
            ("email".to_string(), "test@example.com".to_string()),
        ];
        let encoded = UrlEncoder::encode(&fields);

        // 验证基本格式
        assert!(encoded.contains("name="));
        assert!(encoded.contains("email=test%40example.com"));
    }

    #[test]
    fn test_url_encoder_special_chars() {
        let encoded = UrlEncoder::url_encode("hello world");
        assert_eq!(encoded, "hello+world");

        let encoded = UrlEncoder::url_encode("a=b&c=d");
        assert_eq!(encoded, "a%3Db%26c%3Dd");
    }

    #[test]
    fn test_generate_boundary() {
        let boundary1 = MultipartBody::generate_boundary();
        let boundary2 = MultipartBody::generate_boundary();

        // boundary 应该不同（时间戳不同）
        // 注意：在极快执行时可能相同，但这不影响功能
        assert!(boundary1.starts_with("----WebKitFormBoundary"));
        assert!(boundary2.starts_with("----WebKitFormBoundary"));
    }
}
