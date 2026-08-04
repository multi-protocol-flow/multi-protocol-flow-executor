//! Pipelined JSON-RPC transport over a child process's stdin/stdout.
//!
//! Single-writer task owning `ChildStdin`, reader task owning `ChildStdout`,
//! pending-request map with per-request timeouts, and bounded backpressure.
//! Implementation lands in M1.
