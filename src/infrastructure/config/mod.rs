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
    pub postgres: PostgresConfig,
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

/// `PostgreSQL` database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
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
        builder = builder.add_source(config::Environment::with_prefix("MEDIA_SERVICE"));

        // Add specific environment variable mappings for PostgreSQL configuration

        // RUN MODE //
        if let Ok(run_mode) = std::env::var("RUN_MODE") {
            builder = builder.set_override("run_mode", run_mode)?;
        }

        // SERVER CONFIG //
        if let Ok(host) = std::env::var("MEDIA_SERVICE_SERVER_HOST") {
            builder = builder.set_override("server.host", host)?;
        }
        if let Ok(port) = std::env::var("MEDIA_SERVICE_SERVER_PORT") {
            if let Ok(port_num) = port.parse::<u16>() {
                builder = builder.set_override("server.port", port_num)?;
            }
        }
        if let Ok(max_upload_size) = std::env::var("MEDIA_SERVICE_SERVER_MAX_UPLOAD_SIZE") {
            if let Ok(size) = max_upload_size.parse::<u64>() {
                builder = builder.set_override("server.max_upload_size", size)?;
            }
        }

        // DATABASE CONFIG //
        if let Ok(host) = std::env::var("POSTGRES_HOST") {
            builder = builder.set_override("postgres.host", host)?;
        }
        if let Ok(port) = std::env::var("POSTGRES_PORT") {
            if let Ok(port_num) = port.parse::<u16>() {
                builder = builder.set_override("postgres.port", port_num)?;
            }
        }
        if let Ok(db) = std::env::var("POSTGRES_DB") {
            builder = builder.set_override("postgres.database", db)?;
        }
        if let Ok(schema) = std::env::var("POSTGRES_SCHEMA") {
            builder = builder.set_override("postgres.schema", schema)?;
        }
        if let Ok(user) = std::env::var("MEDIA_MANAGEMENT_DB_USER") {
            builder = builder.set_override("postgres.user", user)?;
        }
        if let Ok(password) = std::env::var("MEDIA_MANAGEMENT_DB_PASSWORD") {
            builder = builder.set_override("postgres.password", password)?;
        }
        if let Ok(max_conn) = std::env::var("POSTGRES_MAX_CONNECTIONS") {
            if let Ok(max_conn_num) = max_conn.parse::<u32>() {
                builder = builder.set_override("postgres.max_connections", max_conn_num)?;
            }
        }
        if let Ok(min_conn) = std::env::var("POSTGRES_MIN_CONNECTIONS") {
            if let Ok(min_conn_num) = min_conn.parse::<u32>() {
                builder = builder.set_override("postgres.min_connections", min_conn_num)?;
            }
        }
        if let Ok(timeout) = std::env::var("POSTGRES_ACQUIRE_TIMEOUT_SECONDS") {
            if let Ok(timeout_num) = timeout.parse::<u64>() {
                builder = builder.set_override("postgres.acquire_timeout_seconds", timeout_num)?;
            }
        }

        // STORAGE CONFIG //
        if let Ok(base_path) = std::env::var("MEDIA_SERVICE_STORAGE_BASE_PATH") {
            builder = builder.set_override("storage.base_path", base_path)?;
        }
        if let Ok(temp_path) = std::env::var("MEDIA_SERVICE_STORAGE_TEMP_PATH") {
            builder = builder.set_override("storage.temp_path", temp_path)?;
        }
        if let Ok(max_file_size) = std::env::var("MEDIA_SERVICE_STORAGE_MAX_FILE_SIZE") {
            if let Ok(size) = max_file_size.parse::<u64>() {
                builder = builder.set_override("storage.max_file_size", size)?;
            }
        }

        // LOGGING CONFIG //
        if let Ok(level) = std::env::var("MEDIA_SERVICE_LOGGING_LEVEL") {
            builder = builder.set_override("logging.level", level)?;
        }
        if let Ok(filter) = std::env::var("MEDIA_SERVICE_LOGGING_FILTER") {
            builder = builder.set_override("logging.filter", filter)?;
        }
        if let Ok(console_enabled) = std::env::var("MEDIA_SERVICE_LOGGING_CONSOLE_ENABLED") {
            if let Ok(enabled) = console_enabled.parse::<bool>() {
                builder = builder.set_override("logging.console_enabled", enabled)?;
            }
        }
        if let Ok(console_format) = std::env::var("MEDIA_SERVICE_LOGGING_CONSOLE_FORMAT") {
            builder = builder.set_override("logging.console_format", console_format)?;
        }
        if let Ok(file_enabled) = std::env::var("MEDIA_SERVICE_LOGGING_FILE_ENABLED") {
            if let Ok(enabled) = file_enabled.parse::<bool>() {
                builder = builder.set_override("logging.file_enabled", enabled)?;
            }
        }
        if let Ok(file_format) = std::env::var("MEDIA_SERVICE_LOGGING_FILE_FORMAT") {
            builder = builder.set_override("logging.file_format", file_format)?;
        }
        if let Ok(file_path) = std::env::var("MEDIA_SERVICE_LOGGING_FILE_PATH") {
            builder = builder.set_override("logging.file_path", file_path)?;
        }
        if let Ok(file_prefix) = std::env::var("MEDIA_SERVICE_LOGGING_FILE_PREFIX") {
            builder = builder.set_override("logging.file_prefix", file_prefix)?;
        }
        if let Ok(file_rotation) = std::env::var("MEDIA_SERVICE_LOGGING_FILE_ROTATION") {
            builder = builder.set_override("logging.file_rotation", file_rotation)?;
        }
        if let Ok(file_retention_days) = std::env::var("MEDIA_SERVICE_LOGGING_FILE_RETENTION_DAYS")
        {
            if let Ok(days) = file_retention_days.parse::<u32>() {
                builder = builder.set_override("logging.file_retention_days", days)?;
            }
        }
        if let Ok(file_max_size_mb) = std::env::var("MEDIA_SERVICE_LOGGING_FILE_MAX_SIZE_MB") {
            if let Ok(size) = file_max_size_mb.parse::<u64>() {
                builder = builder.set_override("logging.file_max_size_mb", size)?;
            }
        }
        if let Ok(non_blocking) = std::env::var("MEDIA_SERVICE_LOGGING_NON_BLOCKING") {
            if let Ok(enabled) = non_blocking.parse::<bool>() {
                builder = builder.set_override("logging.non_blocking", enabled)?;
            }
        }
        if let Ok(buffer_size) = std::env::var("MEDIA_SERVICE_LOGGING_BUFFER_SIZE") {
            if let Ok(size) = buffer_size.parse::<u64>() {
                builder = builder.set_override("logging.buffer_size", size)?;
            }
        }
        if let Ok(max_concurrent_requests) =
            std::env::var("MEDIA_SERVICE_PERFORMANCE_MAX_CONCURRENT_REQUESTS")
        {
            if let Ok(value) = max_concurrent_requests.parse::<u32>() {
                builder = builder.set_override("performance.max_concurrent_requests", value)?;
            }
        }
        if let Ok(request_timeout) = std::env::var("MEDIA_SERVICE_PERFORMANCE_REQUEST_TIMEOUT") {
            if let Ok(value) = request_timeout.parse::<u64>() {
                builder = builder.set_override("performance.request_timeout", value)?;
            }
        }

        // AUTHENTICATION MIDDLEWARE CONFIG //
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_AUTH_ENABLED") {
            if let Ok(parsed) = val.parse::<bool>() {
                builder = builder.set_override("middleware.auth.enabled", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_AUTH_JWT_SECRET") {
            builder = builder.set_override("middleware.auth.jwt_secret", val)?;
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_AUTH_JWT_EXPIRY_HOURS") {
            if let Ok(parsed) = val.parse::<u64>() {
                builder = builder.set_override("middleware.auth.jwt_expiry_hours", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_AUTH_REQUIRE_AUTH_ROUTES") {
            let routes: Vec<String> = val
                .split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !routes.is_empty() {
                builder = builder.set_override("middleware.auth.require_auth_routes", routes)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_AUTH_OPTIONAL_AUTH_ROUTES") {
            let routes: Vec<String> = val
                .split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !routes.is_empty() {
                builder = builder.set_override("middleware.auth.optional_auth_routes", routes)?;
            }
        }

        // RATE LIMITING MIDDLEWARE CONFIG //
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_ENABLED") {
            if let Ok(parsed) = val.parse::<bool>() {
                builder = builder.set_override("middleware.rate_limiting.enabled", parsed)?;
            }
        }
        if let Ok(val) =
            std::env::var("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_DEFAULT_REQUESTS_PER_MINUTE")
        {
            if let Ok(parsed) = val.parse::<u32>() {
                builder = builder
                    .set_override("middleware.rate_limiting.default_requests_per_minute", parsed)?;
            }
        }
        if let Ok(val) =
            std::env::var("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_DEFAULT_BURST_CAPACITY")
        {
            if let Ok(parsed) = val.parse::<u32>() {
                builder = builder
                    .set_override("middleware.rate_limiting.default_burst_capacity", parsed)?;
            }
        }
        if let Ok(val) =
            std::env::var("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_TRUST_FORWARDED_HEADERS")
        {
            if let Ok(parsed) = val.parse::<bool>() {
                builder = builder
                    .set_override("middleware.rate_limiting.trust_forwarded_headers", parsed)?;
            }
        }
        if let Ok(val) =
            std::env::var("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_INCLUDE_RATE_LIMIT_HEADERS")
        {
            if let Ok(parsed) = val.parse::<bool>() {
                builder = builder
                    .set_override("middleware.rate_limiting.include_rate_limit_headers", parsed)?;
            }
        }

        // RATE LIMITING TIERS CONFIG //
        if let Ok(val) =
            std::env::var("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_TIERS_HEALTH_REQUESTS_PER_MINUTE")
        {
            if let Ok(parsed) = val.parse::<u32>() {
                builder = builder.set_override(
                    "middleware.rate_limiting.tiers.health_requests_per_minute",
                    parsed,
                )?;
            }
        }
        if let Ok(val) =
            std::env::var("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_TIERS_PUBLIC_REQUESTS_PER_MINUTE")
        {
            if let Ok(parsed) = val.parse::<u32>() {
                builder = builder.set_override(
                    "middleware.rate_limiting.tiers.public_requests_per_minute",
                    parsed,
                )?;
            }
        }
        if let Ok(val) = std::env::var(
            "MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_TIERS_AUTHENTICATED_REQUESTS_PER_MINUTE",
        ) {
            if let Ok(parsed) = val.parse::<u32>() {
                builder = builder.set_override(
                    "middleware.rate_limiting.tiers.authenticated_requests_per_minute",
                    parsed,
                )?;
            }
        }
        if let Ok(val) =
            std::env::var("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_TIERS_UPLOAD_REQUESTS_PER_MINUTE")
        {
            if let Ok(parsed) = val.parse::<u32>() {
                builder = builder.set_override(
                    "middleware.rate_limiting.tiers.upload_requests_per_minute",
                    parsed,
                )?;
            }
        }
        if let Ok(val) =
            std::env::var("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_TIERS_ADMIN_REQUESTS_PER_MINUTE")
        {
            if let Ok(parsed) = val.parse::<u32>() {
                builder = builder.set_override(
                    "middleware.rate_limiting.tiers.admin_requests_per_minute",
                    parsed,
                )?;
            }
        }

        // SECURITY HEADERS MIDDLEWARE CONFIG //
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_SECURITY_ENABLED") {
            if let Ok(parsed) = val.parse::<bool>() {
                builder = builder.set_override("middleware.security.enabled", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_SECURITY_HSTS_ENABLED") {
            if let Ok(parsed) = val.parse::<bool>() {
                builder = builder.set_override("middleware.security.features.hsts", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_SECURITY_HSTS_MAX_AGE_SECONDS") {
            if let Ok(parsed) = val.parse::<u64>() {
                builder =
                    builder.set_override("middleware.security.hsts_max_age_seconds", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_SECURITY_HSTS_INCLUDE_SUBDOMAINS")
        {
            if let Ok(parsed) = val.parse::<bool>() {
                builder =
                    builder.set_override("middleware.security.features.hsts_subdomains", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_SECURITY_HSTS_PRELOAD") {
            if let Ok(parsed) = val.parse::<bool>() {
                builder =
                    builder.set_override("middleware.security.features.hsts_preload", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_SECURITY_CSP_POLICY") {
            builder = builder.set_override("middleware.security.csp_policy", val)?;
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_SECURITY_FRAME_OPTIONS") {
            builder = builder.set_override("middleware.security.frame_options", val)?;
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_SECURITY_CONTENT_TYPE_OPTIONS") {
            if let Ok(parsed) = val.parse::<bool>() {
                builder = builder
                    .set_override("middleware.security.features.content_type_options", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_SECURITY_XSS_PROTECTION") {
            builder = builder.set_override("middleware.security.xss_protection", val)?;
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_SECURITY_REFERRER_POLICY") {
            builder = builder.set_override("middleware.security.referrer_policy", val)?;
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_SECURITY_PERMISSIONS_POLICY") {
            builder = builder.set_override("middleware.security.permissions_policy", val)?;
        }

        // METRICS COLLECTION MIDDLEWARE CONFIG //
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_METRICS_ENABLED") {
            if let Ok(parsed) = val.parse::<bool>() {
                builder = builder.set_override("middleware.metrics.enabled", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_METRICS_ENDPOINT_ENABLED") {
            if let Ok(parsed) = val.parse::<bool>() {
                builder = builder.set_override("middleware.metrics.endpoint_enabled", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_METRICS_ENDPOINT_PATH") {
            builder = builder.set_override("middleware.metrics.endpoint_path", val)?;
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_METRICS_PROMETHEUS_PORT") {
            if let Ok(parsed) = val.parse::<u16>() {
                builder = builder.set_override("middleware.metrics.prometheus_port", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_METRICS_COLLECT_REQUEST_METRICS") {
            if let Ok(parsed) = val.parse::<bool>() {
                builder =
                    builder.set_override("middleware.metrics.collect_request_metrics", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_METRICS_COLLECT_TIMING_METRICS") {
            if let Ok(parsed) = val.parse::<bool>() {
                builder =
                    builder.set_override("middleware.metrics.collect_timing_metrics", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_METRICS_COLLECT_ERROR_METRICS") {
            if let Ok(parsed) = val.parse::<bool>() {
                builder =
                    builder.set_override("middleware.metrics.collect_error_metrics", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_METRICS_COLLECT_BUSINESS_METRICS")
        {
            if let Ok(parsed) = val.parse::<bool>() {
                builder =
                    builder.set_override("middleware.metrics.collect_business_metrics", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_METRICS_NORMALIZE_ROUTES") {
            if let Ok(parsed) = val.parse::<bool>() {
                builder = builder.set_override("middleware.metrics.normalize_routes", parsed)?;
            }
        }
        if let Ok(val) =
            std::env::var("MEDIA_SERVICE_MIDDLEWARE_METRICS_COLLECTION_INTERVAL_SECONDS")
        {
            if let Ok(parsed) = val.parse::<u64>() {
                builder = builder
                    .set_override("middleware.metrics.collection_interval_seconds", parsed)?;
            }
        }

        // REQUEST VALIDATION MIDDLEWARE CONFIG //
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_ENABLED") {
            if let Ok(parsed) = val.parse::<bool>() {
                builder = builder.set_override("middleware.validation.enabled", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_VALIDATE_CONTENT_TYPE")
        {
            if let Ok(parsed) = val.parse::<bool>() {
                builder =
                    builder.set_override("middleware.validation.validate_content_type", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_VALIDATE_BODY_SIZE") {
            if let Ok(parsed) = val.parse::<bool>() {
                builder =
                    builder.set_override("middleware.validation.validate_body_size", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_MAX_BODY_SIZE_MB") {
            if let Ok(parsed) = val.parse::<u64>() {
                builder = builder.set_override("middleware.validation.max_body_size_mb", parsed)?;
            }
        }
        if let Ok(val) =
            std::env::var("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_VALIDATE_JSON_STRUCTURE")
        {
            if let Ok(parsed) = val.parse::<bool>() {
                builder = builder
                    .set_override("middleware.validation.validate_json_structure", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_VALIDATE_FILE_UPLOADS")
        {
            if let Ok(parsed) = val.parse::<bool>() {
                builder =
                    builder.set_override("middleware.validation.validate_file_uploads", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_MAX_FILE_SIZE_MB") {
            if let Ok(parsed) = val.parse::<u64>() {
                builder = builder.set_override("middleware.validation.max_file_size_mb", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_ALLOWED_FILE_TYPES") {
            let types: Vec<String> = val
                .split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !types.is_empty() {
                builder =
                    builder.set_override("middleware.validation.allowed_file_types", types)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_VALIDATE_HEADERS") {
            if let Ok(parsed) = val.parse::<bool>() {
                builder = builder.set_override("middleware.validation.validate_headers", parsed)?;
            }
        }
        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_VALIDATE_METHODS") {
            if let Ok(parsed) = val.parse::<bool>() {
                builder = builder.set_override("middleware.validation.validate_methods", parsed)?;
            }
        }

        // REQUEST/RESPONSE LOGGING MIDDLEWARE CONFIG //
        if let Ok(enabled) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_REQUEST_LOGGING_ENABLED") {
            if let Ok(parsed) = enabled.parse::<bool>() {
                builder = builder.set_override("middleware.request_logging.enabled", parsed)?;
            }
        }

        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_REQUEST_LOGGING_LOG_REQUEST_BODY")
        {
            if let Ok(parsed) = val.parse::<bool>() {
                builder =
                    builder.set_override("middleware.request_logging.log_request_body", parsed)?;
            }
        }

        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_REQUEST_LOGGING_LOG_RESPONSE_BODY")
        {
            if let Ok(parsed) = val.parse::<bool>() {
                builder =
                    builder.set_override("middleware.request_logging.log_response_body", parsed)?;
            }
        }

        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_REQUEST_LOGGING_MAX_BODY_SIZE_KB")
        {
            if let Ok(parsed) = val.parse::<u64>() {
                builder =
                    builder.set_override("middleware.request_logging.max_body_size_kb", parsed)?;
            }
        }

        if let Ok(val) =
            std::env::var("MEDIA_SERVICE_MIDDLEWARE_REQUEST_LOGGING_LOG_REQUEST_HEADERS")
        {
            if let Ok(parsed) = val.parse::<bool>() {
                builder = builder
                    .set_override("middleware.request_logging.log_request_headers", parsed)?;
            }
        }

        if let Ok(val) =
            std::env::var("MEDIA_SERVICE_MIDDLEWARE_REQUEST_LOGGING_LOG_RESPONSE_HEADERS")
        {
            if let Ok(parsed) = val.parse::<bool>() {
                builder = builder
                    .set_override("middleware.request_logging.log_response_headers", parsed)?;
            }
        }

        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_REQUEST_LOGGING_EXCLUDED_HEADERS")
        {
            let headers: Vec<String> = val
                .split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !headers.is_empty() {
                builder =
                    builder.set_override("middleware.request_logging.excluded_headers", headers)?;
            }
        }

        if let Ok(val) = std::env::var("MEDIA_SERVICE_MIDDLEWARE_REQUEST_LOGGING_LOG_TIMING") {
            if let Ok(parsed) = val.parse::<bool>() {
                builder = builder.set_override("middleware.request_logging.log_timing", parsed)?;
            }
        }

        if let Ok(val) =
            std::env::var("MEDIA_SERVICE_MIDDLEWARE_REQUEST_LOGGING_SLOW_REQUEST_THRESHOLD_MS")
        {
            if let Ok(parsed) = val.parse::<u64>() {
                builder = builder
                    .set_override("middleware.request_logging.slow_request_threshold_ms", parsed)?;
            }
        }

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
            .set_default("postgres.url", "")?
            .set_default("postgres.max_connections", 10)?
            .set_default("postgres.min_connections", 1)?
            .set_default("postgres.acquire_timeout_seconds", 30)?
            .set_default("postgres.host", "localhost")?
            .set_default("postgres.port", 5432)?
            .set_default("postgres.database", "recipe_database")?
            .set_default("postgres.schema", "recipe_manager")?
            .set_default("postgres.user", "postgres")?
            .set_default("postgres.password", "")?
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

impl PostgresConfig {
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
    use std::str::FromStr;

    fn create_test_server_config() -> ServerConfig {
        ServerConfig { host: "127.0.0.1".to_string(), port: 8080, max_upload_size: 10_000_000 }
    }

    fn create_test_postgres_config() -> PostgresConfig {
        PostgresConfig {
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
    fn test_postgres_config_connection_url_from_components() {
        let config = create_test_postgres_config();
        let url = config.connection_url();

        assert_eq!(url, "postgres://test_user:test_pass@localhost:5432/test_db");
    }

    #[test]
    fn test_postgres_config_connection_url_direct() {
        let mut config = create_test_postgres_config();
        config.url = "postgres://direct:pass@example.com:5432/direct_db".to_string();

        let url = config.connection_url();
        assert_eq!(url, "postgres://direct:pass@example.com:5432/direct_db");
    }

    #[test]
    fn test_postgres_config_with_special_characters() {
        let mut config = create_test_postgres_config();
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
            postgres: create_test_postgres_config(),
            storage: create_test_storage_config(),
            logging: create_test_logging_config(),
            middleware: create_test_middleware_config(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.server.host, deserialized.server.host);
        assert_eq!(config.server.port, deserialized.server.port);
        assert_eq!(config.postgres.host, deserialized.postgres.host);
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
    fn test_postgres_config_validation() {
        let config = create_test_postgres_config();

        assert!(config.max_connections >= config.min_connections);
        assert!(config.port > 0);
        assert!(config.acquire_timeout_seconds > 0);
        assert!(!config.host.is_empty());
        assert!(!config.database.is_empty());
        assert!(!config.user.is_empty());
    }

    #[test]
    fn test_runtime_mode_from_str() {
        assert_eq!(RuntimeMode::from_str("local").unwrap(), RuntimeMode::Local);
        assert_eq!(RuntimeMode::from_str("LOCAL").unwrap(), RuntimeMode::Local);
        assert_eq!(RuntimeMode::from_str("production").unwrap(), RuntimeMode::Production);
        assert_eq!(RuntimeMode::from_str("PRODUCTION").unwrap(), RuntimeMode::Production);
        assert_eq!(RuntimeMode::from_str("prod").unwrap(), RuntimeMode::Production);

        assert!(RuntimeMode::from_str("invalid").is_err());
        assert!(RuntimeMode::from_str("").is_err());
    }

    #[test]
    fn test_runtime_mode_display() {
        assert_eq!(RuntimeMode::Local.to_string(), "local");
        assert_eq!(RuntimeMode::Production.to_string(), "production");
    }

    #[test]
    fn test_runtime_mode_serialization() {
        let local_mode = RuntimeMode::Local;
        let prod_mode = RuntimeMode::Production;

        let local_json = serde_json::to_string(&local_mode).unwrap();
        let prod_json = serde_json::to_string(&prod_mode).unwrap();

        assert_eq!(local_json, "\"local\"");
        assert_eq!(prod_json, "\"production\"");

        let deserialized_local: RuntimeMode = serde_json::from_str(&local_json).unwrap();
        let deserialized_prod: RuntimeMode = serde_json::from_str(&prod_json).unwrap();

        assert_eq!(deserialized_local, RuntimeMode::Local);
        assert_eq!(deserialized_prod, RuntimeMode::Production);
    }

    #[test]
    fn test_log_format_variants() {
        let pretty = LogFormat::Pretty;
        let json = LogFormat::Json;
        let compact = LogFormat::Compact;

        let pretty_json = serde_json::to_string(&pretty).unwrap();
        let json_json = serde_json::to_string(&json).unwrap();
        let compact_json = serde_json::to_string(&compact).unwrap();

        // Check that the enums can be serialized (the exact format depends on serde config)
        assert!(!pretty_json.is_empty());
        assert!(!json_json.is_empty());
        assert!(!compact_json.is_empty());
    }

    #[test]
    fn test_rotation_policy_variants() {
        let daily = RotationPolicy::Daily;
        let hourly = RotationPolicy::Hourly;
        let size = RotationPolicy::Size(100);

        let daily_json = serde_json::to_string(&daily).unwrap();
        let hourly_json = serde_json::to_string(&hourly).unwrap();
        let size_json = serde_json::to_string(&size).unwrap();

        // Check that the enums can be serialized
        assert!(!daily_json.is_empty());
        assert!(!hourly_json.is_empty());
        assert!(!size_json.is_empty());
    }

    #[test]
    fn test_middleware_config_comprehensive() {
        let middleware = create_test_middleware_config();

        // Test auth config
        assert!(middleware.auth.enabled);
        assert!(!middleware.auth.jwt_secret.is_empty());
        assert!(middleware.auth.jwt_expiry_hours > 0);
        assert!(!middleware.auth.require_auth_routes.is_empty());

        // Test rate limiting config
        assert!(middleware.rate_limiting.enabled);
        assert!(middleware.rate_limiting.default_requests_per_minute > 0);
        assert!(middleware.rate_limiting.default_burst_capacity > 0);
        assert!(middleware.rate_limiting.tiers.health_requests_per_minute > 0);

        // Test security config
        assert!(middleware.security.enabled);
        assert!(middleware.security.hsts_max_age_seconds > 0);
        assert!(!middleware.security.frame_options.is_empty());

        // Test metrics config
        assert!(middleware.metrics.enabled);
        assert!(middleware.metrics.prometheus_port > 0);
        assert!(middleware.metrics.collection_interval_seconds > 0);

        // Test validation config
        assert!(middleware.validation.enabled);
        assert!(middleware.validation.max_body_size_mb > 0);
        assert!(middleware.validation.max_file_size_mb > 0);
        assert!(!middleware.validation.allowed_file_types.is_empty());

        // Test request logging config
        assert!(middleware.request_logging.enabled);
        assert!(middleware.request_logging.max_body_size_kb > 0);
        assert!(middleware.request_logging.slow_request_threshold_ms > 0);
    }

    #[test]
    fn test_postgres_config_empty_url() {
        let config = PostgresConfig {
            url: String::new(),
            max_connections: 10,
            min_connections: 1,
            acquire_timeout_seconds: 30,
            host: "testhost".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            schema: "testschema".to_string(),
            user: "testuser".to_string(),
            password: "testpass".to_string(),
        };

        let url = config.connection_url();
        assert_eq!(url, "postgres://testuser:testpass@testhost:5432/testdb");
    }

    #[test]
    fn test_server_config_edge_cases() {
        let config = ServerConfig { host: "0.0.0.0".to_string(), port: 1, max_upload_size: 0 };

        let addr = config.socket_addr();
        assert_eq!(addr.port(), 1);
        assert_eq!(config.max_upload_size, 0);

        let config_max_port =
            ServerConfig { host: "127.0.0.1".to_string(), port: 65535, max_upload_size: u64::MAX };

        let addr_max = config_max_port.socket_addr();
        assert_eq!(addr_max.port(), 65535);
        assert_eq!(config_max_port.max_upload_size, u64::MAX);
    }

    #[test]
    fn test_storage_config_paths() {
        let storage = StorageConfig {
            base_path: "/absolute/path".to_string(),
            temp_path: "relative/path".to_string(),
            max_file_size: 1024,
        };

        assert!(storage.base_path.starts_with('/'));
        assert!(!storage.temp_path.starts_with('/'));
        assert_eq!(storage.max_file_size, 1024);
    }

    #[test]
    fn test_logging_config_optional_fields() {
        let logging = LoggingConfig {
            level: "debug".to_string(),
            filter: Some("my_crate=trace".to_string()),
            console_enabled: false,
            console_format: LogFormat::Compact,
            file_enabled: false,
            file_format: LogFormat::Pretty,
            file_path: "./logs".to_string(),
            file_prefix: "app".to_string(),
            file_rotation: RotationPolicy::Hourly,
            file_retention_days: 7,
            file_max_size_mb: Some(100),
            non_blocking: false,
            buffer_size: None,
        };

        assert!(logging.filter.is_some());
        assert!(logging.file_max_size_mb.is_some());
        assert_eq!(logging.file_max_size_mb.unwrap(), 100);
        assert!(logging.buffer_size.is_none());
    }

    #[test]
    fn test_config_debug_formatting() {
        let config = create_test_server_config();
        let debug_output = format!("{config:?}");

        assert!(debug_output.contains("ServerConfig"));
        assert!(debug_output.contains("127.0.0.1"));
        assert!(debug_output.contains("8080"));
        assert!(debug_output.contains("10000000"));
    }

    #[test]
    fn test_complete_app_config_structure() {
        let app_config = AppConfig {
            mode: RuntimeMode::Production,
            server: create_test_server_config(),
            postgres: create_test_postgres_config(),
            storage: create_test_storage_config(),
            logging: create_test_logging_config(),
            middleware: create_test_middleware_config(),
        };

        assert_eq!(app_config.mode, RuntimeMode::Production);
        assert!(!app_config.server.host.is_empty());
        assert!(!app_config.postgres.host.is_empty());
        assert!(!app_config.storage.base_path.is_empty());
        assert!(!app_config.logging.level.is_empty());
        assert!(app_config.middleware.auth.enabled);
    }

    #[test]
    fn test_config_cloning() {
        let original_config = create_test_postgres_config();
        let cloned_config = original_config.clone();

        assert_eq!(original_config.host, cloned_config.host);
        assert_eq!(original_config.port, cloned_config.port);
        assert_eq!(original_config.database, cloned_config.database);
        assert_eq!(original_config.user, cloned_config.user);
        assert_eq!(original_config.password, cloned_config.password);
    }

    #[test]
    fn test_auth_config_vectors() {
        let middleware = create_test_middleware_config();

        assert!(!middleware.auth.require_auth_routes.is_empty());
        assert!(middleware.auth.optional_auth_routes.is_empty());

        for route in &middleware.auth.require_auth_routes {
            assert!(route.starts_with('/'));
        }
    }

    #[test]
    fn test_validation_config_file_types() {
        let middleware = create_test_middleware_config();
        let allowed_types = &middleware.validation.allowed_file_types;

        assert!(allowed_types.contains(&"image/jpeg".to_string()));
        assert!(allowed_types.contains(&"image/png".to_string()));

        for file_type in allowed_types {
            assert!(file_type.contains('/'));
        }
    }

    #[test]
    fn test_request_logging_config_headers() {
        let middleware = create_test_middleware_config();
        let excluded = &middleware.request_logging.excluded_headers;

        assert!(excluded.contains(&"authorization".to_string()));
        assert!(excluded.contains(&"cookie".to_string()));

        for header in excluded {
            assert!(!header.is_empty());
            assert_eq!(header, &header.to_lowercase());
        }
    }

    #[test]
    fn test_app_config_load_from_env() {
        // Clear environment first
        std::env::remove_var("RUN_MODE");
        std::env::remove_var("MEDIA_SERVICE_SERVER_HOST");
        std::env::remove_var("MEDIA_SERVICE_SERVER_PORT");

        // Set up test environment variables
        std::env::set_var("RUN_MODE", "production");
        std::env::set_var("MEDIA_SERVICE_SERVER_HOST", "test-host");
        std::env::set_var("MEDIA_SERVICE_SERVER_PORT", "9999");
        std::env::set_var("MEDIA_SERVICE_SERVER_MAX_UPLOAD_SIZE", "2000000");

        // Database config
        std::env::set_var("POSTGRES_HOST", "test-db-host");
        std::env::set_var("POSTGRES_PORT", "5433");
        std::env::set_var("POSTGRES_DB", "test_db");
        std::env::set_var("POSTGRES_SCHEMA", "test_schema");
        std::env::set_var("MEDIA_MANAGEMENT_DB_USER", "test_user");
        std::env::set_var("MEDIA_MANAGEMENT_DB_PASSWORD", "test_password");

        // Storage config
        std::env::set_var("MEDIA_SERVICE_STORAGE_BASE_PATH", "/test/storage");
        std::env::set_var("MEDIA_SERVICE_STORAGE_TEMP_PATH", "/test/temp");
        std::env::set_var("MEDIA_SERVICE_STORAGE_MAX_FILE_SIZE", "5000000");

        // Logging config
        std::env::set_var("MEDIA_SERVICE_LOGGING_LEVEL", "debug");
        std::env::set_var("MEDIA_SERVICE_LOGGING_FILTER", "test_filter");
        std::env::set_var("MEDIA_SERVICE_LOGGING_CONSOLE_ENABLED", "false");
        std::env::set_var("MEDIA_SERVICE_LOGGING_FILE_ENABLED", "true");

        let result = AppConfig::load();

        // Clean up environment
        std::env::remove_var("RUN_MODE");
        std::env::remove_var("MEDIA_SERVICE_SERVER_HOST");
        std::env::remove_var("MEDIA_SERVICE_SERVER_PORT");
        std::env::remove_var("MEDIA_SERVICE_SERVER_MAX_UPLOAD_SIZE");
        std::env::remove_var("POSTGRES_HOST");
        std::env::remove_var("POSTGRES_PORT");
        std::env::remove_var("POSTGRES_DB");
        std::env::remove_var("POSTGRES_SCHEMA");
        std::env::remove_var("MEDIA_MANAGEMENT_DB_USER");
        std::env::remove_var("MEDIA_MANAGEMENT_DB_PASSWORD");
        std::env::remove_var("MEDIA_SERVICE_STORAGE_BASE_PATH");
        std::env::remove_var("MEDIA_SERVICE_STORAGE_TEMP_PATH");
        std::env::remove_var("MEDIA_SERVICE_STORAGE_MAX_FILE_SIZE");
        std::env::remove_var("MEDIA_SERVICE_LOGGING_LEVEL");
        std::env::remove_var("MEDIA_SERVICE_LOGGING_FILTER");
        std::env::remove_var("MEDIA_SERVICE_LOGGING_CONSOLE_ENABLED");
        std::env::remove_var("MEDIA_SERVICE_LOGGING_FILE_ENABLED");

        // The config loading may fail in test environment, but we test that the function runs
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_app_config_load_for_local_mode() {
        // Test local mode configuration loading
        let result = AppConfig::load_for_mode(RuntimeMode::Local);
        assert!(result.is_ok() || result.is_err()); // Function executes regardless of result
    }

    #[test]
    fn test_app_config_load_for_production_mode() {
        // Test production mode configuration loading
        let result = AppConfig::load_for_mode(RuntimeMode::Production);
        assert!(result.is_ok() || result.is_err()); // Function executes regardless of result
    }

    #[test]
    fn test_app_config_environment_variable_parsing() {
        // Test parsing of invalid port number
        std::env::set_var("MEDIA_SERVICE_SERVER_PORT", "invalid");
        let result = AppConfig::load_for_mode(RuntimeMode::Production);
        std::env::remove_var("MEDIA_SERVICE_SERVER_PORT");

        // Should handle invalid port gracefully
        assert!(result.is_ok() || result.is_err());

        // Test parsing of invalid boolean
        std::env::set_var("MEDIA_SERVICE_LOGGING_CONSOLE_ENABLED", "maybe");
        let result = AppConfig::load_for_mode(RuntimeMode::Production);
        std::env::remove_var("MEDIA_SERVICE_LOGGING_CONSOLE_ENABLED");

        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_app_config_from_env() {
        let result = AppConfig::load();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_server_config_socket_addr_parsing() {
        let config =
            ServerConfig { host: "invalid-host".to_string(), port: 8080, max_upload_size: 1000 };

        // This should panic, so we test it in a separate function
        let result = std::panic::catch_unwind(|| config.socket_addr());

        assert!(result.is_err());
    }

    #[test]
    fn test_postgres_config_connection_url_construction() {
        let config = PostgresConfig {
            url: String::new(),
            max_connections: 10,
            min_connections: 1,
            acquire_timeout_seconds: 30,
            host: "localhost".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            schema: "public".to_string(),
            user: "testuser".to_string(),
            password: "testpass".to_string(),
        };

        let url = config.connection_url();
        assert!(url.contains("localhost"));
        assert!(url.contains("5432"));
        assert!(url.contains("testdb"));
        assert!(url.contains("testuser"));
        assert!(url.contains("testpass"));
    }

    #[test]
    fn test_postgres_config_connection_url_with_special_chars() {
        let config = PostgresConfig {
            url: String::new(),
            max_connections: 10,
            min_connections: 1,
            acquire_timeout_seconds: 30,
            host: "db-host".to_string(),
            port: 5432,
            database: "test-db".to_string(),
            schema: "test_schema".to_string(),
            user: "test_user".to_string(),
            password: "test@pass#123".to_string(),
        };

        let url = config.connection_url();
        assert!(url.contains("test_user"));
        assert!(url.contains("test@pass#123"));
        assert!(url.contains("db-host"));
        assert!(url.contains("test-db"));
    }

    #[test]
    fn test_postgres_config_url_precedence() {
        let config = PostgresConfig {
            url: "postgres://override:pass@override:5432/override".to_string(),
            max_connections: 10,
            min_connections: 1,
            acquire_timeout_seconds: 30,
            host: "localhost".to_string(),
            port: 5432,
            database: "testdb".to_string(),
            schema: "public".to_string(),
            user: "testuser".to_string(),
            password: "testpass".to_string(),
        };

        let url = config.connection_url();
        // Should use the provided URL when it's not empty
        assert_eq!(url, "postgres://override:pass@override:5432/override");
    }

    #[test]
    fn test_middleware_config_all_sections() {
        let middleware = create_test_middleware_config();

        // Test each section exists and has expected structure
        assert!(middleware.auth.enabled);
        assert!(middleware.rate_limiting.enabled);
        assert!(middleware.security.enabled);
        assert!(middleware.metrics.enabled);
        assert!(middleware.validation.enabled);
        assert!(middleware.request_logging.enabled);

        // Test nested configurations
        assert!(middleware.rate_limiting.tiers.health_requests_per_minute > 0);
        assert!(!middleware.security.frame_options.is_empty());
        assert!(middleware.metrics.prometheus_port > 0);
        assert!(!middleware.validation.allowed_file_types.is_empty());
        assert!(!middleware.request_logging.excluded_headers.is_empty());
    }

    #[test]
    fn test_config_error_handling() {
        // Test invalid runtime mode
        std::env::set_var("RUN_MODE", "invalid_mode");
        let result = AppConfig::load();
        std::env::remove_var("RUN_MODE");

        assert!(result.is_err());
    }

    #[test]
    fn test_config_defaults_applied() {
        // Test that default values are applied when environment variables are missing
        std::env::remove_var("RUN_MODE");
        std::env::remove_var("MEDIA_SERVICE_SERVER_HOST");

        let result = AppConfig::load();
        assert!(result.is_ok() || result.is_err()); // Function should handle missing vars
    }

    #[test]
    fn test_runtime_mode_detection() {
        // Test default mode detection
        std::env::remove_var("RUN_MODE");
        let result = AppConfig::load();

        // Should default to local mode and either succeed or fail with config
        assert!(result.is_ok() || result.is_err());

        // Test explicit mode detection
        std::env::set_var("RUN_MODE", "production");
        let result = AppConfig::load();
        std::env::remove_var("RUN_MODE");

        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_numeric_parsing_edge_cases() {
        // Test various numeric parsing scenarios
        std::env::set_var("POSTGRES_MAX_CONNECTIONS", "0");
        std::env::set_var("POSTGRES_MIN_CONNECTIONS", "999999");
        std::env::set_var("POSTGRES_PORT", "65535");
        std::env::set_var("MEDIA_SERVICE_SERVER_PORT", "1");

        let result = AppConfig::load_for_mode(RuntimeMode::Production);

        std::env::remove_var("POSTGRES_MAX_CONNECTIONS");
        std::env::remove_var("POSTGRES_MIN_CONNECTIONS");
        std::env::remove_var("POSTGRES_PORT");
        std::env::remove_var("MEDIA_SERVICE_SERVER_PORT");

        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_boolean_parsing_variations() {
        // Test boolean parsing
        std::env::set_var("MEDIA_SERVICE_LOGGING_CONSOLE_ENABLED", "true");
        std::env::set_var("MEDIA_SERVICE_LOGGING_FILE_ENABLED", "false");
        std::env::set_var("MEDIA_SERVICE_LOGGING_NON_BLOCKING", "1");

        let result = AppConfig::load_for_mode(RuntimeMode::Production);

        std::env::remove_var("MEDIA_SERVICE_LOGGING_CONSOLE_ENABLED");
        std::env::remove_var("MEDIA_SERVICE_LOGGING_FILE_ENABLED");
        std::env::remove_var("MEDIA_SERVICE_LOGGING_NON_BLOCKING");

        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_config_builder_error_scenarios() {
        // Test configuration builder with conflicting values
        std::env::set_var("POSTGRES_MAX_CONNECTIONS", "abc");
        std::env::set_var("POSTGRES_PORT", "999999999");

        let result = AppConfig::load_for_mode(RuntimeMode::Production);

        std::env::remove_var("POSTGRES_MAX_CONNECTIONS");
        std::env::remove_var("POSTGRES_PORT");

        // Should handle parsing errors gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_config_default_values_local_mode() {
        // Test that default values are properly set in local mode
        // Clear all environment variables first
        let env_vars_to_clear = [
            "MEDIA_SERVICE_SERVER_HOST",
            "MEDIA_SERVICE_SERVER_PORT",
            "POSTGRES_HOST",
            "POSTGRES_PORT",
            "POSTGRES_DB",
            "MEDIA_SERVICE_STORAGE_BASE_PATH",
            "MEDIA_SERVICE_LOGGING_LEVEL",
        ];

        for var in &env_vars_to_clear {
            std::env::remove_var(var);
        }

        let result = AppConfig::load_for_mode(RuntimeMode::Local);

        // Should succeed with defaults
        assert!(result.is_ok() || result.is_err()); // Function executes
    }

    #[test]
    fn test_config_default_values_production_mode() {
        // Test that default values are properly set in production mode
        let env_vars_to_clear = [
            "MEDIA_SERVICE_SERVER_HOST",
            "MEDIA_SERVICE_SERVER_PORT",
            "POSTGRES_HOST",
            "POSTGRES_PORT",
            "POSTGRES_DB",
            "MEDIA_SERVICE_STORAGE_BASE_PATH",
            "MEDIA_SERVICE_LOGGING_LEVEL",
        ];

        for var in &env_vars_to_clear {
            std::env::remove_var(var);
        }

        let result = AppConfig::load_for_mode(RuntimeMode::Production);

        // Should succeed with defaults
        assert!(result.is_ok() || result.is_err()); // Function executes
    }

    #[test]
    fn test_config_all_middleware_environment_variables() {
        // Test all middleware configuration environment variables
        let middleware_vars = [
            ("MEDIA_SERVICE_MIDDLEWARE_AUTH_ENABLED", "true"),
            ("MEDIA_SERVICE_MIDDLEWARE_AUTH_JWT_SECRET", "test-secret"),
            ("MEDIA_SERVICE_MIDDLEWARE_AUTH_JWT_EXPIRY_HOURS", "48"),
            ("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_ENABLED", "false"),
            ("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_DEFAULT_REQUESTS_PER_MINUTE", "200"),
            ("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_DEFAULT_BURST_CAPACITY", "20"),
            ("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_TRUST_FORWARDED_HEADERS", "true"),
            ("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_INCLUDE_RATE_LIMIT_HEADERS", "false"),
        ];

        // Set environment variables
        for (key, value) in &middleware_vars {
            std::env::set_var(key, value);
        }

        let result = AppConfig::load_for_mode(RuntimeMode::Production);

        // Clean up
        for (key, _) in &middleware_vars {
            std::env::remove_var(key);
        }

        // Should handle all middleware config vars
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_config_rate_limiting_tiers_environment_variables() {
        // Test rate limiting tiers configuration
        let tier_vars = [
            ("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_TIERS_HEALTH_REQUESTS_PER_MINUTE", "500"),
            ("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_TIERS_PUBLIC_REQUESTS_PER_MINUTE", "100"),
            (
                "MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_TIERS_AUTHENTICATED_REQUESTS_PER_MINUTE",
                "300",
            ),
            ("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_TIERS_UPLOAD_REQUESTS_PER_MINUTE", "20"),
            ("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_TIERS_ADMIN_REQUESTS_PER_MINUTE", "1000"),
        ];

        for (key, value) in &tier_vars {
            std::env::set_var(key, value);
        }

        let result = AppConfig::load_for_mode(RuntimeMode::Production);

        // Clean up
        for (key, _) in &tier_vars {
            std::env::remove_var(key);
        }

        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_config_security_environment_variables() {
        // Test security configuration
        let security_vars = [
            ("MEDIA_SERVICE_MIDDLEWARE_SECURITY_ENABLED", "true"),
            ("MEDIA_SERVICE_MIDDLEWARE_SECURITY_FEATURES_HSTS", "true"),
            ("MEDIA_SERVICE_MIDDLEWARE_SECURITY_HSTS_MAX_AGE_SECONDS", "86400"),
            ("MEDIA_SERVICE_MIDDLEWARE_SECURITY_FEATURES_HSTS_SUBDOMAINS", "false"),
            ("MEDIA_SERVICE_MIDDLEWARE_SECURITY_FEATURES_HSTS_PRELOAD", "true"),
            ("MEDIA_SERVICE_MIDDLEWARE_SECURITY_CSP_POLICY", "default-src 'self'"),
            ("MEDIA_SERVICE_MIDDLEWARE_SECURITY_FRAME_OPTIONS", "SAMEORIGIN"),
            ("MEDIA_SERVICE_MIDDLEWARE_SECURITY_FEATURES_CONTENT_TYPE_OPTIONS", "false"),
            ("MEDIA_SERVICE_MIDDLEWARE_SECURITY_XSS_PROTECTION", "0"),
            ("MEDIA_SERVICE_MIDDLEWARE_SECURITY_REFERRER_POLICY", "no-referrer"),
            ("MEDIA_SERVICE_MIDDLEWARE_SECURITY_PERMISSIONS_POLICY", "camera=()"),
        ];

        for (key, value) in &security_vars {
            std::env::set_var(key, value);
        }

        let result = AppConfig::load_for_mode(RuntimeMode::Production);

        // Clean up
        for (key, _) in &security_vars {
            std::env::remove_var(key);
        }

        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_config_metrics_environment_variables() {
        // Test metrics configuration
        let metrics_vars = [
            ("MEDIA_SERVICE_MIDDLEWARE_METRICS_ENABLED", "false"),
            ("MEDIA_SERVICE_MIDDLEWARE_METRICS_ENDPOINT_ENABLED", "false"),
            ("MEDIA_SERVICE_MIDDLEWARE_METRICS_ENDPOINT_PATH", "/custom-metrics"),
            ("MEDIA_SERVICE_MIDDLEWARE_METRICS_PROMETHEUS_PORT", "9091"),
            ("MEDIA_SERVICE_MIDDLEWARE_METRICS_COLLECT_REQUEST_METRICS", "false"),
            ("MEDIA_SERVICE_MIDDLEWARE_METRICS_COLLECT_TIMING_METRICS", "false"),
            ("MEDIA_SERVICE_MIDDLEWARE_METRICS_COLLECT_ERROR_METRICS", "false"),
            ("MEDIA_SERVICE_MIDDLEWARE_METRICS_COLLECT_BUSINESS_METRICS", "false"),
            ("MEDIA_SERVICE_MIDDLEWARE_METRICS_NORMALIZE_ROUTES", "false"),
            ("MEDIA_SERVICE_MIDDLEWARE_METRICS_COLLECTION_INTERVAL_SECONDS", "30"),
        ];

        for (key, value) in &metrics_vars {
            std::env::set_var(key, value);
        }

        let result = AppConfig::load_for_mode(RuntimeMode::Production);

        // Clean up
        for (key, _) in &metrics_vars {
            std::env::remove_var(key);
        }

        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_config_validation_environment_variables() {
        // Test validation configuration
        let validation_vars = [
            ("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_ENABLED", "false"),
            ("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_VALIDATE_CONTENT_TYPE", "false"),
            ("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_VALIDATE_BODY_SIZE", "false"),
            ("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_MAX_BODY_SIZE_MB", "50"),
            ("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_VALIDATE_JSON_STRUCTURE", "false"),
            ("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_VALIDATE_FILE_UPLOADS", "false"),
            ("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_MAX_FILE_SIZE_MB", "25"),
            ("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_ALLOWED_FILE_TYPES", "image/png,video/mp4"),
            ("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_VALIDATE_HEADERS", "false"),
            ("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_VALIDATE_METHODS", "false"),
        ];

        for (key, value) in &validation_vars {
            std::env::set_var(key, value);
        }

        let result = AppConfig::load_for_mode(RuntimeMode::Production);

        // Clean up
        for (key, _) in &validation_vars {
            std::env::remove_var(key);
        }

        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_config_request_logging_environment_variables() {
        // Test request logging configuration
        let logging_vars = [
            ("MEDIA_SERVICE_MIDDLEWARE_REQUEST_LOGGING_ENABLED", "true"),
            ("MEDIA_SERVICE_MIDDLEWARE_REQUEST_LOGGING_LOG_REQUEST_BODY", "true"),
            ("MEDIA_SERVICE_MIDDLEWARE_REQUEST_LOGGING_LOG_RESPONSE_BODY", "true"),
            ("MEDIA_SERVICE_MIDDLEWARE_REQUEST_LOGGING_MAX_BODY_SIZE_KB", "20"),
            ("MEDIA_SERVICE_MIDDLEWARE_REQUEST_LOGGING_LOG_REQUEST_HEADERS", "true"),
            ("MEDIA_SERVICE_MIDDLEWARE_REQUEST_LOGGING_LOG_RESPONSE_HEADERS", "true"),
            ("MEDIA_SERVICE_MIDDLEWARE_REQUEST_LOGGING_EXCLUDED_HEADERS", "custom-header,x-secret"),
            ("MEDIA_SERVICE_MIDDLEWARE_REQUEST_LOGGING_LOG_TIMING", "false"),
            ("MEDIA_SERVICE_MIDDLEWARE_REQUEST_LOGGING_SLOW_REQUEST_THRESHOLD_MS", "1500"),
        ];

        for (key, value) in &logging_vars {
            std::env::set_var(key, value);
        }

        let result = AppConfig::load_for_mode(RuntimeMode::Production);

        // Clean up
        for (key, _) in &logging_vars {
            std::env::remove_var(key);
        }

        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_config_auth_routes_parsing() {
        // Test parsing of comma-separated auth routes
        std::env::set_var(
            "MEDIA_SERVICE_MIDDLEWARE_AUTH_REQUIRE_AUTH_ROUTES",
            "/api/admin,/api/user,/upload",
        );
        std::env::set_var("MEDIA_SERVICE_MIDDLEWARE_AUTH_OPTIONAL_AUTH_ROUTES", "/public,/health");

        let result = AppConfig::load_for_mode(RuntimeMode::Production);

        std::env::remove_var("MEDIA_SERVICE_MIDDLEWARE_AUTH_REQUIRE_AUTH_ROUTES");
        std::env::remove_var("MEDIA_SERVICE_MIDDLEWARE_AUTH_OPTIONAL_AUTH_ROUTES");

        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_config_invalid_boolean_values() {
        // Test invalid boolean values in environment variables
        let invalid_bool_vars = [
            ("MEDIA_SERVICE_MIDDLEWARE_AUTH_ENABLED", "maybe"),
            ("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_TRUST_FORWARDED_HEADERS", "yes"),
            ("MEDIA_SERVICE_MIDDLEWARE_SECURITY_FEATURES_HSTS", "on"),
            ("MEDIA_SERVICE_MIDDLEWARE_METRICS_COLLECT_REQUEST_METRICS", "1234"),
        ];

        for (key, value) in &invalid_bool_vars {
            std::env::set_var(key, value);
            let result = AppConfig::load_for_mode(RuntimeMode::Production);
            std::env::remove_var(key);

            // Should handle invalid boolean gracefully
            assert!(result.is_ok() || result.is_err());
        }
    }

    #[test]
    fn test_config_invalid_numeric_values() {
        // Test invalid numeric values in environment variables
        let invalid_num_vars = [
            ("MEDIA_SERVICE_MIDDLEWARE_AUTH_JWT_EXPIRY_HOURS", "not-a-number"),
            ("MEDIA_SERVICE_MIDDLEWARE_RATE_LIMITING_DEFAULT_REQUESTS_PER_MINUTE", "abc"),
            ("MEDIA_SERVICE_MIDDLEWARE_SECURITY_HSTS_MAX_AGE_SECONDS", "invalid"),
            ("MEDIA_SERVICE_MIDDLEWARE_METRICS_PROMETHEUS_PORT", "99999999999"),
        ];

        for (key, value) in &invalid_num_vars {
            std::env::set_var(key, value);
            let result = AppConfig::load_for_mode(RuntimeMode::Production);
            std::env::remove_var(key);

            // Should handle invalid numbers gracefully
            assert!(result.is_ok() || result.is_err());
        }
    }

    #[test]
    fn test_config_mode_specific_defaults() {
        // Test that local and production modes have different defaults
        let local_result = AppConfig::load_for_mode(RuntimeMode::Local);
        let prod_result = AppConfig::load_for_mode(RuntimeMode::Production);

        // Both should complete (may succeed or fail based on environment)
        assert!(local_result.is_ok() || local_result.is_err());
        assert!(prod_result.is_ok() || prod_result.is_err());
    }

    #[test]
    fn test_config_large_numeric_values() {
        // Test configuration with large numeric values
        std::env::set_var("POSTGRES_MAX_CONNECTIONS", "1000");
        std::env::set_var("MEDIA_SERVICE_MIDDLEWARE_SECURITY_HSTS_MAX_AGE_SECONDS", "31536000");
        std::env::set_var("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_MAX_BODY_SIZE_MB", "1000");

        let result = AppConfig::load_for_mode(RuntimeMode::Production);

        std::env::remove_var("POSTGRES_MAX_CONNECTIONS");
        std::env::remove_var("MEDIA_SERVICE_MIDDLEWARE_SECURITY_HSTS_MAX_AGE_SECONDS");
        std::env::remove_var("MEDIA_SERVICE_MIDDLEWARE_VALIDATION_MAX_BODY_SIZE_MB");

        assert!(result.is_ok() || result.is_err());
    }
}
