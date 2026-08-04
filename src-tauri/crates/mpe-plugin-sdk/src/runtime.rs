//! Async JSON-RPC event loop for plugins.
//!
//! Framed stdin/stdout dispatcher implementing the `mpe-protocol` wire
//! contract: `describe`/`execute`/`validate`/`shutdown` methods, pipelined
//! responses. Implementation lands in M5.
