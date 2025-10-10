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
            // For file paths, SQLite URLs need three slashes for absolute paths
            let path = std::path::Path::new(database_path);
            if path.is_absolute() {
                format!("sqlite:///{}", database_path.trim_start_matches('/'))
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
        tracing::info!("Starting database migrations");

        // Server configurations table
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
        tracing::info!("server_configs table created successfully");

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

        tracing::info!("Database migrations completed successfully");
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
        sqlx::query("DELETE FROM server_configs WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

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
            "SELECT id, collection_id, status, started_at, completed_at, summary, results FROM workflow_executions WHERE id = ?"
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
            > = serde_json::from_str(&row.get::<String, _>("results"))?;

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
                collection_version: "1.0.0".to_string(), // Default version since not stored in DB yet
                final_variables: HashMap::new(),
                environment_name: "default".to_string(),
                user_variables: HashMap::new(),
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
            "SELECT id, collection_id, status, started_at, completed_at, summary, results FROM workflow_executions WHERE collection_id = ? ORDER BY started_at DESC"
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
            > = serde_json::from_str(&row.get::<String, _>("results"))?;

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
                collection_version: "1.0.0".to_string(), // Default version since not stored in DB yet
                final_variables: HashMap::new(),
                environment_name: "default".to_string(),
                user_variables: HashMap::new(),
            });
        }

        Ok(executions)
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
}
