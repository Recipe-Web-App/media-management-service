use axum::Router;
use axum::routing::get;

use crate::health;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    let health_routes = Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(health::ready))
        .with_state(state);

    Router::new().nest("/api/v1/media-management", health_routes)
}
