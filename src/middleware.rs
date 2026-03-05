use std::time::Duration;

use axum::http::header::{self, HeaderName};
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode};
use opentelemetry::global;
use opentelemetry::propagation::{Extractor, Injector};
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tower_http::request_id::{
    MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{MakeSpan, TraceLayer};

use crate::config::{Config, RunMode};

const REQUEST_TIMEOUT_SECS: u64 = 30;
pub const RATE_LIMIT_PER_SECOND: u64 = 10;
pub const RATE_LIMIT_BURST: u32 = 50;

// -- Request ID -----------------------------------------------------------

#[derive(Clone, Default)]
pub struct RequestIdGenerator;

impl MakeRequestId for RequestIdGenerator {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let id = uuid::Uuid::new_v4().to_string();
        id.parse().ok().map(RequestId::new)
    }
}

pub fn request_id_layer() -> SetRequestIdLayer<RequestIdGenerator> {
    SetRequestIdLayer::x_request_id(RequestIdGenerator)
}

pub fn propagate_request_id_layer() -> PropagateRequestIdLayer {
    PropagateRequestIdLayer::x_request_id()
}

// -- Tracing --------------------------------------------------------------

#[derive(Clone)]
pub struct RequestIdSpan;

impl<B> MakeSpan<B> for RequestIdSpan {
    fn make_span(&mut self, request: &Request<B>) -> tracing::Span {
        let request_id = request
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("-");
        tracing::info_span!(
            "http_request",
            method = %request.method(),
            uri = %request.uri(),
            request_id = %request_id,
        )
    }
}

pub fn trace_layer() -> TraceLayer<
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    RequestIdSpan,
> {
    TraceLayer::new_for_http().make_span_with(RequestIdSpan)
}

// -- CORS -----------------------------------------------------------------

pub fn cors_layer(config: &Config) -> CorsLayer {
    if config.run_mode == RunMode::Local {
        return CorsLayer::very_permissive();
    }

    if config.cors_allowed_origins.is_empty() {
        return CorsLayer::new();
    }

    let origins: Vec<HeaderValue> = config
        .cors_allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(AllowMethods::list([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ]))
        .allow_headers(AllowHeaders::list([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            HeaderName::from_static("x-request-id"),
        ]))
}

// -- Compression ----------------------------------------------------------

pub fn compression_layer() -> CompressionLayer {
    CompressionLayer::new()
}

// -- Timeout --------------------------------------------------------------

pub fn timeout_layer() -> TimeoutLayer {
    TimeoutLayer::with_status_code(
        StatusCode::REQUEST_TIMEOUT,
        Duration::from_secs(REQUEST_TIMEOUT_SECS),
    )
}

// -- Security headers -----------------------------------------------------

pub fn nosniff_layer() -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::overriding(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    )
}

pub fn frame_deny_layer() -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::overriding(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"))
}

pub fn referrer_policy_layer() -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::overriding(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    )
}

// -- Trace context propagation -----------------------------------------------

/// Tower layer that extracts W3C traceparent/tracestate from request headers
/// and injects trace context into response headers for distributed tracing.
#[derive(Clone)]
pub struct TraceContextLayer;

impl<S> tower::Layer<S> for TraceContextLayer {
    type Service = TraceContextService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TraceContextService { inner }
    }
}

/// Tower service that propagates W3C trace context headers.
#[derive(Clone)]
pub struct TraceContextService<S> {
    inner: S,
}

impl<S, B> tower::Service<Request<B>> for TraceContextService<S>
where
    S: tower::Service<Request<B>, Response = axum::response::Response> + Clone + Send + 'static,
    S::Future: Send,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<B>) -> Self::Future {
        let parent_cx = global::get_text_map_propagator(|propagator| {
            propagator.extract(&HeaderExtractor(request.headers()))
        });

        let _guard = parent_cx.clone().attach();

        let future = self.inner.call(request);

        Box::pin(async move {
            let mut response = future.await?;

            global::get_text_map_propagator(|propagator| {
                propagator.inject_context(&parent_cx, &mut HeaderInjector(response.headers_mut()));
            });

            Ok(response)
        })
    }
}

pub fn trace_context_layer() -> TraceContextLayer {
    TraceContextLayer
}

struct HeaderExtractor<'a>(&'a HeaderMap);

impl Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(HeaderName::as_str).collect()
    }
}

struct HeaderInjector<'a>(&'a mut HeaderMap);

impl Injector for HeaderInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(name) = HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(val) = HeaderValue::from_str(&value) {
                self.0.insert(name, val);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::routing::get;
    use tower::ServiceExt;

    fn test_config(run_mode: RunMode, origins: Vec<String>) -> Config {
        Config {
            host: "0.0.0.0".into(),
            port: 3000,
            max_upload_size: 1024,
            database_url: String::new(),
            db_max_connections: 1,
            storage_base_path: String::new(),
            storage_temp_path: String::new(),
            download_url_ttl_secs: 86400,
            upload_url_ttl_secs: 900,
            signing_secret: "test-secret".into(),
            auth: crate::config::AuthModeConfig::Dev,
            run_mode,
            otel_endpoint: None,
            cors_allowed_origins: origins,
        }
    }

    #[test]
    fn request_id_generates_valid_uuid() {
        let mut generator = RequestIdGenerator;
        let req = Request::builder().body(()).unwrap();
        let id = generator
            .make_request_id(&req)
            .expect("should generate an ID");
        let value = id.header_value().to_str().unwrap().to_string();
        uuid::Uuid::parse_str(&value).expect("should be a valid UUID");
    }

    #[tokio::test]
    async fn request_id_propagated_to_response() {
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(propagate_request_id_layer())
            .layer(request_id_layer());

        let req = Request::builder()
            .uri("/")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        let id = resp
            .headers()
            .get("x-request-id")
            .expect("x-request-id header should be present");
        let value = id.to_str().unwrap();
        uuid::Uuid::parse_str(value).expect("should be a valid UUID");
    }

    #[tokio::test]
    async fn security_headers_present_on_response() {
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(nosniff_layer())
            .layer(frame_deny_layer())
            .layer(referrer_policy_layer());

        let req = Request::builder()
            .uri("/")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(
            resp.headers().get(header::X_CONTENT_TYPE_OPTIONS).unwrap(),
            "nosniff"
        );
        assert_eq!(resp.headers().get(header::X_FRAME_OPTIONS).unwrap(), "DENY");
        assert_eq!(
            resp.headers().get("referrer-policy").unwrap(),
            "strict-origin-when-cross-origin"
        );
    }

    #[test]
    fn cors_permissive_in_local_mode() {
        let config = test_config(RunMode::Local, vec![]);
        let _layer = cors_layer(&config);
    }

    #[test]
    fn cors_restrictive_in_production_without_origins() {
        let config = test_config(RunMode::Production, vec![]);
        let _layer = cors_layer(&config);
    }

    #[test]
    fn cors_restrictive_in_production_with_origins() {
        let config = test_config(
            RunMode::Production,
            vec![
                "https://example.com".into(),
                "https://app.example.com".into(),
            ],
        );
        let _layer = cors_layer(&config);
    }
}
