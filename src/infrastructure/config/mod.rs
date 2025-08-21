use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub storage: StorageConfig,
    pub logging: LoggingConfig,
}

/// HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_upload_size: u64, // bytes
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_seconds: u64,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub schema: String,
    pub user: String,
    pub password: String,
}

/// File storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub base_path: String,
    pub temp_path: String,
    pub max_file_size: u64, // bytes
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String, // "json" or "pretty"
}

impl AppConfig {
    /// Load configuration from environment variables
    ///
    /// # Errors
    /// Returns an error if required environment variables are missing or invalid
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::Environment::with_prefix("MEDIA_SERVICE"))
            .add_source(config::Environment::default())
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 3000)?
            .set_default("server.max_upload_size", 100_000_000)? // 100MB
            .set_default("database.url", "")?
            .set_default("database.max_connections", 10)?
            .set_default("database.min_connections", 1)?
            .set_default("database.acquire_timeout_seconds", 30)?
            .set_default("database.host", "localhost")?
            .set_default("database.port", 5432)?
            .set_default("database.database", "recipe_database")?
            .set_default("database.schema", "recipe_manager")?
            .set_default("database.user", "postgres")?
            .set_default("database.password", "")?
            .set_default("storage.base_path", "./media")?
            .set_default("storage.temp_path", "./media/temp")?
            .set_default("storage.max_file_size", 500_000_000)? // 500MB
            .set_default("logging.level", "info")?
            .set_default("logging.format", "json")?
            .build()?;

        settings.try_deserialize()
    }
}

impl ServerConfig {
    /// Get the socket address for binding
    ///
    /// # Panics
    /// Panics if the host/port configuration cannot be parsed into a valid socket address
    #[must_use]
    pub fn socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port).parse().expect("Invalid host/port configuration")
    }
}

impl DatabaseConfig {
    /// Get the `PostgreSQL` connection URL
    ///
    /// If url is provided, use it directly. Otherwise, construct from components.
    #[must_use]
    pub fn connection_url(&self) -> String {
        if self.url.is_empty() {
            format!(
                "postgres://{}:{}@{}:{}/{}",
                self.user, self.password, self.host, self.port, self.database
            )
        } else {
            self.url.clone()
        }
    }
}
