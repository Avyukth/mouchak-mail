use axum::http::header::{HeaderName, HeaderValue};
use axum::routing::get;
use axum::{Router, extract::State, http::StatusCode, response::IntoResponse};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::net::SocketAddr;
use std::sync::OnceLock;
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

// Modules
pub mod api;
pub mod auth;
pub mod error;
pub mod mcp;
pub mod openapi;
pub mod ratelimit;
pub mod tools;

#[cfg(feature = "with-web-ui")]
pub mod embedded;
#[cfg(feature = "with-web-ui")]
pub mod static_files;

use utoipa::{OpenApi, ToSchema};

use auth::{AuthConfig, JwksClient, auth_middleware};
pub use error::ServerError;
use lib_common::config::ServerConfig;
pub use lib_core::ModelManager;

// --- Application State
#[derive(Clone, axum::extract::FromRef)]
pub struct AppState {
    pub mm: ModelManager,
    pub metrics_handle: PrometheusHandle,
    pub start_time: Instant,
    pub auth_config: AuthConfig,
    pub jwks_client: Option<JwksClient>,
    pub ratelimit_config: ratelimit::RateLimitConfig,
}

static METRICS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

#[allow(clippy::expect_used)] // Metrics setup is infallible; panic acceptable during initialization
fn setup_metrics() -> PrometheusHandle {
    METRICS_HANDLE
        .get_or_init(|| {
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
        })
        .clone()
}

pub async fn run(config: ServerConfig) -> std::result::Result<(), ServerError> {
    // Initialize tracing is handled by caller (main binary) now

    // Initialize metrics
    let metrics_handle = setup_metrics();

    // Initialize ModelManager
    let mm = ModelManager::new().await?;

    // Create MCP routes with shared ModelManager (clone before move)
    let mcp_routes = mcp::mcp_routes(mm.clone());

    // Initialize Auth
    let auth_config = AuthConfig::from_env();
    tracing::info!("Auth Mode: {:?}", auth_config.mode);

    let jwks_client = auth_config
        .jwks_url
        .as_ref()
        .map(|url| JwksClient::new(url.clone()));

    let app_state = AppState {
        mm,
        metrics_handle,
        start_time: Instant::now(),
        auth_config,
        jwks_client,
        ratelimit_config: ratelimit::RateLimitConfig::new(),
    };

    // Build our application with routes
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let mut app = Router::new()
        .merge(api::routes())
        .merge(mcp_routes)
        .route_layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            auth_middleware,
        ))
        // Public routes (no auth)
        // .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()))
        .route("/api-docs/openapi.json", get(openapi_json))
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .route("/metrics", get(metrics_handler))
        // Prod Hardening: Liveness/Readiness probes (k8s style)
        .route("/healthz", get(health_handler))
        .layer(TraceLayer::new_for_http())
        // 4. Rate Limiting (Hardening 577.13)
        // Global middleware using Axum 0.8 middleware::from_fn_with_state
        .route_layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            ratelimit::rate_limit_middleware,
        ))
        .layer(cors) // Enable CORS
        // 5. Security Headers (Hardening CSP/XSS Protection)
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static(
                "script-src 'self'; connect-src 'self'; style-src 'self' 'unsafe-inline'",
            ),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ));

    // Conditionally add embedded web UI routes
    #[cfg(feature = "with-web-ui")]
    if config.serve_ui {
        tracing::info!("Web UI enabled at /");
        app = app.fallback(static_files::serve_embedded_file);
    } else {
        app = app.route("/", get(root_handler));
    }

    #[cfg(not(feature = "with-web-ui"))]
    {
        app = app.route("/", get(root_handler));
    }

    let app = app.with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("MCP Agent Mail Server starting on {}", addr);
    tracing::info!("Health check: http://{}/health", addr);

    // Axum 0.8+ serve with ConnectInfo for localhost bypass
    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Graceful shutdown - use into_make_service_with_connect_info to enable
    // ConnectInfo<SocketAddr> extraction in middleware for localhost bypass
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

async fn openapi_json() -> impl IntoResponse {
    axum::Json(openapi::ApiDoc::openapi())
}

#[allow(clippy::expect_used)] // Signal handler setup is infallible in practice; panic is acceptable
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

#[derive(serde::Serialize, ToSchema)]
struct HealthResponse {
    status: &'static str,
    #[schema(example = 120)]
    uptime_seconds: u64,
}

#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (status = 200, description = "Server Health", body = HealthResponse)
    )
)]
pub async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
    let uptime = state.start_time.elapsed().as_secs();
    let response = HealthResponse {
        status: "healthy",
        uptime_seconds: uptime,
    };
    (StatusCode::OK, axum::Json(response))
}

#[derive(serde::Serialize, ToSchema)]
struct ReadyResponse {
    status: &'static str,
    database: &'static str,
}

#[utoipa::path(
    get,
    path = "/api/ready",
    responses(
        (status = 200, description = "Readiness Check", body = ReadyResponse)
    )
)]
pub async fn ready_handler(State(state): State<AppState>) -> impl IntoResponse {
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
