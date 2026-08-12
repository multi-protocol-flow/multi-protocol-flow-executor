//! Type-safe provider key pattern
//!
//! Provides a `ProviderKey` trait that maps each protocol provider to
//! a unique string key. Using typed marker structs instead of raw string
//! literals eliminates typos and enables IDE-assisted refactoring.

/// Type-safe key for accessing protocol providers in `ExecutionContext`.
///
/// Each protocol gets a zero-sized marker type that implements this trait
/// with the provider's string key. Use with `get_provider`:
///
/// ```ignore
/// let ws = ctx.get_provider::<WebSocketConnectionManager>(Ws::key());
/// ```
pub trait ProviderKey {
    /// Returns the unique string key for this provider type.
    fn key() -> &'static str;
}

// ============================================================================
// Protocol marker types and their ProviderKey implementations
// ============================================================================

/// WebSocket provider key
pub struct Ws;
/// gRPC provider key
pub struct Grpc;
/// MCP provider key
pub struct Mcp;
/// TCP provider key
pub struct Tcp;
/// UDP provider key
pub struct Udp;
/// MQTT provider key
pub struct Mqtt;
/// Kafka provider key
pub struct Kafka;
/// SSE provider key
pub struct Sse;
/// GraphQL provider key
pub struct Graphql;
/// AMQP provider key
pub struct Amqp;
/// SMTP provider key
pub struct Smtp;
/// IMAP provider key
pub struct Imap;

impl ProviderKey for Ws {
    fn key() -> &'static str {
        "ws"
    }
}
impl ProviderKey for Grpc {
    fn key() -> &'static str {
        "grpc"
    }
}
impl ProviderKey for Mcp {
    fn key() -> &'static str {
        "mcp"
    }
}
impl ProviderKey for Tcp {
    fn key() -> &'static str {
        "tcp"
    }
}
impl ProviderKey for Udp {
    fn key() -> &'static str {
        "udp"
    }
}
impl ProviderKey for Mqtt {
    fn key() -> &'static str {
        "mqtt"
    }
}
impl ProviderKey for Kafka {
    fn key() -> &'static str {
        "kafka"
    }
}
impl ProviderKey for Sse {
    fn key() -> &'static str {
        "sse"
    }
}
impl ProviderKey for Graphql {
    fn key() -> &'static str {
        "graphql"
    }
}
impl ProviderKey for Amqp {
    fn key() -> &'static str {
        "amqp"
    }
}
impl ProviderKey for Smtp {
    fn key() -> &'static str {
        "smtp"
    }
}
impl ProviderKey for Imap {
    fn key() -> &'static str {
        "imap"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_key() {
        assert_eq!(Ws::key(), "ws");
    }

    #[test]
    fn test_grpc_key() {
        assert_eq!(Grpc::key(), "grpc");
    }

    #[test]
    fn test_mcp_key() {
        assert_eq!(Mcp::key(), "mcp");
    }

    #[test]
    fn test_tcp_key() {
        assert_eq!(Tcp::key(), "tcp");
    }

    #[test]
    fn test_udp_key() {
        assert_eq!(Udp::key(), "udp");
    }

    #[test]
    fn test_mqtt_key() {
        assert_eq!(Mqtt::key(), "mqtt");
    }

    #[test]
    fn test_kafka_key() {
        assert_eq!(Kafka::key(), "kafka");
    }

    #[test]
    fn test_sse_key() {
        assert_eq!(Sse::key(), "sse");
    }

    #[test]
    fn test_graphql_key() {
        assert_eq!(Graphql::key(), "graphql");
    }

    #[test]
    fn test_amqp_key() {
        assert_eq!(Amqp::key(), "amqp");
    }

    #[test]
    fn test_smtp_key() {
        assert_eq!(Smtp::key(), "smtp");
    }

    #[test]
    fn test_imap_key() {
        assert_eq!(Imap::key(), "imap");
    }
}
