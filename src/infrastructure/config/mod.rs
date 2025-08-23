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
    pub middleware: MiddlewareConfig,
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

/// Middleware configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiddlewareConfig {
    pub auth: AuthConfig,
    pub rate_limiting: RateLimitingConfig,
    pub security: SecurityConfig,
    pub metrics: MetricsConfig,
    pub validation: ValidationConfig,
    pub request_logging: RequestLoggingConfig,
}

/// Authentication middleware configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub jwt_secret: String,
    pub jwt_expiry_hours: u64,
    pub require_auth_routes: Vec<String>,
    pub optional_auth_routes: Vec<String>,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    pub enabled: bool,
    pub default_requests_per_minute: u32,
    pub default_burst_capacity: u32,
    pub trust_forwarded_headers: bool,
    pub include_rate_limit_headers: bool,
    pub tiers: RateLimitTiersConfig,
}

/// Rate limit tiers configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitTiersConfig {
    pub health_requests_per_minute: u32,
    pub public_requests_per_minute: u32,
    pub authenticated_requests_per_minute: u32,
    pub upload_requests_per_minute: u32,
    pub admin_requests_per_minute: u32,
}

/// Security features flags
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct SecurityFeatures {
    pub hsts: bool,
    pub hsts_subdomains: bool,
    pub hsts_preload: bool,
    pub content_type_options: bool,
}

/// Security headers configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct SecurityConfig {
    pub enabled: bool,
    pub features: SecurityFeatures,
    pub hsts_max_age_seconds: u64,
    pub csp_policy: Option<String>,
    pub frame_options: String,  // "DENY", "SAMEORIGIN", "ALLOW-FROM uri"
    pub xss_protection: String, // "0", "1", "1; mode=block"
    pub referrer_policy: String,
    pub permissions_policy: Option<String>,
}

/// Metrics collection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub endpoint_enabled: bool,
    pub endpoint_path: String,
    pub prometheus_port: u16,
    pub collect_request_metrics: bool,
    pub collect_timing_metrics: bool,
    pub collect_error_metrics: bool,
    pub collect_business_metrics: bool,
    pub normalize_routes: bool,
    pub collection_interval_seconds: u64,
}

/// Request validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct ValidationConfig {
    pub enabled: bool,
    pub validate_content_type: bool,
    pub validate_body_size: bool,
    pub max_body_size_mb: u64,
    pub validate_json_structure: bool,
    pub validate_file_uploads: bool,
    pub max_file_size_mb: u64,
    pub allowed_file_types: Vec<String>,
    pub validate_headers: bool,
    pub validate_methods: bool,
}

/// Request/response logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct RequestLoggingConfig {
    pub enabled: bool,
    pub log_request_body: bool,
    pub log_response_body: bool,
    pub max_body_size_kb: u64,
    pub log_request_headers: bool,
    pub log_response_headers: bool,
    pub excluded_headers: Vec<String>,
    pub log_timing: bool,
    pub slow_request_threshold_ms: u64,
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
    #[allow(clippy::too_many_lines)]
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
            // Middleware configuration defaults
            .set_default("middleware.auth.enabled", true)?
            .set_default("middleware.auth.jwt_secret", "change-me-in-production")?
            .set_default("middleware.auth.jwt_expiry_hours", 24)?
            .set_default("middleware.auth.require_auth_routes", Vec::<String>::new())?
            .set_default("middleware.auth.optional_auth_routes", Vec::<String>::new())?
            .set_default("middleware.rate_limiting.enabled", true)?
            .set_default("middleware.rate_limiting.default_requests_per_minute", 100)?
            .set_default("middleware.rate_limiting.default_burst_capacity", 10)?
            .set_default("middleware.rate_limiting.trust_forwarded_headers", false)?
            .set_default("middleware.rate_limiting.include_rate_limit_headers", true)?
            .set_default("middleware.rate_limiting.tiers.health_requests_per_minute", 1000)?
            .set_default("middleware.rate_limiting.tiers.public_requests_per_minute", 60)?
            .set_default("middleware.rate_limiting.tiers.authenticated_requests_per_minute", 200)?
            .set_default("middleware.rate_limiting.tiers.upload_requests_per_minute", 10)?
            .set_default("middleware.rate_limiting.tiers.admin_requests_per_minute", 500)?
            .set_default("middleware.security.enabled", true)?
            .set_default("middleware.security.features.hsts", mode == RuntimeMode::Production)?
            .set_default("middleware.security.hsts_max_age_seconds", 31_536_000)? // 1 year
            .set_default("middleware.security.features.hsts_subdomains", true)?
            .set_default("middleware.security.features.hsts_preload", false)?
            .set_default("middleware.security.csp_policy", "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; media-src 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; base-uri 'self'")?
            .set_default("middleware.security.frame_options", "DENY")?
            .set_default("middleware.security.features.content_type_options", true)?
            .set_default("middleware.security.xss_protection", "1; mode=block")?
            .set_default("middleware.security.referrer_policy", "strict-origin-when-cross-origin")?
            .set_default("middleware.security.permissions_policy", "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()")?
            .set_default("middleware.metrics.enabled", true)?
            .set_default("middleware.metrics.endpoint_enabled", true)?
            .set_default("middleware.metrics.endpoint_path", "/metrics")?
            .set_default("middleware.metrics.prometheus_port", 9090)?
            .set_default("middleware.metrics.collect_request_metrics", true)?
            .set_default("middleware.metrics.collect_timing_metrics", true)?
            .set_default("middleware.metrics.collect_error_metrics", true)?
            .set_default("middleware.metrics.collect_business_metrics", true)?
            .set_default("middleware.metrics.normalize_routes", true)?
            .set_default("middleware.metrics.collection_interval_seconds", 10)?
            .set_default("middleware.validation.enabled", true)?
            .set_default("middleware.validation.validate_content_type", true)?
            .set_default("middleware.validation.validate_body_size", true)?
            .set_default("middleware.validation.max_body_size_mb", 100)?
            .set_default("middleware.validation.validate_json_structure", true)?
            .set_default("middleware.validation.validate_file_uploads", true)?
            .set_default("middleware.validation.max_file_size_mb", 50)?
            .set_default("middleware.validation.allowed_file_types", vec!["image/jpeg", "image/png", "image/webp", "image/avif", "video/mp4", "video/webm"])?
            .set_default("middleware.validation.validate_headers", true)?
            .set_default("middleware.validation.validate_methods", true)?
            .set_default("middleware.request_logging.enabled", mode == RuntimeMode::Local)?
            .set_default("middleware.request_logging.log_request_body", mode == RuntimeMode::Local)?
            .set_default("middleware.request_logging.log_response_body", mode == RuntimeMode::Local)?
            .set_default("middleware.request_logging.max_body_size_kb", if mode == RuntimeMode::Local { 10 } else { 1 })?
            .set_default("middleware.request_logging.log_request_headers", mode == RuntimeMode::Local)?
            .set_default("middleware.request_logging.log_response_headers", false)?
            .set_default("middleware.request_logging.excluded_headers", vec!["authorization", "cookie", "set-cookie", "x-api-key", "x-auth-token"])?
            .set_default("middleware.request_logging.log_timing", true)?
            .set_default("middleware.request_logging.slow_request_threshold_ms", if mode == RuntimeMode::Local { 500 } else { 2000 })?
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

    fn create_test_middleware_config() -> MiddlewareConfig {
        MiddlewareConfig {
            auth: AuthConfig {
                enabled: true,
                jwt_secret: "test-secret-key".to_string(),
                jwt_expiry_hours: 24,
                require_auth_routes: vec!["/api/v1/media-management/media".to_string()],
                optional_auth_routes: vec![],
            },
            rate_limiting: RateLimitingConfig {
                enabled: true,
                default_requests_per_minute: 100,
                default_burst_capacity: 10,
                trust_forwarded_headers: false,
                include_rate_limit_headers: true,
                tiers: RateLimitTiersConfig {
                    health_requests_per_minute: 1000,
                    public_requests_per_minute: 60,
                    authenticated_requests_per_minute: 200,
                    upload_requests_per_minute: 10,
                    admin_requests_per_minute: 500,
                },
            },
            security: SecurityConfig {
                enabled: true,
                features: SecurityFeatures {
                    hsts: false,
                    hsts_subdomains: true,
                    hsts_preload: false,
                    content_type_options: true,
                },
                hsts_max_age_seconds: 31_536_000,
                csp_policy: Some("default-src 'self'".to_string()),
                frame_options: "DENY".to_string(),
                xss_protection: "1; mode=block".to_string(),
                referrer_policy: "strict-origin-when-cross-origin".to_string(),
                permissions_policy: Some("camera=(), microphone=()".to_string()),
            },
            metrics: MetricsConfig {
                enabled: true,
                endpoint_enabled: true,
                endpoint_path: "/metrics".to_string(),
                prometheus_port: 9090,
                collect_request_metrics: true,
                collect_timing_metrics: true,
                collect_error_metrics: true,
                collect_business_metrics: true,
                normalize_routes: true,
                collection_interval_seconds: 10,
            },
            validation: ValidationConfig {
                enabled: true,
                validate_content_type: true,
                validate_body_size: true,
                max_body_size_mb: 100,
                validate_json_structure: true,
                validate_file_uploads: true,
                max_file_size_mb: 50,
                allowed_file_types: vec!["image/jpeg".to_string(), "image/png".to_string()],
                validate_headers: true,
                validate_methods: true,
            },
            request_logging: RequestLoggingConfig {
                enabled: true,
                log_request_body: true,
                log_response_body: false,
                max_body_size_kb: 10,
                log_request_headers: true,
                log_response_headers: false,
                excluded_headers: vec!["authorization".to_string(), "cookie".to_string()],
                log_timing: true,
                slow_request_threshold_ms: 500,
            },
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
            middleware: create_test_middleware_config(),
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
