mod commands;
mod database;
mod error;
mod hitl_sampling;
mod llm_config;
mod llm_providers;
mod mcp_client;
pub mod registry;
mod types;
mod workflow_engine;
use database::Database;
use error::McpStudioError;
use hitl_sampling::HITLSamplingManager;
use llm_config::LLMConfigManager;
use mcp_client::McpClientManager;
use std::sync::Arc;
use tauri::{Emitter, Listener, Manager};
use types::InitializationError;

/// Application state shared across all commands
#[derive(Clone)]
pub struct AppState {
    pub mcp_manager: Arc<McpClientManager>,
    pub llm_config: Arc<LLMConfigManager>,
    pub hitl_sampling: Arc<HITLSamplingManager>,
    pub database: Arc<tokio::sync::RwLock<Option<Arc<Database>>>>,
    /// Issue #18 fix: Store monitoring loop handle for graceful shutdown
    pub monitoring_handle: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Single-instance plugin MUST be initialized first to work properly
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            tracing::info!("Single instance plugin triggered");
            tracing::debug!("Arguments: {:?}", args);
            tracing::debug!("Current directory: {:?}", cwd);

            // Focus the main window when another instance is attempted
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
                let _ = window.unminimize();
                tracing::info!("✅ Focused main window for single instance");
            }
        }))
        // Initialize logging with tauri-plugin-log
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .level_for("turbomcpstudio", log::LevelFilter::Debug)
                .level_for("turbomcp", log::LevelFilter::Debug)
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("mcp-studio".to_string()),
                    }),
                ])
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        // Window state plugin - remembers window position and size
        .plugin(tauri_plugin_window_state::Builder::default().build())
        // OS info plugin - provides system information
        .plugin(tauri_plugin_os::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Issue #5 fix: Set up panic hook to catch crashes and notify frontend
            // This replaces the frontend timeout as the primary failure detection mechanism
            let panic_app_handle = app_handle.clone();
            std::panic::set_hook(Box::new(move |panic_info| {
                // Extract panic details
                let panic_message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic occurred".to_string()
                };

                // Extract panic location
                let location = if let Some(location) = panic_info.location() {
                    format!("{}:{}:{}", location.file(), location.line(), location.column())
                } else {
                    "unknown location".to_string()
                };

                // Log panic details
                tracing::error!("💥 PANIC: {} at {}", panic_message, location);
                tracing::error!("Panic info: {:?}", panic_info);

                // Emit critical error to frontend
                let _ = panic_app_handle.emit("initialization-error", InitializationError {
                    critical: true,
                    component: "runtime".to_string(),
                    message: format!("Application panic: {} ({})", panic_message, location),
                    fallback_used: None,
                    user_action: Some("Please restart the application and check logs".to_string()),
                });

                // Emit app-ready to unblock UI (same pattern as critical database errors)
                let _ = panic_app_handle.emit("app-ready", ());

                // Show main window even on panic so user can see error (Tauri v2 pattern)
                if let Some(window) = panic_app_handle.get_webview_window("main") {
                    let _ = window.show();
                }
            }));

            tracing::info!("✅ Panic hook installed - will emit events on crashes");

            // Initialize LLM config first (needed by MCP manager)
            let llm_config = Arc::new(LLMConfigManager::new());

            // Database will be initialized asynchronously and set later
            let database = Arc::new(tokio::sync::RwLock::new(None));

            // Initialize managers immediately (lightweight)
            let (mcp_manager, mut event_receiver) = McpClientManager::new(
                app_handle.clone(),
                llm_config.clone(),
                database.clone(),
            );
            let mcp_manager = Arc::new(mcp_manager);

            // Initialize HITL sampling manager with LLM config
            let (hitl_sampling, mut sampling_event_receiver) = HITLSamplingManager::new(llm_config.clone());
            let hitl_sampling = Arc::new(hitl_sampling);

            // Issue #18 fix: Create monitoring handle storage for graceful shutdown
            let monitoring_handle = Arc::new(tokio::sync::Mutex::new(None));

            // Store lightweight state immediately to unblock UI
            app.manage(AppState {
                mcp_manager: mcp_manager.clone(),
                llm_config: llm_config.clone(),
                hitl_sampling: hitl_sampling.clone(),
                database: database.clone(),
                monitoring_handle: monitoring_handle.clone(),
            });

            // Issue #5 fix: Proper handshake pattern to prevent race condition
            // Create a oneshot channel to signal when frontend is ready
            // This ensures app-ready is only emitted AFTER frontend listeners are registered
            let (frontend_ready_tx, frontend_ready_rx) = tokio::sync::oneshot::channel::<()>();

            let app_handle_for_ready_handshake = app_handle.clone();
            app.once("frontend-ready", move |_event| {
                tracing::info!("🔔 Received frontend-ready signal, emitting app-early-ready");
                let _ = app_handle_for_ready_handshake.emit("app-early-ready", ());
                tracing::info!("✅ Emitted app-early-ready event (handshake pattern - deterministic)");

                // Signal the background task that frontend is ready for app-ready event
                let _ = frontend_ready_tx.send(());
            });

            // Defer heavy initialization to background task
            let app_handle_clone = app_handle.clone();
            let database_clone = database.clone();
            let mcp_manager_clone = mcp_manager.clone();
            let monitoring_handle_clone = monitoring_handle.clone();  // Issue #18 fix
            tauri::async_runtime::spawn(async move {
                // Issue #5 fix: Wait for frontend to signal readiness before proceeding
                // This prevents race condition where app-ready fires before listeners are registered
                match frontend_ready_rx.await {
                    Ok(_) => tracing::info!("Frontend ready signal received, proceeding with initialization"),
                    Err(_) => tracing::warn!("Frontend ready channel closed unexpectedly, continuing anyway"),
                }
                let bg_init_start = std::time::Instant::now();
                tracing::info!("Starting background initialization");

                // Issue #18 fix: Start monitoring and STORE the handle for graceful shutdown
                let handle = mcp_manager_clone.start_monitoring();
                *monitoring_handle_clone.lock().await = Some(handle);
                tracing::info!("MCP connection monitoring loop started (handle stored for shutdown)");

                // Use Tauri's native path APIs for platform-agnostic directory resolution
                let app_data_dir = match app_handle_clone.path().app_data_dir() {
                    Ok(path) => path,
                    Err(e) => {
                        tracing::error!("Failed to get app data directory: {}, using fallback", e);
                        std::path::PathBuf::from(".").join(".turbomcpstudio")
                    }
                };

                tracing::info!("App data directory: {:?}", app_data_dir);

                // Create directory if it doesn't exist
                // Issue #16 fix: Emit error instead of early return to prevent frontend hang
                let directory_created = if let Err(e) = std::fs::create_dir_all(&app_data_dir) {
                    tracing::error!("Failed to create data directory: {}", e);

                    // Emit initialization error to frontend
                    let _ = app_handle_clone.emit("initialization-error", InitializationError {
                        critical: false,
                        component: "filesystem".to_string(),
                        message: format!("Cannot create app directory: {}", e),
                        fallback_used: Some("in-memory database".to_string()),
                        user_action: Some("Check folder permissions or disk space".to_string()),
                    });

                    false  // Will use in-memory database as fallback
                } else {
                    tracing::info!("Data directory created successfully");
                    true
                };

                // Initialize database with robust fallback strategy
                let db_path = if directory_created {
                    app_data_dir.join("mcp_studio.db")
                } else {
                    // Directory creation failed, skip file-based database
                    std::path::PathBuf::from(":memory:")
                };
                tracing::info!("Attempting to initialize database at: {:?}", db_path);

                // Convert path to string, handling invalid Unicode gracefully
                let db_path_str = match db_path.to_str() {
                    Some(s) => s,
                    None => {
                        tracing::error!("Database path contains invalid Unicode characters");
                        ":memory:" // Fallback to in-memory if path is invalid
                    }
                };

                let database = match Database::new_with_full_migration(db_path_str).await {
                    Ok(db) => {
                        tracing::info!("✅ Successfully using persistent database at {:?}", db_path);
                        db
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to initialize persistent database: {}", e);
                        tracing::warn!("🔄 Attempting fallback strategies...");

                        // Try alternative database location in user's home directory
                        let home_fallback = match std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
                            Ok(home) => {
                                let fallback_path = std::path::PathBuf::from(home).join(".turbomcpstudio").join("mcp_studio.db");
                                tracing::info!("🏠 Trying home directory fallback: {:?}", fallback_path);
                                let fallback_path_str = match fallback_path.to_str() {
                                    Some(s) => s,
                                    None => {
                                        tracing::error!("Fallback path contains invalid Unicode characters");
                                        ":memory:" // Final fallback
                                    }
                                };
                                Database::new_with_full_migration(fallback_path_str).await
                            }
                            Err(_) => {
                                tracing::warn!("Cannot determine home directory");
                                Err(McpStudioError::ConfigError("No home directory found".to_string()))
                            }
                        };

                        match home_fallback {
                            Ok(db) => {
                                tracing::info!("✅ Using home directory fallback database");
                                db
                            }
                            Err(e) => {
                                tracing::error!("❌ Home directory fallback failed: {}", e);
                                tracing::warn!("💾 Using in-memory database as final fallback");

                                match Database::new_with_full_migration(":memory:").await {
                                    Ok(db) => {
                                        tracing::warn!("⚠️ Using in-memory database - data will not persist between sessions!");
                                        // Emit warning to frontend using InitializationError
                                        let _ = app_handle_clone.emit("initialization-error", InitializationError {
                                            critical: false,
                                            component: "database".to_string(),
                                            message: "Using in-memory database - data will not persist".to_string(),
                                            fallback_used: Some("in-memory".to_string()),
                                            user_action: None,
                                        });
                                        db
                                    }
                                    Err(e) => {
                                        tracing::error!("💥 Critical error: Even in-memory database failed: {}", e);

                                        // Issue #16 fix: Emit error AND app-ready to unblock frontend
                                        let _ = app_handle_clone.emit("initialization-error", InitializationError {
                                            critical: true,
                                            component: "database".to_string(),
                                            message: format!("Database initialization failed: {}", e),
                                            fallback_used: None,
                                            user_action: Some("Please restart the application".to_string()),
                                        });

                                        // Still emit app-ready to unblock UI (app won't work, but frontend won't hang)
                                        let _ = app_handle_clone.emit("app-ready", ());

                                        // Show main window so user can see critical error (Tauri v2 pattern)
                                        if let Some(window) = app_handle_clone.get_webview_window("main") {
                                            let _ = window.show();
                                        }
                                        return;
                                    }
                                }
                            }
                        }
                    }
                };

                tracing::info!("Database initialized successfully");

                // Store database once it's ready in AppState (wrapped in Arc)
                *database_clone.write().await = Some(Arc::new(database));

                // Initialize default LLM providers
                if let Err(e) = llm_config.initialize_default_providers().await {
                    tracing::error!("Failed to initialize LLM providers: {}", e);
                } else {
                    tracing::info!("LLM providers initialized");
                }

                let bg_init_duration = bg_init_start.elapsed();
                tracing::info!("✅ MCP Studio background initialization complete in {:?}", bg_init_duration);

                // Emit ready event to frontend
                let _ = app_handle_clone.emit("app-ready", ());
                tracing::info!("app-ready event emitted");

                // Show main window now that initialization is complete (Tauri v2 best practice)
                // Window starts hidden (visible: false in tauri.conf.json) to prevent white flash
                if let Some(window) = app_handle_clone.get_webview_window("main") {
                    if let Err(e) = window.show() {
                        tracing::error!("Failed to show main window: {}", e);
                    } else {
                        tracing::info!("✅ Main window shown");
                    }
                } else {
                    tracing::warn!("Main window not found, cannot show");
                }
            });

            // Note: Monitoring will be started lazily when first MCP connection is made

            // Handle connection events in background
            let app_handle_clone = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                while let Some(event) = event_receiver.recv().await {
                    // Emit events to frontend
                    let _ = app_handle_clone.emit("mcp-event", &event);
                }
            });

            // Handle HITL sampling events in background
            let app_handle_clone = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                while let Ok(event) = sampling_event_receiver.recv().await {
                    // Emit sampling events to frontend
                    let _ = app_handle_clone.emit("hitl-sampling-event", &event);
                }
            });

            tracing::info!("MCP Studio setup complete, background initialization started");
            Ok(())
        })
        // Issue #18 fix: Handle window close event on Rust side for cleanup
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                tracing::info!("🛑 Window close requested, shutting down background tasks...");

                // Get app state and shutdown background tasks
                let app_handle = window.app_handle();
                if let Some(state) = app_handle.try_state::<AppState>() {
                    tauri::async_runtime::block_on(async {
                        // Stop monitoring loop by aborting the task
                        if let Some(handle) = state.monitoring_handle.lock().await.take() {
                            handle.abort();
                            tracing::info!("✅ Monitoring loop stopped");
                        }

                        // Give tasks time to clean up gracefully
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                        tracing::info!("✅ Background tasks shutdown complete");
                    });
                }

                // Don't call api.prevent_close() - let window close naturally after cleanup
            }
        })
        .invoke_handler(tauri::generate_handler![
            // Server management commands
            commands::connect_server,
            commands::disconnect_server,
            commands::get_server_info,
            commands::list_servers,
            commands::create_server_config,
            commands::test_server_config,
            commands::get_server_templates,
            // Server persistence commands
            commands::save_server_config,
            commands::update_server_config,
            commands::load_server_configs,
            commands::delete_server_config,
            // MCP operation commands
            commands::call_tool,
            commands::list_tools,
            commands::list_prompts,
            commands::get_prompt,
            commands::list_resources,
            commands::read_resource,
            // Advanced MCP features
            commands::create_sampling_request,
            commands::send_elicitation_response,
            commands::get_elicitation_requests,
            // Protocol Inspector commands (used by ProtocolInspector.svelte)
            commands::get_message_history,
            commands::clear_message_history,
            commands::get_connection_metrics,
            commands::get_all_connection_metrics,
            commands::get_all_server_info,
            // LLM Configuration management
            commands::get_llm_config,
            commands::get_llm_provider_statuses,
            commands::set_llm_api_key,
            commands::remove_llm_api_key,
            commands::set_active_llm_provider,
            commands::update_llm_provider_config,
            commands::is_sampling_available,
            commands::validate_llm_config,
            // TurboMCP 1.0.10 features
            commands::get_completions,
            commands::list_filesystem_roots,
            commands::get_handler_status,
            // Collections & Workflow execution
            commands::execute_workflow,
            commands::get_workflow_execution,
            commands::stop_workflow_execution,
            commands::list_workflow_executions,
            commands::save_collection,
            commands::load_collection,
            commands::list_collections,
            commands::delete_collection,
            // Collection import/export
            commands::export_collection,
            commands::import_collection,
            commands::export_collection_to_file,
            commands::import_collection_from_file,
            commands::get_collection_templates,
            commands::create_collection_from_template,
            // HITL Sampling Commands
            commands::get_hitl_sampling_mode,
            commands::set_hitl_sampling_mode,
            commands::get_pending_sampling_requests,
            commands::get_completed_sampling_requests,
            commands::approve_sampling_request,
            commands::submit_manual_sampling_response,
            commands::reject_sampling_request,
            commands::process_hitl_sampling_request,
            commands::test_sampling_request,
            // LLM API commands (avoids CORS issues)
            commands::fetch_llm_models,
            commands::llm_completion_request,
            // Application utility commands
            commands::get_app_paths,
            commands::get_system_info,
            commands::shutdown_background_tasks,  // Issue #18 fix
            // Server Profile commands (enterprise server management)
            commands::create_server_profile,
            commands::update_server_profile,
            commands::delete_server_profile,
            commands::list_server_profiles,
            commands::get_server_profile,
            commands::get_profile_servers,
            commands::get_all_profile_server_relationships,
            commands::add_server_to_profile,
            commands::remove_server_from_profile,
            commands::activate_profile,
            commands::deactivate_profile,
            commands::get_active_profile,
            commands::get_active_profiles,  // Multi-profile support
            // Client Installation commands
            commands::detect_installed_clients,
            commands::install_servers_to_client,
            // Docker MCP Registry commands
            commands::fetch_registry_catalog,
            commands::refresh_registry_catalog,
            commands::search_registry_servers,
            commands::filter_registry_by_category,
            commands::get_registry_categories,
            commands::generate_client_config,
            commands::get_server_details,
            commands::add_server_from_registry,
            commands::check_docker_available,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
