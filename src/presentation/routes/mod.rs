use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    infrastructure::http::{health_check, readiness_check_no_db},
    presentation::handlers::{self, media::AppState},
};

/// Create all application routes with application state
pub fn create_routes_with_state(app_state: AppState) -> Router {
    Router::new().nest("/api/v1/media-management", media_management_routes()).with_state(app_state)
}

/// Create all application routes without state (backward compatibility)
pub fn create_routes() -> Router {
    Router::new().nest("/api/v1/media-management", media_management_routes_no_state())
}

/// Create media management service routes with state
fn media_management_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check_no_db))
        .nest("/media", media_routes())
}

/// Create media management service routes without state (backward compatibility)
fn media_management_routes_no_state() -> Router {
    Router::new().route("/health", get(health_check)).route("/ready", get(readiness_check_no_db))
    // Note: media routes with actual functionality require state
    // These will return not implemented errors
}

/// Create media-related routes with state
fn media_routes() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::media::upload_media))
        .route("/", get(handlers::media::list_media))
        .route("/{id}", get(handlers::media::get_media))
        .route("/{id}/download", get(handlers::media::download_media))
}
