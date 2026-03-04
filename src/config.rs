use std::env;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunMode {
    Local,
    Production,
}

#[derive(Debug, Clone)]
pub enum AuthModeConfig {
    OAuth2 {
        base_url: String,
        client_id: String,
        client_secret: String,
    },
    Jwt {
        secret: String,
    },
    Dev,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub max_upload_size: u64,
    pub database_url: String,
    pub db_max_connections: u32,
    pub storage_base_path: String,
    pub storage_temp_path: String,
    pub download_url_ttl_secs: u64,
    pub signing_secret: String,
    pub auth: AuthModeConfig,
    pub run_mode: RunMode,
    pub otel_endpoint: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let run_mode = match env::var("RUN_MODE").unwrap_or_default().as_str() {
            "production" => RunMode::Production,
            _ => RunMode::Local,
        };

        if run_mode == RunMode::Local {
            let _ = dotenvy::from_filename(".env.local");
        }

        let host = env::var("MEDIA_SERVICE_SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".into());
        let port = env::var("MEDIA_SERVICE_SERVER_PORT")
            .unwrap_or_else(|_| "3000".into())
            .parse()
            .expect("MEDIA_SERVICE_SERVER_PORT must be a valid u16");
        let max_upload_size = env::var("MEDIA_SERVICE_SERVER_MAX_UPLOAD_SIZE")
            .unwrap_or_else(|_| "104857600".into())
            .parse()
            .expect("MEDIA_SERVICE_SERVER_MAX_UPLOAD_SIZE must be a valid u64");

        let pg_host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".into());
        let pg_port = env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".into());
        let pg_db = env::var("POSTGRES_DB").unwrap_or_else(|_| "recipe_database".into());
        let pg_schema = env::var("POSTGRES_SCHEMA").unwrap_or_else(|_| "recipe_manager".into());
        let pg_user = env::var("MEDIA_MANAGEMENT_DB_USER").unwrap_or_else(|_| "postgres".into());
        let pg_password = env::var("MEDIA_MANAGEMENT_DB_PASSWORD").unwrap_or_default();
        let database_url = format!(
            "postgres://{pg_user}:{pg_password}@{pg_host}:{pg_port}/{pg_db}?options=-c search_path%3D{pg_schema}"
        );

        let db_max_connections = env::var("POSTGRES_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "10".into())
            .parse()
            .expect("POSTGRES_MAX_CONNECTIONS must be a valid u32");

        let storage_base_path =
            env::var("MEDIA_SERVICE_STORAGE_BASE_PATH").unwrap_or_else(|_| "./media".into());
        let storage_temp_path =
            env::var("MEDIA_SERVICE_STORAGE_TEMP_PATH").unwrap_or_else(|_| "./media/temp".into());
        let download_url_ttl_secs = env::var("MEDIA_SERVICE_DOWNLOAD_URL_TTL_SECS")
            .unwrap_or_else(|_| "86400".into())
            .parse()
            .expect("MEDIA_SERVICE_DOWNLOAD_URL_TTL_SECS must be a valid u64");

        let auth = Self::parse_auth_mode();

        let signing_secret = match &auth {
            AuthModeConfig::Jwt { secret } => {
                env::var("MEDIA_SERVICE_SIGNING_SECRET").unwrap_or_else(|_| secret.clone())
            }
            AuthModeConfig::Dev => env::var("MEDIA_SERVICE_SIGNING_SECRET")
                .unwrap_or_else(|_| "dev-signing-secret-not-for-production".into()),
            AuthModeConfig::OAuth2 { .. } => env::var("MEDIA_SERVICE_SIGNING_SECRET")
                .expect("MEDIA_SERVICE_SIGNING_SECRET required in OAuth2 mode"),
        };

        let otel_endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
            .ok()
            .filter(|s| !s.is_empty());

        Self {
            host,
            port,
            max_upload_size,
            database_url,
            db_max_connections,
            storage_base_path,
            storage_temp_path,
            download_url_ttl_secs,
            signing_secret,
            auth,
            run_mode,
            otel_endpoint,
        }
    }

    fn parse_auth_mode() -> AuthModeConfig {
        let oauth2_enabled = env::var("OAUTH2_SERVICE_ENABLED")
            .unwrap_or_else(|_| "false".into())
            .parse::<bool>()
            .unwrap_or(false);

        if !oauth2_enabled {
            return AuthModeConfig::Dev;
        }

        let introspection_enabled = env::var("OAUTH2_INTROSPECTION_ENABLED")
            .unwrap_or_else(|_| "false".into())
            .parse::<bool>()
            .unwrap_or(false);

        if introspection_enabled {
            AuthModeConfig::OAuth2 {
                base_url: env::var("OAUTH2_SERVICE_BASE_URL")
                    .expect("OAUTH2_SERVICE_BASE_URL required when introspection is enabled"),
                client_id: env::var("OAUTH2_CLIENT_ID")
                    .expect("OAUTH2_CLIENT_ID required when introspection is enabled"),
                client_secret: env::var("OAUTH2_CLIENT_SECRET")
                    .expect("OAUTH2_CLIENT_SECRET required when introspection is enabled"),
            }
        } else {
            AuthModeConfig::Jwt {
                secret: env::var("JWT_SECRET")
                    .expect("JWT_SECRET required when OAuth2 is enabled without introspection"),
            }
        }
    }
}
