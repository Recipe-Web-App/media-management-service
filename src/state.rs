use sqlx::PgPool;

use crate::auth::AuthMode;
use crate::config::Config;
use crate::storage::Storage;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub storage: Storage,
    pub config: Config,
    pub auth_mode: AuthMode,
}
