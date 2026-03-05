use axum::Router;
use axum::routing::{get, post, put};

use crate::handlers;
use crate::health;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    // Media CRUD routes: Phase 5 will wrap these with auth middleware.
    let media_routes = Router::new()
        .route(
            "/media",
            post(handlers::upload_media).get(handlers::list_media),
        )
        .route(
            "/media/{id}",
            get(handlers::get_media).delete(handlers::delete_media),
        )
        .route("/media/{id}/status", get(handlers::get_upload_status))
        .route("/media/upload-request", post(handlers::initiate_upload))
        .route("/media/recipe/{id}", get(handlers::get_media_by_recipe))
        .route(
            "/media/recipe/{rid}/ingredient/{id}",
            get(handlers::get_media_by_ingredient),
        )
        .route(
            "/media/recipe/{rid}/step/{id}",
            get(handlers::get_media_by_step),
        );

    // Download endpoint handles its own dual auth (bearer or signed URL).
    let download_route = Router::new().route("/media/{id}/download", get(handlers::download_media));

    // Upload endpoint uses signed URL auth (token in path + HMAC signature).
    let upload_route = Router::new().route("/media/upload/{token}", put(handlers::upload_file));

    let health_routes = Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(health::ready));

    Router::new()
        .nest(
            "/api/v1/media-management",
            media_routes
                .merge(download_route)
                .merge(upload_route)
                .merge(health_routes),
        )
        .with_state(state)
}
