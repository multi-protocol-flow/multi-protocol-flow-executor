//! 产物捕获
//!
//! 与 `docs/media-preview.md` §3.2 的 V1 契约完全同构，因此前端 `MediaPreview`
//! **零改动**即可渲染 `local:exec` 的产物：
//!
//! ```jsonc
//! { "kind": "video", "mime": "video/mp4", "bytes": 662150, "data_url": "data:video/mp4;base64,…" }
//! ```
//!
//! 超过 `max_bytes` 时不读取全文，只产出 `{ kind, mime, bytes, omitted: true }`。

use base64::Engine;
use flow_engine_core::error::{ErrorCode, ErrorDetail, ErrorKind};
use std::io::Read;
use std::path::Path;

/// MIME 判定所需的 magic 字节数
const MAGIC_PROBE_BYTES: usize = 64;

/// 读取产物并生成 `media` 契约
pub fn capture_media(path: &Path, max_bytes: usize) -> Result<serde_json::Value, ErrorDetail> {
    let metadata = std::fs::metadata(path).map_err(|e| capture_error(path, e.to_string()))?;
    if !metadata.is_file() {
        return Err(capture_error(path, "not a regular file".to_string()));
    }
    let size = metadata.len();

    let head = read_head(path)?;
    let (kind, mime) = detect_media(path, &head)
        .ok_or_else(|| capture_error(path, "unrecognized media format".to_string()))?;

    if size > max_bytes as u64 {
        return Ok(serde_json::json!({
            "kind": kind,
            "mime": mime,
            "bytes": size,
            "omitted": true,
        }));
    }

    let bytes = read_all(path)?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(serde_json::json!({
        "kind": kind,
        "mime": mime,
        "bytes": bytes.len(),
        "data_url": format!("data:{mime};base64,{encoded}"),
    }))
}

fn capture_error(path: &Path, message: String) -> ErrorDetail {
    let rendered = format!("cannot read artifact `{}`: {message}", path.display());
    ErrorDetail::new(ErrorKind::Execution, rendered.clone())
        .with_error_code_typed(ErrorCode::E_EXEC_CAPTURE_FAILED)
        .with_i18n_arg("path", path.display().to_string())
        .with_i18n_arg("message", rendered)
}

fn read_head(path: &Path) -> Result<Vec<u8>, ErrorDetail> {
    let mut file = std::fs::File::open(path).map_err(|e| capture_error(path, e.to_string()))?;
    let mut head = vec![0u8; MAGIC_PROBE_BYTES];
    let read = file
        .read(&mut head)
        .map_err(|e| capture_error(path, e.to_string()))?;
    head.truncate(read);
    Ok(head)
}

fn read_all(path: &Path) -> Result<Vec<u8>, ErrorDetail> {
    let mut file = std::fs::File::open(path).map_err(|e| capture_error(path, e.to_string()))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|e| capture_error(path, e.to_string()))?;
    Ok(bytes)
}

/// MIME 判定：扩展名 + magic 双重判定，冲突时以 magic 为准
pub fn detect_media(path: &Path, head: &[u8]) -> Option<(&'static str, &'static str)> {
    if let Some(mime) = detect_by_magic(path, head) {
        return Some((kind_of(mime), mime));
    }
    detect_by_extension(path).map(|mime| (kind_of(mime), mime))
}

fn kind_of(mime: &str) -> &'static str {
    if mime.starts_with("image/") {
        "image"
    } else {
        "video"
    }
}

fn starts_with(head: &[u8], magic: &[u8]) -> bool {
    head.len() >= magic.len() && &head[..magic.len()] == magic
}

fn detect_by_magic(path: &Path, head: &[u8]) -> Option<&'static str> {
    if starts_with(head, &[0x89, 0x50, 0x4E, 0x47]) {
        return Some("image/png");
    }
    if starts_with(head, &[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    if starts_with(head, b"GIF8") {
        return Some("image/gif");
    }
    // RIFF....WEBP
    if starts_with(head, b"RIFF") && head.len() >= 12 && &head[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    if starts_with(head, b"BM") {
        return Some("image/bmp");
    }
    // ISO BMFF：偏移 4 处为 `ftyp`
    if head.len() >= 12 && &head[4..8] == b"ftyp" {
        let brand = &head[8..12];
        if brand == b"qt  " {
            return Some("video/quicktime");
        }
        return Some("video/mp4");
    }
    // EBML（WebM / Matroska）——靠扩展名区分
    if starts_with(head, &[0x1A, 0x45, 0xDF, 0xA3]) {
        let is_mkv = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("mkv"));
        return Some(if is_mkv {
            "video/x-matroska"
        } else {
            "video/webm"
        });
    }
    None
}

fn detect_by_extension(path: &Path) -> Option<&'static str> {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())?
        .to_ascii_lowercase();
    match ext.as_str() {
        "mp4" | "m4v" => Some("video/mp4"),
        "webm" => Some("video/webm"),
        "mov" => Some("video/quicktime"),
        "mkv" => Some("video/x-matroska"),
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "bmp" => Some("image/bmp"),
        "avif" => Some("image/avif"),
        "svg" => Some("image/svg+xml"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).unwrap_or_else(|e| panic!("{e}"));
        file.write_all(bytes).unwrap_or_else(|e| panic!("{e}"));
        path
    }

    #[test]
    fn detects_png_and_inlines() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let mut bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        bytes.extend_from_slice(&[0u8; 16]);
        let path = write_file(dir.path(), "a.png", &bytes);
        let media = capture_media(&path, 1024).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(media["kind"], serde_json::json!("image"));
        assert_eq!(media["mime"], serde_json::json!("image/png"));
        assert!(media["data_url"]
            .as_str()
            .is_some_and(|u| u.starts_with("data:image/png;base64,")));
        assert!(media.get("omitted").is_none());
    }

    #[test]
    fn magic_wins_over_extension() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let mut bytes = vec![0u8; 8];
        bytes[4..8].copy_from_slice(b"ftyp");
        bytes.extend_from_slice(b"isom");
        bytes.extend_from_slice(&[0u8; 8]);
        let path = write_file(dir.path(), "mislabeled.png", &bytes);
        let media = capture_media(&path, 1024).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(media["mime"], serde_json::json!("video/mp4"));
        assert_eq!(media["kind"], serde_json::json!("video"));
    }

    #[test]
    fn oversize_file_is_omitted() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let mut bytes = vec![0u8; 8];
        bytes[4..8].copy_from_slice(b"ftyp");
        bytes.extend_from_slice(b"isom");
        bytes.resize(5 * 1024 * 1024, 0);
        let path = write_file(dir.path(), "big.mp4", &bytes);
        let media = capture_media(&path, 4 * 1024 * 1024).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(media["omitted"], serde_json::json!(true));
        assert_eq!(media["bytes"], serde_json::json!(5 * 1024 * 1024));
        assert!(media.get("data_url").is_none());
    }

    #[test]
    fn missing_file_is_capture_error() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let err =
            capture_media(&dir.path().join("nope.mp4"), 1024).expect_err("missing file must fail");
        assert_eq!(err.error_code, Some(ErrorCode::E_EXEC_CAPTURE_FAILED));
    }

    #[test]
    fn unknown_format_is_capture_error() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let path = write_file(dir.path(), "a.bin", b"not media at all");
        let err = capture_media(&path, 1024).expect_err("unknown format must fail");
        assert_eq!(err.error_code, Some(ErrorCode::E_EXEC_CAPTURE_FAILED));
    }

    #[test]
    fn extension_fallback_detects_svg() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let path = write_file(
            dir.path(),
            "a.svg",
            b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
        );
        let media = capture_media(&path, 1024).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(media["mime"], serde_json::json!("image/svg+xml"));
    }
}
