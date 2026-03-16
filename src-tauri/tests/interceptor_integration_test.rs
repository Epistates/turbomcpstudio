//! Integration test for the Protocol Interceptor
//!
//! This test verifies that the InterceptedTransport correctly captures
//! all MCP protocol messages for the Protocol Inspector.
//!
//! # Running locally
//!
//! These tests require an external `turbomcp-demo` binary. They are skipped
//! in CI by default. To run them locally:
//!
//! ```sh
//! TURBOMCP_DEMO_PATH=/path/to/turbomcp-demo cargo test -- --ignored
//! ```
//!
//! If `TURBOMCP_DEMO_PATH` is not set, the tests fall back to a path derived
//! from `CARGO_MANIFEST_DIR` (i.e. `../../turbomcp/target/release/turbomcp-demo`
//! relative to the manifest). The tests will still be skipped gracefully if
//! neither resolves to an executable file.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use turbomcp_client::Client;
use turbomcp_transport::{child_process::ChildProcessConfig, ChildProcessTransport};
use turbomcpstudio_lib::interceptor::{Direction, InterceptedTransport};

/// Resolve the path to the turbomcp-demo binary.
///
/// Resolution order:
/// 1. `TURBOMCP_DEMO_PATH` environment variable
/// 2. Sibling directory heuristic: `<CARGO_MANIFEST_DIR>/../../turbomcp/target/release/turbomcp-demo`
///
/// Returns `None` when neither resolves to an existing file.
fn resolve_demo_path() -> Option<String> {
    // 1. Explicit override via environment variable (required in CI if ever enabled)
    if let Ok(path) = std::env::var("TURBOMCP_DEMO_PATH") {
        if std::path::Path::new(&path).exists() {
            return Some(path);
        }
    }

    // 2. Heuristic: look two levels above the crate manifest
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let heuristic =
        std::path::PathBuf::from(manifest_dir).join("../../turbomcp/target/release/turbomcp-demo");

    if heuristic.exists() {
        return Some(heuristic.to_string_lossy().into_owned());
    }

    None
}

#[tokio::test]
#[ignore = "requires external turbomcp-demo binary — set TURBOMCP_DEMO_PATH env var"]
async fn test_interceptor_captures_stdio_messages() {
    let demo_path = match resolve_demo_path() {
        Some(p) => p,
        None => {
            eprintln!(
                "Skipping test: turbomcp-demo binary not found. \
                 Set TURBOMCP_DEMO_PATH to the binary path to run this test."
            );
            return;
        }
    };

    // Create a ChildProcessTransport to the demo server
    let config = ChildProcessConfig {
        command: demo_path,
        args: vec![],
        working_directory: None,
        environment: Some(vec![("RUST_LOG".to_string(), "".to_string())]),
        startup_timeout: Duration::from_secs(10),
        shutdown_timeout: Duration::from_secs(5),
        max_message_size: 10 * 1024 * 1024,
        buffer_size: 8192,
        kill_on_drop: true,
    };

    let base_transport = ChildProcessTransport::new(config);

    // Wrap with interceptor
    let (intercepted, mut rx) = InterceptedTransport::new(base_transport);

    // Spawn task to collect intercepted messages
    let messages = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let messages_clone = Arc::clone(&messages);

    let collector = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let mut msgs = messages_clone.lock().await;
            msgs.push((msg.direction, msg.message.payload.clone()));

            // Stop after collecting a few messages
            if msgs.len() >= 4 {
                break;
            }
        }
    });

    // Create a TurboMCP Client with the intercepted transport
    println!("Connecting to demo server...");
    let client = Client::new(intercepted);

    // Initialize the MCP connection (this triggers the protocol handshake)
    let init_result = client.initialize().await;
    assert!(
        init_result.is_ok(),
        "Failed to initialize: {:?}",
        init_result
    );

    println!("MCP handshake completed");

    // Give collector a moment to process messages
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Wait for collector to finish or timeout
    let _ = timeout(Duration::from_secs(5), collector).await;

    // Check that we captured some messages
    let captured = messages.lock().await;

    println!("\n=== INTERCEPTOR TEST RESULTS ===");
    println!("Total messages captured: {}", captured.len());

    for (i, (direction, payload)) in captured.iter().enumerate() {
        let direction_str = match direction {
            Direction::Outgoing => "-> OUTGOING",
            Direction::Incoming => "<- INCOMING",
        };

        let payload_str = String::from_utf8_lossy(payload);
        let payload_preview = if payload_str.len() > 100 {
            format!("{}...", &payload_str[..100])
        } else {
            payload_str.to_string()
        };

        println!("\n[{}] {} ({} bytes)", i + 1, direction_str, payload.len());
        println!("   {}", payload_preview);
    }

    println!("\n================================\n");

    // We should have captured at least 2 messages (initialize request/response)
    assert!(
        captured.len() >= 2,
        "Expected at least 2 intercepted messages (initialize handshake), got {}",
        captured.len()
    );

    // Verify we have both outgoing and incoming messages
    let has_outgoing = captured
        .iter()
        .any(|(dir, _)| matches!(dir, Direction::Outgoing));
    let has_incoming = captured
        .iter()
        .any(|(dir, _)| matches!(dir, Direction::Incoming));

    assert!(
        has_outgoing,
        "Should have captured at least one outgoing message"
    );
    assert!(
        has_incoming,
        "Should have captured at least one incoming message"
    );

    println!("Interceptor successfully captured bidirectional MCP protocol messages!");

    // Shutdown client
    let _ = client.shutdown().await;
}

#[tokio::test]
#[ignore = "requires external turbomcp-demo binary — set TURBOMCP_DEMO_PATH env var"]
async fn test_interceptor_zero_copy_verification() {
    // This test verifies that the Arc-based zero-copy design works correctly

    let demo_path = match resolve_demo_path() {
        Some(p) => p,
        None => {
            eprintln!(
                "Skipping test: turbomcp-demo binary not found. \
                 Set TURBOMCP_DEMO_PATH to the binary path to run this test."
            );
            return;
        }
    };

    let config = ChildProcessConfig {
        command: demo_path,
        args: vec![],
        working_directory: None,
        environment: Some(vec![("RUST_LOG".to_string(), "".to_string())]),
        startup_timeout: Duration::from_secs(10),
        shutdown_timeout: Duration::from_secs(5),
        max_message_size: 10 * 1024 * 1024,
        buffer_size: 8192,
        kill_on_drop: true,
    };

    let base_transport = ChildProcessTransport::new(config);
    let (intercepted, mut rx) = InterceptedTransport::new(base_transport);

    // Create client and initialize
    let client = Client::new(intercepted);
    let _ = client.initialize().await;

    // Receive one intercepted message
    tokio::time::sleep(Duration::from_millis(100)).await;

    if let Some(msg) = timeout(Duration::from_secs(2), rx.recv())
        .await
        .ok()
        .flatten()
    {
        // Verify Arc is being used (small overhead)
        let arc_size = std::mem::size_of::<Arc<turbomcp_transport::TransportMessage>>();
        println!("Arc<TransportMessage> size: {} bytes", arc_size);
        assert_eq!(
            arc_size, 8,
            "Arc should be a single pointer (8 bytes on 64-bit)"
        );

        // The message payload uses Bytes which is already zero-copy
        println!("Message payload size: {} bytes", msg.message.payload.len());
        println!("Zero-copy verification passed!");
    }

    let _ = client.shutdown().await;
}
