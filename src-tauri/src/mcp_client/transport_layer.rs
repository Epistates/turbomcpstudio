//! Transport Layer Module
//!
//! Handles all MCP transport initialization and connection management:
//! - STDIO (ChildProcess) - Most common, spawns process
//! - HTTP/SSE - Server-Sent Events for streaming
//! - WebSocket - Bidirectional real-time
//! - TCP - Low-level socket communication
//! - Unix - Unix domain sockets
//!
//! All transports support the full MCP 2025-06-18 protocol including
//! bidirectional features (sampling + elicitation).

use crate::error::{McpResult, McpStudioError};
use crate::mcp_client::connection::ManagedConnection;
use crate::mcp_client::elicitation::StudioElicitationHandler;
use crate::mcp_client::sampling::StudioSamplingHandler;
use crate::mcp_client::transport_client::McpTransportClient;
use crate::types::{ConnectionStatus, TransportConfig};
use chrono::Utc;
use std::sync::Arc;
use turbomcp_client::Client;
use turbomcp_transport::child_process::{ChildProcessConfig, ChildProcessTransport};

#[cfg(feature = "http")]
use turbomcp_transport::streamable_http_client::{
    StreamableHttpClientConfig, StreamableHttpClientTransport,
};

#[cfg(feature = "websocket")]
use turbomcp_transport::websocket_bidirectional::{
    WebSocketBidirectionalConfig, WebSocketBidirectionalTransport,
};

#[cfg(feature = "tcp")]
use turbomcp_transport::tcp::TcpTransport;

#[cfg(feature = "unix")]
use turbomcp_transport::unix::UnixTransport;

use super::configuration::Configuration;
use super::initialization::Initialization;

/// Transport Layer Operations
///
/// Provides stateless operations for all MCP transport types.
/// Handles initialization, connection, and client setup.
pub struct TransportLayer;

impl TransportLayer {
    /// Establish MCP connection with enterprise-grade reliability and monitoring
    pub async fn establish_mcp_connection(
        connection: Arc<ManagedConnection>,
        sampling_handler: Arc<StudioSamplingHandler>,
        elicitation_handler: Arc<StudioElicitationHandler>,
    ) -> McpResult<()> {
        tracing::info!(
            "Establishing MCP connection to: {} (transport: {:?})",
            connection.config.name,
            connection.config.transport_config
        );

        match &connection.config.transport_config {
            TransportConfig::Stdio {
                command,
                args,
                working_directory,
            } => {
                // Start STDIO process with robust process management
                Self::connect_stdio(
                    connection.clone(),
                    command,
                    args,
                    working_directory.as_deref(),
                    sampling_handler,
                    elicitation_handler,
                )
                .await
            }
            #[cfg(feature = "http")]
            TransportConfig::Http { url, headers } => {
                tracing::info!("HTTP/SSE transport connection to: {}", url);
                Self::connect_http(
                    connection.clone(),
                    url,
                    headers,
                    sampling_handler,
                    elicitation_handler,
                )
                .await
            }
            #[cfg(feature = "websocket")]
            TransportConfig::WebSocket { url, headers } => {
                tracing::info!("WebSocket transport connection to: {}", url);
                Self::connect_websocket(
                    connection.clone(),
                    url,
                    headers,
                    sampling_handler,
                    elicitation_handler,
                )
                .await
            }
            #[cfg(feature = "tcp")]
            TransportConfig::Tcp { host, port } => {
                tracing::info!("TCP transport connection to: {}:{}", host, port);
                Self::connect_tcp(
                    connection.clone(),
                    host,
                    *port,
                    sampling_handler,
                    elicitation_handler,
                )
                .await
            }
            #[cfg(feature = "unix")]
            TransportConfig::Unix { path } => {
                tracing::info!("Unix socket transport connection to: {}", path);
                Self::connect_unix(
                    connection.clone(),
                    path,
                    sampling_handler,
                    elicitation_handler,
                )
                .await
            }

            // Feature not enabled - handle unsupported transports
            #[cfg(not(feature = "http"))]
            TransportConfig::Http { .. } => Err(McpStudioError::UnsupportedTransport(
                "HTTP transport not enabled in build".to_string(),
            )),
            #[cfg(not(feature = "websocket"))]
            TransportConfig::WebSocket { .. } => Err(McpStudioError::UnsupportedTransport(
                "WebSocket transport not enabled in build".to_string(),
            )),
            #[cfg(not(feature = "tcp"))]
            TransportConfig::Tcp { .. } => Err(McpStudioError::UnsupportedTransport(
                "TCP transport not enabled in build".to_string(),
            )),
            #[cfg(not(feature = "unix"))]
            TransportConfig::Unix { .. } => Err(McpStudioError::UnsupportedTransport(
                "Unix transport not enabled in build".to_string(),
            )),
        }
    }

    /// Connect to STDIO MCP server using TurboMCP ChildProcessTransport
    async fn connect_stdio(
        connection: Arc<ManagedConnection>,
        command: &str,
        args: &[String],
        working_directory: Option<&str>,
        sampling_handler: Arc<StudioSamplingHandler>,
        elicitation_handler: Arc<StudioElicitationHandler>,
    ) -> McpResult<()> {
        tracing::info!("Connecting to STDIO MCP server using TurboMCP ChildProcessTransport:");
        tracing::info!("  Command: {} {:?}", command, args);
        tracing::info!("  Working directory: {:?}", working_directory);
        tracing::info!(
            "  Environment variables: {} entries",
            connection.config.environment_variables.len()
        );
        for (key, value) in &connection.config.environment_variables {
            tracing::info!("    {}={}", key, value);
        }

        // Initialize TurboMCP ChildProcessTransport directly - let it handle the process lifecycle
        match Self::initialize_child_process_client(
            &connection,
            command,
            args,
            working_directory,
            sampling_handler,
            elicitation_handler,
        )
        .await
        {
            Ok(client) => {
                *connection.client.write() = Some(client);
                tracing::info!(
                    "TurboMCP ChildProcessTransport initialized successfully for: {}",
                    command
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to initialize TurboMCP ChildProcessTransport: {}", e);
                Err(e)
            }
        }
    }

    /// Connect to HTTP/SSE MCP server
    #[cfg(feature = "http")]
    async fn connect_http(
        connection: Arc<ManagedConnection>,
        url: &str,
        _headers: &std::collections::HashMap<String, String>,
        sampling_handler: Arc<StudioSamplingHandler>,
        elicitation_handler: Arc<StudioElicitationHandler>,
    ) -> McpResult<()> {
        // Initialize TurboMCP HTTP/SSE transport and client (DOGFOODING)
        match Self::initialize_http_client(&connection, url, sampling_handler, elicitation_handler)
            .await
        {
            Ok(client) => {
                // Wrap TurboMCP client in SharedClient for thread-safe access
                *connection.client.write() = Some(McpTransportClient::Http(client));
                *connection.status.write() = ConnectionStatus::Connected;
                *connection.last_seen.write() = Some(Utc::now());
                tracing::info!(
                    "✅ TurboMCP HTTP/SSE client connected successfully: {} (MCP 2025-06-18)",
                    url
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    "Failed to initialize TurboMCP HTTP client for {}: {}",
                    url,
                    e
                );
                Err(e)
            }
        }
    }

    /// Connect to WebSocket MCP server using TurboMCP 1.0.8
    #[cfg(feature = "websocket")]
    async fn connect_websocket(
        connection: Arc<ManagedConnection>,
        url: &str,
        headers: &std::collections::HashMap<String, String>,
        sampling_handler: Arc<StudioSamplingHandler>,
        elicitation_handler: Arc<StudioElicitationHandler>,
    ) -> McpResult<()> {
        tracing::info!("Establishing TurboMCP WebSocket connection to: {}", url);

        // Initialize WebSocket transport and client
        match Self::initialize_websocket_client(
            &connection,
            url,
            headers,
            sampling_handler,
            elicitation_handler,
        )
        .await
        {
            Ok(client) => {
                *connection.client.write() = Some(McpTransportClient::WebSocket(client));
                *connection.status.write() = ConnectionStatus::Connected;
                *connection.last_seen.write() = Some(Utc::now());
                tracing::info!(
                    "TurboMCP WebSocket client initialized successfully for: {}",
                    url
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    "Failed to initialize TurboMCP WebSocket client for {}: {}",
                    url,
                    e
                );
                Err(e)
            }
        }
    }

    #[cfg(not(feature = "websocket"))]
    async fn connect_websocket(
        _connection: Arc<ManagedConnection>,
        _url: &str,
        _headers: &std::collections::HashMap<String, String>,
        _sampling_handler: Arc<StudioSamplingHandler>,
        _elicitation_handler: Arc<StudioElicitationHandler>,
    ) -> McpResult<()> {
        tracing::error!("WebSocket transport not compiled - enable 'websocket' feature");
        Err(McpStudioError::UnsupportedTransport(
            "WebSocket transport not compiled - enable 'websocket' feature".to_string(),
        ))
    }

    /// Connect to TCP MCP server using TurboMCP 1.0.8
    #[cfg(feature = "tcp")]
    async fn connect_tcp(
        connection: Arc<ManagedConnection>,
        host: &str,
        port: u16,
        sampling_handler: Arc<StudioSamplingHandler>,
        elicitation_handler: Arc<StudioElicitationHandler>,
    ) -> McpResult<()> {
        tracing::info!("Establishing TurboMCP TCP connection to: {}:{}", host, port);

        // Initialize TCP transport and client
        match Self::initialize_tcp_client(
            &connection,
            host,
            port,
            sampling_handler,
            elicitation_handler,
        )
        .await
        {
            Ok(client) => {
                *connection.client.write() = Some(McpTransportClient::Tcp(client));
                *connection.status.write() = ConnectionStatus::Connected;
                *connection.last_seen.write() = Some(Utc::now());
                tracing::info!(
                    "TurboMCP TCP client initialized successfully for: {}:{}",
                    host,
                    port
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    "Failed to initialize TurboMCP TCP client for {}:{}: {}",
                    host,
                    port,
                    e
                );
                Err(e)
            }
        }
    }

    #[cfg(not(feature = "tcp"))]
    async fn connect_tcp(
        _connection: Arc<ManagedConnection>,
        _host: &str,
        _port: u16,
        _sampling_handler: Arc<StudioSamplingHandler>,
        _elicitation_handler: Arc<StudioElicitationHandler>,
    ) -> McpResult<()> {
        tracing::error!("TCP transport not compiled - enable 'tcp' feature");
        Err(McpStudioError::UnsupportedTransport(
            "TCP transport not compiled - enable 'tcp' feature".to_string(),
        ))
    }

    /// Connect to Unix socket MCP server using TurboMCP 1.0.8
    #[cfg(feature = "unix")]
    async fn connect_unix(
        connection: Arc<ManagedConnection>,
        path: &str,
        sampling_handler: Arc<StudioSamplingHandler>,
        elicitation_handler: Arc<StudioElicitationHandler>,
    ) -> McpResult<()> {
        tracing::info!("Establishing TurboMCP Unix socket connection to: {}", path);

        // Initialize Unix socket transport and client
        match Self::initialize_unix_client(&connection, path, sampling_handler, elicitation_handler)
            .await
        {
            Ok(client) => {
                *connection.client.write() = Some(McpTransportClient::Unix(client));
                *connection.status.write() = ConnectionStatus::Connected;
                *connection.last_seen.write() = Some(Utc::now());
                tracing::info!(
                    "TurboMCP Unix socket client initialized successfully for: {}",
                    path
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    "Failed to initialize TurboMCP Unix socket client for {}: {}",
                    path,
                    e
                );
                Err(e)
            }
        }
    }

    #[cfg(not(feature = "unix"))]
    async fn connect_unix(
        _connection: Arc<ManagedConnection>,
        _path: &str,
        _sampling_handler: Arc<StudioSamplingHandler>,
        _elicitation_handler: Arc<StudioElicitationHandler>,
    ) -> McpResult<()> {
        tracing::error!("Unix socket transport not compiled - enable 'unix' feature");
        Err(McpStudioError::UnsupportedTransport(
            "Unix socket transport not compiled - enable 'unix' feature".to_string(),
        ))
    }

    /// Initialize TurboMCP client for ChildProcess transport - World-class implementation
    async fn initialize_child_process_client(
        connection: &Arc<ManagedConnection>,
        command: &str,
        args: &[String],
        working_directory: Option<&str>,
        sampling_handler: Arc<StudioSamplingHandler>,
        elicitation_handler: Arc<StudioElicitationHandler>,
    ) -> McpResult<McpTransportClient> {
        tracing::info!("Creating world-class TurboMCP ChildProcessTransport client...");

        // Create ChildProcessConfig with enterprise-grade settings
        let env_vars: Vec<(String, String)> = {
            let mut env = connection
                .config
                .environment_variables
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect::<Vec<_>>();

            // Suppress logging to prevent stdout pollution in MCP protocol
            env.push(("RUST_LOG".to_string(), "".to_string()));

            // Log the final environment that will be passed
            tracing::info!("  Final environment variables that will be passed to process:");
            for (k, v) in &env {
                tracing::info!("    {}={}", k, v);
            }

            env
        };

        let config = ChildProcessConfig {
            command: command.to_string(),
            args: args.to_vec(),
            working_directory: working_directory.map(|s| s.to_string()),
            environment: Some(env_vars),
            startup_timeout: std::time::Duration::from_secs(30),
            shutdown_timeout: std::time::Duration::from_secs(10),
            max_message_size: 10 * 1024 * 1024, // 10MB
            buffer_size: 8192,
            kill_on_drop: true,
        };

        // Create the ChildProcessTransport
        let transport = ChildProcessTransport::new(config);

        // Build TurboMCP client with enterprise connection config
        let connection_config = Configuration::create_enterprise_connection_config();
        Configuration::configure_client_capabilities();
        let client = Configuration::build_client_with_capabilities(transport, connection_config);

        // Initialize with custom error handling for ChildProcess-specific issues
        let init_result = client.initialize().await.map_err(|e| {
            let error_msg = e.to_string();
            if error_msg.contains("Invalid JSON-RPC response")
                || error_msg.contains("expected value at line 1 column 1")
            {
                McpStudioError::ConnectionFailed(format!(
                    "Server is not responding with valid JSON-RPC. This usually means:\n\
                        • The server is not an MCP server\n\
                        • The server is outputting logs or other text to stdout\n\
                        • The server path is incorrect or the server failed to start\n\
                        \nOriginal error: {}",
                    error_msg
                ))
            } else if error_msg.contains("No such file or directory")
                || error_msg.contains("not found")
            {
                McpStudioError::ConnectionFailed(format!(
                    "Server executable not found. Please check:\n\
                        • The path '{}' is correct\n\
                        • The server executable exists and is built\n\
                        • You have permission to execute the file\n\
                        \nOriginal error: {}",
                    command, error_msg
                ))
            } else {
                McpStudioError::ConnectionFailed(format!(
                    "Failed to initialize MCP client: {}",
                    error_msg
                ))
            }
        })?;

        tracing::info!(
            "Server info: {} v{}",
            init_result.server_info.name,
            init_result.server_info.version
        );

        // Register capabilities and bidirectional handlers (DRY helper)
        Initialization::register_capabilities_and_handlers(
            &client,
            &init_result,
            connection,
            "ChildProcess",
            sampling_handler,
            elicitation_handler,
        );

        Ok(McpTransportClient::ChildProcess(client))
    }

    /// Initialize TurboMCP HTTP/SSE client transport (TurboMCP 2.0 DOGFOODING)
    #[cfg(feature = "http")]
    async fn initialize_http_client(
        connection: &Arc<ManagedConnection>,
        url: &str,
        sampling_handler: Arc<StudioSamplingHandler>,
        elicitation_handler: Arc<StudioElicitationHandler>,
    ) -> McpResult<Client<StreamableHttpClientTransport>> {
        tracing::info!(
            "🔗 Initializing TurboMCP Streamable HTTP client for URL: {}",
            url
        );

        // Extract custom headers from TransportConfig::Http variant
        let custom_headers = match &connection.config.transport_config {
            crate::types::server_types::TransportConfig::Http { headers, .. } => headers.clone(),
            _ => std::collections::HashMap::new(),
        };

        // Parse URL to separate base_url and endpoint_path
        // Examples:
        // - "http://localhost:8080/mcp" → base_url: "http://localhost:8080", endpoint_path: "/mcp"
        // - "http://localhost:8080" → base_url: "http://localhost:8080", endpoint_path: "" (will use default)
        let parsed_url = url::Url::parse(url).map_err(|e| {
            let error_msg = format!("Invalid HTTP URL '{}': {}", url, e);
            tracing::error!("{}", error_msg);
            McpStudioError::ConnectionFailed(error_msg)
        })?;

        let base_url = format!(
            "{}://{}",
            parsed_url.scheme(),
            parsed_url.host_str().ok_or_else(|| {
                let error_msg = format!("Missing host in HTTP URL: {}", url);
                tracing::error!("{}", error_msg);
                McpStudioError::ConnectionFailed(error_msg)
            })?
        );

        // Add port if present (and not default for scheme)
        let base_url = if let Some(port) = parsed_url.port() {
            format!("{}:{}", base_url, port)
        } else {
            base_url
        };

        // Extract path as endpoint_path, default to "/mcp" if empty
        let endpoint_path = if parsed_url.path().is_empty() || parsed_url.path() == "/" {
            "/mcp".to_string()
        } else {
            parsed_url.path().to_string()
        };

        tracing::info!(
            "🔗 Parsed HTTP URL: base_url='{}', endpoint_path='{}'",
            base_url,
            endpoint_path
        );

        // Create HTTP/SSE transport (MCP 2025-06-18 compliant)
        let config = StreamableHttpClientConfig {
            base_url,
            endpoint_path,
            timeout: std::time::Duration::from_secs(30),
            headers: custom_headers, // Use custom headers from configuration
            ..Default::default()
        };
        let transport = StreamableHttpClientTransport::new(config);

        // Build client with enterprise connection config
        let connection_config = Configuration::create_enterprise_connection_config();
        Configuration::configure_client_capabilities();
        let client = Configuration::build_client_with_capabilities(transport, connection_config);

        // Finalize with init handshake and handler registration (DRY helper)
        Initialization::finalize_client_initialization(
            client,
            connection,
            "HTTP/SSE",
            sampling_handler,
            elicitation_handler,
        )
        .await
    }

    /// Initialize TurboMCP client for WebSocket bidirectional transport
    #[cfg(feature = "websocket")]
    async fn initialize_websocket_client(
        connection: &Arc<ManagedConnection>,
        url: &str,
        _headers: &std::collections::HashMap<String, String>,
        sampling_handler: Arc<StudioSamplingHandler>,
        elicitation_handler: Arc<StudioElicitationHandler>,
    ) -> McpResult<Client<WebSocketBidirectionalTransport>> {
        tracing::info!(
            "🔗 Initializing TurboMCP WebSocket bidirectional transport for URL: {}",
            url
        );

        // Create WebSocket bidirectional config with enterprise settings
        // Note: Headers are not supported by WebSocket bidirectional transport
        use turbomcp_transport::websocket_bidirectional::ReconnectConfig;

        let reconnect_config = ReconnectConfig::new()
            .with_enabled(true)
            .with_max_retries(5)
            .with_initial_delay(std::time::Duration::from_millis(500))
            .with_max_delay(std::time::Duration::from_secs(30))
            .with_backoff_factor(2.0);

        let config = WebSocketBidirectionalConfig::client(url.to_string())
            .with_keep_alive_interval(std::time::Duration::from_secs(30))
            .with_elicitation_timeout(std::time::Duration::from_secs(120))
            .with_max_concurrent_elicitations(10)
            .with_reconnect_config(reconnect_config);

        tracing::info!(
            "🔗 WebSocket config: keep_alive=30s, elicitation_timeout=120s, max_elicitations=10, reconnect=enabled"
        );

        // Create bidirectional transport
        let transport = WebSocketBidirectionalTransport::new(config)
            .await
            .map_err(|e| {
                let error_msg = format!(
                    "Failed to create WebSocket bidirectional transport for {}: {}",
                    connection.config.name, e
                );
                tracing::error!("{}", error_msg);
                McpStudioError::ConnectionFailed(error_msg)
            })?;

        // Connect the transport
        transport.connect().await.map_err(|e| {
            let error_msg = format!(
                "Failed to connect WebSocket bidirectional transport for {}: {}",
                connection.config.name, e
            );
            tracing::error!("{}", error_msg);
            McpStudioError::ConnectionFailed(error_msg)
        })?;

        tracing::info!("✅ WebSocket bidirectional transport connected successfully");

        // Build client with enterprise connection config
        let connection_config = Configuration::create_enterprise_connection_config();
        Configuration::configure_client_capabilities();
        let client = Configuration::build_client_with_capabilities(transport, connection_config);

        // Finalize with init handshake and handler registration (DRY helper)
        Initialization::finalize_client_initialization(
            client,
            connection,
            "WebSocket Bidirectional",
            sampling_handler,
            elicitation_handler,
        )
        .await
    }

    /// Initialize TurboMCP client for TCP transport using TurboMCP 1.0.8
    #[cfg(feature = "tcp")]
    async fn initialize_tcp_client(
        connection: &Arc<ManagedConnection>,
        host: &str,
        port: u16,
        sampling_handler: Arc<StudioSamplingHandler>,
        elicitation_handler: Arc<StudioElicitationHandler>,
    ) -> McpResult<Client<TcpTransport>> {
        tracing::info!("Establishing TurboMCP TCP connection to: {}:{}", host, port);

        // Create TCP transport
        let address = format!("{}:{}", host, port);
        let socket_addr: std::net::SocketAddr = address.parse().map_err(|e| {
            let error_msg = format!("Invalid TCP address {}: {}", address, e);
            tracing::error!("{}", error_msg);
            McpStudioError::ConnectionFailed(error_msg)
        })?;
        let transport = TcpTransport::new_client(socket_addr, socket_addr);

        // Build client with enterprise connection config
        let connection_config = Configuration::create_enterprise_connection_config();
        Configuration::configure_client_capabilities();
        let client = Configuration::build_client_with_capabilities(transport, connection_config);

        // Finalize with init handshake and handler registration (DRY helper)
        Initialization::finalize_client_initialization(
            client,
            connection,
            "TCP",
            sampling_handler,
            elicitation_handler,
        )
        .await
    }

    /// Initialize TurboMCP client for Unix socket transport using TurboMCP 1.0.8
    #[cfg(feature = "unix")]
    async fn initialize_unix_client(
        connection: &Arc<ManagedConnection>,
        path: &str,
        sampling_handler: Arc<StudioSamplingHandler>,
        elicitation_handler: Arc<StudioElicitationHandler>,
    ) -> McpResult<Client<UnixTransport>> {
        tracing::info!("Establishing TurboMCP Unix socket connection to: {}", path);

        // Create Unix socket transport
        let socket_path = std::path::PathBuf::from(path);
        let transport = UnixTransport::new_client(socket_path);

        // Build client with enterprise connection config
        let connection_config = Configuration::create_enterprise_connection_config();
        Configuration::configure_client_capabilities();
        let client = Configuration::build_client_with_capabilities(transport, connection_config);

        // Finalize with init handshake and handler registration (DRY helper)
        Initialization::finalize_client_initialization(
            client,
            connection,
            "Unix",
            sampling_handler,
            elicitation_handler,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_transport_layer_module_exists() {
        // Smoke test - module compiles
    }

    // TODO(testing): Add integration tests with mock transports
    // - Test each transport type initialization
    // - Test feature flag behavior
    // - Test error handling (file not found, permission denied, etc.)
    // - Test handler registration
    // - Test connection state updates
}
