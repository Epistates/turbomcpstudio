use crate::error::{McpResult, McpStudioError};
use crate::types::{collections::*, MessageHistory, ServerConfig};
use sqlx::{Pool, Row, Sqlite, SqlitePool};
use std::collections::HashMap;
use uuid::Uuid;

/// Database manager for MCP Studio
#[derive(Debug)]
pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    /// Get a reference to the database connection pool
    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    /// Create a new database connection with lightweight initialization
    pub async fn new(database_path: &str) -> McpResult<Self> {
        let database_url = if database_path == ":memory:" {
            "sqlite://:memory:".to_string()
        } else {
            // For file paths, SQLite URLs need proper platform-specific formatting
            let path = std::path::Path::new(database_path);
            if path.is_absolute() {
                // Use the path as-is for absolute paths (works on all platforms)
                // SQLx handles the platform-specific path format internally
                format!("sqlite://{}", database_path)
            } else {
                format!("sqlite://{}", database_path)
            }
        };
        let pool = SqlitePool::connect(&database_url).await?;

        let db = Self { pool };

        // Only create essential tables immediately, defer full migration
        db.create_essential_tables().await?;

        Ok(db)
    }

    /// Create a new database connection with full migrations (for background initialization)
    pub async fn new_with_full_migration(database_path: &str) -> McpResult<Self> {
        tracing::info!("Database initialization: path={}", database_path);

        // Handle in-memory database
        if database_path == ":memory:" {
            let database_url = "sqlite://:memory:".to_string();
            tracing::info!("Using in-memory database: {}", database_url);
            let pool = SqlitePool::connect(&database_url).await?;
            let db = Self { pool };
            db.run_migrations().await?;

            // ✅ FIXED: Clear active profile state on startup (in-memory path)
            match db.clear_active_profile_on_startup().await {
                Ok(_) => {
                    tracing::info!("Cleared active profile state for fresh startup (in-memory)");
                }
                Err(e) => {
                    tracing::warn!("Failed to clear active profile state (non-critical): {}", e);
                }
            }

            return Ok(db);
        }

        // Handle file-based database with robust directory creation
        let db_path = std::path::Path::new(database_path);
        tracing::info!("Database file path: {:?}", db_path);

        // Ensure parent directory exists with proper permissions
        if let Some(parent_dir) = db_path.parent() {
            tracing::info!("Ensuring parent directory exists: {:?}", parent_dir);
            if !parent_dir.exists() {
                match std::fs::create_dir_all(parent_dir) {
                    Ok(_) => {
                        tracing::info!("Created parent directory: {:?}", parent_dir);
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to create parent directory {:?}: {}",
                            parent_dir,
                            e
                        );
                        return Err(McpStudioError::ConfigError(format!(
                            "Cannot create database directory: {}",
                            e
                        )));
                    }
                }
            }

            // Verify directory is writable by attempting to create a test file
            let test_file = parent_dir.join(".test_write_access");
            match std::fs::write(&test_file, "test") {
                Ok(_) => {
                    // Clean up test file
                    let _ = std::fs::remove_file(&test_file);
                    tracing::info!("Directory is writable: {:?}", parent_dir);
                }
                Err(e) => {
                    tracing::error!("Directory not writable {:?}: {}", parent_dir, e);
                    return Err(McpStudioError::ConfigError(format!(
                        "Database directory not writable: {}",
                        e
                    )));
                }
            }
        }

        // Create SQLite URL - SQLx format for SQLite with creation flag
        let database_url = if database_path == ":memory:" {
            "sqlite://:memory:".to_string()
        } else {
            // For file paths, use sqlite:// prefix with create flag to ensure file creation
            format!("sqlite://{}?mode=rwc", database_path)
        };

        tracing::info!("Database URL: {}", database_url);
        tracing::info!("Database file exists: {}", db_path.exists());
        tracing::info!("Database path: {:?}", db_path);

        // Attempt connection with better error handling
        match SqlitePool::connect(&database_url).await {
            Ok(pool) => {
                tracing::info!("Successfully connected to database");
                let db = Self { pool };
                match db.run_migrations().await {
                    Ok(_) => {
                        tracing::info!("Database migrations completed successfully");

                        // ✅ FIXED: Clear active profile state on startup
                        // This ensures every app session starts fresh with no auto-connections
                        match db.clear_active_profile_on_startup().await {
                            Ok(_) => {
                                tracing::info!("Cleared active profile state for fresh startup");
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to clear active profile state (non-critical): {}",
                                    e
                                );
                            }
                        }

                        Ok(db)
                    }
                    Err(e) => {
                        tracing::error!("Database migration failed: {}", e);
                        Err(e)
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to connect to database at {}: {}", database_url, e);
                Err(McpStudioError::ConfigError(format!(
                    "Cannot connect to database file: {}",
                    e
                )))
            }
        }
    }

    /// Create only essential tables for immediate functionality
    async fn create_essential_tables(&self) -> McpResult<()> {
        // Only create the server configs table for basic functionality
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS server_configs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                transport_type TEXT NOT NULL,
                transport_config TEXT NOT NULL,
                environment_variables TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Run database migrations to set up schema
    async fn run_migrations(&self) -> McpResult<()> {
        let migration_start = std::time::Instant::now();
        tracing::info!("Starting database migrations");

        // Server configurations table
        let step_start = std::time::Instant::now();
        tracing::info!("Creating server_configs table");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS server_configs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                transport_type TEXT NOT NULL,
                transport_config TEXT NOT NULL,
                environment_variables TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        tracing::info!(
            "server_configs table created successfully ({:?})",
            step_start.elapsed()
        );

        // Enhanced Collections table with workflow support
        tracing::info!("Creating enhanced collections table");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS collections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                tags TEXT NOT NULL,
                workflow TEXT NOT NULL,
                variables TEXT NOT NULL,
                environment TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                created_by TEXT,
                version TEXT NOT NULL,
                last_run TEXT,
                run_count INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        tracing::info!("Enhanced collections table created successfully");

        // Workflow executions table for tracking execution history
        tracing::info!("Creating workflow_executions table");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workflow_executions (
                id TEXT PRIMARY KEY,
                collection_id TEXT NOT NULL,
                collection_version TEXT NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                status TEXT NOT NULL,
                step_results TEXT NOT NULL,
                final_variables TEXT NOT NULL,
                summary TEXT NOT NULL,
                environment_name TEXT NOT NULL,
                user_variables TEXT NOT NULL,
                FOREIGN KEY(collection_id) REFERENCES collections(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        tracing::info!("workflow_executions table created successfully");

        // Migration: Fix workflow_executions column name (results -> step_results)
        // This handles databases created with the incorrect column name
        tracing::info!("Checking for workflow_executions column migration");
        let column_check = sqlx::query(
            "SELECT COUNT(*) as count FROM pragma_table_info('workflow_executions') WHERE name = 'results'"
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = column_check {
            let has_old_column: i32 = row.get("count");
            if has_old_column > 0 {
                tracing::info!("Found old 'results' column, migrating to 'step_results'");

                // SQLite doesn't support ALTER TABLE RENAME COLUMN in older versions
                // We'll use the safe migration pattern: create temp table, copy data, swap
                sqlx::query(
                    r#"
                    CREATE TABLE workflow_executions_new (
                        id TEXT PRIMARY KEY,
                        collection_id TEXT NOT NULL,
                        collection_version TEXT NOT NULL,
                        started_at TEXT NOT NULL,
                        completed_at TEXT,
                        status TEXT NOT NULL,
                        step_results TEXT NOT NULL,
                        final_variables TEXT NOT NULL,
                        summary TEXT NOT NULL,
                        environment_name TEXT NOT NULL,
                        user_variables TEXT NOT NULL,
                        FOREIGN KEY(collection_id) REFERENCES collections(id) ON DELETE CASCADE
                    )
                    "#,
                )
                .execute(&self.pool)
                .await?;

                // Copy data from old table to new table
                sqlx::query(
                    r#"
                    INSERT INTO workflow_executions_new
                        (id, collection_id, collection_version, started_at, completed_at, status,
                         step_results, final_variables, summary, environment_name, user_variables)
                    SELECT id, collection_id, collection_version, started_at, completed_at, status,
                           results, final_variables, summary, environment_name, user_variables
                    FROM workflow_executions
                    "#,
                )
                .execute(&self.pool)
                .await?;

                // Drop old table and rename new table
                sqlx::query("DROP TABLE workflow_executions")
                    .execute(&self.pool)
                    .await?;

                sqlx::query("ALTER TABLE workflow_executions_new RENAME TO workflow_executions")
                    .execute(&self.pool)
                    .await?;

                tracing::info!("Successfully migrated 'results' column to 'step_results'");
            } else {
                tracing::info!("No migration needed - 'step_results' column already exists");
            }
        }

        // Collection templates table for reusable templates
        tracing::info!("Creating collection_templates table");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS collection_templates (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                category TEXT NOT NULL,
                tags TEXT NOT NULL,
                author TEXT,
                documentation_url TEXT,
                required_server_types TEXT NOT NULL,
                template_collection TEXT NOT NULL,
                setup_instructions TEXT NOT NULL,
                usage_examples TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        tracing::info!("collection_templates table created successfully");

        // Message history table
        tracing::info!("Creating message_history table");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS message_history (
                id TEXT PRIMARY KEY,
                server_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                direction TEXT NOT NULL,
                content TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                processing_time_ms INTEGER,
                FOREIGN KEY(server_id) REFERENCES server_configs(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        tracing::info!("message_history table created successfully");

        // Workspaces table
        tracing::info!("Creating workspaces table");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workspaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                collections TEXT NOT NULL,
                members TEXT NOT NULL,
                settings TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        tracing::info!("workspaces table created successfully");

        // Server sessions table for connection tracking
        tracing::info!("Creating server_sessions table");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS server_sessions (
                id TEXT PRIMARY KEY,
                server_id TEXT NOT NULL,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                status TEXT NOT NULL,
                capabilities TEXT,
                metrics TEXT NOT NULL,
                process_info TEXT,
                FOREIGN KEY(server_id) REFERENCES server_configs(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        tracing::info!("server_sessions table created successfully");

        // Server profiles table
        tracing::info!("Creating server_profiles table");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS server_profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                icon TEXT,
                color TEXT,
                auto_activate INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        tracing::info!("server_profiles table created successfully");

        // Profile servers relationship table
        tracing::info!("Creating profile_servers table");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS profile_servers (
                profile_id TEXT NOT NULL,
                server_id TEXT NOT NULL,
                startup_order INTEGER NOT NULL DEFAULT 0,
                startup_delay_ms INTEGER NOT NULL DEFAULT 0,
                auto_connect INTEGER NOT NULL DEFAULT 1,
                auto_restart INTEGER NOT NULL DEFAULT 0,
                required INTEGER NOT NULL DEFAULT 0,
                environment_overrides TEXT,
                created_at TEXT NOT NULL,
                PRIMARY KEY (profile_id, server_id),
                FOREIGN KEY(profile_id) REFERENCES server_profiles(id) ON DELETE CASCADE,
                FOREIGN KEY(server_id) REFERENCES server_configs(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        tracing::info!("profile_servers table created successfully");

        // Profile activations table
        tracing::info!("Creating profile_activations table");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS profile_activations (
                id TEXT PRIMARY KEY,
                profile_id TEXT NOT NULL,
                activated_at TEXT NOT NULL,
                deactivated_at TEXT,
                success_count INTEGER NOT NULL DEFAULT 0,
                failure_count INTEGER NOT NULL DEFAULT 0,
                errors TEXT,
                FOREIGN KEY(profile_id) REFERENCES server_profiles(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        tracing::info!("profile_activations table created successfully");

        // Active profile state table (supports multiple active profiles)
        tracing::info!("Creating/migrating active_profile_state table");

        // First, check if we need to migrate from old singleton schema
        let needs_migration = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT COUNT(*) FROM sqlite_master
            WHERE type='table' AND name='active_profile_state'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        if needs_migration > 0 {
            // Check if table has old schema (with id column)
            let has_old_schema = sqlx::query_scalar::<_, i32>(
                r#"
                SELECT COUNT(*) FROM pragma_table_info('active_profile_state')
                WHERE name = 'id'
                "#,
            )
            .fetch_one(&self.pool)
            .await?;

            if has_old_schema > 0 {
                tracing::info!(
                    "Migrating active_profile_state from singleton to multi-profile schema"
                );

                // Drop old table (we clear on startup anyway, no data loss)
                sqlx::query("DROP TABLE IF EXISTS active_profile_state")
                    .execute(&self.pool)
                    .await?;

                tracing::info!("Old active_profile_state table dropped");
            }
        }

        // Create new multi-profile schema
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS active_profile_state (
                profile_id TEXT PRIMARY KEY,
                activated_at TEXT NOT NULL,
                FOREIGN KEY(profile_id) REFERENCES server_profiles(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        tracing::info!("active_profile_state table created successfully (multi-profile)");

        // =====================================================================
        // Test Generation & Execution Tables
        // =====================================================================

        tracing::info!("Creating test_suites table");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS test_suites (
                id TEXT PRIMARY KEY,
                server_id TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                version INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                generated_at TEXT,
                schema_hash TEXT,
                FOREIGN KEY(server_id) REFERENCES server_configs(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        tracing::info!("test_suites table created successfully");

        tracing::info!("Creating tests table");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tests (
                id TEXT PRIMARY KEY,
                suite_id TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                kind TEXT NOT NULL,
                test_data TEXT NOT NULL,
                assertions TEXT NOT NULL,
                category TEXT NOT NULL,
                complexity TEXT NOT NULL,
                auto_generated BOOLEAN NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                edited_at TEXT,
                FOREIGN KEY(suite_id) REFERENCES test_suites(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        tracing::info!("tests table created successfully");

        tracing::info!("Creating test_runs table");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS test_runs (
                id TEXT PRIMARY KEY,
                suite_id TEXT NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                duration_ms INTEGER,
                total_tests INTEGER NOT NULL,
                passed INTEGER NOT NULL DEFAULT 0,
                failed INTEGER NOT NULL DEFAULT 0,
                errors INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL,
                triggered_by TEXT NOT NULL,
                FOREIGN KEY(suite_id) REFERENCES test_suites(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        tracing::info!("test_runs table created successfully");

        tracing::info!("Creating test_results table");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS test_results (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                test_id TEXT NOT NULL,
                passed BOOLEAN NOT NULL,
                error_message TEXT,
                actual_result TEXT,
                duration_ms INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                FOREIGN KEY(run_id) REFERENCES test_runs(id) ON DELETE CASCADE,
                FOREIGN KEY(test_id) REFERENCES tests(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        tracing::info!("test_results table created successfully");

        // Create indexes for performance
        tracing::info!("Creating database indexes");

        tracing::info!("Creating idx_message_history_server_id index");
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_message_history_server_id ON message_history(server_id)")
            .execute(&self.pool)
            .await?;

        tracing::info!("Creating idx_message_history_timestamp index");
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_message_history_timestamp ON message_history(timestamp)")
            .execute(&self.pool)
            .await?;

        tracing::info!("Creating idx_server_sessions_server_id index");
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_server_sessions_server_id ON server_sessions(server_id)")
            .execute(&self.pool)
            .await?;

        tracing::info!("Creating idx_profile_servers_server_id index");
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_profile_servers_server_id ON profile_servers(server_id)")
            .execute(&self.pool)
            .await?;

        tracing::info!("Creating idx_profile_activations_profile_id index");
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_profile_activations_profile_id ON profile_activations(profile_id)")
            .execute(&self.pool)
            .await?;

        // =====================================================================
        // Proxy Configuration Table
        // =====================================================================
        tracing::info!("Creating proxies table");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS proxies (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                backend_type TEXT NOT NULL,
                backend_config TEXT NOT NULL,
                frontend_type TEXT NOT NULL,
                frontend_config TEXT NOT NULL,
                auth_type TEXT,
                auth_config TEXT,
                metrics_enabled BOOLEAN NOT NULL DEFAULT 1,
                max_requests_tracked INTEGER NOT NULL DEFAULT 10000,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_started_at TEXT,
                last_stopped_at TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        tracing::info!("proxies table created successfully");

        tracing::info!("Creating idx_proxies_created_at index");
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_proxies_created_at ON proxies(created_at)")
            .execute(&self.pool)
            .await?;

        // =====================================================================
        // OAuth 2.1 Visual Debugger Tables
        // =====================================================================

        // OAuth configurations table
        tracing::info!("Creating oauth_configs table");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS oauth_configs (
                id TEXT PRIMARY KEY,
                server_id TEXT NOT NULL,
                protocol_version TEXT NOT NULL,
                auth_server_url TEXT NOT NULL,
                token_endpoint TEXT,
                client_id TEXT,
                client_secret TEXT,
                redirect_uri TEXT NOT NULL,
                scopes TEXT NOT NULL,
                resource_uri TEXT NOT NULL,
                use_pkce BOOLEAN NOT NULL DEFAULT 1,
                use_dpop BOOLEAN NOT NULL DEFAULT 0,
                metadata TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(server_id) REFERENCES server_configs(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        tracing::info!("oauth_configs table created successfully");

        // OAuth flow execution logs table (for debugging)
        tracing::info!("Creating oauth_flow_logs table");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS oauth_flow_logs (
                id TEXT PRIMARY KEY,
                config_id TEXT NOT NULL,
                flow_type TEXT NOT NULL,
                state_param TEXT,
                pkce_verifier TEXT,
                status TEXT NOT NULL,
                error_code TEXT,
                error_description TEXT,
                steps TEXT NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                FOREIGN KEY(config_id) REFERENCES oauth_configs(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        tracing::info!("oauth_flow_logs table created successfully");

        // OAuth tokens table (keyring-backed metadata)
        tracing::info!("Creating oauth_tokens table");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS oauth_tokens (
                id TEXT PRIMARY KEY,
                server_id TEXT NOT NULL,
                keyring_key TEXT NOT NULL,
                token_type TEXT NOT NULL,
                scope TEXT,
                expires_at TEXT,
                refresh_keyring_key TEXT,
                dpop_jkt TEXT,
                created_at TEXT NOT NULL,
                last_refreshed_at TEXT,
                FOREIGN KEY(server_id) REFERENCES server_configs(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        tracing::info!("oauth_tokens table created successfully");

        // Create indexes for OAuth tables
        tracing::info!("Creating idx_oauth_configs_server_id index");
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth_configs_server_id ON oauth_configs(server_id)")
            .execute(&self.pool)
            .await?;

        tracing::info!("Creating idx_oauth_flow_logs_config_id index");
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth_flow_logs_config_id ON oauth_flow_logs(config_id)")
            .execute(&self.pool)
            .await?;

        tracing::info!("Creating idx_oauth_tokens_server_id index");
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth_tokens_server_id ON oauth_tokens(server_id)")
            .execute(&self.pool)
            .await?;

        let migration_duration = migration_start.elapsed();
        tracing::info!("Database migrations completed successfully in {:?} (15 tables + 7 indexes, typically 3-15ms)", migration_duration);
        Ok(())
    }

    /// Save a server configuration
    pub async fn save_server_config(&self, config: &ServerConfig) -> McpResult<()> {
        let transport_config = serde_json::to_string(&config.transport_config)?;
        let env_vars = serde_json::to_string(&config.environment_variables)?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO server_configs 
            (id, name, description, transport_type, transport_config, environment_variables, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(config.id.to_string())
        .bind(&config.name)
        .bind(config.description.as_deref())
        .bind(match &config.transport_config {
            crate::types::TransportConfig::Stdio { .. } => "stdio",
            crate::types::TransportConfig::Http { .. } => "http",
            crate::types::TransportConfig::WebSocket { .. } => "websocket",
            crate::types::TransportConfig::Tcp { .. } => "tcp",
            crate::types::TransportConfig::Unix { .. } => "unix",
        })
        .bind(transport_config)
        .bind(env_vars)
        .bind(config.created_at.to_rfc3339())
        .bind(config.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Load a server configuration by ID
    pub async fn load_server_config(&self, id: Uuid) -> McpResult<Option<ServerConfig>> {
        let row = sqlx::query(
            "SELECT id, name, description, transport_type, transport_config, environment_variables, created_at, updated_at FROM server_configs WHERE id = ?"
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let transport_config: crate::types::TransportConfig =
                serde_json::from_str(&row.get::<String, _>("transport_config"))?;
            let env_vars: std::collections::HashMap<String, String> =
                serde_json::from_str(&row.get::<String, _>("environment_variables"))?;

            Ok(Some(ServerConfig {
                id: Uuid::parse_str(&row.get::<String, _>("id"))
                    .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?,
                name: row.get("name"),
                description: row.get("description"),
                transport_config,
                environment_variables: env_vars,
                created_at: chrono::DateTime::parse_from_rfc3339(
                    &row.get::<String, _>("created_at"),
                )
                .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?
                .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(
                    &row.get::<String, _>("updated_at"),
                )
                .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?
                .with_timezone(&chrono::Utc),
            }))
        } else {
            Ok(None)
        }
    }

    /// List all server configurations
    pub async fn list_server_configs(&self) -> McpResult<Vec<ServerConfig>> {
        let rows = sqlx::query(
            "SELECT id, name, description, transport_type, transport_config, environment_variables, created_at, updated_at FROM server_configs ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut configs = Vec::new();

        for row in rows {
            let transport_config: crate::types::TransportConfig =
                serde_json::from_str(&row.get::<String, _>("transport_config"))?;
            let env_vars: std::collections::HashMap<String, String> =
                serde_json::from_str(&row.get::<String, _>("environment_variables"))?;

            configs.push(ServerConfig {
                id: Uuid::parse_str(&row.get::<String, _>("id"))
                    .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?,
                name: row.get("name"),
                description: row.get("description"),
                transport_config,
                environment_variables: env_vars,
                created_at: chrono::DateTime::parse_from_rfc3339(
                    &row.get::<String, _>("created_at"),
                )
                .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?
                .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(
                    &row.get::<String, _>("updated_at"),
                )
                .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?
                .with_timezone(&chrono::Utc),
            });
        }

        Ok(configs)
    }

    /// Delete a server configuration
    pub async fn delete_server_config(&self, id: Uuid) -> McpResult<()> {
        log::debug!("Executing DELETE query for id: {}", id);

        let id_str = id.to_string();

        // ✅ FIXED: Delete from profile_servers first (foreign key dependency)
        log::debug!("Removing server from all profiles...");
        let profile_result = sqlx::query("DELETE FROM profile_servers WHERE server_id = ?")
            .bind(&id_str)
            .execute(&self.pool)
            .await?;

        if profile_result.rows_affected() > 0 {
            log::info!(
                "Removed server from {} profile(s)",
                profile_result.rows_affected()
            );
        } else {
            log::debug!("Server was not part of any profiles");
        }

        // ✅ FIXED: Delete from message_history (another foreign key)
        log::debug!("Removing message history...");
        let history_result = sqlx::query("DELETE FROM message_history WHERE server_id = ?")
            .bind(&id_str)
            .execute(&self.pool)
            .await?;

        if history_result.rows_affected() > 0 {
            log::info!("Deleted {} message(s)", history_result.rows_affected());
        }

        // ✅ FIXED: Delete from server_sessions (another foreign key)
        log::debug!("Removing server sessions...");
        let session_result = sqlx::query("DELETE FROM server_sessions WHERE server_id = ?")
            .bind(&id_str)
            .execute(&self.pool)
            .await?;

        if session_result.rows_affected() > 0 {
            log::info!("Deleted {} session(s)", session_result.rows_affected());
        }

        // Now delete the server config itself
        log::debug!("Deleting server configuration...");
        let result = sqlx::query("DELETE FROM server_configs WHERE id = ?")
            .bind(&id_str)
            .execute(&self.pool)
            .await?;

        log::debug!(
            "DELETE query result: rows_affected = {}",
            result.rows_affected()
        );

        if result.rows_affected() == 0 {
            log::warn!("No rows affected - server {} may not exist in database", id);
        } else {
            log::info!("Successfully deleted server configuration from database");
        }

        Ok(())
    }

    /// Save an enhanced collection with workflow support
    pub async fn save_collection(&self, collection: &Collection) -> McpResult<()> {
        let tags = serde_json::to_string(&collection.tags)?;
        let workflow = serde_json::to_string(&collection.workflow)?;
        let variables = serde_json::to_string(&collection.variables)?;
        let environment = serde_json::to_string(&collection.environment)?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO collections
            (id, name, description, tags, workflow, variables, environment,
             created_at, updated_at, created_by, version, last_run, run_count)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(collection.id.to_string())
        .bind(&collection.name)
        .bind(collection.description.as_deref())
        .bind(tags)
        .bind(workflow)
        .bind(variables)
        .bind(environment)
        .bind(collection.created_at.to_rfc3339())
        .bind(collection.updated_at.to_rfc3339())
        .bind(collection.created_by.as_deref())
        .bind(&collection.version)
        .bind(collection.last_run.map(|dt| dt.to_rfc3339()))
        .bind(collection.run_count as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Load a collection by ID
    pub async fn load_collection(&self, id: Uuid) -> McpResult<Option<Collection>> {
        let row = sqlx::query(
            "SELECT id, name, description, tags, workflow, variables, environment, created_at, updated_at, created_by, version, last_run, run_count FROM collections WHERE id = ?"
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let tags: Vec<String> = serde_json::from_str(&row.get::<String, _>("tags"))?;
            let workflow: Vec<crate::types::collections::WorkflowStep> =
                serde_json::from_str(&row.get::<String, _>("workflow"))?;
            let variables: std::collections::HashMap<
                String,
                crate::types::collections::CollectionVariable,
            > = serde_json::from_str(&row.get::<String, _>("variables"))?;
            let environment: crate::types::collections::CollectionEnvironment =
                serde_json::from_str(&row.get::<String, _>("environment"))?;
            let last_run: Option<chrono::DateTime<chrono::Utc>> = row
                .get::<Option<String>, _>("last_run")
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc));

            Ok(Some(Collection {
                id: Uuid::parse_str(&row.get::<String, _>("id"))
                    .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?,
                name: row.get("name"),
                description: row.get("description"),
                tags,
                workflow,
                variables,
                environment,
                created_at: chrono::DateTime::parse_from_rfc3339(
                    &row.get::<String, _>("created_at"),
                )
                .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?
                .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(
                    &row.get::<String, _>("updated_at"),
                )
                .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?
                .with_timezone(&chrono::Utc),
                created_by: row.get("created_by"),
                version: row.get("version"),
                last_run,
                run_count: row.get::<i64, _>("run_count") as u32,
            }))
        } else {
            Ok(None)
        }
    }

    /// List all collections
    pub async fn list_collections(&self) -> McpResult<Vec<Collection>> {
        let rows = sqlx::query(
            "SELECT id, name, description, tags, workflow, variables, environment, created_at, updated_at, created_by, version, last_run, run_count FROM collections ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut collections = Vec::new();

        for row in rows {
            let tags: Vec<String> = serde_json::from_str(&row.get::<String, _>("tags"))?;
            let workflow: Vec<crate::types::collections::WorkflowStep> =
                serde_json::from_str(&row.get::<String, _>("workflow"))?;
            let variables: std::collections::HashMap<
                String,
                crate::types::collections::CollectionVariable,
            > = serde_json::from_str(&row.get::<String, _>("variables"))?;
            let environment: crate::types::collections::CollectionEnvironment =
                serde_json::from_str(&row.get::<String, _>("environment"))?;
            let last_run: Option<chrono::DateTime<chrono::Utc>> = row
                .get::<Option<String>, _>("last_run")
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc));

            collections.push(Collection {
                id: Uuid::parse_str(&row.get::<String, _>("id"))
                    .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?,
                name: row.get("name"),
                description: row.get("description"),
                tags,
                workflow,
                variables,
                environment,
                created_at: chrono::DateTime::parse_from_rfc3339(
                    &row.get::<String, _>("created_at"),
                )
                .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?
                .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(
                    &row.get::<String, _>("updated_at"),
                )
                .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?
                .with_timezone(&chrono::Utc),
                created_by: row.get("created_by"),
                version: row.get("version"),
                last_run,
                run_count: row.get::<i64, _>("run_count") as u32,
            });
        }

        Ok(collections)
    }

    /// Delete a collection by ID
    pub async fn delete_collection(&self, id: Uuid) -> McpResult<()> {
        sqlx::query("DELETE FROM collections WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get a workflow execution by ID
    pub async fn get_workflow_execution(
        &self,
        id: Uuid,
    ) -> McpResult<Option<crate::types::collections::WorkflowExecution>> {
        let row = sqlx::query(
            "SELECT id, collection_id, status, started_at, completed_at, summary, step_results, collection_version, environment_name, user_variables, final_variables FROM workflow_executions WHERE id = ?"
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let summary: crate::types::collections::ExecutionSummary =
                serde_json::from_str(&row.get::<String, _>("summary"))?;
            let step_results: std::collections::HashMap<
                uuid::Uuid,
                crate::types::collections::StepResult,
            > = serde_json::from_str(&row.get::<String, _>("step_results"))?;
            let final_variables: HashMap<String, serde_json::Value> =
                serde_json::from_str(&row.get::<String, _>("final_variables"))?;
            let user_variables: HashMap<String, serde_json::Value> =
                serde_json::from_str(&row.get::<String, _>("user_variables"))?;

            Ok(Some(crate::types::collections::WorkflowExecution {
                id: Uuid::parse_str(&row.get::<String, _>("id"))
                    .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?,
                collection_id: Uuid::parse_str(&row.get::<String, _>("collection_id"))
                    .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?,
                status: serde_json::from_str(&format!("\"{}\"", row.get::<String, _>("status")))?,
                started_at: chrono::DateTime::parse_from_rfc3339(
                    &row.get::<String, _>("started_at"),
                )
                .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?
                .with_timezone(&chrono::Utc),
                completed_at: row
                    .get::<Option<String>, _>("completed_at")
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc)),
                summary,
                step_results,
                collection_version: row.get("collection_version"),
                final_variables,
                environment_name: row.get("environment_name"),
                user_variables,
            }))
        } else {
            Ok(None)
        }
    }

    /// List workflow executions for a collection
    pub async fn list_workflow_executions(
        &self,
        collection_id: Uuid,
    ) -> McpResult<Vec<crate::types::collections::WorkflowExecution>> {
        let rows = sqlx::query(
            "SELECT id, collection_id, status, started_at, completed_at, summary, step_results, collection_version, environment_name, user_variables, final_variables FROM workflow_executions WHERE collection_id = ? ORDER BY started_at DESC"
        )
        .bind(collection_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut executions = Vec::new();

        for row in rows {
            let summary: crate::types::collections::ExecutionSummary =
                serde_json::from_str(&row.get::<String, _>("summary"))?;
            let step_results: std::collections::HashMap<
                uuid::Uuid,
                crate::types::collections::StepResult,
            > = serde_json::from_str(&row.get::<String, _>("step_results"))?;
            let final_variables: HashMap<String, serde_json::Value> =
                serde_json::from_str(&row.get::<String, _>("final_variables"))?;
            let user_variables: HashMap<String, serde_json::Value> =
                serde_json::from_str(&row.get::<String, _>("user_variables"))?;

            executions.push(crate::types::collections::WorkflowExecution {
                id: Uuid::parse_str(&row.get::<String, _>("id"))
                    .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?,
                collection_id: Uuid::parse_str(&row.get::<String, _>("collection_id"))
                    .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?,
                status: serde_json::from_str(&format!("\"{}\"", row.get::<String, _>("status")))?,
                started_at: chrono::DateTime::parse_from_rfc3339(
                    &row.get::<String, _>("started_at"),
                )
                .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?
                .with_timezone(&chrono::Utc),
                completed_at: row
                    .get::<Option<String>, _>("completed_at")
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc)),
                summary,
                step_results,
                collection_version: row.get("collection_version"),
                final_variables,
                environment_name: row.get("environment_name"),
                user_variables,
            });
        }

        Ok(executions)
    }

    /// Save or update a workflow execution
    pub async fn save_workflow_execution(
        &self,
        execution: &crate::types::collections::WorkflowExecution,
    ) -> McpResult<()> {
        // Serialize complex fields to JSON
        let step_results_json = serde_json::to_string(&execution.step_results)?;
        let final_variables_json = serde_json::to_string(&execution.final_variables)?;
        let summary_json = serde_json::to_string(&execution.summary)?;
        let user_variables_json = serde_json::to_string(&execution.user_variables)?;

        // Serialize status enum to string
        let status_str = match execution.status {
            crate::types::collections::ExecutionStatus::Running => "Running",
            crate::types::collections::ExecutionStatus::Completed => "Completed",
            crate::types::collections::ExecutionStatus::Failed => "Failed",
            crate::types::collections::ExecutionStatus::Cancelled => "Cancelled",
            crate::types::collections::ExecutionStatus::Paused => "Paused",
        };

        // Use INSERT OR REPLACE to handle both insert and update
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO workflow_executions (
                id, collection_id, collection_version, started_at, completed_at, status,
                step_results, final_variables, summary, environment_name, user_variables
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(execution.id.to_string())
        .bind(execution.collection_id.to_string())
        .bind(&execution.collection_version)
        .bind(execution.started_at.to_rfc3339())
        .bind(execution.completed_at.as_ref().map(|dt| dt.to_rfc3339()))
        .bind(status_str)
        .bind(step_results_json)
        .bind(final_variables_json)
        .bind(summary_json)
        .bind(&execution.environment_name)
        .bind(user_variables_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Save message to history
    pub async fn save_message(&self, message: &MessageHistory) -> McpResult<()> {
        let content = serde_json::to_string(&message.content)?;
        let direction = match message.direction {
            crate::types::MessageDirection::ClientToServer => "client_to_server",
            crate::types::MessageDirection::ServerToClient => "server_to_client",
        };

        sqlx::query(
            r#"
            INSERT INTO message_history 
            (id, server_id, timestamp, direction, content, size_bytes, processing_time_ms)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(message.id.to_string())
        .bind(message.server_id.to_string())
        .bind(message.timestamp.to_rfc3339())
        .bind(direction)
        .bind(content)
        .bind(message.size_bytes)
        .bind(message.processing_time_ms)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Prune old messages to keep the message history bounded
    ///
    /// Keeps the most recent `max_messages` per server.
    /// Returns the number of rows deleted.
    pub async fn prune_messages(
        &self,
        server_id: Uuid,
        max_messages: i64,
    ) -> McpResult<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM message_history
            WHERE server_id = ? AND id NOT IN (
                SELECT id FROM message_history
                WHERE server_id = ?
                ORDER BY timestamp DESC
                LIMIT ?
            )
            "#,
        )
        .bind(server_id.to_string())
        .bind(server_id.to_string())
        .bind(max_messages)
        .execute(&self.pool)
        .await?;

        let deleted = result.rows_affected();
        if deleted > 0 {
            tracing::debug!(
                "Pruned {} old messages for server {} (keeping {})",
                deleted,
                server_id,
                max_messages
            );
        }

        Ok(deleted)
    }

    /// Prune all servers' message histories
    ///
    /// Keeps the most recent `max_messages_per_server` for each server.
    pub async fn prune_all_messages(&self, max_messages_per_server: i64) -> McpResult<u64> {
        // Get all distinct server IDs
        let rows = sqlx::query("SELECT DISTINCT server_id FROM message_history")
            .fetch_all(&self.pool)
            .await?;

        let mut total_deleted = 0u64;
        for row in rows {
            let server_id_str: String = row.get("server_id");
            if let Ok(server_id) = Uuid::parse_str(&server_id_str) {
                total_deleted += self.prune_messages(server_id, max_messages_per_server).await?;
            }
        }

        if total_deleted > 0 {
            tracing::info!(
                "Pruned {} total old messages across all servers",
                total_deleted
            );
        }

        Ok(total_deleted)
    }

    /// Get message history for a server
    pub async fn get_message_history(
        &self,
        server_id: Uuid,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> McpResult<Vec<MessageHistory>> {
        let query = if let (Some(limit), Some(offset)) = (limit, offset) {
            sqlx::query(
                "SELECT id, server_id, timestamp, direction, content, size_bytes, processing_time_ms 
                 FROM message_history WHERE server_id = ? ORDER BY timestamp DESC LIMIT ? OFFSET ?"
            )
            .bind(server_id.to_string())
            .bind(limit)
            .bind(offset)
        } else if let Some(limit) = limit {
            sqlx::query(
                "SELECT id, server_id, timestamp, direction, content, size_bytes, processing_time_ms 
                 FROM message_history WHERE server_id = ? ORDER BY timestamp DESC LIMIT ?"
            )
            .bind(server_id.to_string())
            .bind(limit)
        } else {
            sqlx::query(
                "SELECT id, server_id, timestamp, direction, content, size_bytes, processing_time_ms 
                 FROM message_history WHERE server_id = ? ORDER BY timestamp DESC"
            )
            .bind(server_id.to_string())
        };

        let rows = query.fetch_all(&self.pool).await?;
        let mut messages = Vec::new();

        for row in rows {
            let content: serde_json::Value =
                serde_json::from_str(&row.get::<String, _>("content"))?;
            let direction = match row.get::<String, _>("direction").as_str() {
                "client_to_server" => crate::types::MessageDirection::ClientToServer,
                "server_to_client" => crate::types::MessageDirection::ServerToClient,
                _ => crate::types::MessageDirection::ServerToClient,
            };

            messages.push(MessageHistory {
                id: Uuid::parse_str(&row.get::<String, _>("id"))
                    .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?,
                server_id: Uuid::parse_str(&row.get::<String, _>("server_id"))
                    .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?,
                timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>("timestamp"))
                    .map_err(|e| McpStudioError::DatabaseError(sqlx::Error::Decode(Box::new(e))))?
                    .with_timezone(&chrono::Utc),
                direction,
                content: content.to_string(),
                size_bytes: row.get::<i64, _>("size_bytes"),
                processing_time_ms: row.get::<Option<i64>, _>("processing_time_ms"),
            });
        }

        Ok(messages)
    }

    /// Clear message history for a server
    pub async fn clear_message_history(&self, server_id: Uuid) -> McpResult<()> {
        sqlx::query("DELETE FROM message_history WHERE server_id = ?")
            .bind(server_id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Clear active profile state on startup to ensure fresh sessions
    /// This prevents auto-reconnection to servers from previous sessions
    pub async fn clear_active_profile_on_startup(&self) -> McpResult<()> {
        tracing::info!("Clearing active profile state for fresh startup");

        // Clear all active profiles (supports multiple active profiles)
        let result = sqlx::query("DELETE FROM active_profile_state")
            .execute(&self.pool)
            .await;

        match result {
            Ok(result) => {
                let rows_deleted = result.rows_affected();
                tracing::info!(
                    "Active profile state cleared successfully ({} profiles deactivated)",
                    rows_deleted
                );
                Ok(())
            }
            Err(e) => {
                // If table doesn't exist yet, that's fine - it means fresh install
                if e.to_string().contains("no such table") {
                    tracing::info!("Active profile state table not yet created (fresh install) - skipping clear");
                    Ok(())
                } else {
                    // Other errors should be logged but not fail startup
                    tracing::warn!("Failed to clear active profile state: {} (non-critical)", e);
                    Ok(())
                }
            }
        }
    }

    // Proxy-related database methods

    /// Save proxy configuration (insert or replace)
    pub async fn save_proxy_config(
        &self,
        config: &crate::proxy::ProxyConfig,
    ) -> McpResult<()> {
        let backend_config_json = serde_json::to_string(&config.backend_config)?;
        let frontend_config_json = serde_json::to_string(&config.frontend_config)?;
        let auth_config_json = serde_json::to_string(&config.auth_config)?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO proxies
            (id, name, description, backend_type, backend_config, frontend_type, frontend_config,
             auth_type, auth_config, metrics_enabled, max_requests_tracked, created_at, updated_at,
             last_started_at, last_stopped_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&config.id.0)
        .bind(&config.name)
        .bind(&config.description)
        .bind(&config.backend_type)
        .bind(&backend_config_json)
        .bind(format!("{:?}", config.frontend_type).to_lowercase())
        .bind(&frontend_config_json)
        .bind(format!("{:?}", match &config.auth_config {
            crate::proxy::AuthConfig::None => "none",
            crate::proxy::AuthConfig::Bearer { .. } => "bearer",
            crate::proxy::AuthConfig::ApiKey { .. } => "api_key",
            crate::proxy::AuthConfig::Jwt { .. } => "jwt",
        }))
        .bind(&auth_config_json)
        .bind(config.metrics_enabled as i32)
        .bind(config.max_requests_tracked as i32)
        .bind(config.created_at.duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string())
        .bind(config.updated_at.duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string())
        .bind(config.last_started_at.map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .to_string()
        }))
        .bind(config.last_stopped_at.map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .to_string()
        }))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get proxy configuration by ID
    pub async fn get_proxy_config(&self, proxy_id: &str) -> McpResult<Option<crate::proxy::ProxyConfig>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, backend_type, backend_config, frontend_type, frontend_config,
                   auth_type, auth_config, metrics_enabled, max_requests_tracked, created_at, updated_at,
                   last_started_at, last_stopped_at
            FROM proxies WHERE id = ?
            "#,
        )
        .bind(proxy_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let frontend_type = match row.get::<String, _>("frontend_type").as_str() {
                "http" => crate::proxy::FrontendType::Http,
                "websocket" => crate::proxy::FrontendType::WebSocket,
                "tcp" => crate::proxy::FrontendType::Tcp,
                _ => crate::proxy::FrontendType::Http,
            };

            let auth_config = match row.get::<Option<String>, _>("auth_type").as_deref() {
                Some("bearer") => {
                    let json: serde_json::Value = serde_json::from_str(
                        &row.get::<String, _>("auth_config")
                    )?;
                    if let Some(token) = json.get("token").and_then(|t| t.as_str()) {
                        crate::proxy::AuthConfig::Bearer { token: token.to_string() }
                    } else {
                        crate::proxy::AuthConfig::None
                    }
                }
                Some("api_key") => {
                    let json: serde_json::Value = serde_json::from_str(
                        &row.get::<String, _>("auth_config")
                    )?;
                    if let (Some(key), Some(header)) = (
                        json.get("key").and_then(|k| k.as_str()),
                        json.get("header").and_then(|h| h.as_str()),
                    ) {
                        crate::proxy::AuthConfig::ApiKey {
                            key: key.to_string(),
                            header: header.to_string(),
                        }
                    } else {
                        crate::proxy::AuthConfig::None
                    }
                }
                Some("jwt") => {
                    let json: serde_json::Value = serde_json::from_str(
                        &row.get::<String, _>("auth_config")
                    )?;
                    if let (Some(issuer), Some(audience)) = (
                        json.get("issuer").and_then(|i| i.as_str()),
                        json.get("audience").and_then(|a| a.as_str()),
                    ) {
                        crate::proxy::AuthConfig::Jwt {
                            issuer: issuer.to_string(),
                            audience: audience.to_string(),
                            secret: json.get("secret").and_then(|s| s.as_str()).map(|s| s.to_string()),
                        }
                    } else {
                        crate::proxy::AuthConfig::None
                    }
                }
                _ => crate::proxy::AuthConfig::None,
            };

            let created_at_secs: i64 = row.get::<String, _>("created_at").parse().unwrap_or(0);
            let updated_at_secs: i64 = row.get::<String, _>("updated_at").parse().unwrap_or(0);

            Ok(Some(crate::proxy::ProxyConfig {
                id: crate::proxy::ProxyId(row.get("id")),
                name: row.get("name"),
                description: row.get("description"),
                backend_type: row.get("backend_type"),
                backend_config: serde_json::from_str(&row.get::<String, _>("backend_config"))?,
                frontend_type,
                frontend_config: serde_json::from_str(&row.get::<String, _>("frontend_config"))?,
                auth_config,
                metrics_enabled: row.get::<i32, _>("metrics_enabled") != 0,
                max_requests_tracked: row.get::<i32, _>("max_requests_tracked") as usize,
                created_at: std::time::UNIX_EPOCH + std::time::Duration::from_secs(created_at_secs as u64),
                updated_at: std::time::UNIX_EPOCH + std::time::Duration::from_secs(updated_at_secs as u64),
                last_started_at: row.get::<Option<String>, _>("last_started_at")
                    .and_then(|s| s.parse::<i64>().ok())
                    .map(|secs| std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs as u64)),
                last_stopped_at: row.get::<Option<String>, _>("last_stopped_at")
                    .and_then(|s| s.parse::<i64>().ok())
                    .map(|secs| std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs as u64)),
            }))
        } else {
            Ok(None)
        }
    }

    /// List all proxy configurations
    pub async fn list_proxy_configs(&self) -> McpResult<Vec<crate::proxy::ProxyConfig>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, backend_type, backend_config, frontend_type, frontend_config,
                   auth_type, auth_config, metrics_enabled, max_requests_tracked, created_at, updated_at,
                   last_started_at, last_stopped_at
            FROM proxies
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut configs = Vec::new();

        for row in rows {
            let frontend_type = match row.get::<String, _>("frontend_type").as_str() {
                "http" => crate::proxy::FrontendType::Http,
                "websocket" => crate::proxy::FrontendType::WebSocket,
                "tcp" => crate::proxy::FrontendType::Tcp,
                _ => crate::proxy::FrontendType::Http,
            };

            let auth_config = match row.get::<Option<String>, _>("auth_type").as_deref() {
                Some("bearer") => {
                    let json: serde_json::Value = serde_json::from_str(
                        &row.get::<String, _>("auth_config")
                    )?;
                    if let Some(token) = json.get("token").and_then(|t| t.as_str()) {
                        crate::proxy::AuthConfig::Bearer { token: token.to_string() }
                    } else {
                        crate::proxy::AuthConfig::None
                    }
                }
                _ => crate::proxy::AuthConfig::None,
            };

            let created_at_secs: i64 = row.get::<String, _>("created_at").parse().unwrap_or(0);
            let updated_at_secs: i64 = row.get::<String, _>("updated_at").parse().unwrap_or(0);

            configs.push(crate::proxy::ProxyConfig {
                id: crate::proxy::ProxyId(row.get("id")),
                name: row.get("name"),
                description: row.get("description"),
                backend_type: row.get("backend_type"),
                backend_config: serde_json::from_str(&row.get::<String, _>("backend_config"))?,
                frontend_type,
                frontend_config: serde_json::from_str(&row.get::<String, _>("frontend_config"))?,
                auth_config,
                metrics_enabled: row.get::<i32, _>("metrics_enabled") != 0,
                max_requests_tracked: row.get::<i32, _>("max_requests_tracked") as usize,
                created_at: std::time::UNIX_EPOCH + std::time::Duration::from_secs(created_at_secs as u64),
                updated_at: std::time::UNIX_EPOCH + std::time::Duration::from_secs(updated_at_secs as u64),
                last_started_at: row.get::<Option<String>, _>("last_started_at")
                    .and_then(|s| s.parse::<i64>().ok())
                    .map(|secs| std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs as u64)),
                last_stopped_at: row.get::<Option<String>, _>("last_stopped_at")
                    .and_then(|s| s.parse::<i64>().ok())
                    .map(|secs| std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs as u64)),
            });
        }

        Ok(configs)
    }

    /// Delete proxy configuration
    pub async fn delete_proxy_config(&self, proxy_id: &str) -> McpResult<()> {
        sqlx::query("DELETE FROM proxies WHERE id = ?")
            .bind(proxy_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // =========================================================================
    // OAuth Configuration Methods
    // =========================================================================

    /// Save OAuth configuration for a server
    pub async fn save_oauth_config(&self, config: &crate::oauth::OAuthConfig) -> McpResult<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string();
        let scopes_json = serde_json::to_string(&config.scopes)?;
        let metadata_json = config.metadata.as_ref().map(|m| serde_json::to_string(m)).transpose()?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO oauth_configs
            (id, server_id, protocol_version, auth_server_url, token_endpoint, client_id,
             client_secret, redirect_uri, scopes, resource_uri, use_pkce, use_dpop,
             metadata, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&config.server_id)
        .bind(&config.protocol_version)
        .bind(&config.auth_server_url)
        .bind(&config.token_endpoint)
        .bind(&config.client_id)
        .bind(&config.client_secret)
        .bind(&config.redirect_uri)
        .bind(&scopes_json)
        .bind(&config.resource_uri)
        .bind(config.use_pkce as i32)
        .bind(config.use_dpop as i32)
        .bind(&metadata_json)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// Get OAuth configuration by server ID
    pub async fn get_oauth_config(&self, server_id: &str) -> McpResult<Option<crate::oauth::OAuthConfig>> {
        let row = sqlx::query(
            r#"
            SELECT id, server_id, protocol_version, auth_server_url, token_endpoint, client_id,
                   client_secret, redirect_uri, scopes, resource_uri, use_pkce, use_dpop, metadata
            FROM oauth_configs
            WHERE server_id = ?
            "#,
        )
        .bind(server_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let scopes_json: String = row.get("scopes");
                let metadata_json: Option<String> = row.get("metadata");

                Ok(Some(crate::oauth::OAuthConfig {
                    server_id: row.get("server_id"),
                    protocol_version: row.get("protocol_version"),
                    auth_server_url: row.get("auth_server_url"),
                    token_endpoint: row.get("token_endpoint"),
                    client_id: row.get("client_id"),
                    client_secret: row.get("client_secret"),
                    redirect_uri: row.get("redirect_uri"),
                    scopes: serde_json::from_str(&scopes_json).unwrap_or_default(),
                    resource_uri: row.get("resource_uri"),
                    use_pkce: row.get::<i32, _>("use_pkce") != 0,
                    use_dpop: row.get::<i32, _>("use_dpop") != 0,
                    metadata: metadata_json.and_then(|s| serde_json::from_str(&s).ok()),
                }))
            }
            None => Ok(None),
        }
    }

    /// Update OAuth configuration
    pub async fn update_oauth_config(&self, server_id: &str, config: &crate::oauth::OAuthConfig) -> McpResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string();
        let scopes_json = serde_json::to_string(&config.scopes)?;
        let metadata_json = config.metadata.as_ref().map(|m| serde_json::to_string(m)).transpose()?;

        sqlx::query(
            r#"
            UPDATE oauth_configs
            SET protocol_version = ?, auth_server_url = ?, token_endpoint = ?, client_id = ?,
                client_secret = ?, redirect_uri = ?, scopes = ?, resource_uri = ?,
                use_pkce = ?, use_dpop = ?, metadata = ?, updated_at = ?
            WHERE server_id = ?
            "#,
        )
        .bind(&config.protocol_version)
        .bind(&config.auth_server_url)
        .bind(&config.token_endpoint)
        .bind(&config.client_id)
        .bind(&config.client_secret)
        .bind(&config.redirect_uri)
        .bind(&scopes_json)
        .bind(&config.resource_uri)
        .bind(config.use_pkce as i32)
        .bind(config.use_dpop as i32)
        .bind(&metadata_json)
        .bind(&now)
        .bind(server_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete OAuth configuration for a server
    pub async fn delete_oauth_config(&self, server_id: &str) -> McpResult<()> {
        sqlx::query("DELETE FROM oauth_configs WHERE server_id = ?")
            .bind(server_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// List all OAuth configurations
    pub async fn list_oauth_configs(&self) -> McpResult<Vec<crate::oauth::OAuthConfig>> {
        let rows = sqlx::query(
            r#"
            SELECT id, server_id, protocol_version, auth_server_url, token_endpoint, client_id,
                   client_secret, redirect_uri, scopes, resource_uri, use_pkce, use_dpop, metadata
            FROM oauth_configs
            ORDER BY updated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut configs = Vec::new();
        for row in rows {
            let scopes_json: String = row.get("scopes");
            let metadata_json: Option<String> = row.get("metadata");

            configs.push(crate::oauth::OAuthConfig {
                server_id: row.get("server_id"),
                protocol_version: row.get("protocol_version"),
                auth_server_url: row.get("auth_server_url"),
                token_endpoint: row.get("token_endpoint"),
                client_id: row.get("client_id"),
                client_secret: row.get("client_secret"),
                redirect_uri: row.get("redirect_uri"),
                scopes: serde_json::from_str(&scopes_json).unwrap_or_default(),
                resource_uri: row.get("resource_uri"),
                use_pkce: row.get::<i32, _>("use_pkce") != 0,
                use_dpop: row.get::<i32, _>("use_dpop") != 0,
                metadata: metadata_json.and_then(|s| serde_json::from_str(&s).ok()),
            });
        }

        Ok(configs)
    }
}
