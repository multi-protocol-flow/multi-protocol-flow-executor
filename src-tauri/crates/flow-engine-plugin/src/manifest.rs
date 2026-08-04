//! Plugin manifest (`plugin.json`) parsing and validation.
//!
//! Wraps `mpe-protocol::manifest` types with host-side validation rules
//! (e.g. entry command presence, semver shape). Implementation lands in M2a.
