//! Child process spawning hygiene.
//!
//! Stderr draining, Windows Job Object (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`),
//! handle-inheritance hygiene, and interpreter env injection. The only
//! module that may use `unsafe` (Job Object FFI); implementation lands in M1.
