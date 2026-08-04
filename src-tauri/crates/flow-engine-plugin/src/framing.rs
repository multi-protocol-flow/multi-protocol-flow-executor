//! Line framing over child stdio.
//!
//! One JSON value per line, `LF`-delimited (see `mpe_protocol::LINE_DELIMITER`).
//! The reader tolerates a trailing `CR` (`CRLF`, as emitted by Python's
//! `print()`), validates byte-exact UTF-8, tolerates a leading BOM on the
//! first frame, and enforces the `2 MiB` line cap
//! (`mpe_protocol::MAX_LINE_BYTES`) — it never buffers without bound.
//! The writer emits whole frames atomically (full payload + delimiter in a
//! single `write_all` + `flush`) and is only ever used by the single writer
//! task, so a frame can never be interleaved with another.

use std::io;

use mpe_protocol::{LINE_DELIMITER, MAX_LINE_BYTES};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Size of the chunk the reader pulls from the underlying stream at a time.
const READ_CHUNK: usize = 8192;

/// Framing-layer errors. These are **transport** errors, distinct from
/// protocol errors: a `LineTooLong` or `InvalidUtf8` is a violation of the
/// framing contract and is handled by the reader's malformed-line policy.
#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    /// Underlying stream I/O failed.
    #[error("frame io error: {0}")]
    Io(#[from] io::Error),
    /// A line exceeded [`MAX_LINE_BYTES`](mpe_protocol::MAX_LINE_BYTES).
    #[error("line exceeds maximum length of {MAX_LINE_BYTES} bytes")]
    LineTooLong,
    /// A line is not byte-exact UTF-8.
    #[error("line is not valid UTF-8")]
    InvalidUtf8,
    /// The stream reached EOF in the middle of a partial line.
    #[error("unexpected EOF in the middle of a line")]
    UnexpectedEof,
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
    /// Whether we have consumed the leading BOM (only stripped from the very
    /// first frame of the stream).
    bom_consumed: bool,
    /// Whether EOF has been observed.
    eof: bool,
    /// Bytes read but not yet delimited into a frame. Persisted across
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

/// Atomic frame writer.
///
/// `write_frame` emits the full payload followed by the `LF` delimiter in one
/// `write_all` + `flush` pair. Atomicity across concurrent callers is provided
/// by construction: the owning writer task is the only task that holds the
/// writer, so no two callers can interleave their frames.
pub struct FrameWriter<W> {
    inner: W,
}

impl<W: AsyncWrite + Unpin> FrameWriter<W> {
    /// Wraps an `AsyncWrite` stream.
    pub fn new(inner: W) -> Self {
        Self { inner }
    }

    /// Writes one complete frame (payload + delimiter) and flushes.
    ///
    /// The payload must be pre-serialized and UTF-8-valid; the framing layer
    /// does not re-validate (the writer is the hot path).
    pub async fn write_frame(&mut self, payload: &[u8]) -> io::Result<()> {
        self.inner.write_all(payload).await?;
        self.inner.write_all(&[LINE_DELIMITER]).await?;
        self.inner.flush().await
    }

    /// Flushes any buffered bytes (no-op after `write_frame`'s flush).
    pub async fn flush(&mut self) -> io::Result<()> {
        self.inner.flush().await
    }

    /// Sends EOF to the peer (half-close), letting it drain and exit.
    pub async fn shutdown(&mut self) -> io::Result<()> {
        self.inner.shutdown().await
    }

    /// Consumes the writer and returns the inner stream.
    pub fn into_inner(self) -> W {
        self.inner
    }
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
        // 0xFF is not valid UTF-8.
        tx.write_all(b"{\"a\":\"\xff\"}\n").await?;
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
        // Leading BOM stripped from the first frame only.
        assert_eq!(reader.next_frame().await?, Some(b"{\"a\":1}".to_vec()));
        assert_eq!(reader.next_frame().await?, Some(b"{\"b\":2}".to_vec()));
        assert_eq!(reader.next_frame().await?, None);
        Ok(())
    }

    #[tokio::test]
    async fn frame_reader_enforces_max_line() -> Result<(), Box<dyn std::error::Error>> {
        let (mut tx, rx) = duplex();
        // 11-byte line with a 10-byte cap → LineTooLong, never unbounded.
        tx.write_all(b"12345678901\n").await?;
        drop(tx);
        let mut reader = FrameReader::with_max_line(rx, 10);
        assert!(matches!(
            reader.next_frame().await,
            Err(FrameError::LineTooLong)
        ));
        Ok(())
    }

    #[tokio::test]
    async fn frame_reader_exact_max_line_allowed() -> Result<(), Box<dyn std::error::Error>> {
        let (mut tx, rx) = duplex();
        tx.write_all(b"1234567890\n").await?; // exactly 10 bytes
        drop(tx);
        let mut reader = FrameReader::with_max_line(rx, 10);
        assert_eq!(reader.next_frame().await?, Some(b"1234567890".to_vec()));
        Ok(())
    }

    #[tokio::test]
    async fn frame_reader_partial_line_at_eof_is_error() -> Result<(), Box<dyn std::error::Error>> {
        let (mut tx, rx) = duplex();
        tx.write_all(b"{\"a\":1}").await?; // no trailing LF
        drop(tx);
        let mut reader = FrameReader::new(rx);
        assert!(matches!(
            reader.next_frame().await,
            Err(FrameError::UnexpectedEof)
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
    async fn frame_writer_atomic() -> Result<(), Box<dyn std::error::Error>> {
        let (tx, mut rx) = duplex();
        let mut writer = FrameWriter::new(tx);
        writer.write_frame(b"{\"a\":1}").await?;
        writer.shutdown().await?; // half-close so the read side reaches EOF
        // Reader sees payload + delimiter atomically.
        let mut buf = Vec::new();
        rx.read_to_end(&mut buf).await?;
        assert_eq!(buf, b"{\"a\":1}\n".to_vec());
        Ok(())
    }

    #[tokio::test]
    async fn frame_writer_shutdown_half_closes() -> Result<(), Box<dyn std::error::Error>> {
        let (tx, mut rx) = duplex();
        let mut writer = FrameWriter::new(tx);
        writer.write_frame(b"bye").await?;
        writer.shutdown().await?;
        let mut buf = Vec::new();
        rx.read_to_end(&mut buf).await?;
        assert_eq!(buf, b"bye\n".to_vec());
        Ok(())
    }
}
