//! 响应媒体的类型判定与内联载荷生成。
//!
//! 职责：当 HTTP 响应的 `Content-Type` 为 `image/*` 或 `video/*` 时，把二进制响应体
//! 转换为可直接用于 `<img>` / `<video>` 的 `data:` URL，避免走 `String::from_utf8`
//! 变成 `"Invalid UTF-8: ..."`。纯函数，无 IO。

use base64::Engine;

/// 内联为 `data:` URL 的最大响应体字节数。
pub const MEDIA_INLINE_MAX_BYTES: usize = 4 * 1024 * 1024;

/// 从响应体中抽取的媒体载荷。
#[derive(Debug, Clone, PartialEq)]
pub struct MediaPayload {
    /// `"image"` 或 `"video"`
    pub kind: &'static str,
    /// 规范化后的 MIME，例如 `image/png`
    pub mime: String,
    /// 响应体字节数
    pub bytes: usize,
    /// 超限时为 `None`
    pub data_url: Option<String>,
    /// 是否因超限而未内联
    pub omitted: bool,
    /// 写回 `output_data.body` 的占位文本
    pub placeholder: String,
}

/// 人类可读的字节数文案。
fn human_size(bytes: usize) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} bytes", bytes)
    }
}

/// `Content-Type` 为 `image/*` 或 `video/*` 时返回载荷，否则 `None`。
///
/// # 参数
///
/// * `content_type` - 响应的 `Content-Type` 头
/// * `body` - 响应体原始字节
///
/// # 返回
///
/// 媒体载荷；非图片/视频时为 `None`
pub fn detect_media(content_type: Option<&str>, body: &[u8]) -> Option<MediaPayload> {
    let mime = content_type
        .map(|ct| ct.split(';').next().unwrap_or(""))
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();

    let kind = if mime.starts_with("image/") {
        "image"
    } else if mime.starts_with("video/") {
        "video"
    } else {
        return None;
    };

    let bytes = body.len();

    if bytes <= MEDIA_INLINE_MAX_BYTES {
        Some(MediaPayload {
            kind,
            mime: mime.clone(),
            bytes,
            data_url: Some(format!(
                "data:{};base64,{}",
                mime,
                base64::engine::general_purpose::STANDARD.encode(body)
            )),
            omitted: false,
            placeholder: format!("[{} {} bytes]", mime, bytes),
        })
    } else {
        Some(MediaPayload {
            kind,
            mime: mime.clone(),
            bytes,
            data_url: None,
            omitted: true,
            placeholder: format!(
                "[{} {} — exceeds 4 MB preview limit]",
                mime,
                human_size(bytes)
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_image_inline() {
        let payload = detect_media(Some("image/png"), &[0x89, b'P', b'N', b'G'])
            .expect("image/png 应被判定为媒体");
        assert_eq!(payload.kind, "image");
        assert_eq!(payload.mime, "image/png");
        assert_eq!(payload.bytes, 4);
        assert!(!payload.omitted);
        let url = payload.data_url.expect("未超限时应内联");
        assert!(url.starts_with("data:image/png;base64,"));
        assert_eq!(payload.placeholder, "[image/png 4 bytes]");
    }

    #[test]
    fn detect_image_with_content_type_params() {
        let payload = detect_media(Some("image/png; charset=binary"), b"x")
            .expect("带参数的 content-type 应被识别");
        assert_eq!(payload.mime, "image/png");
        assert_eq!(payload.kind, "image");
        assert_eq!(payload.bytes, 1);
        assert!(!payload.omitted);
        assert!(payload
            .data_url
            .expect("未超限时应内联")
            .starts_with("data:image/png;base64,"));
    }

    #[test]
    fn detect_video_over_limit_omitted() {
        let body = vec![0u8; MEDIA_INLINE_MAX_BYTES + 1];
        let payload = detect_media(Some("video/mp4"), &body).expect("video/mp4 应被判定为媒体");
        assert_eq!(payload.kind, "video");
        assert!(payload.omitted);
        assert!(payload.data_url.is_none());
        assert!(payload.placeholder.contains("exceeds 4 MB preview limit"));
        assert!(payload.placeholder.starts_with("[video/mp4 4.0 MB"));
    }

    #[test]
    fn detect_non_media_returns_none() {
        assert!(detect_media(Some("application/json"), b"{}").is_none());
        assert!(detect_media(None, b"x").is_none());
        assert!(detect_media(Some("text/plain"), b"x").is_none());
    }
}
