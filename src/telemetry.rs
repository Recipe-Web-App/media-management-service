//! Observability initialization: structured logging + optional OpenTelemetry.
//!
//! When `OTEL_EXPORTER_OTLP_ENDPOINT` is configured, traces and metrics are
//! exported via OTLP/HTTP. When empty or unset, only structured logging is
//! active (no-op, no errors, no overhead).

use opentelemetry::trace::TracerProvider as _;
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

use crate::config::{Config, RunMode};

const SERVICE_NAME: &str = "media-management-service";

/// Guard that must be held for the lifetime of the application.
/// On drop or explicit shutdown, flushes pending telemetry and shuts down
/// OTEL providers.
pub struct OtelGuard {
    tracer_provider: Option<SdkTracerProvider>,
    meter_provider: Option<SdkMeterProvider>,
}

impl OtelGuard {
    /// Flush pending telemetry and shut down providers.
    /// Call this after the server stops accepting requests.
    pub fn shutdown(&self) {
        if let Some(tp) = &self.tracer_provider {
            if let Err(e) = tp.shutdown() {
                eprintln!("failed to shutdown tracer provider: {e}");
            }
        }
        if let Some(mp) = &self.meter_provider {
            if let Err(e) = mp.shutdown() {
                eprintln!("failed to shutdown meter provider: {e}");
            }
        }
    }
}

/// Initialize observability: tracing subscriber + optional OTEL export.
///
/// When `config.otel_endpoint` is `Some`, traces and metrics are exported
/// via OTLP/HTTP to the configured collector. When `None`, only structured
/// logging is active.
pub fn init(config: &Config) -> OtelGuard {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "media_management_service=info,tower_http=info".into());

    let fmt_layer = if config.run_mode == RunMode::Production {
        tracing_subscriber::fmt::layer().json().boxed()
    } else {
        tracing_subscriber::fmt::layer().pretty().boxed()
    };

    let (otel_layer, tracer_provider, meter_provider) =
        if let Some(ref endpoint) = config.otel_endpoint {
            let resource = Resource::builder_empty()
                .with_attributes([
                    KeyValue::new("service.name", SERVICE_NAME),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ])
                .build();

            // Traces
            let span_exporter = opentelemetry_otlp::SpanExporter::builder()
                .with_http()
                .with_endpoint(format!("{endpoint}/v1/traces"))
                .build()
                .expect("failed to build OTLP span exporter");

            let tp = SdkTracerProvider::builder()
                .with_batch_exporter(span_exporter)
                .with_resource(resource.clone())
                .build();

            global::set_tracer_provider(tp.clone());
            global::set_text_map_propagator(TraceContextPropagator::new());

            let tracer = tp.tracer(SERVICE_NAME);
            let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

            // Metrics
            let metric_exporter = opentelemetry_otlp::MetricExporter::builder()
                .with_http()
                .with_endpoint(format!("{endpoint}/v1/metrics"))
                .build()
                .expect("failed to build OTLP metric exporter");

            let mp = SdkMeterProvider::builder()
                .with_periodic_exporter(metric_exporter)
                .with_resource(resource)
                .build();

            global::set_meter_provider(mp.clone());

            tracing::info!("OpenTelemetry enabled, exporting to {endpoint}");

            (Some(otel_layer), Some(tp), Some(mp))
        } else {
            (None, None, None)
        };

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    OtelGuard {
        tracer_provider,
        meter_provider,
    }
}
