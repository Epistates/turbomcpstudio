/// Local HTTP server for OAuth 2.0/2.1 redirect callbacks
///
/// Runs on localhost:8080 to capture OAuth authorization codes after user authorization.
/// This server handles the redirect from the authorization server back to the application.
///
/// Flow:
/// 1. User authorizes in browser at auth server
/// 2. Auth server redirects to http://localhost:8080/callback?code=...&state=...
/// 3. This server captures the code and state
/// 4. Returns success page to user
/// 5. Notifies waiting OAuth flow via oneshot channel

use axum::{extract::Query, response::Html, routing::get, Router};
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::oneshot;
use turbomcp_protocol::{Error as McpError, Result as McpResult};

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
/// Listens on localhost:8080/callback for OAuth authorization responses.
/// Uses a oneshot channel to notify the OAuth flow manager when a callback is received.
pub struct CallbackServer {
    /// Port to listen on (default: 8080)
    port: u16,
    /// Pending callback sender (only one flow can be active at a time)
    pending_callback:
        Arc<Mutex<Option<oneshot::Sender<Result<(String, String), (String, String)>>>>>,
    /// Flag to track if server is running
    is_running: Arc<Mutex<bool>>,
}

impl CallbackServer {
    /// Create a new callback server
    ///
    /// # Arguments
    /// * `port` - Port to listen on (typically 8080)
    pub fn new(port: u16) -> Self {
        Self {
            port,
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
            .map_err(|e| McpError::internal(format!("Callback receive failed: {}", e)))?
            .map_err(|(error, description)| {
                McpError::authentication(format!("OAuth error: {} - {}", error, description))
            })
    }

    /// Start the HTTP server
    ///
    /// Spawns an Axum server on localhost:{port} if not already running.
    /// The server handles GET requests to /callback and displays a success page.
    async fn start_server(&self) -> McpResult<()> {
        // Check if already running
        {
            let mut is_running = self.is_running.lock();
            if *is_running {
                return Ok(());
            }
            *is_running = true;
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
                                        <h1> Authorization Failed</h1>
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
                                <h1> Authorization Successful</h1>
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

        let addr = format!("127.0.0.1:{}", self.port);
        let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
            McpError::internal(format!("Failed to bind OAuth callback server to {}: {}", addr, e))
        })?;

        tracing::info!("OAuth callback server listening on {}", addr);

        // Spawn server in background
        let is_running = self.is_running.clone();
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("OAuth callback server error: {}", e);
            }
            *is_running.lock() = false;
        });

        Ok(())
    }

    /// Get the redirect URI for this callback server
    ///
    /// Returns the full URI that should be registered with OAuth providers.
    pub fn redirect_uri(&self) -> String {
        format!("http://localhost:{}/callback", self.port)
    }
}

impl Default for CallbackServer {
    fn default() -> Self {
        Self::new(8080)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redirect_uri() {
        let server = CallbackServer::new(8080);
        assert_eq!(server.redirect_uri(), "http://localhost:8080/callback");
    }

    #[test]
    fn test_custom_port() {
        let server = CallbackServer::new(3000);
        assert_eq!(server.redirect_uri(), "http://localhost:3000/callback");
    }
}
