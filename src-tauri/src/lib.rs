mod commands;
mod database;
mod error;
mod hitl_sampling;
mod llm_config;
mod mcp_client;
mod types;
mod workflow_engine;
use database::Database;
use error::McpStudioError;
use hitl_sampling::HITLSamplingManager;
use llm_config::LLMConfigManager;
use mcp_client::McpClientManager;
use std::sync::Arc;
use tauri::{Emitter, Manager};

/// Application state shared across all commands
#[derive(Clone)]
pub struct AppState {
    pub mcp_manager: Arc<McpClientManager>,
    pub llm_config: Arc<LLMConfigManager>,
    pub hitl_sampling: Arc<HITLSamplingManager>,
    pub database: Arc<tokio::sync::RwLock<Option<Arc<Database>>>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Initialize managers immediately (lightweight)
            let (mcp_manager, mut event_receiver) = McpClientManager::new(app_handle.clone());
            let mcp_manager = Arc::new(mcp_manager);

            let llm_config = Arc::new(LLMConfigManager::new());

            // Initialize HITL sampling manager with LLM config
            let (hitl_sampling, mut sampling_event_receiver) = HITLSamplingManager::new(llm_config.clone());
            let hitl_sampling = Arc::new(hitl_sampling);

            // Database will be initialized asynchronously and set later
            let database = Arc::new(tokio::sync::RwLock::new(None));

            // Store lightweight state immediately to unblock UI
            app.manage(AppState {
                mcp_manager: mcp_manager.clone(),
                llm_config: llm_config.clone(),
                hitl_sampling: hitl_sampling.clone(),
                database: database.clone(),
            });

            // Emit immediate ready event so UI can start working
            let _ = app_handle.emit("app-early-ready", ());

            // Defer heavy initialization to background task
            let app_handle_clone = app_handle.clone();
            let database_clone = database.clone();
            tauri::async_runtime::spawn(async move {
                tracing::info!("Starting background initialization");

                // Heavy initialization in background
                // Use simple path without spaces to avoid SQLite issues
                let app_data_dir = match std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
                    Ok(home) => std::path::PathBuf::from(home).join(".turbomcpstudio"),
                    Err(_) => {
                        // Final fallback
                        std::path::PathBuf::from(".").join(".turbomcpstudio")
                    }
                };

                tracing::info!("App data directory: {:?}", app_data_dir);

                // Create directory if it doesn't exist
                if let Err(e) = std::fs::create_dir_all(&app_data_dir) {
                    tracing::error!("Failed to create data directory: {}", e);
                    return;
                }

                tracing::info!("Data directory created successfully");

                // Initialize database with robust fallback strategy
                let db_path = app_data_dir.join("mcp_studio.db");
                tracing::info!("Attempting to initialize database at: {:?}", db_path);

                let database = match Database::new_with_full_migration(db_path.to_str().unwrap()).await {
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
                                Database::new_with_full_migration(fallback_path.to_str().unwrap()).await
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
                                        // Emit warning to frontend
                                        let _ = app_handle_clone.emit("database-fallback", "in-memory");
                                        db
                                    }
                                    Err(e) => {
                                        tracing::error!("💥 Critical error: Even in-memory database failed: {}", e);
                                        // Emit critical error to frontend
                                        let _ = app_handle_clone.emit("database-critical-error", e.to_string());
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

                tracing::info!("MCP Studio background initialization complete");

                // Emit ready event to frontend
                let _ = app_handle_clone.emit("app-ready", ());
                tracing::info!("app-ready event emitted");
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
            commands::reject_sampling_request,
            commands::process_hitl_sampling_request,
            commands::test_sampling_request,
            // LLM API commands (avoids CORS issues)
            commands::fetch_llm_models,
            commands::llm_completion_request,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
