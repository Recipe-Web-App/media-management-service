use axum::Router;
use axum::routing::get;

use crate::health;

pub fn router() -> Router {
    let health_routes = Router::new().route("/health", get(health::health));

    Router::new().nest("/api/v1/media-management", health_routes)
}
