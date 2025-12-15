use axum::routing::{get, post};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Router};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::net::SocketAddr;
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use std::sync::OnceLock;

// Modules
pub mod error;
pub mod api;
pub mod auth;
pub mod tools;

pub use error::ServerError;
pub use lib_core::ModelManager;
use auth::{auth_middleware, AuthConfig, JwksClient};
use lib_common::config::ServerConfig;

// --- Application State
#[derive(Clone)]
pub struct AppState {
    pub mm: ModelManager,
    pub metrics_handle: PrometheusHandle,
    pub start_time: Instant,
    pub auth_config: AuthConfig,
    pub jwks_client: Option<JwksClient>,
}

static METRICS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

fn setup_metrics() -> PrometheusHandle {
    METRICS_HANDLE.get_or_init(|| {
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
    }).clone()
}

pub async fn run(config: ServerConfig) -> std::result::Result<(), ServerError> {
    // Initialize tracing is handled by caller (main binary) now

    // Initialize metrics
    let metrics_handle = setup_metrics();

    // Initialize ModelManager
    let mm = ModelManager::new().await?;
    
    // Initialize Auth
    let auth_config = AuthConfig::from_env();
    tracing::info!("Auth Mode: {:?}", auth_config.mode);

    let jwks_client = auth_config.jwks_url.as_ref().map(|url| JwksClient::new(url.clone()));

    let app_state = AppState {
        mm,
        metrics_handle,
        start_time: Instant::now(),
        auth_config,
        jwks_client,
    };

    // Build our application with routes
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .merge(api::routes())
        .route("/mcp", post(mcp_handler))
        .route_layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            auth_middleware,
        ))
        // Public routes (no auth)
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .route("/metrics", get(metrics_handler))
        // Prod Hardening: Liveness/Readiness probes (k8s style)
        .route("/healthz", get(health_handler)) 
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("MCP Agent Mail Server starting on {}", addr);
    tracing::info!("Health check: http://{}/health", addr);

    // Axum 0.8+ serve with ConnectInfo for localhost bypass
    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Graceful shutdown - use into_make_service_with_connect_info to enable
    // ConnectInfo<SocketAddr> extraction in middleware for localhost bypass
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Signal received, starting graceful shutdown");
}

async fn root_handler() -> &'static str {
    "MCP Agent Mail Server is running!"
}

async fn mcp_handler() -> &'static str {
    "MCP endpoint - not yet implemented"
}

#[derive(serde::Serialize)]
struct HealthResponse {
    status: &'static str,
    uptime_seconds: u64,
}

async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
    let uptime = state.start_time.elapsed().as_secs();
    let response = HealthResponse {
        status: "healthy",
        uptime_seconds: uptime,
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
