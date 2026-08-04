//! Plugin directory scanning and discovery.
//!
//! Spawn-free scan of the plugins directory, one plugin per subdir; invalid
//! or missing manifests are skipped with a warning, never fatal.
//! Implementation lands in M2a.
