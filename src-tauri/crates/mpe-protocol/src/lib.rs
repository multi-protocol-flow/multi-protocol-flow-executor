//! MPE plugin wire protocol — shared types between host and plugins.
//!
//! A minimal JSON-RPC 2.0 subset (requests, discriminated-union responses,
//! error objects) plus the plugin manifest schema and framing constants.
//! Consumed by both the host (`flow-engine-plugin`) and the SDK
//! (`mpe-plugin-sdk`) so the wire contract can never drift between them.

pub mod describe;
pub mod error;
pub mod manifest;
pub mod notification;
pub mod request;
pub mod response;
pub mod ui;

/// Maximum length of a single framed line in bytes (`2 MiB`).
///
/// The cap applies in both directions on the stdio transport; lines longer
/// than this are protocol violations and must be rejected by the framing
/// reader, never buffered without bound.
pub const MAX_LINE_BYTES: usize = 2 * 1024 * 1024;

/// The line delimiter for the plugin transport: LF.
///
/// Writers must emit a bare `\n`; readers must tolerate a trailing `\r`
/// (`CRLF`, as emitted by Python `print()`).
pub const LINE_DELIMITER: u8 = b'\n';

/// Environment variable the host injects into a plugin process carrying the
/// plugin's effective UI language (a BCP-47 tag such as `en-US`).
///
/// Injected once at spawn: the host picks the host's current language when it
/// appears in the plugin manifest's `locales`, otherwise the plugin's
/// `default_locale`. A plugin that declares neither is spawned without this
/// variable and keeps its own hard-coded default. The value is fixed for the
/// process lifetime; changing it requires restarting/reloading the plugin.
pub const LOCALE_ENV: &str = "MPE_LOCALE";
