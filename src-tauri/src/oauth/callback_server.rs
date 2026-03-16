/// Local HTTP server for OAuth 2.0/2.1 redirect callbacks
///
/// Binds to a random OS-assigned port on 127.0.0.1 to capture OAuth
/// authorization codes after user authorization.  Using port 0 avoids the
/// fixed-port conflict risk that existed with the previous hardcoded port 8080.
///
/// Flow:
/// 1. User authorizes in browser at auth server
/// 2. Auth server redirects to http://localhost:{actual_port}/callback?code=...&state=...
/// 3. This server captures the code and state
/// 4. Returns success page to user
/// 5. Notifies waiting OAuth flow via oneshot channel

use axum::{extract::Query, response::Html, routing::get, Router};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use turbomcp_protocol::{Error as McpError, ErrorKind, Result as McpResult};

/// Query parameters from OAuth callback
#[derive(serde::Deserialize, Debug)]
pub struct CallbackParams {
    /// Authorization code from auth server
    pub code: String,
    /// State parameter for CSRF protection
    pub state: String,
    /// Optional error code
    pub error: Option<String>,
    /// Optional error description
    pub error_description: Option<String>,
}

/// Local HTTP server for OAuth redirects
///
/// Listens on a random available port on 127.0.0.1/callback for OAuth
/// authorization responses.  The actual port is chosen by the OS at bind time
/// (port 0 strategy) so that there are no conflicts with other processes.
/// Uses a oneshot channel to notify the OAuth flow manager when a callback
/// is received.
///
/// Note: Only one OAuth flow can be pending at a time. Starting a new flow
/// while one is active will cancel the previous flow.
pub struct CallbackServer {
    /// Actual port chosen by the OS after binding (0 until the server starts).
    port: Arc<Mutex<u16>>,
    /// Pending callback sender (only one flow can be active at a time)
    pending_callback:
        Arc<Mutex<Option<oneshot::Sender<Result<(String, String), (String, String)>>>>>,
    /// Flag to track if server is running
    is_running: Arc<Mutex<bool>>,
}

impl CallbackServer {
    /// Create a new callback server.
    ///
    /// The actual listening port is determined at startup time when the OS
    /// assigns a free port.  Call `redirect_uri()` after `wait_for_callback()`
    /// (or after `start_server()`) to obtain the URI with the concrete port.
    pub fn new() -> Self {
        Self {
            port: Arc::new(Mutex::new(0)),
            pending_callback: Arc::new(Mutex::new(None)),
            is_running: Arc::new(Mutex::new(false)),
        }
    }

    /// Wait for OAuth callback
    ///
    /// Starts the server (if not already running) and waits for an OAuth callback.
    /// Returns the authorization code and state parameter, or an error if the callback
    /// contains an error response.
    ///
    /// # Returns
    /// * `Ok((code, state))` - Authorization code and state parameter
    /// * `Err((error, description))` - Error code and description from auth server
    pub async fn wait_for_callback(&self) -> McpResult<(String, String)> {
        let (tx, rx) = oneshot::channel();
        *self.pending_callback.lock() = Some(tx);

        // Start server if not already running
        self.start_server().await?;

        // Wait for callback
        rx.await
            .map_err(|e| McpError::new(ErrorKind::Internal, format!("Callback receive failed: {}", e)))?
            .map_err(|(error, description)| {
                McpError::authentication(format!("OAuth error: {} - {}", error, description))
            })
    }

    /// Start the HTTP server (crate-visible for pre-binding before building the auth URL).
    ///
    /// Binds to 127.0.0.1:0 so the OS assigns a free port, then records the
    /// actual port for use in redirect URIs.  Spawns an Axum server in the
    /// background if not already running.  Safe to call multiple times; the
    /// second and subsequent calls are no-ops if the server is already running.
    pub(crate) async fn start_server_internal(&self) -> McpResult<()> {
        self.start_server().await
    }

    async fn start_server(&self) -> McpResult<()> {
        // Check if already running — do NOT set is_running here to avoid TOCTOU:
        // the flag is only set to true after the bind succeeds.
        {
            let is_running = self.is_running.lock();
            if *is_running {
                return Ok(());
            }
        }

        let pending = self.pending_callback.clone();

        let app = Router::new().route(
            "/callback",
            get(
                move |Query(params): Query<CallbackParams>| async move {
                    tracing::info!(
                        "OAuth callback received: code={}, state={}, error={:?}",
                        if params.code.is_empty() {
                            "none"
                        } else {
                            "present"
                        },
                        &params.state,
                        params.error
                    );

                    // Send callback data to waiting flow
                    if let Some(tx) = pending.lock().take() {
                        if let Some(error) = params.error {
                            let description =
                                params.error_description.unwrap_or_else(|| "Unknown error".to_string());
                            let _ = tx.send(Err((error.clone(), description.clone())));

                            // Return error page
                            return Html(format!(
                                r#"
                                <!DOCTYPE html>
                                <html>
                                <head>
                                    <title>OAuth Error</title>
                                    <style>
                                        body {{
                                            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
                                            display: flex;
                                            justify-content: center;
                                            align-items: center;
                                            height: 100vh;
                                            margin: 0;
                                            background: linear-gradient(135deg, #f56565 0%, #c53030 100%);
                                        }}
                                        .container {{
                                            background: white;
                                            border-radius: 8px;
                                            padding: 2rem;
                                            box-shadow: 0 4px 6px rgba(0,0,0,0.1);
                                            text-align: center;
                                            max-width: 500px;
                                        }}
                                        h1 {{
                                            color: #c53030;
                                            margin: 0 0 1rem 0;
                                        }}
                                        p {{
                                            color: #6b7280;
                                            margin: 0.5rem 0;
                                        }}
                                        .error-code {{
                                            font-family: monospace;
                                            background: #fee;
                                            padding: 0.5rem;
                                            border-radius: 4px;
                                            margin-top: 1rem;
                                        }}
                                    </style>
                                </head>
                                <body>
                                    <div class="container">
                                        <h1> Authorization Failed</h1>
                                        <p>The OAuth authorization was not successful.</p>
                                        <div class="error-code">
                                            <strong>Error:</strong> {}<br>
                                            <strong>Description:</strong> {}
                                        </div>
                                        <p style="margin-top: 1rem;">You can close this window and return to TurboMCP Studio.</p>
                                    </div>
                                    <script>
                                        setTimeout(() => window.close(), 5000);
                                    </script>
                                </body>
                                </html>
                                "#,
                                error,
                                description
                            ));
                        } else {
                            let _ = tx.send(Ok((params.code.clone(), params.state.clone())));
                        }
                    }

                    // Return success page
                    Html(
                        r#"
                        <!DOCTYPE html>
                        <html>
                        <head>
                            <title>OAuth Success</title>
                            <style>
                                body {
                                    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
                                    display: flex;
                                    justify-content: center;
                                    align-items: center;
                                    height: 100vh;
                                    margin: 0;
                                    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                                }
                                .container {
                                    background: white;
                                    border-radius: 8px;
                                    padding: 2rem;
                                    box-shadow: 0 4px 6px rgba(0,0,0,0.1);
                                    text-align: center;
                                }
                                h1 {
                                    color: #10b981;
                                    margin: 0 0 1rem 0;
                                }
                                p {
                                    color: #6b7280;
                                    margin: 0;
                                }
                                .spinner {
                                    display: inline-block;
                                    width: 20px;
                                    height: 20px;
                                    border: 3px solid rgba(16, 185, 129, 0.3);
                                    border-radius: 50%;
                                    border-top-color: #10b981;
                                    animation: spin 1s ease-in-out infinite;
                                }
                                @keyframes spin {
                                    to { transform: rotate(360deg); }
                                }
                            </style>
                        </head>
                        <body>
                            <div class="container">
                                <h1> Authorization Successful</h1>
                                <p>Exchanging authorization code for access token...</p>
                                <div style="margin-top: 1rem;">
                                    <div class="spinner"></div>
                                </div>
                                <p style="margin-top: 1rem; font-size: 0.875rem;">
                                    This window will close automatically.
                                </p>
                            </div>
                            <script>
                                setTimeout(() => window.close(), 3000);
                            </script>
                        </body>
                        </html>
                        "#.to_string(),
                    )
                },
            ),
        );

        // Bind to port 0 so the OS assigns a free port, eliminating the fixed
        // port 8080 conflict risk.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.map_err(|e| {
            McpError::new(ErrorKind::Internal, format!("Failed to bind OAuth callback server: {}", e))
        })?;

        // Bind succeeded — mark the server as running only now to avoid TOCTOU
        // (is_running stayed false while bind was in progress; if bind had
        // failed, no caller would observe a stuck is_running=true).
        *self.is_running.lock() = true;

        // Record the actual port assigned by the OS so redirect_uri() returns
        // the correct value before the browser redirect happens.
        let actual_port = listener
            .local_addr()
            .map_err(|e| McpError::new(ErrorKind::Internal, format!("Failed to get local address: {}", e)))?
            .port();
        *self.port.lock() = actual_port;

        tracing::info!("OAuth callback server listening on 127.0.0.1:{}", actual_port);

        // Spawn server in background with a 5-minute hard timeout.
        // If no callback arrives within 300 seconds the listener is dropped
        // and is_running is reset, allowing a future flow to bind a new port.
        let is_running = self.is_running.clone();
        tokio::spawn(async move {
            let serve_future = axum::serve(listener, app);
            match tokio::time::timeout(Duration::from_secs(300), serve_future).await {
                Ok(Err(e)) => {
                    tracing::error!("OAuth callback server error: {}", e);
                }
                Err(_) => {
                    tracing::warn!(
                        "OAuth callback server timed out after 300 seconds — \
                         shutting down listener and resetting state"
                    );
                }
                Ok(Ok(())) => {}
            }
            *is_running.lock() = false;
        });

        Ok(())
    }

    /// Get the redirect URI for this callback server.
    ///
    /// Returns the full URI that should be registered with OAuth providers.
    /// The port reflects the OS-assigned value and is only valid after the
    /// server has been started via `wait_for_callback()`.
    pub fn redirect_uri(&self) -> String {
        format!("http://localhost:{}/callback", *self.port.lock())
    }

    /// Get the actual port the server is listening on.
    ///
    /// Returns 0 if the server has not been started yet.
    pub fn port(&self) -> u16 {
        *self.port.lock()
    }
}

impl Default for CallbackServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redirect_uri_before_start() {
        // Before start_server() is called the port is 0.
        let server = CallbackServer::new();
        assert_eq!(server.redirect_uri(), "http://localhost:0/callback");
        assert_eq!(server.port(), 0);
    }
}
