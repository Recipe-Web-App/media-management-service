use crate::infrastructure::config::DatabaseConfig;
use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use tracing::{info, warn};

/// Database connection pool wrapper
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database connection pool
    ///
    /// # Errors
    /// Returns an error if the database connection fails
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        let connection_url = config.connection_url();

        info!("Connecting to PostgreSQL database at {}:{}", config.host, config.port);

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.acquire_timeout_seconds))
            .connect(&connection_url)
            .await?;

        // Test the connection
        let _ = sqlx::query("SELECT 1").fetch_one(&pool).await?;

        info!("Successfully connected to PostgreSQL database");

        Ok(Self { pool })
    }

    /// Get a reference to the connection pool
    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Check if the database is healthy
    ///
    /// # Errors
    /// Returns an error if the health check fails
    pub async fn health_check(&self) -> Result<()> {
        let _ = sqlx::query("SELECT 1").fetch_one(&self.pool).await?;
        Ok(())
    }

    /// Close the database connection pool
    pub async fn close(&self) {
        if !self.pool.is_closed() {
            info!("Closing database connection pool");
            self.pool.close().await;
        }
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        if !self.pool.is_closed() {
            warn!("Database pool dropped without being explicitly closed");
        }
    }
}
