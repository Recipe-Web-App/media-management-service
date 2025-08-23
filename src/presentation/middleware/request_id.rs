use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use tower_http::request_id::{MakeRequestId, RequestId};
use tracing::Span;
use uuid::Uuid;

/// Enhanced request ID maker that creates UUID v4 request IDs
#[derive(Clone, Debug)]
pub struct EnhancedRequestId;

impl MakeRequestId for EnhancedRequestId {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let request_id = Uuid::new_v4().to_string();
        let header_value = HeaderValue::try_from(request_id).ok()?;
        Some(RequestId::new(header_value))
    }
}

/// Middleware to enhance request ID functionality
pub async fn enhance_request_id(request: Request, next: Next) -> Response {
    // Extract the request ID that was set by the SetRequestIdLayer
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string(); // Convert to owned String

    // Add request ID to the current tracing span
    Span::current().record("request_id", &request_id);

    // Process the request
    let mut response = next.run(request).await;

    // Add request ID to response headers for client visibility
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("invalid")),
    );

    // Add correlation ID (same as request ID for now, but could be different)
    response.headers_mut().insert(
        "x-correlation-id",
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("invalid")),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        response::Json,
        routing::get,
        Router,
    };
    use serde_json::json;
    use tower::ServiceExt;
    use tower_http::request_id::SetRequestIdLayer;

    async fn test_handler() -> Json<serde_json::Value> {
        Json(json!({"status": "ok"}))
    }

    #[tokio::test]
    async fn test_enhanced_request_id_make_request_id() {
        let mut maker = EnhancedRequestId;
        let request = Request::builder().body(Body::empty()).unwrap();

        let request_id = maker.make_request_id(&request);
        assert!(request_id.is_some());

        // Verify it's a valid UUID format
        let request_id_value = request_id.unwrap();
        let header_value = request_id_value.header_value();
        let id_str = header_value.to_str().unwrap();
        let parsed_uuid = Uuid::parse_str(id_str);
        assert!(parsed_uuid.is_ok());
    }

    #[tokio::test]
    async fn test_enhance_request_id_middleware() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(axum::middleware::from_fn(enhance_request_id))
            .layer(SetRequestIdLayer::new(
                axum::http::HeaderName::from_static("x-request-id"),
                EnhancedRequestId,
            ));

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Check that response has request ID headers
        assert!(response.headers().get("x-request-id").is_some());
        assert!(response.headers().get("x-correlation-id").is_some());

        // Verify they're the same value
        let request_id = response.headers().get("x-request-id").unwrap();
        let correlation_id = response.headers().get("x-correlation-id").unwrap();
        assert_eq!(request_id, correlation_id);

        // Verify it's a valid UUID
        let id_str = request_id.to_str().unwrap();
        let parsed_uuid = Uuid::parse_str(id_str);
        assert!(parsed_uuid.is_ok());
    }

    #[tokio::test]
    async fn test_request_id_with_existing_header() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(axum::middleware::from_fn(enhance_request_id))
            .layer(SetRequestIdLayer::new(
                axum::http::HeaderName::from_static("x-request-id"),
                EnhancedRequestId,
            ));

        let existing_id = Uuid::new_v4().to_string();
        let request = Request::builder()
            .uri("/test")
            .header("x-request-id", &existing_id)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // The middleware should preserve the existing request ID from SetRequestIdLayer
        let response_id = response.headers().get("x-request-id").unwrap().to_str().unwrap();

        // Should be a valid UUID (could be the existing one or a new one depending on SetRequestIdLayer behavior)
        let parsed_uuid = Uuid::parse_str(response_id);
        assert!(parsed_uuid.is_ok());
    }

    #[tokio::test]
    async fn test_request_id_multiple_requests() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(axum::middleware::from_fn(enhance_request_id))
            .layer(SetRequestIdLayer::new(
                axum::http::HeaderName::from_static("x-request-id"),
                EnhancedRequestId,
            ));

        // Make multiple requests
        let mut request_ids = Vec::new();

        for _ in 0..3 {
            let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

            let response = app.clone().oneshot(request).await.unwrap();
            let request_id = response.headers().get("x-request-id").unwrap().to_str().unwrap();
            request_ids.push(request_id.to_string());
        }

        // All request IDs should be different
        assert_eq!(request_ids.len(), 3);
        for i in 0..request_ids.len() {
            for j in (i + 1)..request_ids.len() {
                assert_ne!(request_ids[i], request_ids[j]);
            }
        }

        // All should be valid UUIDs
        for id in request_ids {
            let parsed_uuid = Uuid::parse_str(&id);
            assert!(parsed_uuid.is_ok());
        }
    }

    #[tokio::test]
    async fn test_correlation_id_matches_request_id() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(axum::middleware::from_fn(enhance_request_id))
            .layer(SetRequestIdLayer::new(
                axum::http::HeaderName::from_static("x-request-id"),
                EnhancedRequestId,
            ));

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();

        let request_id = response.headers().get("x-request-id").unwrap();
        let correlation_id = response.headers().get("x-correlation-id").unwrap();

        // They should be identical
        assert_eq!(request_id, correlation_id);
    }
}
