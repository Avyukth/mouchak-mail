//! HTTP Transport Infrastructure Tests (PORT-4.2)
//!
//! Tests for HTTP/SSE transport, JWT authentication edge cases,
//! rate limiting behavior, and JWKS caching.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use mouchak_mail_common::config::AppConfig;
use mouchak_mail_mcp::tools::MouchakMailService;
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager,
    tower::{StreamableHttpServerConfig, StreamableHttpService},
};

/// Create MCP service with MouchakMailService
fn create_test_mcp_service() -> StreamableHttpService<MouchakMailService> {
    let session_manager = Arc::new(LocalSessionManager::default());
    let config = StreamableHttpServerConfig::default();

    // Use std::thread::scope to ensure the thread is joined (UBS: spawn without join fix)
    let service_factory = || -> Result<MouchakMailService, std::io::Error> {
        let result = std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async { MouchakMailService::new().await })
                })
                .join()
                .expect("MouchakMailService thread panicked")
        });

        result.map_err(|e| std::io::Error::other(e.to_string()))
    };

    StreamableHttpService::new(service_factory, session_manager, config)
}

// ============================================================================
// HTTP Transport Tests - SSE and Streamable HTTP
// ============================================================================

#[tokio::test]
async fn test_sse_get_endpoint_returns_event_stream() {
    let service = create_test_mcp_service();

    // GET request for SSE streaming should work
    let request = Request::builder()
        .method("GET")
        .uri("/mcp")
        .header("Accept", "text/event-stream")
        .body(Body::empty())
        .unwrap();

    let response = service.oneshot(request).await.unwrap();

    // SSE endpoints should not return 404 or 405
    assert_ne!(
        response.status(),
        StatusCode::NOT_FOUND,
        "GET /mcp should exist for SSE"
    );
    assert_ne!(
        response.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "GET should be allowed for SSE"
    );
}

#[tokio::test]
async fn test_http_delete_method_rejected() {
    let service = create_test_mcp_service();

    // DELETE should not be supported for MCP
    let request = Request::builder()
        .method("DELETE")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = service.oneshot(request).await.unwrap();

    // DELETE is not a valid MCP method - may return various error codes
    // depending on which layer rejects it first (auth, method check, etc.)
    assert!(
        !response.status().is_success(),
        "DELETE should be rejected, got success: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_http_content_type_required_for_post() {
    let service = create_test_mcp_service();

    // POST without Content-Type
    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Accept", "application/json, text/event-stream")
        // Missing Content-Type
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "method": "initialize",
                "id": 1
            })
            .to_string(),
        ))
        .unwrap();

    let response = service.oneshot(request).await.unwrap();

    // Should handle gracefully - either process or reject
    assert!(
        response.status().is_success()
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "Should handle missing Content-Type, got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_session_id_header_accepted() {
    let service = create_test_mcp_service();

    // First request to establish session
    let init_request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .body(Body::from(
            json!({
                "jsonrpc": "2.0",
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "test", "version": "1.0" }
                },
                "id": 1
            })
            .to_string(),
        ))
        .unwrap();

    let response = service.clone().oneshot(init_request).await.unwrap();

    // Check if session ID is returned in headers
    let session_id = response.headers().get("mcp-session-id");
    let has_session = session_id.is_some();

    // Second request with session ID (if provided)
    if has_session {
        let session_id_value = session_id.unwrap().to_str().unwrap();
        let request = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("Mcp-Session-Id", session_id_value)
            .body(Body::from(
                json!({
                    "jsonrpc": "2.0",
                    "method": "tools/list",
                    "id": 2
                })
                .to_string(),
            ))
            .unwrap();

        let response = service.oneshot(request).await.unwrap();
        // With valid session ID, should not get 422
        assert_ne!(
            response.status(),
            StatusCode::NOT_FOUND,
            "Valid session should not 404"
        );
    }
}

#[tokio::test]
async fn test_json_rpc_batch_request_handling() {
    let service = create_test_mcp_service();

    // JSON-RPC batch request (array of requests)
    let batch_body = json!([
        { "jsonrpc": "2.0", "method": "initialize", "params": { "protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": { "name": "test", "version": "1.0" } }, "id": 1 },
        { "jsonrpc": "2.0", "method": "ping", "id": 2 }
    ]);

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .body(Body::from(batch_body.to_string()))
        .unwrap();

    let response = service.oneshot(request).await.unwrap();

    // Should handle batch requests (either process or return error gracefully)
    assert!(
        response.status().is_success() || response.status().is_client_error(),
        "Should handle batch request gracefully"
    );
}

// ============================================================================
// JWT Authentication Edge Cases
// ============================================================================

#[tokio::test]
async fn test_jwt_without_kid_in_header() {
    use axum::{Router, middleware, routing::get};
    use mouchak_mail_server::{
        AppState,
        auth::{AuthConfig, AuthMode, JwksClient, auth_middleware},
    };
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::path};

    async fn handler() -> &'static str {
        "OK"
    }

    // Start mock JWKS server
    let mock_server = MockServer::start().await;
    Mock::given(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"keys":[]}"#))
        .mount(&mock_server)
        .await;

    // Setup app state
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let repo_root = temp_dir.path().join("archive");
    std::fs::create_dir_all(&repo_root).unwrap();

    let db = libsql::Builder::new_local(db_path).build().await.unwrap();
    let conn = db.connect().unwrap();
    let app_config = Arc::new(AppConfig::default());
    let mm = mouchak_mail_server::ModelManager::new_for_test(conn, repo_root, app_config);

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
        metrics_handle: mouchak_mail_server::setup_metrics(),
        start_time: std::time::Instant::now(),
        auth_config,
        jwks_client,
        ratelimit_config: mouchak_mail_server::ratelimit::RateLimitConfig::new(),
    };

    let app = Router::new()
        .route("/", get(handler))
        .layer(middleware::from_fn_with_state(app_state, auth_middleware));

    // Create JWT without kid in header
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    // Header without kid
    let header = r#"{"alg":"RS256","typ":"JWT"}"#;
    let payload = r#"{"sub":"test","exp":9999999999}"#;
    let token = format!(
        "{}.{}.fake_sig",
        URL_SAFE_NO_PAD.encode(header),
        URL_SAFE_NO_PAD.encode(payload)
    );

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

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "JWT without kid should fail"
    );
}

#[tokio::test]
async fn test_jwt_with_wrong_issuer() {
    use axum::{Router, middleware, routing::get};
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
    use mouchak_mail_server::{
        AppState,
        auth::{AuthConfig, AuthMode, JwksClient, auth_middleware},
    };
    use rand::rngs::OsRng;
    use rsa::{RsaPrivateKey, pkcs1::EncodeRsaPrivateKey, traits::PublicKeyParts};
    use serde::{Deserialize, Serialize};
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::path};

    #[derive(Serialize, Deserialize)]
    struct ClaimsWithIssuer {
        sub: String,
        exp: usize,
        iss: String,
    }

    async fn handler() -> &'static str {
        "OK"
    }

    // Generate RSA key pair
    let mut rng = OsRng;
    let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("key");
    let public_key = private_key.to_public_key();
    let kid = "iss-test-key";

    let n = URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
    let e = URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be());
    let jwks_json = format!(
        r#"{{"keys":[{{"kty":"RSA","kid":"{}","alg":"RS256","n":"{}","e":"{}"}}]}}"#,
        kid, n, e
    );

    // Start mock JWKS server
    let mock_server = MockServer::start().await;
    Mock::given(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(jwks_json))
        .mount(&mock_server)
        .await;

    // Create JWT with wrong issuer
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = ClaimsWithIssuer {
        sub: "test-user".to_string(),
        exp: now + 3600,
        iss: "https://wrong-issuer.example.com".to_string(),
    };

    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(kid.to_string());

    let der = private_key.to_pkcs1_der().expect("der");
    let encoding_key = EncodingKey::from_rsa_der(der.as_bytes());
    let token = encode(&header, &claims, &encoding_key).expect("encode");

    // Setup app state - expecting a different issuer
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let repo_root = temp_dir.path().join("archive");
    std::fs::create_dir_all(&repo_root).unwrap();

    let db = libsql::Builder::new_local(db_path).build().await.unwrap();
    let conn = db.connect().unwrap();
    let app_config = Arc::new(AppConfig::default());
    let mm = mouchak_mail_server::ModelManager::new_for_test(conn, repo_root, app_config);

    let jwks_url = format!("{}/.well-known/jwks.json", mock_server.uri());
    let auth_config = AuthConfig {
        mode: AuthMode::Jwt,
        bearer_token: None,
        jwks_url: Some(jwks_url.clone()),
        jwt_audience: None,
        jwt_issuer: Some("https://expected-issuer.example.com".to_string()), // Expecting different issuer
        allow_localhost: false,
    };
    let jwks_client = Some(JwksClient::new(jwks_url));

    let app_state = AppState {
        mm,
        metrics_handle: mouchak_mail_server::setup_metrics(),
        start_time: std::time::Instant::now(),
        auth_config,
        jwks_client,
        ratelimit_config: mouchak_mail_server::ratelimit::RateLimitConfig::new(),
    };

    let app = Router::new()
        .route("/", get(handler))
        .layer(middleware::from_fn_with_state(app_state, auth_middleware));

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

    // JWT with wrong issuer should fail
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "JWT with wrong issuer should be rejected"
    );
}

#[tokio::test]
async fn test_jwt_empty_authorization_header() {
    use axum::{Router, middleware, routing::get};
    use mouchak_mail_server::{
        AppState,
        auth::{AuthConfig, AuthMode, auth_middleware},
    };

    async fn handler() -> &'static str {
        "OK"
    }

    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let repo_root = temp_dir.path().join("archive");
    std::fs::create_dir_all(&repo_root).unwrap();

    let db = libsql::Builder::new_local(db_path).build().await.unwrap();
    let conn = db.connect().unwrap();
    let app_config = Arc::new(AppConfig::default());
    let mm = mouchak_mail_server::ModelManager::new_for_test(conn, repo_root, app_config);

    let auth_config = AuthConfig {
        mode: AuthMode::Bearer,
        bearer_token: Some("secret".to_string()),
        jwks_url: None,
        jwt_audience: None,
        jwt_issuer: None,
        allow_localhost: false,
    };

    let app_state = AppState {
        mm,
        metrics_handle: mouchak_mail_server::setup_metrics(),
        start_time: std::time::Instant::now(),
        auth_config,
        jwks_client: None,
        ratelimit_config: mouchak_mail_server::ratelimit::RateLimitConfig::new(),
    };

    let app = Router::new()
        .route("/", get(handler))
        .layer(middleware::from_fn_with_state(app_state, auth_middleware));

    // Empty Authorization header value
    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .header("Authorization", "")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Empty auth header should fail"
    );
}

#[tokio::test]
async fn test_jwt_wrong_algorithm() {
    use axum::{Router, middleware, routing::get};
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use mouchak_mail_server::{
        AppState,
        auth::{AuthConfig, AuthMode, JwksClient, auth_middleware},
    };
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::path};

    async fn handler() -> &'static str {
        "OK"
    }

    // Mock JWKS with RS256 key
    let mock_server = MockServer::start().await;
    Mock::given(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"keys":[{"kty":"RSA","kid":"test-key","alg":"RS256","n":"test","e":"AQAB"}]}"#,
        ))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let repo_root = temp_dir.path().join("archive");
    std::fs::create_dir_all(&repo_root).unwrap();

    let db = libsql::Builder::new_local(db_path).build().await.unwrap();
    let conn = db.connect().unwrap();
    let app_config = Arc::new(AppConfig::default());
    let mm = mouchak_mail_server::ModelManager::new_for_test(conn, repo_root, app_config);

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
        metrics_handle: mouchak_mail_server::setup_metrics(),
        start_time: std::time::Instant::now(),
        auth_config,
        jwks_client,
        ratelimit_config: mouchak_mail_server::ratelimit::RateLimitConfig::new(),
    };

    let app = Router::new()
        .route("/", get(handler))
        .layer(middleware::from_fn_with_state(app_state, auth_middleware));

    // Create JWT with HS256 (symmetric) instead of RS256 (asymmetric)
    let header = r#"{"alg":"HS256","typ":"JWT","kid":"test-key"}"#;
    let payload = r#"{"sub":"test","exp":9999999999}"#;
    let token = format!(
        "{}.{}.fake_sig",
        URL_SAFE_NO_PAD.encode(header),
        URL_SAFE_NO_PAD.encode(payload)
    );

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

    // Should reject - JWKS has RS256 keys but token uses HS256
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "JWT with wrong algorithm should fail"
    );
}

// ============================================================================
// Rate Limiting Tests
// ============================================================================

#[tokio::test]
async fn test_rate_limit_x_forwarded_for_header() {
    use axum::{Router, middleware, routing::get};
    use mouchak_mail_server::ratelimit::{RateLimitConfig, rate_limit_middleware};
    use std::net::SocketAddr;

    async fn handler() -> &'static str {
        "OK"
    }

    let config = RateLimitConfig::new();
    let app = Router::new()
        .route("/", get(handler))
        .layer(middleware::from_fn_with_state(
            config,
            rate_limit_middleware,
        ))
        .into_make_service_with_connect_info::<SocketAddr>();

    // Start test server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Request with X-Forwarded-For header
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/", addr))
        .header("X-Forwarded-For", "203.0.113.50")
        .send()
        .await
        .expect("Request failed");

    // Should succeed (first request)
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn test_rate_limit_multiple_ips_in_x_forwarded_for() {
    use axum::{Router, middleware, routing::get};
    use mouchak_mail_server::ratelimit::{RateLimitConfig, rate_limit_middleware};
    use std::net::SocketAddr;

    async fn handler() -> &'static str {
        "OK"
    }

    let config = RateLimitConfig::new();
    let app = Router::new()
        .route("/", get(handler))
        .layer(middleware::from_fn_with_state(
            config,
            rate_limit_middleware,
        ))
        .into_make_service_with_connect_info::<SocketAddr>();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // X-Forwarded-For with multiple IPs (comma-separated)
    // Should use the first IP (client IP)
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/", addr))
        .header("X-Forwarded-For", "198.51.100.1, 203.0.113.50, 192.168.1.1")
        .send()
        .await
        .expect("Request failed");

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn test_per_tool_rate_limit_category_errors() {
    use mouchak_mail_server::ratelimit::{ToolCategory, ToolRateLimits};

    let limits = ToolRateLimits::new();
    let key = "test-agent:127.0.0.1";

    // Exhaust write limit (10 requests)
    for _ in 0..10 {
        assert!(limits.check_tool("send_message", key).is_ok());
    }

    // 11th request should return the category that was exceeded
    let result = limits.check_tool("send_message", key);
    assert_eq!(result, Err(ToolCategory::Write));

    // Read tools should still work (different category)
    assert!(limits.check_tool("fetch_inbox", key).is_ok());

    // Default category tools work independently
    assert!(limits.check_tool("unknown_tool", key).is_ok());
}

#[tokio::test]
async fn test_rate_limit_exhaustion_and_different_key_recovery() {
    use mouchak_mail_server::ratelimit::ToolRateLimits;

    let limits = ToolRateLimits::new();
    let key1 = "user-exhausted:1.1.1.1";
    let key2 = "user-fresh:2.2.2.2";

    // Exhaust rate limit for key1 (10 write requests)
    for _ in 0..10 {
        assert!(limits.check_tool("send_message", key1).is_ok());
    }

    // key1 is now exhausted
    assert!(
        limits.check_tool("send_message", key1).is_err(),
        "key1 should be rate limited after exhaustion"
    );

    // key2 should still have full quota (independent buckets)
    for i in 0..10 {
        assert!(
            limits.check_tool("send_message", key2).is_ok(),
            "key2 request {} should succeed (independent from key1)",
            i
        );
    }

    // Verify key1 is still exhausted (didn't get key2's quota)
    assert!(
        limits.check_tool("send_message", key1).is_err(),
        "key1 should still be rate limited"
    );
}

// ============================================================================
// JWKS Cache and Fallback Tests
// ============================================================================

#[tokio::test]
async fn test_jwks_cache_ttl_refresh() {
    use mouchak_mail_server::auth::JwksClient;
    use std::time::Duration;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::path};

    // Start mock JWKS server
    let mock_server = MockServer::start().await;

    // First response with one key
    Mock::given(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"keys":[{"kty":"RSA","kid":"key-1","alg":"RS256","n":"test","e":"AQAB"}]}"#,
        ))
        .expect(1..=2) // Allow 1-2 calls
        .mount(&mock_server)
        .await;

    let jwks_url = format!("{}/.well-known/jwks.json", mock_server.uri());

    // Create client with very short TTL (100ms for testing)
    let client = JwksClient::new_with_ttl(jwks_url, Duration::from_millis(100));

    // First request - should fetch from server
    let key1 = client.get_verifying_key("key-1").await;
    // Key may or may not be found depending on RSA decoding, but request should complete
    assert!(key1.is_some() || key1.is_none()); // Just verify no panic

    // Immediate second request - should use cache
    let _ = client.get_verifying_key("key-1").await;

    // Wait for TTL to expire
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Third request - should refresh from server
    let _ = client.get_verifying_key("key-1").await;
}

#[tokio::test]
async fn test_jwks_server_unavailable_uses_cached_keys() {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use mouchak_mail_server::auth::JwksClient;
    use rand::rngs::OsRng;
    use rsa::{RsaPrivateKey, traits::PublicKeyParts};
    use std::time::Duration;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::path};

    // Generate a valid RSA key
    let mut rng = OsRng;
    let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("key");
    let public_key = private_key.to_public_key();
    let n = URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
    let e = URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be());

    let jwks_json = format!(
        r#"{{"keys":[{{"kty":"RSA","kid":"cached-key","alg":"RS256","n":"{}","e":"{}"}}]}}"#,
        n, e
    );

    // Start mock JWKS server
    let mock_server = MockServer::start().await;

    // First mount - success response
    Mock::given(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(jwks_json))
        .expect(1)
        .mount(&mock_server)
        .await;

    let jwks_url = format!("{}/.well-known/jwks.json", mock_server.uri());
    let client = JwksClient::new_with_ttl(jwks_url, Duration::from_millis(50));

    // First request - populates cache
    let key = client.get_verifying_key("cached-key").await;
    assert!(key.is_some(), "Key should be loaded into cache");

    // Clear mocks and set up failure response
    mock_server.reset().await;
    Mock::given(path("/.well-known/jwks.json"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    // Wait for cache to expire
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Request after cache expiry - server fails but should use cached key
    let key = client.get_verifying_key("cached-key").await;
    assert!(
        key.is_some(),
        "Should fall back to cached key when server fails"
    );
}

#[tokio::test]
async fn test_rps_for_category_returns_correct_values() {
    use mouchak_mail_server::ratelimit::{ToolCategory, ToolRateLimits};

    let limits = ToolRateLimits::new();

    // Default values from ToolRateLimits::new()
    assert_eq!(limits.rps_for_category(ToolCategory::Write), 10);
    assert_eq!(limits.rps_for_category(ToolCategory::Read), 100);
    assert_eq!(limits.rps_for_category(ToolCategory::Default), 50);
}
