use axum::{
    body::Body,
    http::StatusCode,
    routing::{get, post},
    Router,
};
use media_management_service::presentation::handlers::media;
use serde_json::json;

mod common;
use common::test_app::TestApp;

fn create_test_router() -> Router {
    Router::new()
        .route("/media", post(media::upload_media))
        .route("/media", get(media::list_media))
        .route("/media/:id", get(media::get_media))
        .route("/media/:id/download", get(media::download_media))
        .route("/media/recipe/:recipe_id", get(media::get_media_by_recipe))
        .route("/media/recipe/:recipe_id/ingredient/:ingredient_id", get(media::get_media_by_ingredient))
        .route("/media/recipe/:recipe_id/step/:step_id", get(media::get_media_by_step))
}

#[tokio::test]
async fn test_upload_media_not_implemented() {
    let app = TestApp::new(create_test_router());

    let response = app.post("/media", Body::empty()).await;

    response.assert_status(StatusCode::NOT_IMPLEMENTED);

    let body: serde_json::Value = response.json();
    assert_eq!(body["error"], "Not Implemented");
    assert!(body["message"].as_str().unwrap().contains("not yet implemented"));
}

#[tokio::test]
async fn test_list_media_returns_empty_list() {
    let app = TestApp::new(create_test_router());

    let response = app.get("/media").await;

    response.assert_status(StatusCode::OK);

    let body: Vec<serde_json::Value> = response.json();
    assert!(body.is_empty());
}

#[tokio::test]
async fn test_get_media_not_found() {
    let app = TestApp::new(create_test_router());
    let media_id = uuid::Uuid::new_v4();

    let response = app.get(&format!("/media/{media_id}")).await;

    response.assert_status(StatusCode::NOT_FOUND);

    let body: serde_json::Value = response.json();
    assert_eq!(body["error"], "Not Found");
    assert_eq!(body["message"], "Media not found");
}

#[tokio::test]
async fn test_download_media_not_implemented() {
    let app = TestApp::new(create_test_router());
    let media_id = uuid::Uuid::new_v4();

    let response = app.get(&format!("/media/{media_id}/download")).await;

    response.assert_status(StatusCode::NOT_IMPLEMENTED);

    let body: serde_json::Value = response.json();
    assert_eq!(body["error"], "Not Implemented");
    assert!(body["message"].as_str().unwrap().contains("not yet implemented"));
}
