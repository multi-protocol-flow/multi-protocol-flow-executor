//! Reserved built-in node type ids (M2c collision policy).
//!
//! OD2: built-in ids are never shadowed by plugins; plugin-vs-plugin
//! collisions are first-wins + warn. [`RESERVED_TYPE_IDS`] is the exact set of
//! `type_id` strings registered by `flow_engine_executor::default_registry`
//! — extracted from the literal `type_id()` implementations of every
//! registered `NodePlugin` (91 ids, matching the registry's own count test).
//!
//! Notes on the extraction (the doc table in `default_registry.rs` is NOT the
//! source of truth):
//! - The Redis family (`redis_connect`/`redis_command`/`redis_close`) IS
//!   registered but missing from the doc table — included here.
//! - `amqp_management` is `#[cfg(feature = "management")]`-gated and NOT part
//!   of the default registry — deliberately excluded (a third-party plugin may
//!   use it until the feature ships).
//! - The registry uses one literal id per node type (`ws_connect`, `db_query`,
//!   ...) — there are no prefix families, so the check is exact-match only.

/// The reserved built-in node type ids (sorted, byte-lexicographic — Rust
/// `str` ordering, where `amqp10_*` sorts before `amqp_*` because `'1'` <
/// `'_'`).
pub const RESERVED_TYPE_IDS: &[&str] = &[
    "amqp10_accept",
    "amqp10_connect",
    "amqp10_disconnect",
    "amqp10_receive",
    "amqp10_reject",
    "amqp10_send",
    "amqp_ack",
    "amqp_close",
    "amqp_connect",
    "amqp_consume",
    "amqp_exchange_declare",
    "amqp_exchange_delete",
    "amqp_get",
    "amqp_publish",
    "amqp_queue_bind",
    "amqp_queue_declare",
    "amqp_queue_delete",
    "assertion",
    "conditional",
    "db_begin",
    "db_close",
    "db_commit",
    "db_connect",
    "db_query",
    "db_rollback",
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
    "imap_close",
    "imap_connect",
    "imap_copy",
    "imap_expunge",
    "imap_fetch",
    "imap_folder_create",
    "imap_folder_delete",
    "imap_idle",
    "imap_list",
    "imap_move",
    "imap_search",
    "imap_select",
    "imap_store_flags",
    "kafka_admin",
    "kafka_close",
    "kafka_connect",
    "kafka_consume",
    "kafka_consume_batch",
    "kafka_produce",
    "kafka_schema_registry",
    "mcp_call_tool",
    "mcp_close",
    "mcp_connect",
    "mcp_get_prompt",
    "mcp_read_resource",
    "mqtt_connect",
    "mqtt_disconnect",
    "mqtt_publish",
    "mqtt_request",
    "mqtt_subscribe",
    "mqtt_unsubscribe",
    "mqtt_wait_message",
    "proxy",
    "redis_close",
    "redis_command",
    "redis_connect",
    "script",
    "smtp_connect",
    "smtp_disconnect",
    "smtp_send",
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
            "db_query",
            "db_begin",
            "script",
            "assertion",
            "grpc_call",
            "mcp_get_prompt",
            "tcp_send",
            "udp_bind",
            "mqtt_wait_message",
            "kafka_schema_registry",
            "sse_listen",
            "graphql_subscribe",
            "amqp_queue_bind",
            "amqp10_reject",
            "smtp_send",
            "imap_folder_create",
            "redis_command",
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
        // 91 = 5 always-on (entry/conditional/end/variable_extractor/proxy)
        // + 1 http + 3 ws + 6 db + 1 script + 1 assertion + 3 grpc + 5 mcp
        // + 8 tcp/udp + 7 mqtt + 7 kafka + 3 sse + 5 graphql + 17 amqp
        // + 3 smtp + 13 imap + 3 redis. Cross-checked against
        // `default_registry.rs::test_plugin_count_matches_features` (91).
        assert_eq!(RESERVED_TYPE_IDS.len(), 91);
    }
}
