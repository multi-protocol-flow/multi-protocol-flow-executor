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
    "mcp_call_tool",
    "mcp_close",
    "mcp_connect",
    "mcp_get_prompt",
    "mcp_read_resource",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_list_is_sorted() {
        assert!(RESERVED_TYPE_IDS.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn builtin_ids_are_reserved() {
        for id in [
            "entry",
            "conditional",
            "end",
            "variable_extractor",
            "proxy",
            "http",
            "ws_connect",
            "ws_send_collect",
            "script",
            "assertion",
            "grpc_call",
            "mcp_get_prompt",
            "tcp_send",
            "udp_bind",
                    "sse_listen",
            "graphql_subscribe",
                                ] {
            assert!(is_reserved_type_id(id), "{id} should be reserved");
        }
    }

    #[test]
    fn third_party_ids_are_not_reserved() {
        for id in [
            "acme:ping",
            "vendor:name",
            "http_fetch",
            "my_http",
            "amqp_management",
            "ws",
            "redis",
        ] {
            assert!(!is_reserved_type_id(id), "{id} should not be reserved");
        }
    }

    #[test]
    fn reserved_count_matches_default_registry() {
        // 52 = 5 always-on (entry/conditional/end/variable_extractor/proxy)
        // + 1 http + 3 ws + 1 script + 1 assertion + 3 grpc + 5 mcp
        // + 8 tcp/udp + 3 sse + 5 graphql.
        // (smtp/mqtt/kafka/imap 已移出宿主 → plugins/{smtp,mqtt,kafka,imap}
        // 第三方插件；redis 已移出宿主 —— redis 变为第三方插件 plugins/redis，
        // type_id 用 `redis:connect` 等 mongo 风格命名，天然不占用保留 id；
        // db 6 个 type_id 同理移出 —— 内置 db 移除后由 plugins/db 以 `db:connect`
        // 命名空间提供，不再占用裸 id。) Cross-checked against
        // `default_registry.rs::test_plugin_count_matches_features` (35).
        assert_eq!(RESERVED_TYPE_IDS.len(), 35);
    }
}
