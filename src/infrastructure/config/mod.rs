use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Runtime mode for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeMode {
    Local,
    Production,
}

impl std::fmt::Display for RuntimeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => write!(f, "local"),
            Self::Production => write!(f, "production"),
        }
    }
}

impl std::str::FromStr for RuntimeMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" | "prod" => Ok(Self::Production),
            _ => Err(format!("Invalid runtime mode: {s}. Valid values: local, production")),
        }
    }
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub mode: RuntimeMode,
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
    // Global settings
    pub level: String,
    pub filter: Option<String>,

    // Console logging
    pub console_enabled: bool,
    pub console_format: LogFormat,

    // File logging
    pub file_enabled: bool,
    pub file_format: LogFormat,
    pub file_path: String,
    pub file_prefix: String,
    pub file_rotation: RotationPolicy,
    pub file_retention_days: u32,
    pub file_max_size_mb: Option<u64>,

    // Performance settings
    pub non_blocking: bool,
    pub buffer_size: Option<usize>,
}

/// Log output format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Pretty,
    Json,
    Compact,
}

/// Log file rotation policy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RotationPolicy {
    Daily,
    Hourly,
    #[serde(rename = "size")]
    Size(u64), // Size in MB
    Never,
}

impl AppConfig {
    /// Load configuration based on runtime mode
    ///
    /// # Errors
    /// Returns an error if required environment variables are missing or invalid
    pub fn load() -> Result<Self, config::ConfigError> {
        // Detect runtime mode from environment (default: local)
        let mode = std::env::var("RUN_MODE")
            .unwrap_or_else(|_| "local".to_string())
            .parse::<RuntimeMode>()
            .map_err(config::ConfigError::Message)?;

        Self::load_for_mode(mode)
    }

    /// Load configuration for a specific runtime mode
    ///
    /// # Errors
    /// Returns an error if required environment variables are missing or invalid
    pub fn load_for_mode(mode: RuntimeMode) -> Result<Self, config::ConfigError> {
        let mut builder = config::Config::builder();

        // For local mode only, load .env.local file (if it exists)
        if mode == RuntimeMode::Local {
            builder = builder.add_source(config::File::with_name(".env.local").required(false));
        }
        // Production mode relies solely on environment variables (no .env file)

        // Add environment variables (these override .env file values)
        builder = builder
            .add_source(config::Environment::with_prefix("MEDIA_SERVICE"))
            .add_source(config::Environment::default());

        // Set mode-specific defaults
        let (storage_base, storage_temp, log_path, console_format, file_format) = match mode {
            RuntimeMode::Local => ("./media", "./media/temp", "./logs", "pretty", "json"),
            RuntimeMode::Production => {
                ("/app/media", "/app/media/temp", "/app/logs", "json", "json")
            }
        };

        let settings = builder
            .set_default("mode", mode.to_string())?
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
            .set_default("storage.base_path", storage_base)?
            .set_default("storage.temp_path", storage_temp)?
            .set_default("storage.max_file_size", 500_000_000)? // 500MB
            // Logging configuration
            .set_default("logging.level", "info")?
            .set_default("logging.filter", None::<String>)?
            .set_default("logging.console_enabled", true)?
            .set_default("logging.console_format", console_format)?
            .set_default("logging.file_enabled", true)?
            .set_default("logging.file_format", file_format)?
            .set_default("logging.file_path", log_path)?
            .set_default("logging.file_prefix", "media-service")?
            .set_default("logging.file_rotation", "daily")?
            .set_default("logging.file_retention_days", 10)?
            .set_default("logging.file_max_size_mb", None::<u64>)?
            .set_default("logging.non_blocking", true)?
            .set_default("logging.buffer_size", 8192_i64)?
            .build()?;

        settings.try_deserialize()
    }

    /// Load configuration from environment variables only (legacy method)
    ///
    /// # Errors
    /// Returns an error if required environment variables are missing or invalid
    #[deprecated(note = "Use AppConfig::load() instead")]
    pub fn from_env() -> Result<Self, config::ConfigError> {
        Self::load_for_mode(RuntimeMode::Production)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_server_config() -> ServerConfig {
        ServerConfig { host: "127.0.0.1".to_string(), port: 8080, max_upload_size: 10_000_000 }
    }

    fn create_test_database_config() -> DatabaseConfig {
        DatabaseConfig {
            url: String::new(),
            max_connections: 10,
            min_connections: 1,
            acquire_timeout_seconds: 30,
            host: "localhost".to_string(),
            port: 5432,
            database: "test_db".to_string(),
            schema: "public".to_string(),
            user: "test_user".to_string(),
            password: "test_pass".to_string(),
        }
    }

    fn create_test_storage_config() -> StorageConfig {
        StorageConfig {
            base_path: "/tmp/media".to_string(),
            temp_path: "/tmp/media/temp".to_string(),
            max_file_size: 100_000_000,
        }
    }

    fn create_test_logging_config() -> LoggingConfig {
        LoggingConfig {
            level: "info".to_string(),
            filter: None,
            console_enabled: true,
            console_format: LogFormat::Pretty,
            file_enabled: true,
            file_format: LogFormat::Json,
            file_path: "./logs".to_string(),
            file_prefix: "test".to_string(),
            file_rotation: RotationPolicy::Daily,
            file_retention_days: 10,
            file_max_size_mb: None,
            non_blocking: true,
            buffer_size: Some(8192),
        }
    }

    #[test]
    fn test_server_config_socket_addr() {
        let config = create_test_server_config();
        let addr = config.socket_addr();

        assert_eq!(addr.ip().to_string(), "127.0.0.1");
        assert_eq!(addr.port(), 8080);
    }

    #[test]
    #[should_panic(expected = "Invalid host/port configuration")]
    fn test_server_config_invalid_socket_addr() {
        let config = ServerConfig {
            host: "invalid-host-name-that-cannot-be-resolved-by-dns".to_string(),
            port: 8080,
            max_upload_size: 1000,
        };
        let _ = config.socket_addr();
    }

    #[test]
    fn test_database_config_connection_url_from_components() {
        let config = create_test_database_config();
        let url = config.connection_url();

        assert_eq!(url, "postgres://test_user:test_pass@localhost:5432/test_db");
    }

    #[test]
    fn test_database_config_connection_url_direct() {
        let mut config = create_test_database_config();
        config.url = "postgres://direct:pass@example.com:5432/direct_db".to_string();

        let url = config.connection_url();
        assert_eq!(url, "postgres://direct:pass@example.com:5432/direct_db");
    }

    #[test]
    fn test_database_config_with_special_characters() {
        let mut config = create_test_database_config();
        config.user = "user@domain".to_string();
        config.password = "p@ssw0rd!".to_string();
        config.database = "my-database".to_string();

        let url = config.connection_url();
        assert_eq!(url, "postgres://user@domain:p@ssw0rd!@localhost:5432/my-database");
    }

    #[test]
    fn test_app_config_serialization() {
        let config = AppConfig {
            mode: RuntimeMode::Local,
            server: create_test_server_config(),
            database: create_test_database_config(),
            storage: create_test_storage_config(),
            logging: create_test_logging_config(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.server.host, deserialized.server.host);
        assert_eq!(config.server.port, deserialized.server.port);
        assert_eq!(config.database.host, deserialized.database.host);
        assert_eq!(config.storage.base_path, deserialized.storage.base_path);
        assert_eq!(config.logging.level, deserialized.logging.level);
    }

    #[test]
    fn test_config_clone_and_debug() {
        let config = create_test_server_config();
        let cloned = config.clone();

        assert_eq!(config.host, cloned.host);
        assert_eq!(config.port, cloned.port);
        assert_eq!(config.max_upload_size, cloned.max_upload_size);

        let debug_str = format!("{config:?}");
        assert!(debug_str.contains("ServerConfig"));
        assert!(debug_str.contains("127.0.0.1"));
    }

    // Note: Testing AppConfig::from_env() requires setting environment variables
    // which can be flaky in test environments. For integration tests, consider
    // using a separate test that sets up a controlled environment.

    #[test]
    fn test_config_defaults_structure() {
        // Test that the config structure supports expected values
        let storage = StorageConfig {
            base_path: "./test-media".to_string(),
            temp_path: "./test-media/temp".to_string(),
            max_file_size: 1_000_000,
        };

        assert!(storage.max_file_size > 0);
        assert!(!storage.base_path.is_empty());
        assert!(!storage.temp_path.is_empty());
    }

    #[test]
    fn test_logging_config_values() {
        let logging = create_test_logging_config();

        assert!(["trace", "debug", "info", "warn", "error"].contains(&logging.level.as_str()));
        assert!(logging.console_enabled || logging.file_enabled); // At least one output should be enabled
        assert!(logging.file_retention_days > 0);
        assert!(!logging.file_path.is_empty());
        assert!(!logging.file_prefix.is_empty());
    }

    #[test]
    fn test_database_config_validation() {
        let config = create_test_database_config();

        assert!(config.max_connections >= config.min_connections);
        assert!(config.port > 0);
        assert!(config.acquire_timeout_seconds > 0);
        assert!(!config.host.is_empty());
        assert!(!config.database.is_empty());
        assert!(!config.user.is_empty());
    }
}
