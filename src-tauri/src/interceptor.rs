//! Universal Transport Interceptor for TurboMCP Studio
//!
//! This module provides a zero-copy, transport-agnostic interception layer
//! that captures all MCP messages for display in the Protocol Inspector UI.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use serde::Serialize;
use tokio::sync::mpsc;
use turbomcp_transport::{
    Transport, TransportCapabilities, TransportConfig, TransportMessage, TransportMetrics,
    TransportResult, TransportState, TransportType,
};

/// Direction of an intercepted message
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// Message sent from client to server
    Outgoing,
    /// Message received from server to client
    Incoming,
}

/// An intercepted message with metadata for the frontend
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterceptedMessage {
    /// Direction of the message
    pub direction: Direction,

    /// Timestamp when intercepted (milliseconds since app start)
    #[serde(serialize_with = "serialize_instant")]
    pub timestamp: Instant,

    /// Message ID
    pub message_id: String,

    /// The actual message payload (JSON string or base64 for binary)
    pub payload: String,

    /// Size of the payload in bytes
    pub size: usize,
}

/// Serialize Instant as milliseconds since app start
fn serialize_instant<S>(instant: &Instant, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let millis = instant.elapsed().as_millis() as u64;
    serializer.serialize_u64(millis)
}

/// Internal message wrapper with Arc for zero-copy
#[derive(Debug, Clone)]
pub struct InternalInterceptedMessage {
    pub direction: Direction,
    pub timestamp: Instant,
    pub message: Arc<TransportMessage>,
}

/// A transport wrapper that intercepts all send/receive operations
#[derive(Debug, Clone)]
pub struct InterceptedTransport<T: Transport> {
    inner: Arc<T>,
    tx: mpsc::UnboundedSender<Arc<InternalInterceptedMessage>>,
}

impl<T: Transport> InterceptedTransport<T> {
    /// Create a new intercepted transport wrapping the given turbomcp transport
    pub fn new(
        inner: T,
    ) -> (
        Self,
        mpsc::UnboundedReceiver<Arc<InternalInterceptedMessage>>,
    ) {
        let (tx, rx) = mpsc::unbounded_channel();
        let intercepted = Self {
            inner: Arc::new(inner),
            tx,
        };
        (intercepted, rx)
    }

    /// Get a reference to the inner transport
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Broadcast an intercepted message to listeners
    fn broadcast(&self, direction: Direction, message: Arc<TransportMessage>) {
        let intercepted = Arc::new(InternalInterceptedMessage {
            direction,
            timestamp: Instant::now(),
            message,
        });
        let _ = self.tx.send(intercepted);
    }
}

impl<T: Transport + 'static> Transport for InterceptedTransport<T> {
    fn transport_type(&self) -> TransportType {
        self.inner.transport_type()
    }

    fn capabilities(&self) -> &TransportCapabilities {
        self.inner.capabilities()
    }

    fn state(&self) -> Pin<Box<dyn Future<Output = TransportState> + Send + '_>> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.state().await })
    }

    fn connect(&self) -> Pin<Box<dyn Future<Output = TransportResult<()>> + Send + '_>> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.connect().await })
    }

    fn disconnect(&self) -> Pin<Box<dyn Future<Output = TransportResult<()>> + Send + '_>> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.disconnect().await })
    }

    fn send(
        &self,
        message: TransportMessage,
    ) -> Pin<Box<dyn Future<Output = TransportResult<()>> + Send + '_>> {
        let message_arc = Arc::new(message);
        self.broadcast(Direction::Outgoing, Arc::clone(&message_arc));
        
        let inner = self.inner.clone();
        Box::pin(async move {
            let message_clone = (*message_arc).clone();
            inner.send(message_clone).await
        })
    }

    fn receive(
        &self,
    ) -> Pin<Box<dyn Future<Output = TransportResult<Option<TransportMessage>>> + Send + '_>> {
        let inner = self.inner.clone();
        let tx = self.tx.clone();
        
        Box::pin(async move {
            let message_opt = inner.receive().await?;
            if let Some(message) = message_opt {
                let message_arc = Arc::new(message);
                
                let intercepted = Arc::new(InternalInterceptedMessage {
                    direction: Direction::Incoming,
                    timestamp: Instant::now(),
                    message: Arc::clone(&message_arc),
                });
                let _ = tx.send(intercepted);

                let message_clone = (*message_arc).clone();
                Ok(Some(message_clone))
            } else {
                Ok(None)
            }
        })
    }

    fn metrics(&self) -> Pin<Box<dyn Future<Output = TransportMetrics> + Send + '_>> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.metrics().await })
    }

    fn is_connected(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.is_connected().await })
    }

    fn endpoint(&self) -> Option<String> {
        self.inner.endpoint()
    }

    fn configure(
        &self,
        config: TransportConfig,
    ) -> Pin<Box<dyn Future<Output = TransportResult<()>> + Send + '_>> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.configure(config).await })
    }
}

/// Helper to convert InternalInterceptedMessage to UI-friendly InterceptedMessage
impl From<&InternalInterceptedMessage> for InterceptedMessage {
    fn from(internal: &InternalInterceptedMessage) -> Self {
        let payload_str = String::from_utf8_lossy(&internal.message.payload).to_string();

        Self {
            direction: internal.direction,
            timestamp: internal.timestamp,
            message_id: internal.message.id.to_string(),
            payload: payload_str,
            size: internal.message.payload.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use turbomcp_transport::core::{TransportError, TransportMessageMetadata};

    /// Mock transport for testing
    #[derive(Debug)]
    struct MockTransport {
        state: std::sync::Arc<std::sync::Mutex<TransportState>>,
        capabilities: TransportCapabilities,
    }

    impl MockTransport {
        fn new() -> Self {
            Self {
                state: std::sync::Arc::new(std::sync::Mutex::new(TransportState::Disconnected)),
                capabilities: TransportCapabilities {
                    max_message_size: Some(1024 * 1024),
                    supports_compression: false,
                    supports_streaming: false,
                    supports_bidirectional: false,
                    supports_multiplexing: false,
                    compression_algorithms: Vec::new(),
                    custom: HashMap::new(),
                },
            }
        }
    }

    impl Transport for MockTransport {
        fn transport_type(&self) -> TransportType {
            TransportType::Stdio
        }

        fn capabilities(&self) -> &TransportCapabilities {
            &self.capabilities
        }

        fn state(&self) -> Pin<Box<dyn Future<Output = TransportState> + Send + '_>> {
            // In MockTransport we don't have Arc around it yet, but Transport trait expects &self
            // and returns a future. For mock it's fine to just return immediate future.
            let state = self.state.clone();
            Box::pin(async move { state.lock().unwrap().clone() })
        }

        fn connect(&self) -> Pin<Box<dyn Future<Output = TransportResult<()>> + Send + '_>> {
            let state = self.state.clone();
            Box::pin(async move {
                *state.lock().unwrap() = TransportState::Connected;
                Ok(())
            })
        }

        fn disconnect(&self) -> Pin<Box<dyn Future<Output = TransportResult<()>> + Send + '_>> {
            let state = self.state.clone();
            Box::pin(async move {
                *state.lock().unwrap() = TransportState::Disconnected;
                Ok(())
            })
        }

        fn send(
            &self,
            _message: TransportMessage,
        ) -> Pin<Box<dyn Future<Output = TransportResult<()>> + Send + '_>> {
            let state = self.state.clone();
            Box::pin(async move {
                if !matches!(*state.lock().unwrap(), TransportState::Connected) {
                    return Err(TransportError::ConnectionFailed(
                        "Not connected".to_string(),
                    ));
                }
                Ok(())
            })
        }

        fn receive(
            &self,
        ) -> Pin<Box<dyn Future<Output = TransportResult<Option<TransportMessage>>> + Send + '_>> {
            Box::pin(async move { Ok(None) })
        }

        fn metrics(&self) -> Pin<Box<dyn Future<Output = TransportMetrics> + Send + '_>> {
            Box::pin(async move { TransportMetrics::default() })
        }
    }

    #[tokio::test]
    async fn test_intercepted_transport_basic() {
        let mock = MockTransport::new();
        let (intercepted, mut rx) = InterceptedTransport::new(mock);

        intercepted.connect().await.unwrap();

        let message = TransportMessage {
            id: 1i64.into(),
            payload: b"test"[..].into(),
            metadata: TransportMessageMetadata::default(),
        };

        intercepted.send(message).await.unwrap();

        // Should have intercepted the message
        let intercepted_msg = rx.recv().await.unwrap();
        assert_eq!(intercepted_msg.direction, Direction::Outgoing);
    }
}
