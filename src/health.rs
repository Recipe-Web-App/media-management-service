use std::time::{Duration, Instant};

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::Utc;
use serde::Serialize;

use crate::db;
use crate::state::AppState;

const SERVICE_NAME: &str = "media-management-service";
const SERVICE_VERSION: &str = env!("CARGO_PKG_VERSION");
const CHECK_TIMEOUT: Duration = Duration::from_secs(10);

// ---------------------------------------------------------------------------
// Health (liveness) DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub timestamp: String,
    pub service: &'static str,
    pub version: &'static str,
    pub response_time_ms: u64,
    pub checks: HealthChecks,
}

#[derive(Debug, Serialize)]
pub struct HealthChecks {
    pub database: DependencyCheck,
    pub storage: DependencyCheck,
    pub overall: &'static str,
}

#[derive(Debug, Serialize)]
pub struct DependencyCheck {
    pub status: &'static str,
    pub response_time_ms: u64,
}

// ---------------------------------------------------------------------------
// Readiness DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ReadinessResponse {
    pub status: &'static str,
    pub timestamp: String,
    pub service: &'static str,
    pub version: &'static str,
    pub response_time_ms: u64,
    pub checks: ReadinessChecks,
}

#[derive(Debug, Serialize)]
pub struct ReadinessChecks {
    pub database: ReadinessDependencyCheck,
    pub storage: ReadinessDependencyCheck,
    pub overall: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ReadinessDependencyCheck {
    pub status: &'static str,
    pub response_time_ms: u64,
}

// ---------------------------------------------------------------------------
// Check result helpers
// ---------------------------------------------------------------------------

enum CheckOutcome {
    Ok(u64),
    Failed(u64),
    Timeout(u64),
}

impl CheckOutcome {
    fn elapsed_ms(&self) -> u64 {
        match *self {
            Self::Ok(ms) | Self::Failed(ms) | Self::Timeout(ms) => ms,
        }
    }

    fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }
}

fn millis_u64(d: std::time::Duration) -> u64 {
    // Health check durations are bounded by CHECK_TIMEOUT (seconds), so this
    // never truncates in practice.  Saturate for safety.
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

async fn run_db_check(state: &AppState) -> CheckOutcome {
    let start = Instant::now();
    match tokio::time::timeout(CHECK_TIMEOUT, db::db_health_check(&state.db_pool)).await {
        Ok(Ok(())) => CheckOutcome::Ok(millis_u64(start.elapsed())),
        Ok(Err(_)) => CheckOutcome::Failed(millis_u64(start.elapsed())),
        Err(_) => CheckOutcome::Timeout(millis_u64(CHECK_TIMEOUT)),
    }
}

async fn run_storage_check(state: &AppState) -> CheckOutcome {
    let start = Instant::now();
    match tokio::time::timeout(CHECK_TIMEOUT, state.storage.health_check()).await {
        Ok(Ok(())) => CheckOutcome::Ok(millis_u64(start.elapsed())),
        Ok(Err(_)) => CheckOutcome::Failed(millis_u64(start.elapsed())),
        Err(_) => CheckOutcome::Timeout(millis_u64(CHECK_TIMEOUT)),
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let start = Instant::now();
    let (db_outcome, storage_outcome) =
        tokio::join!(run_db_check(&state), run_storage_check(&state));

    let db_status = match &db_outcome {
        CheckOutcome::Ok(_) => "healthy",
        CheckOutcome::Failed(_) => "unhealthy",
        CheckOutcome::Timeout(_) => "timeout",
    };
    let storage_status = match &storage_outcome {
        CheckOutcome::Ok(_) => "healthy",
        CheckOutcome::Failed(_) => "unhealthy",
        CheckOutcome::Timeout(_) => "timeout",
    };

    let overall = compute_health_overall(&db_outcome, &storage_outcome);
    let http_status = if overall == "unhealthy" {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };

    let body = HealthResponse {
        status: overall,
        timestamp: Utc::now().to_rfc3339(),
        service: SERVICE_NAME,
        version: SERVICE_VERSION,
        response_time_ms: millis_u64(start.elapsed()),
        checks: HealthChecks {
            database: DependencyCheck {
                status: db_status,
                response_time_ms: db_outcome.elapsed_ms(),
            },
            storage: DependencyCheck {
                status: storage_status,
                response_time_ms: storage_outcome.elapsed_ms(),
            },
            overall,
        },
    };

    (http_status, Json(body))
}

pub async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    let start = Instant::now();
    let (db_outcome, storage_outcome) =
        tokio::join!(run_db_check(&state), run_storage_check(&state));

    let db_status = match &db_outcome {
        CheckOutcome::Ok(_) => "ready",
        CheckOutcome::Failed(_) => "not_ready",
        CheckOutcome::Timeout(_) => "timeout",
    };
    let storage_status = match &storage_outcome {
        CheckOutcome::Ok(_) => "ready",
        CheckOutcome::Failed(_) => "not_ready",
        CheckOutcome::Timeout(_) => "timeout",
    };

    let all_ready = db_outcome.is_ok() && storage_outcome.is_ok();
    let overall: &'static str = if all_ready { "ready" } else { "not_ready" };
    let http_status = if all_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let body = ReadinessResponse {
        status: overall,
        timestamp: Utc::now().to_rfc3339(),
        service: SERVICE_NAME,
        version: SERVICE_VERSION,
        response_time_ms: millis_u64(start.elapsed()),
        checks: ReadinessChecks {
            database: ReadinessDependencyCheck {
                status: db_status,
                response_time_ms: db_outcome.elapsed_ms(),
            },
            storage: ReadinessDependencyCheck {
                status: storage_status,
                response_time_ms: storage_outcome.elapsed_ms(),
            },
            overall,
        },
    };

    (http_status, Json(body))
}

// ---------------------------------------------------------------------------
// Status aggregation
// ---------------------------------------------------------------------------

fn compute_health_overall(db: &CheckOutcome, storage: &CheckOutcome) -> &'static str {
    match (db.is_ok(), storage.is_ok()) {
        (true, true) => "healthy",
        (false, false) => "unhealthy",
        _ => "degraded",
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- compute_health_overall ---

    #[test]
    fn overall_healthy_when_both_ok() {
        let db = CheckOutcome::Ok(5);
        let storage = CheckOutcome::Ok(3);
        assert_eq!(compute_health_overall(&db, &storage), "healthy");
    }

    #[test]
    fn overall_degraded_when_db_fails() {
        let db = CheckOutcome::Failed(100);
        let storage = CheckOutcome::Ok(3);
        assert_eq!(compute_health_overall(&db, &storage), "degraded");
    }

    #[test]
    fn overall_degraded_when_storage_fails() {
        let db = CheckOutcome::Ok(5);
        let storage = CheckOutcome::Failed(100);
        assert_eq!(compute_health_overall(&db, &storage), "degraded");
    }

    #[test]
    fn overall_degraded_when_db_timeout() {
        let db = CheckOutcome::Timeout(10000);
        let storage = CheckOutcome::Ok(3);
        assert_eq!(compute_health_overall(&db, &storage), "degraded");
    }

    #[test]
    fn overall_unhealthy_when_both_fail() {
        let db = CheckOutcome::Failed(100);
        let storage = CheckOutcome::Failed(200);
        assert_eq!(compute_health_overall(&db, &storage), "unhealthy");
    }

    #[test]
    fn overall_unhealthy_when_both_timeout() {
        let db = CheckOutcome::Timeout(10000);
        let storage = CheckOutcome::Timeout(10000);
        assert_eq!(compute_health_overall(&db, &storage), "unhealthy");
    }

    #[test]
    fn overall_unhealthy_when_mixed_failures() {
        let db = CheckOutcome::Failed(100);
        let storage = CheckOutcome::Timeout(10000);
        assert_eq!(compute_health_overall(&db, &storage), "unhealthy");
    }

    // --- CheckOutcome ---

    #[test]
    fn check_outcome_elapsed_ms() {
        assert_eq!(CheckOutcome::Ok(42).elapsed_ms(), 42);
        assert_eq!(CheckOutcome::Failed(99).elapsed_ms(), 99);
        assert_eq!(CheckOutcome::Timeout(10000).elapsed_ms(), 10000);
    }

    #[test]
    fn check_outcome_is_ok() {
        assert!(CheckOutcome::Ok(1).is_ok());
        assert!(!CheckOutcome::Failed(1).is_ok());
        assert!(!CheckOutcome::Timeout(1).is_ok());
    }

    // --- DTO serialisation ---

    #[test]
    fn health_response_serialises_correctly() {
        let resp = HealthResponse {
            status: "healthy",
            timestamp: "2025-01-15T10:30:00+00:00".to_string(),
            service: SERVICE_NAME,
            version: SERVICE_VERSION,
            response_time_ms: 25,
            checks: HealthChecks {
                database: DependencyCheck {
                    status: "healthy",
                    response_time_ms: 5,
                },
                storage: DependencyCheck {
                    status: "healthy",
                    response_time_ms: 3,
                },
                overall: "healthy",
            },
        };

        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["status"], "healthy");
        assert_eq!(json["service"], SERVICE_NAME);
        assert_eq!(json["version"], SERVICE_VERSION);
        assert_eq!(json["response_time_ms"], 25);
        assert_eq!(json["checks"]["database"]["status"], "healthy");
        assert_eq!(json["checks"]["database"]["response_time_ms"], 5);
        assert_eq!(json["checks"]["storage"]["status"], "healthy");
        assert_eq!(json["checks"]["storage"]["response_time_ms"], 3);
        assert_eq!(json["checks"]["overall"], "healthy");
    }

    #[test]
    fn readiness_response_serialises_correctly() {
        let resp = ReadinessResponse {
            status: "not_ready",
            timestamp: "2025-01-15T10:30:00+00:00".to_string(),
            service: SERVICE_NAME,
            version: SERVICE_VERSION,
            response_time_ms: 2010,
            checks: ReadinessChecks {
                database: ReadinessDependencyCheck {
                    status: "timeout",
                    response_time_ms: 2000,
                },
                storage: ReadinessDependencyCheck {
                    status: "ready",
                    response_time_ms: 3,
                },
                overall: "not_ready",
            },
        };

        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["status"], "not_ready");
        assert_eq!(json["checks"]["database"]["status"], "timeout");
        assert_eq!(json["checks"]["storage"]["status"], "ready");
        assert_eq!(json["checks"]["overall"], "not_ready");
    }

    #[test]
    fn health_response_contains_all_required_fields() {
        let resp = HealthResponse {
            status: "degraded",
            timestamp: "2025-01-15T10:30:00+00:00".to_string(),
            service: SERVICE_NAME,
            version: SERVICE_VERSION,
            response_time_ms: 50,
            checks: HealthChecks {
                database: DependencyCheck {
                    status: "unhealthy",
                    response_time_ms: 45,
                },
                storage: DependencyCheck {
                    status: "healthy",
                    response_time_ms: 3,
                },
                overall: "degraded",
            },
        };

        let json = serde_json::to_value(&resp).unwrap();
        let obj = json.as_object().unwrap();

        // Top-level required fields per OpenAPI HealthResponse schema
        for field in [
            "status",
            "timestamp",
            "service",
            "version",
            "response_time_ms",
            "checks",
        ] {
            assert!(obj.contains_key(field), "missing required field: {field}");
        }

        // Checks required fields
        let checks = json["checks"].as_object().unwrap();
        for field in ["database", "storage", "overall"] {
            assert!(
                checks.contains_key(field),
                "missing required checks field: {field}"
            );
        }

        // DependencyCheck required fields
        for dep in ["database", "storage"] {
            let dep_obj = json["checks"][dep].as_object().unwrap();
            for field in ["status", "response_time_ms"] {
                assert!(
                    dep_obj.contains_key(field),
                    "missing required {dep} field: {field}"
                );
            }
        }
    }
}
