//! `SidecarPlugin`: the `NodePlugin` wrapper around a plugin connection.
//!
//! Presents a plugin node type as a host `NodePlugin`: variable resolution,
//! execute RPC, flat-key `variable_updates` merge, timeout/cancel/kill
//! ownership. Implementation lands in M3.
