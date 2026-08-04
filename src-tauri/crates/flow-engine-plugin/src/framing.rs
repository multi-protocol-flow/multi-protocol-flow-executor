//! Line framing over child stdio.
//!
//! `LF`-delimited frames with `CRLF` tolerance, byte-exact UTF-8 validation,
//! and the `mpe-protocol::MAX_LINE_BYTES` cap. Implementation lands in M1.
