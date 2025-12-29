use axum::{
    extract::{ConnectInfo, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use axum_extra::headers::{Authorization, HeaderMapExt, authorization::Bearer};
use jsonwebtoken::{DecodingKey, Validation, decode, decode_header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::AppState;

/// Authenticated user information stored as request extension
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// The subject from JWT (typically user/agent identifier)
    pub subject: String,
    /// Optional agent name extracted from claims or subject
    pub agent_name: Option<String>,
    /// Optional project context
    pub project_slug: Option<String>,
}

/// Authentication configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub mode: AuthMode,
    pub bearer_token: Option<String>,
    pub jwks_url: Option<String>,
    pub jwt_audience: Option<String>,
    pub jwt_issuer: Option<String>,
    pub allow_localhost: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMode {
    None,
    Bearer,
    Jwt,
}

impl AuthConfig {
    pub fn from_env() -> Self {
        let mode_str = std::env::var("HTTP_AUTH_MODE").unwrap_or_else(|_| "none".to_string());
        let mode = match mode_str.to_lowercase().as_str() {
            "bearer" => AuthMode::Bearer,
            "jwt" => AuthMode::Jwt,
            _ => AuthMode::None,
        };

        let bearer_token = std::env::var("HTTP_BEARER_TOKEN").ok();
        let jwks_url = std::env::var("HTTP_JWKS_URL").ok();
        let jwt_audience = std::env::var("HTTP_JWT_AUDIENCE").ok();
        let jwt_issuer = std::env::var("HTTP_JWT_ISSUER").ok();
        let allow_localhost = std::env::var("HTTP_ALLOW_LOCALHOST_UNAUTHENTICATED")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(true);

        // Validation
        if mode == AuthMode::Bearer && bearer_token.is_none() {
            warn!("HTTP_AUTH_MODE=bearer but HTTP_BEARER_TOKEN is not set. Auth will fail.");
        }
        if mode == AuthMode::Jwt && jwks_url.is_none() {
            warn!("HTTP_AUTH_MODE=jwt but HTTP_JWKS_URL is not set. Auth will fail.");
        }
        if mode == AuthMode::Jwt {
            if jwt_audience.is_some() {
                info!("JWT audience validation enabled");
            }
            if jwt_issuer.is_some() {
                info!("JWT issuer validation enabled");
            }
        }

        Self {
            mode,
            bearer_token,
            jwks_url,
            jwt_audience,
            jwt_issuer,
            allow_localhost,
        }
    }
}

/// JWKS Key structure
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct Jwk {
    kid: String,
    kty: String,
    alg: Option<String>,
    n: String,
    e: String,
}

#[derive(Debug, Deserialize)]
struct JwksResponse {
    keys: Vec<Jwk>,
}

/// JWKS Client for fetching and caching keys with TTL-based refresh
#[derive(Clone)]
pub struct JwksClient {
    url: String,
    client: Client,
    keys: Arc<RwLock<HashMap<String, DecodingKey>>>,
    last_refresh: Arc<RwLock<Option<Instant>>>,
    cache_ttl: Duration,
}

impl JwksClient {
    /// Create a new JWKS client with default 1-hour cache TTL
    pub fn new(url: String) -> Self {
        Self::new_with_ttl(url, Duration::from_secs(3600))
    }

    /// Create a new JWKS client with custom cache TTL
    pub fn new_with_ttl(url: String, cache_ttl: Duration) -> Self {
        Self {
            url,
            client: Client::new(),
            keys: Arc::new(RwLock::new(HashMap::new())),
            last_refresh: Arc::new(RwLock::new(None)),
            cache_ttl,
        }
    }

    /// Check if the cache needs refresh based on TTL
    async fn should_refresh(&self) -> bool {
        let last_refresh = self.last_refresh.read().await;
        match *last_refresh {
            None => true,
            Some(last) => last.elapsed() >= self.cache_ttl,
        }
    }

    pub async fn get_verifying_key(&self, kid: &str) -> Option<DecodingKey> {
        // Fast path: check cache if not expired
        if !self.should_refresh().await {
            let keys = self.keys.read().await;
            if let Some(key) = keys.get(kid) {
                return Some(key.clone());
            }
        }

        // Cache miss: refresh keys from JWKS endpoint
        if let Err(e) = self.refresh_keys().await {
            error!("Failed to refresh JWKS: {}", e);
            // Try to return cached key even if refresh failed
            let keys = self.keys.read().await;
            return keys.get(kid).cloned();
        }

        // Check cache again
        let keys = self.keys.read().await;
        keys.get(kid).cloned()
    }

    async fn refresh_keys(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Refreshing JWKS from {}", self.url);
        let resp = self
            .client
            .get(&self.url)
            .send()
            .await?
            .json::<JwksResponse>()
            .await?;

        let mut new_keys = HashMap::new();
        for jwk in resp.keys {
            if jwk.kty == "RSA"
                && let Ok(decoding_key) = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
            {
                new_keys.insert(jwk.kid.clone(), decoding_key);
            }
        }

        // Update keys and refresh timestamp
        let mut keys = self.keys.write().await;
        *keys = new_keys;
        let key_count = keys.len();
        drop(keys);

        let mut last_refresh = self.last_refresh.write().await;
        *last_refresh = Some(Instant::now());

        info!(
            "JWKS refreshed, loaded {} keys (TTL: {:?})",
            key_count, self.cache_ttl
        );
        Ok(())
    }
}

/// JWT Claims - Standard JWT claims plus custom fields
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    /// Subject (usually user ID)
    sub: String,
    /// Expiration time (as UTC timestamp)
    exp: usize,
    /// Issuer
    #[serde(skip_serializing_if = "Option::is_none")]
    iss: Option<String>,
    /// Audience
    #[serde(skip_serializing_if = "Option::is_none")]
    aud: Option<String>,
    /// Issued at
    #[serde(skip_serializing_if = "Option::is_none")]
    iat: Option<usize>,
    /// Not before
    #[serde(skip_serializing_if = "Option::is_none")]
    nbf: Option<usize>,
    /// JWT ID
    #[serde(skip_serializing_if = "Option::is_none")]
    jti: Option<String>,
}

/// Check if the IP address is localhost (127.0.0.1 or ::1)
fn is_localhost(addr: &SocketAddr) -> bool {
    match addr.ip() {
        std::net::IpAddr::V4(ipv4) => ipv4.is_loopback(),
        std::net::IpAddr::V6(ipv6) => ipv6.is_loopback(),
    }
}

/// Validate bearer token against expected value
fn validate_bearer_token(token: &str, expected: Option<&String>) -> Result<(), StatusCode> {
    match expected {
        Some(exp) if exp == token => Ok(()),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

/// Validate JWT token and return authenticated user
async fn validate_jwt_token(
    token: &str,
    jwks_client: &JwksClient,
    auth_config: &AuthConfig,
) -> Result<AuthenticatedUser, StatusCode> {
    let header = decode_header(token).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let kid = header.kid.ok_or(StatusCode::UNAUTHORIZED)?;
    let key = jwks_client
        .get_verifying_key(&kid)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let mut validation = Validation::new(header.alg);
    if let Some(ref audience) = auth_config.jwt_audience {
        validation.set_audience(&[audience]);
    }
    if let Some(ref issuer) = auth_config.jwt_issuer {
        validation.set_issuer(&[issuer]);
    }

    match decode::<Claims>(token, &key, &validation) {
        Ok(token_data) => {
            info!(
                "JWT validated successfully for subject: {}",
                token_data.claims.sub
            );
            Ok(AuthenticatedUser {
                subject: token_data.claims.sub.clone(),
                agent_name: Some(token_data.claims.sub),
                project_slug: None,
            })
        }
        Err(e) => {
            warn!("JWT validation failed: {}", e);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Check if request should bypass authentication
fn should_bypass_auth(req: &Request<axum::body::Body>, auth_config: &AuthConfig) -> bool {
    if auth_config.mode == AuthMode::None {
        return true;
    }
    if auth_config.allow_localhost
        && let Some(connect_info) = req.extensions().get::<ConnectInfo<SocketAddr>>()
        && is_localhost(&connect_info.0)
    {
        info!(
            "Localhost bypass: allowing unauthenticated request from {}",
            connect_info.0
        );
        return true;
    }
    false
}

/// Auth Middleware
pub async fn auth_middleware(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_config = &state.auth_config;

    if should_bypass_auth(&req, auth_config) {
        return Ok(next.run(req).await);
    }

    let token = req
        .headers()
        .typed_get::<Authorization<Bearer>>()
        .map(|auth| auth.token().to_string())
        .ok_or_else(|| {
            warn!("No Authorization header and localhost bypass not applicable");
            StatusCode::UNAUTHORIZED
        })?;

    match auth_config.mode {
        AuthMode::Bearer => {
            validate_bearer_token(&token, auth_config.bearer_token.as_ref())?;
            Ok(next.run(req).await)
        }
        AuthMode::Jwt => {
            let jwks_client = state.jwks_client.as_ref().ok_or_else(|| {
                error!("Auth mode is JWT but JwksClient is missing");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            let auth_user = validate_jwt_token(&token, jwks_client, auth_config).await?;
            let mut req = req;
            req.extensions_mut().insert(auth_user);
            Ok(next.run(req).await)
        }
        AuthMode::None => unreachable!(),
    }
}

/// Route-to-capability mapping for RBAC enforcement
/// Returns the required capability for a given route path, or None if no capability check needed
pub fn get_required_capability(path: &str) -> Option<&'static str> {
    let normalized = path.trim_end_matches('/');
    match normalized {
        // Messaging operations
        "/api/message/send" | "/api/send_message" => Some("send_message"),
        "/api/message/reply" | "/api/reply_message" => Some("send_message"),
        "/api/inbox" | "/api/fetch_inbox" | "/api/list_inbox" | "/api/get_inbox" => {
            Some("fetch_inbox")
        }
        "/api/outbox" | "/api/fetch_outbox" | "/api/list_outbox" | "/api/get_outbox" => {
            Some("fetch_outbox")
        }
        "/api/message/acknowledge" | "/api/acknowledge_message" => Some("acknowledge_message"),
        "/api/message/read" | "/api/mark_message_read" => Some("fetch_inbox"),
        "/api/messages/search" | "/api/search_messages" => Some("fetch_inbox"),
        // File reservations
        "/api/file_reservations/paths" | "/api/file_reservation_paths" => Some("file_reservation"),
        "/api/file_reservations/list" | "/api/list_file_reservations" | "/api/reservations" => {
            Some("file_reservation")
        }
        "/api/file_reservations/release" | "/api/release_file_reservation" => {
            Some("file_reservation")
        }
        "/api/file_reservations/renew" | "/api/renew_file_reservation" => Some("file_reservation"),
        "/api/file_reservations/force_release" | "/api/force_release_file_reservation" => {
            Some("admin")
        }
        // Build slots
        "/api/build_slots/acquire" | "/api/acquire_build_slot" => Some("build"),
        "/api/build_slots/renew" | "/api/renew_build_slot" => Some("build"),
        "/api/build_slots/release" | "/api/release_build_slot" => Some("build"),
        // Overseer and archive
        "/api/overseer/send" | "/api/send_overseer_message" => Some("overseer"),
        "/api/archive/commit" | "/api/commit_archive" => Some("archive"),
        _ => None,
    }
}

/// Configuration for capabilities middleware behavior
#[derive(Debug, Clone)]
pub struct CapabilitiesConfig {
    pub enabled: bool,
    pub log_checks: bool,
}

impl Default for CapabilitiesConfig {
    fn default() -> Self {
        Self {
            enabled: std::env::var("RBAC_ENABLED")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(false),
            log_checks: std::env::var("RBAC_LOG_CHECKS")
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(false),
        }
    }
}

/// Middleware to enforce capability checks (RBAC)
///
/// Checks if the authenticated user has the required capability for a route.
/// Configuration: RBAC_ENABLED=true to enable, RBAC_LOG_CHECKS=true to log.
pub async fn capabilities_middleware(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let config = CapabilitiesConfig::default();
    if !config.enabled {
        return Ok(next.run(req).await);
    }

    let path = req.uri().path();
    let required_capability = match get_required_capability(path) {
        Some(cap) => cap,
        None => return Ok(next.run(req).await),
    };

    let auth_user = match req.extensions().get::<AuthenticatedUser>() {
        Some(user) => user.clone(),
        None => return Ok(next.run(req).await), // No auth, allow through (RBAC only for authenticated)
    };

    let agent_name = match &auth_user.agent_name {
        Some(name) => name.clone(),
        None => {
            warn!(
                "RBAC: No agent_name for {} (requires: {})",
                path, required_capability
            );
            return Err(StatusCode::FORBIDDEN);
        }
    };

    let mm = &state.mm;
    let ctx = mouchak_mail_core::Ctx::root_ctx();
    let projects = mouchak_mail_core::model::project::ProjectBmc::list_all(&ctx, mm)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for project in projects {
        if let Ok(agent) =
            mouchak_mail_core::model::agent::AgentBmc::get_by_name(&ctx, mm, project.id, &agent_name).await
            && let Ok(true) = mouchak_mail_core::model::agent_capabilities::AgentCapabilityBmc::check(
                &ctx,
                mm,
                agent.id.get(),
                required_capability,
            )
            .await
        {
            if config.log_checks {
                info!(
                    "RBAC: {} has {} in {}",
                    agent_name, required_capability, project.slug
                );
            }
            return Ok(next.run(req).await);
        }
    }

    warn!(
        "RBAC: {} denied {} (missing: {})",
        agent_name, path, required_capability
    );
    Err(StatusCode::FORBIDDEN)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::get,
    };
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    use mouchak_mail_common::config::AppConfig;
    use rand::rngs::OsRng;
    use rsa::{RsaPrivateKey, pkcs1::EncodeRsaPrivateKey, traits::PublicKeyParts};
    use std::sync::Arc;
    use tower::util::ServiceExt; // for oneshot
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::path};

    async fn handler() -> &'static str {
        "OK"
    }

    /// Helper to generate RSA key pair and JWKS JSON
    fn generate_test_keys(kid: &str) -> (RsaPrivateKey, String) {
        let mut rng = OsRng;
        let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("Failed to generate key");
        let public_key = private_key.to_public_key();

        // Extract n and e for JWKS
        let n = URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
        let e = URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be());

        let jwks_json = format!(
            r#"{{"keys":[{{"kty":"RSA","kid":"{}","alg":"RS256","n":"{}","e":"{}"}}]}}"#,
            kid, n, e
        );

        (private_key, jwks_json)
    }

    /// Helper to create a JWT with given claims
    fn create_test_jwt(private_key: &RsaPrivateKey, kid: &str, exp: usize) -> String {
        create_test_jwt_with_claims(private_key, kid, exp, None, None)
    }

    /// Helper to create a JWT with optional audience and issuer
    fn create_test_jwt_with_claims(
        private_key: &RsaPrivateKey,
        kid: &str,
        exp: usize,
        aud: Option<String>,
        iss: Option<String>,
    ) -> String {
        let claims = Claims {
            sub: "test-user".to_string(),
            exp,
            iss,
            aud,
            iat: Some(chrono::Utc::now().timestamp() as usize),
            nbf: None,
            jti: None,
        };

        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(kid.to_string());

        let der = private_key
            .to_pkcs1_der()
            .expect("Failed to encode private key");
        let encoding_key = EncodingKey::from_rsa_der(der.as_bytes());

        encode(&header, &claims, &encoding_key).expect("Failed to encode JWT")
    }

    #[tokio::test]
    async fn test_auth_none() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let repo_root = temp_dir.path().join("archive");
        std::fs::create_dir_all(&repo_root).unwrap();

        let db = libsql::Builder::new_local(db_path).build().await.unwrap();
        let conn = db.connect().unwrap();
        let app_config = Arc::new(AppConfig::default());
        let mm = crate::ModelManager::new_for_test(conn, repo_root, app_config);

        let auth_config = AuthConfig {
            mode: AuthMode::None,
            bearer_token: None,
            jwks_url: None,
            jwt_audience: None,
            jwt_issuer: None,
            allow_localhost: false,
        };
        let app_state = AppState {
            mm,
            metrics_handle: crate::setup_metrics(),
            start_time: std::time::Instant::now(),
            auth_config,
            jwks_client: None,
            ratelimit_config: crate::ratelimit::RateLimitConfig::new(),
        };

        let app = Router::new()
            .route("/", get(handler))
            .layer(middleware::from_fn_with_state(app_state, auth_middleware));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_bearer_success() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let repo_root = temp_dir.path().join("archive");
        std::fs::create_dir_all(&repo_root).unwrap();

        let db = libsql::Builder::new_local(db_path).build().await.unwrap();
        let conn = db.connect().unwrap();
        let app_config = Arc::new(AppConfig::default());
        let mm = crate::ModelManager::new_for_test(conn, repo_root, app_config);

        let auth_config = AuthConfig {
            mode: AuthMode::Bearer,
            bearer_token: Some("secret123".to_string()),
            jwks_url: None,
            jwt_audience: None,
            jwt_issuer: None,
            allow_localhost: false,
        };
        let app_state = AppState {
            mm,
            metrics_handle: crate::setup_metrics(),
            start_time: std::time::Instant::now(),
            auth_config,
            jwks_client: None,
            ratelimit_config: crate::ratelimit::RateLimitConfig::new(),
        };

        let app = Router::new()
            .route("/", get(handler))
            .layer(middleware::from_fn_with_state(app_state, auth_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("Authorization", "Bearer secret123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_bearer_failure() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let repo_root = temp_dir.path().join("archive");
        std::fs::create_dir_all(&repo_root).unwrap();

        let db = libsql::Builder::new_local(db_path).build().await.unwrap();
        let conn = db.connect().unwrap();
        let app_config = Arc::new(AppConfig::default());
        let mm = crate::ModelManager::new_for_test(conn, repo_root, app_config);

        let auth_config = AuthConfig {
            mode: AuthMode::Bearer,
            bearer_token: Some("secret123".to_string()),
            jwks_url: None,
            jwt_audience: None,
            jwt_issuer: None,
            allow_localhost: false,
        };
        let app_state = AppState {
            mm,
            metrics_handle: crate::setup_metrics(),
            start_time: std::time::Instant::now(),
            auth_config,
            jwks_client: None,
            ratelimit_config: crate::ratelimit::RateLimitConfig::new(),
        };

        let app = Router::new()
            .route("/", get(handler))
            .layer(middleware::from_fn_with_state(app_state, auth_middleware));

        // Wrong token
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("Authorization", "Bearer wrong")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // No token
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_jwt_success() {
        // 1. Generate RSA key pair and JWKS
        let kid = "test-key-1";
        let (private_key, jwks_json) = generate_test_keys(kid);

        // 2. Start mock JWKS server
        let mock_server = MockServer::start().await;
        Mock::given(path("/.well-known/jwks.json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(jwks_json))
            .mount(&mock_server)
            .await;

        // 3. Create valid JWT (expires in 1 hour)
        let exp = (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize;
        let token = create_test_jwt(&private_key, kid, exp);

        // 4. Setup app state with JWT auth
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let repo_root = temp_dir.path().join("archive");
        std::fs::create_dir_all(&repo_root).unwrap();

        let db = libsql::Builder::new_local(db_path).build().await.unwrap();
        let conn = db.connect().unwrap();
        let app_config = Arc::new(AppConfig::default());
        let mm = crate::ModelManager::new_for_test(conn, repo_root, app_config);

        let jwks_url = format!("{}/.well-known/jwks.json", mock_server.uri());
        let auth_config = AuthConfig {
            mode: AuthMode::Jwt,
            bearer_token: None,
            jwks_url: Some(jwks_url.clone()),
            jwt_audience: None,
            jwt_issuer: None,
            allow_localhost: false,
        };
        let jwks_client = Some(JwksClient::new(jwks_url));

        let app_state = AppState {
            mm,
            metrics_handle: crate::setup_metrics(),
            start_time: std::time::Instant::now(),
            auth_config,
            jwks_client,
            ratelimit_config: crate::ratelimit::RateLimitConfig::new(),
        };

        let app = Router::new()
            .route("/", get(handler))
            .layer(middleware::from_fn_with_state(app_state, auth_middleware));

        // 5. Make request with valid JWT
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("Authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_jwt_expired() {
        // 1. Generate RSA key pair and JWKS
        let kid = "test-key-2";
        let (private_key, jwks_json) = generate_test_keys(kid);

        // 2. Start mock JWKS server
        let mock_server = MockServer::start().await;
        Mock::given(path("/.well-known/jwks.json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(jwks_json))
            .mount(&mock_server)
            .await;

        // 3. Create expired JWT (expired 1 hour ago)
        let exp = (chrono::Utc::now() - chrono::Duration::hours(1)).timestamp() as usize;
        let token = create_test_jwt(&private_key, kid, exp);

        // 4. Setup app state
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let repo_root = temp_dir.path().join("archive");
        std::fs::create_dir_all(&repo_root).unwrap();

        let db = libsql::Builder::new_local(db_path).build().await.unwrap();
        let conn = db.connect().unwrap();
        let app_config = Arc::new(AppConfig::default());
        let mm = crate::ModelManager::new_for_test(conn, repo_root, app_config);

        let jwks_url = format!("{}/.well-known/jwks.json", mock_server.uri());
        let auth_config = AuthConfig {
            mode: AuthMode::Jwt,
            bearer_token: None,
            jwks_url: Some(jwks_url.clone()),
            jwt_audience: None,
            jwt_issuer: None,
            allow_localhost: false,
        };
        let jwks_client = Some(JwksClient::new(jwks_url));

        let app_state = AppState {
            mm,
            metrics_handle: crate::setup_metrics(),
            start_time: std::time::Instant::now(),
            auth_config,
            jwks_client,
            ratelimit_config: crate::ratelimit::RateLimitConfig::new(),
        };

        let app = Router::new()
            .route("/", get(handler))
            .layer(middleware::from_fn_with_state(app_state, auth_middleware));

        // 5. Make request with expired JWT
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("Authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_jwt_unknown_kid() {
        // 1. Generate RSA key pair and JWKS with one kid
        let (private_key, jwks_json) = generate_test_keys("known-key");

        // 2. Start mock JWKS server
        let mock_server = MockServer::start().await;
        Mock::given(path("/.well-known/jwks.json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(jwks_json))
            .mount(&mock_server)
            .await;

        // 3. Create JWT with different kid (unknown to JWKS)
        let exp = (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize;
        let token = create_test_jwt(&private_key, "unknown-key", exp);

        // 4. Setup app state
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let repo_root = temp_dir.path().join("archive");
        std::fs::create_dir_all(&repo_root).unwrap();

        let db = libsql::Builder::new_local(db_path).build().await.unwrap();
        let conn = db.connect().unwrap();
        let app_config = Arc::new(AppConfig::default());
        let mm = crate::ModelManager::new_for_test(conn, repo_root, app_config);

        let jwks_url = format!("{}/.well-known/jwks.json", mock_server.uri());
        let auth_config = AuthConfig {
            mode: AuthMode::Jwt,
            bearer_token: None,
            jwks_url: Some(jwks_url.clone()),
            jwt_audience: None,
            jwt_issuer: None,
            allow_localhost: false,
        };
        let jwks_client = Some(JwksClient::new(jwks_url));

        let app_state = AppState {
            mm,
            metrics_handle: crate::setup_metrics(),
            start_time: std::time::Instant::now(),
            auth_config,
            jwks_client,
            ratelimit_config: crate::ratelimit::RateLimitConfig::new(),
        };

        let app = Router::new()
            .route("/", get(handler))
            .layer(middleware::from_fn_with_state(app_state, auth_middleware));

        // 5. Make request with JWT that has unknown kid
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("Authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_jwt_with_audience_validation_success() {
        // 1. Generate RSA key pair and JWKS
        let kid = "test-key-aud";
        let (private_key, jwks_json) = generate_test_keys(kid);

        // 2. Start mock JWKS server
        let mock_server = MockServer::start().await;
        Mock::given(path("/.well-known/jwks.json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(jwks_json))
            .mount(&mock_server)
            .await;

        // 3. Create valid JWT with audience
        let exp = (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize;
        let token = create_test_jwt_with_claims(
            &private_key,
            kid,
            exp,
            Some("test-audience".to_string()),
            None,
        );

        // 4. Setup app state with JWT auth and audience validation
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let repo_root = temp_dir.path().join("archive");
        std::fs::create_dir_all(&repo_root).unwrap();

        let db = libsql::Builder::new_local(db_path).build().await.unwrap();
        let conn = db.connect().unwrap();
        let app_config = Arc::new(AppConfig::default());
        let mm = crate::ModelManager::new_for_test(conn, repo_root, app_config);

        let jwks_url = format!("{}/.well-known/jwks.json", mock_server.uri());
        let auth_config = AuthConfig {
            mode: AuthMode::Jwt,
            bearer_token: None,
            jwks_url: Some(jwks_url.clone()),
            jwt_audience: Some("test-audience".to_string()),
            jwt_issuer: None,
            allow_localhost: false,
        };
        let jwks_client = Some(JwksClient::new(jwks_url));

        let app_state = AppState {
            mm,
            metrics_handle: crate::setup_metrics(),
            start_time: std::time::Instant::now(),
            auth_config,
            jwks_client,
            ratelimit_config: crate::ratelimit::RateLimitConfig::new(),
        };

        let app = Router::new()
            .route("/", get(handler))
            .layer(middleware::from_fn_with_state(app_state, auth_middleware));

        // 5. Make request with valid JWT
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("Authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_jwt_with_audience_validation_failure() {
        // 1. Generate RSA key pair and JWKS
        let kid = "test-key-aud-fail";
        let (private_key, jwks_json) = generate_test_keys(kid);

        // 2. Start mock JWKS server
        let mock_server = MockServer::start().await;
        Mock::given(path("/.well-known/jwks.json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(jwks_json))
            .mount(&mock_server)
            .await;

        // 3. Create JWT with wrong audience
        let exp = (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize;
        let token = create_test_jwt_with_claims(
            &private_key,
            kid,
            exp,
            Some("wrong-audience".to_string()),
            None,
        );

        // 4. Setup app state with JWT auth expecting different audience
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let repo_root = temp_dir.path().join("archive");
        std::fs::create_dir_all(&repo_root).unwrap();

        let db = libsql::Builder::new_local(db_path).build().await.unwrap();
        let conn = db.connect().unwrap();
        let app_config = Arc::new(AppConfig::default());
        let mm = crate::ModelManager::new_for_test(conn, repo_root, app_config);

        let jwks_url = format!("{}/.well-known/jwks.json", mock_server.uri());
        let auth_config = AuthConfig {
            mode: AuthMode::Jwt,
            bearer_token: None,
            jwks_url: Some(jwks_url.clone()),
            jwt_audience: Some("expected-audience".to_string()),
            jwt_issuer: None,
            allow_localhost: false,
        };
        let jwks_client = Some(JwksClient::new(jwks_url));

        let app_state = AppState {
            mm,
            metrics_handle: crate::setup_metrics(),
            start_time: std::time::Instant::now(),
            auth_config,
            jwks_client,
            ratelimit_config: crate::ratelimit::RateLimitConfig::new(),
        };

        let app = Router::new()
            .route("/", get(handler))
            .layer(middleware::from_fn_with_state(app_state, auth_middleware));

        // 5. Make request with JWT having wrong audience
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("Authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_jwt_with_issuer_validation_success() {
        // 1. Generate RSA key pair and JWKS
        let kid = "test-key-iss";
        let (private_key, jwks_json) = generate_test_keys(kid);

        // 2. Start mock JWKS server
        let mock_server = MockServer::start().await;
        Mock::given(path("/.well-known/jwks.json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(jwks_json))
            .mount(&mock_server)
            .await;

        // 3. Create valid JWT with issuer
        let exp = (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize;
        let token = create_test_jwt_with_claims(
            &private_key,
            kid,
            exp,
            None,
            Some("https://test-issuer.example.com".to_string()),
        );

        // 4. Setup app state with JWT auth and issuer validation
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let repo_root = temp_dir.path().join("archive");
        std::fs::create_dir_all(&repo_root).unwrap();

        let db = libsql::Builder::new_local(db_path).build().await.unwrap();
        let conn = db.connect().unwrap();
        let app_config = Arc::new(AppConfig::default());
        let mm = crate::ModelManager::new_for_test(conn, repo_root, app_config);

        let jwks_url = format!("{}/.well-known/jwks.json", mock_server.uri());
        let auth_config = AuthConfig {
            mode: AuthMode::Jwt,
            bearer_token: None,
            jwks_url: Some(jwks_url.clone()),
            jwt_audience: None,
            jwt_issuer: Some("https://test-issuer.example.com".to_string()),
            allow_localhost: false,
        };
        let jwks_client = Some(JwksClient::new(jwks_url));

        let app_state = AppState {
            mm,
            metrics_handle: crate::setup_metrics(),
            start_time: std::time::Instant::now(),
            auth_config,
            jwks_client,
            ratelimit_config: crate::ratelimit::RateLimitConfig::new(),
        };

        let app = Router::new()
            .route("/", get(handler))
            .layer(middleware::from_fn_with_state(app_state, auth_middleware));

        // 5. Make request with valid JWT
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("Authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_jwt_malformed_token() {
        // 1. Setup mock JWKS (won't be needed but required for JWT mode)
        let mock_server = MockServer::start().await;
        Mock::given(path("/.well-known/jwks.json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"keys":[]}"#))
            .mount(&mock_server)
            .await;

        // 2. Setup app state
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let repo_root = temp_dir.path().join("archive");
        std::fs::create_dir_all(&repo_root).unwrap();

        let db = libsql::Builder::new_local(db_path).build().await.unwrap();
        let conn = db.connect().unwrap();
        let app_config = Arc::new(AppConfig::default());
        let mm = crate::ModelManager::new_for_test(conn, repo_root, app_config);

        let jwks_url = format!("{}/.well-known/jwks.json", mock_server.uri());
        let auth_config = AuthConfig {
            mode: AuthMode::Jwt,
            bearer_token: None,
            jwks_url: Some(jwks_url.clone()),
            jwt_audience: None,
            jwt_issuer: None,
            allow_localhost: false,
        };
        let jwks_client = Some(JwksClient::new(jwks_url));

        let app_state = AppState {
            mm,
            metrics_handle: crate::setup_metrics(),
            start_time: std::time::Instant::now(),
            auth_config,
            jwks_client,
            ratelimit_config: crate::ratelimit::RateLimitConfig::new(),
        };

        let app = Router::new()
            .route("/", get(handler))
            .layer(middleware::from_fn_with_state(app_state, auth_middleware));

        // 3. Make request with malformed JWT
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("Authorization", "Bearer not.a.valid.jwt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_localhost_bypass() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let repo_root = temp_dir.path().join("archive");
        std::fs::create_dir_all(&repo_root).unwrap();

        let db = libsql::Builder::new_local(db_path).build().await.unwrap();
        let conn = db.connect().unwrap();
        let app_config = Arc::new(AppConfig::default());
        let mm = crate::ModelManager::new_for_test(conn, repo_root, app_config);

        // Auth enabled but localhost bypass also enabled
        let auth_config = AuthConfig {
            mode: AuthMode::Bearer,
            bearer_token: Some("secret123".to_string()),
            jwks_url: None,
            jwt_audience: None,
            jwt_issuer: None,
            allow_localhost: true, // Enable localhost bypass
        };
        let app_state = AppState {
            mm,
            metrics_handle: crate::setup_metrics(),
            start_time: std::time::Instant::now(),
            auth_config,
            jwks_client: None,
            ratelimit_config: crate::ratelimit::RateLimitConfig::new(),
        };

        // Create router with ConnectInfo support
        let app = Router::new()
            .route("/", get(handler))
            .layer(middleware::from_fn_with_state(app_state, auth_middleware))
            .into_make_service_with_connect_info::<std::net::SocketAddr>();

        // Start test server
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Wait briefly for server to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Make request WITHOUT auth header from localhost - should succeed due to bypass
        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/", addr))
            .send()
            .await
            .expect("Request failed");

        assert_eq!(
            response.status().as_u16(),
            200,
            "Localhost bypass should allow unauthenticated request"
        );
    }

    #[tokio::test]
    async fn test_auth_localhost_bypass_disabled() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let repo_root = temp_dir.path().join("archive");
        std::fs::create_dir_all(&repo_root).unwrap();

        let db = libsql::Builder::new_local(db_path).build().await.unwrap();
        let conn = db.connect().unwrap();
        let app_config = Arc::new(AppConfig::default());
        let mm = crate::ModelManager::new_for_test(conn, repo_root, app_config);

        // Auth enabled, localhost bypass disabled
        let auth_config = AuthConfig {
            mode: AuthMode::Bearer,
            bearer_token: Some("secret123".to_string()),
            jwks_url: None,
            jwt_audience: None,
            jwt_issuer: None,
            allow_localhost: false, // Disable localhost bypass
        };
        let app_state = AppState {
            mm,
            metrics_handle: crate::setup_metrics(),
            start_time: std::time::Instant::now(),
            auth_config,
            jwks_client: None,
            ratelimit_config: crate::ratelimit::RateLimitConfig::new(),
        };

        // Create router with ConnectInfo support
        let app = Router::new()
            .route("/", get(handler))
            .layer(middleware::from_fn_with_state(app_state, auth_middleware))
            .into_make_service_with_connect_info::<std::net::SocketAddr>();

        // Start test server
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Wait briefly for server to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Make request WITHOUT auth header from localhost - should fail since bypass is disabled
        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/", addr))
            .send()
            .await
            .expect("Request failed");

        assert_eq!(
            response.status().as_u16(),
            401,
            "With localhost bypass disabled, request should be unauthorized"
        );
    }

    #[test]
    fn test_is_localhost_ipv4() {
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};

        let localhost = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        assert!(
            super::is_localhost(&localhost),
            "127.0.0.1 should be localhost"
        );

        let external = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 8080);
        assert!(
            !super::is_localhost(&external),
            "192.168.1.1 should not be localhost"
        );
    }

    #[test]
    fn test_is_localhost_ipv6() {
        use std::net::{IpAddr, Ipv6Addr, SocketAddr};

        let localhost = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 8080);
        assert!(super::is_localhost(&localhost), "::1 should be localhost");

        let external = SocketAddr::new(
            IpAddr::V6(Ipv6Addr::new(2001, 0x0db8, 0, 0, 0, 0, 0, 1)),
            8080,
        );
        assert!(
            !super::is_localhost(&external),
            "2001:db8::1 should not be localhost"
        );
    }
}
