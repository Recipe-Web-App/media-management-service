use sqlx::PgPool;

use crate::storage::Storage;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub storage: Storage,
}
