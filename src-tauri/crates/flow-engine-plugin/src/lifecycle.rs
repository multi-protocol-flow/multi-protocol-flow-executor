//! Plugin process lifecycle.
//!
//! On-demand start, 60s idle recycle (gated on `capabilities.streaming`),
//! restart-once crash policy, per-generation epoch tokens, and clean
//! shutdown. Implementation lands in M2b.
