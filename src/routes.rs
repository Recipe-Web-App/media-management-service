use std::sync::Arc;

use axum::Router;
use axum::middleware as axum_mw;
use axum::routing::{get, post, put};
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::GlobalKeyExtractor;

use crate::auth;
use crate::handlers;
use crate::health;
use crate::middleware;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    // Media CRUD routes: protected by auth middleware.
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
        )
        .layer(axum_mw::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ));

    // Download endpoint handles its own dual auth (bearer or signed URL).
    let download_route = Router::new().route("/media/{id}/download", get(handlers::download_media));

    // Upload endpoint uses signed URL auth (token in path + HMAC signature).
    let upload_route = Router::new().route("/media/upload/{token}", put(handlers::upload_file));

    let health_routes = Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(health::ready));

    let cors = middleware::cors_layer(&state.config);

    let governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(middleware::RATE_LIMIT_PER_SECOND)
            .burst_size(middleware::RATE_LIMIT_BURST)
            .key_extractor(GlobalKeyExtractor)
            .finish()
            .expect("rate limiter config is valid"),
    );

    Router::new()
        .nest(
            "/api/v1/media-management",
            media_routes
                .merge(download_route)
                .merge(upload_route)
                .merge(health_routes),
        )
        .with_state(state)
        // Layers applied in reverse: last .layer() call = outermost middleware.
        // Outermost → Innermost: RequestId, Trace, PropagateId, CORS,
        //   RateLimit, Timeout, Compression, SecurityHeaders
        .layer(middleware::nosniff_layer())
        .layer(middleware::frame_deny_layer())
        .layer(middleware::referrer_policy_layer())
        .layer(middleware::compression_layer())
        .layer(middleware::timeout_layer())
        .layer(GovernorLayer::new(governor_config))
        .layer(cors)
        .layer(middleware::propagate_request_id_layer())
        .layer(middleware::trace_context_layer())
        .layer(middleware::trace_layer())
        .layer(middleware::request_id_layer())
}
