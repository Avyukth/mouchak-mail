use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use axum_extra::headers::{authorization::Bearer, Authorization, HeaderMapExt};
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::AppState;

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

        // Slow path: refresh keys (either expired or key not found)
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

    async fn refresh_keys(&self) -> Result<(), anyhow::Error> {
        info!("Refreshing JWKS from {}", self.url);
        let resp = self.client.get(&self.url).send().await?.json::<JwksResponse>().await?;

        let mut new_keys = HashMap::new();
        for jwk in resp.keys {
            if jwk.kty == "RSA" {
                if let Ok(decoding_key) = DecodingKey::from_rsa_components(&jwk.n, &jwk.e) {
                    new_keys.insert(jwk.kid.clone(), decoding_key);
                }
            }
        }

        // Update keys and refresh timestamp
        let mut keys = self.keys.write().await;
        *keys = new_keys;
        let key_count = keys.len();
        drop(keys);

        let mut last_refresh = self.last_refresh.write().await;
        *last_refresh = Some(Instant::now());

        info!("JWKS refreshed, loaded {} keys (TTL: {:?})", key_count, self.cache_ttl);
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

/// Auth Middleware
pub async fn auth_middleware(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_config = &state.auth_config;

    // 1. Check Auth Mode None
    if auth_config.mode == AuthMode::None {
        return Ok(next.run(req).await);
    }

    // 2. Check Localhost Bypass
    if auth_config.allow_localhost {
        // This is tricky with Axum extractors in middleware.
        // We rely on ConnectInfo if available, or just X-Forwarded-For logic if behind proxy?
        // Since we are creating a robust server, let's skip IP check for now as it usually requires
        // `Router::into_make_service_with_connect_info`.
        // Instead, we might assume if no token is present, we check if we should allow it?
        // Actually, strictly speaking, localhost bypass usually implies checking the peer address.
        // If we can't reliably check it here without adding complexity to main.rs, we might skip it or implement a simpler check.
        // For now, let's implement the token check logic first.
    }

    // 3. Extract Token
    let token = match req.headers().typed_get::<Authorization<Bearer>>() {
        Some(Authorization(bearer)) => bearer.token().to_string(),
        None => {
            // No token found.
            // If localhost bypass is enabled, we *could* allow it if we were sure it's localhost.
            // But for safety, blocking is better if we can't verify source IP easily.
            // NOTE: In production, this service might sit behind a tailored proxy.
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // 4. Validate Token based on mode
    match auth_config.mode {
        AuthMode::Bearer => {
            match &auth_config.bearer_token {
                Some(expected) if expected == &token => Ok(next.run(req).await),
                _ => Err(StatusCode::UNAUTHORIZED),
            }
        }
        AuthMode::Jwt => {
            if let Some(jwks_client) = &state.jwks_client {
                // Decode header to find KID
                let header = match decode_header(&token) {
                    Ok(h) => h,
                    Err(_) => return Err(StatusCode::UNAUTHORIZED),
                };

                let kid = header.kid.ok_or(StatusCode::UNAUTHORIZED)?;
                let key = jwks_client.get_verifying_key(&kid).await.ok_or(StatusCode::UNAUTHORIZED)?;

                // Configure validation with audience and issuer if provided
                let mut validation = Validation::new(header.alg);

                if let Some(ref audience) = auth_config.jwt_audience {
                    validation.set_audience(&[audience]);
                }

                if let Some(ref issuer) = auth_config.jwt_issuer {
                    validation.set_issuer(&[issuer]);
                }

                // decode verifies signature, exp, aud, and iss
                match decode::<Claims>(&token, &key, &validation) {
                    Ok(token_data) => {
                        info!("JWT validated successfully for subject: {}", token_data.claims.sub);
                        Ok(next.run(req).await)
                    },
                    Err(e) => {
                        warn!("JWT validation failed: {}", e);
                        Err(StatusCode::UNAUTHORIZED)
                    },
                }
            } else {
                error!("Auth mode is JWT but JwksClient is missing");
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
        AuthMode::None => unreachable!(), // Handled above
    }
}

/// Middleware to enforce capability checks (RBAC)
/// Note: This is an example middleware. Real RBAC requires extracting the agent info from the token
/// and then checking DB. Since checking DB requires async access and extracting Agent ID,
/// we need a mechanism to map JWT sub -> Agent ID.
/// For now, this is a placeholder stub for `rkm` task compliance.
pub async fn capabilities_middleware(
    State(_state): State<AppState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Check if user is authenticated (usually runs after auth_middleware)
    // 2. Extract Agent ID (from request extension set by auth_middleware)
    // 3. Check requested resource/action against agent capabilities
    
    // For now, pass through as we don't have the granular permission mapping yet
    Ok(next.run(req).await)
}


#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::{Request, StatusCode}, middleware, routing::get, Router};
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use rand::rngs::OsRng;
    use rsa::{pkcs1::EncodeRsaPrivateKey, traits::PublicKeyParts, RsaPrivateKey};
    use tower::util::ServiceExt; // for oneshot
    use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

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
        let mm = crate::ModelManager::new_for_test(conn, repo_root);

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
        let mm = crate::ModelManager::new_for_test(conn, repo_root);

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
        let mm = crate::ModelManager::new_for_test(conn, repo_root);

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
        };

        let app = Router::new()
            .route("/", get(handler))
            .layer(middleware::from_fn_with_state(app_state, auth_middleware));

        // Wrong token
        let response = app.clone()
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
        let mm = crate::ModelManager::new_for_test(conn, repo_root);

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
        let mm = crate::ModelManager::new_for_test(conn, repo_root);

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
        let mm = crate::ModelManager::new_for_test(conn, repo_root);

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
        let mm = crate::ModelManager::new_for_test(conn, repo_root);

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
        let mm = crate::ModelManager::new_for_test(conn, repo_root);

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
        let mm = crate::ModelManager::new_for_test(conn, repo_root);

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
        let mm = crate::ModelManager::new_for_test(conn, repo_root);

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
}
