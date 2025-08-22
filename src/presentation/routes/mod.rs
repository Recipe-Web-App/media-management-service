use axum::{
    routing::{get, post},
    Router,
};

use crate::presentation::handlers;

/// Create all application routes
pub fn create_routes() -> Router {
    Router::new().nest("/api/v1/media-management", media_management_routes())
}

/// Create media management service routes
fn media_management_routes() -> Router {
    Router::new().nest("/media", media_routes())
}

/// Create media-related routes
fn media_routes() -> Router {
    Router::new()
        .route("/", post(handlers::media::upload_media))
        .route("/", get(handlers::media::list_media))
        .route("/{id}", get(handlers::media::get_media))
        .route("/{id}/download", get(handlers::media::download_media))
}
