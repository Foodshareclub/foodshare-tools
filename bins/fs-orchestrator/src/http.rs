//! The HTTP surface of the orchestrator daemon.
//!
//! The routes follow the probe contract the backend's `job-worker` uses,
//! so one Prometheus, one set of dashboards and one Quadlet health check
//! cover both services:
//!
//! - `/healthz` — unauthenticated, answers even when a dependency is
//!   unhappy, so a health check does not restart a process that is
//!   working apart from one thing.
//! - `/readyz` — unauthenticated, answers whether the daemon can do work.
//! - `/metrics` — Prometheus text.
//! - `/status` — the state of every domain.
//! - `/run/{domain}` — start a domain, or the whole plan.
//! - `/cancel` — stop the run in progress.

use std::time::{Duration, Instant};

use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use foodshare_core::health::HealthChecker;
use foodshare_orchestrator::{DomainKind, DomainOutcome, Orchestrator, PlanOutcome};

/// The name this service reports itself as.
pub const SERVICE_NAME: &str = "fs-orchestrator";

/// How long a single request may take.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

/// The shared state of the daemon.
#[derive(Clone)]
pub struct AppState {
    /// The orchestrator the routes drive.
    pub orchestrator: Orchestrator,
    /// When the daemon started.
    pub started_at: Instant,
}

impl AppState {
    /// Build the shared state.
    #[must_use]
    pub fn new(orchestrator: Orchestrator) -> Self {
        Self {
            orchestrator,
            started_at: Instant::now(),
        }
    }

    /// How long the daemon has been up, in seconds.
    #[must_use]
    pub fn uptime_seconds(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }
}

/// Build the router.
#[must_use]
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/health", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics))
        .route("/status", get(status))
        .route("/run/{domain}", post(run_domain))
        .route("/cancel", post(cancel))
        // The last layer added is the outermost, so the order below reads
        // from the handler outwards: the timeout is closest to the
        // handler, and the panic catcher is furthest out.
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            REQUEST_TIMEOUT,
        ))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(CatchPanicLayer::custom(panic_response))
        .with_state(state)
}

/// Turn a panic into a 500 rather than a dead process.
fn panic_response(panic: Box<dyn std::any::Any + Send + 'static>) -> Response {
    let detail = panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_string()))
        .unwrap_or_else(|| "unknown panic".to_string());
    tracing::error!(panic = %detail, "handler panicked");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        [(header::CONTENT_TYPE, "application/json")],
        r#"{"error":"internal_error","message":"the handler panicked"}"#,
    )
        .into_response()
}

/// The health report.
async fn healthz(State(state): State<AppState>) -> Response {
    let report = HealthChecker::new()
        .with_standard_checks()
        .with_web_checks()
        .run();

    // A degraded service is still serving, so the answer stays 200: a
    // health check that restarts the process would turn one unhappy
    // dependency into an outage.
    let status = match report.status {
        foodshare_core::health::HealthStatus::Healthy => "healthy",
        foodshare_core::health::HealthStatus::Degraded => "degraded",
        foodshare_core::health::HealthStatus::Unhealthy => "unhealthy",
        foodshare_core::health::HealthStatus::Unknown => "unknown",
    };

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": status,
            "service": SERVICE_NAME,
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_seconds": state.uptime_seconds(),
            "running": state.orchestrator.snapshot().running,
            "checks": report.checks,
            "total_duration_ms": report.total_duration_ms,
        })),
    )
        .into_response()
}

/// Whether the daemon can do work.
async fn readyz(State(state): State<AppState>) -> Response {
    let workspace = state.orchestrator.workspace();
    let tools = workspace.tools();
    let ready = tools.is_dir();
    let body = serde_json::json!({
        "ready": ready,
        "reason": (!ready).then(|| format!("{} is not a directory", tools.display())),
        "workspace": workspace.root().display().to_string(),
    });
    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(body)).into_response()
}

/// The Prometheus exposition format.
async fn metrics() -> Response {
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        foodshare_telemetry::prometheus::render(),
    )
        .into_response()
}

/// The state of every domain.
async fn status(State(state): State<AppState>) -> Response {
    let snapshot = state.orchestrator.snapshot();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "service": SERVICE_NAME,
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_seconds": state.uptime_seconds(),
            "running": snapshot.running,
            "domains": snapshot.domains,
            "metrics": foodshare_telemetry::metrics().export_json(),
        })),
    )
        .into_response()
}

/// Whether the caller wants the result of the run.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RunQuery {
    /// Wait for the run to finish and return its outcome.
    #[serde(default)]
    pub wait: bool,
}

/// Start a domain, or the whole plan when the domain is `all`.
async fn run_domain(
    State(state): State<AppState>,
    Path(domain): Path<String>,
    Query(query): Query<RunQuery>,
) -> Response {
    let orchestrator = state.orchestrator.clone();
    let requested = domain.to_ascii_lowercase();

    if requested == "all" {
        if query.wait {
            return match orchestrator.run_plan().await {
                Ok(outcome) => plan_response(outcome),
                Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
            };
        }
        let background = tokio::spawn(async move {
            if let Err(e) = orchestrator.run_plan().await {
                tracing::error!(error = %e, "plan run failed");
            }
        });
        return accepted_response("all", background.abort_handle());
    }

    let Some(kind) = DomainKind::from_name(&requested) else {
        return error_response(
            StatusCode::BAD_REQUEST,
            &format!("unknown domain `{requested}`"),
        );
    };

    if query.wait {
        return match orchestrator.run_domain(kind).await {
            Ok(outcome) => domain_response(outcome),
            Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
        };
    }

    let background = tokio::spawn(async move {
        if let Err(e) = orchestrator.run_domain(kind).await {
            tracing::error!(error = %e, domain = kind.name(), "domain run failed");
        }
    });
    accepted_response(kind.name(), background.abort_handle())
}

/// Stop the run in progress.
async fn cancel(State(state): State<AppState>) -> Response {
    state.orchestrator.cancel();
    (
        StatusCode::OK,
        Json(serde_json::json!({ "cancelled": true })),
    )
        .into_response()
}

fn accepted_response(domain: &str, handle: tokio::task::AbortHandle) -> Response {
    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "accepted": true,
            "domain": domain,
            "running": true,
            "abort_handle": format!("{:?}", handle),
        })),
    )
        .into_response()
}

fn domain_response(outcome: DomainOutcome) -> Response {
    let status = if outcome.success {
        StatusCode::OK
    } else {
        StatusCode::MULTI_STATUS
    };
    (status, Json(outcome)).into_response()
}

fn plan_response(outcome: PlanOutcome) -> Response {
    let status = if outcome.success {
        StatusCode::OK
    } else {
        StatusCode::MULTI_STATUS
    };
    (status, Json(outcome)).into_response()
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use foodshare_orchestrator::Workspace;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn app() -> Router {
        let workspace = Workspace::with_root("/tmp/foodshare-orchestrator-test");
        let orchestrator = Orchestrator::new(workspace);
        router(AppState::new(orchestrator))
    }

    async fn get(path: &str) -> (StatusCode, String) {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let status = response.status();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        (status, String::from_utf8_lossy(&body).to_string())
    }

    #[tokio::test]
    async fn test_healthz_answers_ok() {
        let (status, body) = get("/healthz").await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            body.contains("\"service\":\"fs-orchestrator\""),
            "got: {body}"
        );
    }

    #[tokio::test]
    async fn test_metrics_are_prometheus_text() {
        let (status, body) = get("/metrics").await;
        assert_eq!(status, StatusCode::OK);
        // The registry always has at least the session's own metrics.
        assert!(!body.is_empty());
    }

    #[tokio::test]
    async fn test_status_lists_every_domain() {
        let (status, body) = get("/status").await;
        assert_eq!(status, StatusCode::OK);
        for domain in DomainKind::ALL {
            assert!(body.contains(domain.name()), "missing {domain} in {body}");
        }
    }

    #[tokio::test]
    async fn test_readyz_reports_the_workspace() {
        let (status, body) = get("/readyz").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(body.contains("\"ready\":false"), "got: {body}");
    }

    #[tokio::test]
    async fn test_unknown_domain_is_rejected() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/run/nonsense")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_cancel_is_accepted() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/cancel")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }
}
