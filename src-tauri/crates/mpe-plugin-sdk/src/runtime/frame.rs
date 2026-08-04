//! Line framing over stdio (private to the runtime).
//!
//! One JSON value per line, `LF`-delimited (`mpe_protocol::LINE_DELIMITER`).
//! The reader tolerates a trailing `CR` (CRLF, as emitted by Python's
//! `print()`), validates byte-exact UTF-8, tolerates a leading BOM on the
//! first frame, and enforces the `2 MiB` line cap
//! (`mpe_protocol::MAX_LINE_BYTES`) — it never buffers without bound.
//!
//! This mirrors the host-side framing in `flow-engine-plugin`; the SDK cannot
//! depend on that crate (leaf-crate rule), so the minimal reader/writer is
//! duplicated here. A future refactor may unify framing into `mpe-protocol`.

use std::io;

use mpe_protocol::{LINE_DELIMITER, MAX_LINE_BYTES};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Size of the chunk the reader pulls from the underlying stream at a time.
const READ_CHUNK: usize = 8192;

/// Framing-layer errors: transport violations, distinct from protocol errors.
#[derive(Debug)]
pub enum FrameError {
    /// Underlying stream I/O failed.
    Io(io::Error),
    /// A line exceeded [`MAX_LINE_BYTES`](mpe_protocol::MAX_LINE_BYTES).
    LineTooLong,
    /// A line is not byte-exact UTF-8.
    InvalidUtf8,
    /// The stream reached EOF in the middle of a partial line.
    UnexpectedEof,
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Io(err) => write!(f, "frame io error: {err}"),
            FrameError::LineTooLong => {
                write!(f, "line exceeds maximum length of {MAX_LINE_BYTES} bytes")
            }
            FrameError::InvalidUtf8 => write!(f, "line is not valid UTF-8"),
            FrameError::UnexpectedEof => write!(f, "unexpected EOF in the middle of a line"),
        }
    }
}

impl std::error::Error for FrameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FrameError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for FrameError {
    fn from(err: io::Error) -> Self {
        FrameError::Io(err)
    }
}

/// Bounded `LF`-delimited frame reader.
///
/// Pulls chunks from the underlying stream, scans for the delimiter, and
/// enforces the max-line cap *while reading* — never via an unbounded
/// `read_line` that could buffer arbitrarily before the cap is checked.
pub struct FrameReader<R> {
    inner: R,
    /// Max accepted line length in bytes.
    max_line: usize,
    /// Whether we have consumed the leading BOM (first frame only).
    bom_consumed: bool,
    /// Whether EOF has been observed.
    eof: bool,
    /// Bytes read but not yet delimited into a frame; persisted across
    /// `next_frame` calls so a partial frame split across reads is never lost.
    buf: Vec<u8>,
}

impl<R: AsyncRead + Unpin> FrameReader<R> {
    /// Creates a reader with the protocol default [`MAX_LINE_BYTES`] cap.
    pub fn new(inner: R) -> Self {
        Self::with_max_line(inner, MAX_LINE_BYTES)
    }

    /// Creates a reader with a custom max-line cap (tests use a small cap).
    pub fn with_max_line(inner: R, max_line: usize) -> Self {
        Self {
            inner,
            max_line,
            bom_consumed: false,
            eof: false,
            buf: Vec::new(),
        }
    }

    /// Reads the next frame.
    ///
    /// - `Ok(Some(frame))` — a complete, UTF-8-valid, CR-stripped frame
    ///   (BOM stripped on the first frame only).
    /// - `Ok(None)` — clean EOF between frames (no partial line pending).
    /// - `Err(FrameError::UnexpectedEof)` — EOF in the middle of a line.
    ///
    /// Never returns a frame longer than the configured cap, and never
    /// buffers without bound (the cap is enforced while reading).
    pub async fn next_frame(&mut self) -> Result<Option<Vec<u8>>, FrameError> {
        loop {
            // A complete line (including delimiter) already buffered?
            if let Some(pos) = self.buf.iter().position(|&b| b == LINE_DELIMITER) {
                let mut line: Vec<u8> = self.buf.drain(..=pos).collect();
                line.pop(); // remove the LF delimiter
                if line.len() > self.max_line {
                    return Err(FrameError::LineTooLong);
                }
                strip_trailing_cr(&mut line);
                self.strip_bom(&mut line);
                if std::str::from_utf8(&line).is_err() {
                    return Err(FrameError::InvalidUtf8);
                }
                return Ok(Some(line));
            }
            if self.eof {
                if self.buf.is_empty() {
                    return Ok(None);
                }
                return Err(FrameError::UnexpectedEof);
            }
            // No delimiter yet: pull more bytes. The cap is enforced before
            // the buffer can grow past `max_line` without a delimiter.
            let mut chunk = [0u8; READ_CHUNK];
            let n = self.inner.read(&mut chunk).await?;
            if n == 0 {
                self.eof = true;
                continue;
            }
            self.buf.extend_from_slice(&chunk[..n]);
            // Enforce the cap only when no terminator is buffered — a buffer
            // holding several short lines may legitimately exceed `max_line`
            // until the first one is delimited.
            if !self.buf.contains(&LINE_DELIMITER) && self.buf.len() > self.max_line {
                return Err(FrameError::LineTooLong);
            }
        }
    }

    /// Strips a UTF-8 BOM (`EF BB BF`) from the first frame of the stream.
    fn strip_bom(&mut self, line: &mut Vec<u8>) {
        if self.bom_consumed {
            return;
        }
        self.bom_consumed = true;
        if line.len() >= 3 && line[..3] == [0xEF, 0xBB, 0xBF] {
            line.drain(..3);
        }
    }
}

/// Removes a single trailing `\r`, tolerating CRLF output (Python `print()`).
fn strip_trailing_cr(line: &mut Vec<u8>) {
    if line.last() == Some(&b'\r') {
        line.pop();
    }
}

/// Writes one complete frame (payload + delimiter) and flushes.
///
/// The payload must be pre-serialized and UTF-8-valid; the framing layer
/// does not re-validate (the writer is the hot path). Atomicity is provided
/// by construction: the runtime's single writer task is the only caller.
pub async fn write_frame<W: AsyncWrite + Unpin>(writer: &mut W, payload: &[u8]) -> io::Result<()> {
    writer.write_all(payload).await?;
    writer.write_all(&[LINE_DELIMITER]).await?;
    writer.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In-memory duplex pair simulating a stdio pipe.
    fn duplex() -> (tokio::io::DuplexStream, tokio::io::DuplexStream) {
        tokio::io::duplex(64 * 1024)
    }

    #[tokio::test]
    async fn frame_reader_parses_lf_json() -> Result<(), Box<dyn std::error::Error>> {
        let (mut tx, rx) = duplex();
        tx.write_all(b"{\"a\":1}\n{\"b\":2}\n").await?;
        drop(tx); // half-close after payload
        let mut reader = FrameReader::new(rx);
        assert_eq!(reader.next_frame().await?, Some(b"{\"a\":1}".to_vec()));
        assert_eq!(reader.next_frame().await?, Some(b"{\"b\":2}".to_vec()));
        assert_eq!(reader.next_frame().await?, None);
        Ok(())
    }

    #[tokio::test]
    async fn frame_reader_strips_crlf() -> Result<(), Box<dyn std::error::Error>> {
        let (mut tx, rx) = duplex();
        // CRLF as emitted by Python `print()`.
        tx.write_all(b"{\"a\":1}\r\n{\"b\":2}\r\n").await?;
        drop(tx);
        let mut reader = FrameReader::new(rx);
        assert_eq!(reader.next_frame().await?, Some(b"{\"a\":1}".to_vec()));
        assert_eq!(reader.next_frame().await?, Some(b"{\"b\":2}".to_vec()));
        assert_eq!(reader.next_frame().await?, None);
        Ok(())
    }

    #[tokio::test]
    async fn frame_reader_rejects_invalid_utf8() -> Result<(), Box<dyn std::error::Error>> {
        let (mut tx, rx) = duplex();
        tx.write_all(b"{\"a\":\"\xff\"}\n").await?; // 0xFF is not valid UTF-8
        drop(tx);
        let mut reader = FrameReader::new(rx);
        assert!(matches!(
            reader.next_frame().await,
            Err(FrameError::InvalidUtf8)
        ));
        Ok(())
    }

    #[tokio::test]
    async fn frame_reader_tolerates_bom() -> Result<(), Box<dyn std::error::Error>> {
        let (mut tx, rx) = duplex();
        tx.write_all(b"\xEF\xBB\xBF{\"a\":1}\n{\"b\":2}\n").await?;
        drop(tx);
        let mut reader = FrameReader::new(rx);
        assert_eq!(reader.next_frame().await?, Some(b"{\"a\":1}".to_vec()));
        assert_eq!(reader.next_frame().await?, Some(b"{\"b\":2}".to_vec()));
        Ok(())
    }

    #[tokio::test]
    async fn frame_reader_enforces_max_line() -> Result<(), Box<dyn std::error::Error>> {
        let (mut tx, rx) = duplex();
        tx.write_all(b"12345678901\n").await?; // 11 bytes with a 10-byte cap
        drop(tx);
        let mut reader = FrameReader::with_max_line(rx, 10);
        assert!(matches!(
            reader.next_frame().await,
            Err(FrameError::LineTooLong)
        ));
        Ok(())
    }

    #[tokio::test]
    async fn frame_reader_fragmented_writes_aggregate() -> Result<(), Box<dyn std::error::Error>> {
        let (mut tx, rx) = duplex();
        // Delimiter split across chunks is still aggregated into one frame.
        tx.write_all(b"{\"a\"").await?;
        tx.write_all(b":1}\n{\"b\"").await?;
        tx.write_all(b":2}\n").await?;
        drop(tx);
        let mut reader = FrameReader::new(rx);
        assert_eq!(reader.next_frame().await?, Some(b"{\"a\":1}".to_vec()));
        assert_eq!(reader.next_frame().await?, Some(b"{\"b\":2}".to_vec()));
        Ok(())
    }

    #[tokio::test]
    async fn frame_writer_emits_payload_and_delimiter() -> Result<(), Box<dyn std::error::Error>> {
        let (mut tx, mut rx) = duplex();
        write_frame(&mut tx, b"{\"a\":1}").await?;
        drop(tx);
        let mut buf = Vec::new();
        rx.read_to_end(&mut buf).await?;
        assert_eq!(buf, b"{\"a\":1}\n".to_vec());
        Ok(())
    }
}
