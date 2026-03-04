# Media Management Service - Technology Stack

Crate choices for the service rewrite with versions, features, and rationale.

## Dependencies

### Core

| Crate              | Version | Features                                                       | Rationale                                                             |
| ------------------ | ------- | -------------------------------------------------------------- | --------------------------------------------------------------------- |
| axum               | 0.8     | multipart                                                      | Rust's leading web framework. Tower middleware, type-safe extractors. |
| axum-extra         | 0.12    | typed-header                                                   | TypedHeader extractor for auth headers.                               |
| tokio              | 1       | full                                                           | Async runtime. No alternative.                                        |
| tokio-util         | 0.7     | io                                                             | ReaderStream for streaming downloads without buffering.               |
| tower-http         | 0.6     | cors, compression-gzip, trace, timeout, set-header, request-id | Production middleware layers.                                         |
| tower              | 0.5     | -                                                              | Service trait and middleware utilities.                               |
| sqlx               | 0.8     | postgres, runtime-tokio-rustls, chrono, uuid                   | Compile-time checked SQL. Async PostgreSQL with connection pooling.   |
| serde              | 1       | derive                                                         | Serialization.                                                        |
| serde_json         | 1       | -                                                              | JSON handling.                                                        |
| chrono             | 0.4     | serde                                                          | DateTime for database compatibility.                                  |
| uuid               | 1       | v4, serde                                                      | UUID generation for user IDs and request tracing.                     |
| thiserror          | 2       | -                                                              | Error derive macros.                                                  |
| tracing            | 0.1     | -                                                              | Structured logging facade.                                            |
| tracing-subscriber | 0.3     | json, env-filter                                               | Log formatting and filtering.                                         |
| sha2               | 0.10    | -                                                              | SHA-256 content hashing.                                              |
| hex                | 0.4     | -                                                              | Hex encoding for content hashes.                                      |
| base64             | 0.22    | -                                                              | Cursor encoding for pagination.                                       |
| hmac               | 0.12    | -                                                              | HMAC-SHA256 for presigned URL signing.                                |
| rand               | 0.9     | -                                                              | Random token generation.                                              |
| jsonwebtoken       | 10      | -                                                              | JWT decoding (HS256) for auth.                                        |
| reqwest            | 0.13    | json                                                           | HTTP client for OAuth2 introspection.                                 |
| dotenvy            | 0.15    | -                                                              | Load .env files. Simple, no framework.                                |
| bytes              | 1       | -                                                              | Byte buffer handling for uploads.                                     |
| futures-util       | 0.3     | -                                                              | Stream utilities.                                                     |

### Observability

| Crate                 | Version | Features | Rationale                                  |
| --------------------- | ------- | -------- | ------------------------------------------ |
| opentelemetry         | 0.28    | -        | OpenTelemetry API for distributed tracing. |
| opentelemetry_sdk     | 0.28    | rt-tokio | SDK with Tokio runtime integration.        |
| opentelemetry-otlp    | 0.28    | -        | OTLP exporter for traces and metrics.      |
| tracing-opentelemetry | 0.29    | -        | Bridge tracing spans to OpenTelemetry.     |

Prometheus metrics are handled by the OTEL collector (not this service). No `/metrics` endpoint.

### Dev Dependencies

| Crate          | Version | Rationale                                        |
| -------------- | ------- | ------------------------------------------------ |
| tempfile       | 3       | Temporary directories for storage tests.         |
| rstest         | 0.24    | Parameterized test cases.                        |
| wiremock       | 0.6     | Mock HTTP server for OAuth2 introspection tests. |
| http-body-util | 0.1     | Body utilities for Axum test requests.           |
| claims         | 0.8     | Assertion helpers for Result/Option types.       |

## Dropped Dependencies

| Crate            | Reason                                                                     |
| ---------------- | -------------------------------------------------------------------------- |
| async-trait      | Rust 1.75+ has native async fn in traits. No longer needed.                |
| config           | Over-complicated config loading. Replace with `dotenvy` + `std::env::var`. |
| tracing-appender | File-based log rotation unnecessary for containers. Log to stdout.         |
| mockall          | Not needed. Use simple test helpers and integration tests.                 |
| proptest         | Overkill. Simple unit tests are sufficient.                                |
| fake             | Test data generation not used in practice.                                 |
| validator        | Not actually used in the current code. Manual validation is clearer.       |
| regex            | Minimal usage can be inlined or removed.                                   |
| anyhow           | Use thiserror for typed errors.                                            |
| multer           | Axum handles multipart via its own extractor.                              |

## Cargo.toml Template

```toml
[package]
name = "media-management-service"
version = "0.2.0"
edition = "2024"
rust-version = "1.85"

[dependencies]
axum = { version = "0.8", features = ["multipart"] }
axum-extra = { version = "0.12", features = ["typed-header"] }
tokio = { version = "1", features = ["full"] }
tokio-util = { version = "0.7", features = ["io"] }
tower-http = { version = "0.6", features = [
    "cors", "compression-gzip", "trace", "timeout", "set-header", "request-id"
] }
tower = "0.5"
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio-rustls", "chrono", "uuid"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
sha2 = "0.10"
hex = "0.4"
base64 = "0.22"
hmac = "0.12"
rand = "0.9"
jsonwebtoken = "10"
reqwest = { version = "0.13", features = ["json"] }
dotenvy = "0.15"
bytes = "1"
futures-util = "0.3"
opentelemetry = "0.28"
opentelemetry_sdk = { version = "0.28", features = ["rt-tokio"] }
opentelemetry-otlp = "0.28"
tracing-opentelemetry = "0.29"

[dev-dependencies]
tempfile = "3"
rstest = "0.24"
wiremock = "0.6"
http-body-util = "0.1"
claims = "0.8"

[profile.dev]
opt-level = 0
debug = "line-tables-only"
split-debuginfo = "unpacked"
codegen-units = 256
incremental = true

[profile.release]
opt-level = 3
lto = "thin"
strip = true
codegen-units = 1
panic = "abort"

[profile.ci]
inherits = "dev"
incremental = false
codegen-units = 16

[lints.clippy]
all = { level = "deny" }
pedantic = { level = "deny" }
must_use_candidate = { level = "allow" }
missing_errors_doc = { level = "allow" }
cast_possible_truncation = { level = "allow" }
cast_sign_loss = { level = "allow" }
module_name_repetitions = { level = "allow" }
```

## OpenTelemetry Strategy

The observability stack uses `tracing` as the logging and spans facade (standard in Rust), bridged to OpenTelemetry via `tracing-opentelemetry`.

- **Traces**: Exported via OTLP to a configured collector endpoint
- **Metrics**: Exported via OTLP to the collector (no `/metrics` endpoint; Prometheus scraping handled by the collector)
- **Logs**: Stdout via `tracing-subscriber` (JSON in production, pretty in dev)
- **Graceful degradation**: When `OTEL_EXPORTER_OTLP_ENDPOINT` is empty or
  unset, the OTEL layer is not installed. No errors, no overhead.

```rust
// Initialization sketch
fn init_observability(config: &Config) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "media_management_service=info,tower_http=info".into());

    let fmt_layer = if config.run_mode == RunMode::Production {
        tracing_subscriber::fmt::layer().json().boxed()
    } else {
        tracing_subscriber::fmt::layer().pretty().boxed()
    };

    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer);

    if let Some(endpoint) = &config.otel_endpoint {
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_endpoint(endpoint))
            .install_batch(opentelemetry_sdk::runtime::Tokio)
            .expect("failed to install OTEL tracer");
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        registry.with(otel_layer).init();
    } else {
        registry.init();
    }
}
```

## Version Requirements

- **Rust**: 1.85+ (edition 2024, native async traits, `#[diagnostic]` support)
- **PostgreSQL**: 14+
- **Targets**: linux/amd64, linux/arm64
