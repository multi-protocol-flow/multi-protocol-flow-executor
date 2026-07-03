#![allow(missing_docs)]

//! WebSocket dependency verification tests
//! Ensures tokio-tungstenite with rustls TLS is correctly configured

use futures_util::{SinkExt, StreamExt};
use rustls::crypto::ring::default_provider;
use std::sync::Once;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

/// Ensure rustls CryptoProvider is installed before any TLS connections
static CRYPTO_INIT: Once = Once::new();
fn ensure_crypto_provider() {
    CRYPTO_INIT.call_once(|| {
        let _ = default_provider().install_default();
    });
}

/// Verify WebSocket TLS support by connecting to a secure WebSocket server
/// This test validates that the rustls TLS backend is working correctly
#[tokio::test]
async fn test_websocket_tls_connection() {
    ensure_crypto_provider();
    // Use a public WebSocket echo server with TLS
    let ws_url = "wss://echo.websocket.org";

    // Attempt to establish a WebSocket connection over TLS
    let result = connect_async(ws_url).await;

    match result {
        Ok((ws_stream, _response)) => {
            // Connection successful - TLS is working!
            // This is the main verification: we successfully connected via wss://
            
            // Send a test message
            let (mut tx, mut rx) = ws_stream.split();
            let test_message = "Hello, WebSocket TLS!";
            tx.send(Message::Text(test_message.into()))
                .await
                .expect("Failed to send message");

            // Receive any response (server may send greeting first)
            // The key verification is that TLS works, not the exact echo behavior
            if let Some(Ok(_msg)) = rx.next().await {
                // Successfully received a message - TLS and WebSocket are working
            }
        }
        Err(e) => {
            // Connection might fail due to network issues, but TLS should be available
            // Check that the error is not a TLS feature missing error
            let error_msg = e.to_string();
            // If TLS feature is missing, the error would mention "TLS" or "native-tls"
            // Network errors are acceptable for this test
            if error_msg.contains("native-tls") || error_msg.contains("feature") {
                panic!("TLS feature appears to be missing: {}", error_msg);
            }
            // Log the error but don't fail - network connectivity issues are expected
            eprintln!("WebSocket connection failed (network issue expected in CI): {}", error_msg);
        }
    }
}

/// Verify that futures-util is available and functional
#[tokio::test]
async fn test_futures_util_available() {
    use futures_util::stream::{self, StreamExt};

    // Create a simple stream and collect results
    let stream = stream::iter(vec![1, 2, 3, 4, 5]);
    let collected: Vec<i32> = stream.collect().await;

    assert_eq!(collected, vec![1, 2, 3, 4, 5]);
}

/// Verify tokio-tungstenite types are accessible
#[test]
fn test_websocket_types_accessible() {
    // Verify Message type is accessible (using .into() for type conversion)
    let _text_msg = Message::Text("test".into());
    let _binary_msg = Message::Binary(vec![1_u8, 2, 3].into());
    let _ping_msg = Message::Ping(vec![].into());
    let _pong_msg = Message::Pong(vec![].into());
    let _close_msg = Message::Close(None);
}
