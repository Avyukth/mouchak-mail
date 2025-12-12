use axum::routing::{get, post};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Router};
use lib_core::ModelManager;
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::net::SocketAddr;
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::error::ServerError;

mod api;
mod error;
mod tools;

// --- Application State
#[derive(Clone)]
pub struct AppState {
    pub mm: ModelManager,
    pub metrics_handle: PrometheusHandle,
    pub start_time: Instant,
}

fn setup_metrics() -> PrometheusHandle {
    const EXPONENTIAL_SECONDS: &[f64] = &[
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ];

    PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full("http_request_duration_seconds".to_string()),
            EXPONENTIAL_SECONDS,
        )
        .expect("Failed to set buckets")
        .install_recorder()
        .expect("Failed to install Prometheus recorder")
}

fn setup_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=debug,axum=debug"));

    // Check if we should use JSON format (for production)
    let json_logs = std::env::var("LOG_FORMAT").map(|v| v == "json").unwrap_or(false);

    if json_logs {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().pretty())
            .init();
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), ServerError> {
    // Initialize tracing
    setup_tracing();

    // Initialize metrics
    let metrics_handle = setup_metrics();

    // Initialize ModelManager
    let mm = ModelManager::new().await?;
    let app_state = AppState {
        mm,
        metrics_handle,
        start_time: Instant::now(),
    };

    // Build our application with routes
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .merge(api::routes())
        .route("/", get(root_handler))
        .route("/mcp", post(mcp_handler))
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .route("/metrics", get(metrics_handler))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("MCP Agent Mail Server starting on {}", addr);
    tracing::info!("Health check: http://{}/health", addr);
    tracing::info!("Metrics: http://{}/metrics", addr);

    // Axum 0.8+ uses axum::serve() directly with tokio listener
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn root_handler() -> &'static str {
    "MCP Agent Mail Server is running!"
}

async fn mcp_handler() -> &'static str {
    // TODO: Integrate mcp-protocol-sdk here
    "MCP endpoint - not yet implemented"
}

#[derive(serde::Serialize)]
struct HealthResponse {
    status: &'static str,
    uptime_seconds: u64,
    version: &'static str,
}

async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
    let uptime = state.start_time.elapsed().as_secs();
    let response = HealthResponse {
        status: "healthy",
        uptime_seconds: uptime,
        version: env!("CARGO_PKG_VERSION"),
    };
    (StatusCode::OK, axum::Json(response))
}

#[derive(serde::Serialize)]
struct ReadyResponse {
    status: &'static str,
    database: &'static str,
}

async fn ready_handler(State(state): State<AppState>) -> impl IntoResponse {
    // Check database connectivity
    let db_status = match state.mm.health_check().await {
        Ok(true) => "connected",
        _ => "disconnected",
    };

    let is_ready = db_status == "connected";
    let response = ReadyResponse {
        status: if is_ready { "ready" } else { "not_ready" },
        database: db_status,
    };

    let status_code = if is_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, axum::Json(response))
}

async fn metrics_handler(State(state): State<AppState>) -> impl IntoResponse {
    state.metrics_handle.render()
}
