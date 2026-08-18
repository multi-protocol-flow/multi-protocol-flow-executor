//! Reserved built-in node type ids (M2c collision policy).
//!
//! OD2: built-in ids are never shadowed by plugins; plugin-vs-plugin
//! collisions are first-wins + warn. [`RESERVED_TYPE_IDS`] is the exact set of
//! `type_id` strings registered by `flow_engine_executor::default_registry`
//! — extracted from the literal `type_id()` implementations of every
//! registered `NodePlugin` (79 ids, matching the registry's own count test).
//!
//! Notes on the extraction (the doc table in `default_registry.rs` is NOT the
//! source of truth):
//! - The Redis family (`redis_connect`/`redis_command`/`redis_close`) is NOT
//!   registered anymore — redis moved to the third-party `plugins/redis`
//!   plugin (`redis:connect` etc., mongo-style namespace), so those ids are
//!   free for third-party use.
//! - `amqp_management` is `#[cfg(feature = "management")]`-gated and NOT part
//!   of the default registry — deliberately excluded (a third-party plugin may
//!   use it until the feature ships).
//! - The registry uses one literal id per node type (`ws_connect`,
//!   ...) — there are no prefix families, so the check is exact-match only.

/// The reserved built-in node type ids (sorted, byte-lexicographic — Rust
/// `str` ordering, where `amqp10_*` sorts before `amqp_*` because `'1'` <
/// `'_'`).
pub const RESERVED_TYPE_IDS: &[&str] = &[
    "assertion",
    "conditional",
    "end",
    "entry",
    "graphql_connect",
    "graphql_disconnect",
    "graphql_introspect",
    "graphql_query",
    "graphql_subscribe",
    "grpc_call",
    "grpc_close",
    "grpc_connect",
    "http",
    "proxy",
    "script",
    "sse_connect",
    "sse_disconnect",
    "sse_listen",
    "tcp_close",
    "tcp_connect",
    "tcp_receive",
    "tcp_send",
    "udp_bind",
    "udp_close",
    "udp_recv_from",
    "udp_send_to",
    "variable_extractor",
    "ws_close",
    "ws_connect",
    "ws_send_collect",
];

/// Returns `true` when `id` is a reserved built-in node type id that plugins
/// may never shadow.
///
/// Exact match over [`RESERVED_TYPE_IDS`] via binary search. The real registry
/// registers one literal id per node type (no `ws_*`-style prefix families),
/// so prefix matching would wrongly reserve third-party namespaces such as
/// `ws_custom` or `my_http`.
pub fn is_reserved_type_id(id: &str) -> bool {
    RESERVED_TYPE_IDS.binary_search(&id).is_ok()
}

