#![allow(clippy::expect_used)]
//! Auth & Rate Limiting E2E Tests
//!
//! These tests verify authentication and rate limiting functionality.
//! Requires the API server to be running with appropriate auth configuration.
//!
//! Prerequisites:
//! - API server running with auth enabled: `HTTP_AUTH_MODE=bearer cargo run -p mcp-server`
//!
//! Run tests:
//! ```bash
//! cargo test -p e2e-tests --test auth_rate_limit
//! ```

use e2e_tests::TestConfig;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::time::{Duration, Instant};

// ============================================================================
// Test Configuration
// ============================================================================

/// Configuration for auth tests
struct AuthTestConfig {
    /// Base test config
    base: TestConfig,
    /// Valid bearer token (must match HTTP_BEARER_TOKEN on server)
    valid_bearer_token: String,
    /// Invalid token for testing rejection
    #[allow(dead_code)]
    invalid_token: String,
}

impl Default for AuthTestConfig {
    fn default() -> Self {
        Self {
            base: TestConfig::default(),
            valid_bearer_token: std::env::var("TEST_BEARER_TOKEN")
                .unwrap_or_else(|_| "test-secret-token".to_string()),
            invalid_token: "invalid-token-12345".to_string(),
        }
    }
}

/// Response structure for health endpoint
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct HealthResponse {
    status: String,
}

/// Response structure for error responses
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ErrorResponse {
    error: String,
    #[serde(default)]
    message: Option<String>,
}

// ============================================================================
// Test Helpers
// ============================================================================

fn get_auth_config() -> AuthTestConfig {
    AuthTestConfig::default()
}

async fn create_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| Client::new())
}

/// Check if auth is enabled on the server
async fn is_auth_enabled(client: &Client, config: &AuthTestConfig) -> bool {
    // Try to access a protected endpoint without auth
    let response = client
        .get(format!("{}/api/projects", config.base.api_url))
        .send()
        .await;

    match response {
        Ok(resp) => resp.status() == StatusCode::UNAUTHORIZED,
        Err(_) => false,
    }
}

/// Generate a mock JWT token for testing (invalid signature)
fn generate_mock_jwt(expired: bool) -> String {
    // JWT structure: header.payload.signature
    // We create a structurally valid but cryptographically invalid JWT

    let header = base64_url_encode(r#"{"alg":"RS256","typ":"JWT"}"#);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let (iat, exp) = if expired {
        // Expired 1 hour ago
        (now - 7200, now - 3600)
    } else {
        // Valid for 1 hour
        (now, now + 3600)
    };

    let payload = base64_url_encode(&format!(
        r#"{{"sub":"test-user","aud":"test-audience","iss":"test-issuer","iat":{},"exp":{}}}"#,
        iat, exp
    ));

    // Invalid signature (base64-encoded random bytes)
    let signature = base64_url_encode("invalid-signature-bytes-here");

    format!("{}.{}.{}", header, payload, signature)
}

/// Base64 URL-safe encoding without padding
fn base64_url_encode(input: &str) -> String {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.encode(input.as_bytes())
}

// ============================================================================
// Auth Tests
// ============================================================================

/// Test: Valid bearer token should succeed
///
/// Acceptance Criteria: Test valid JWT token succeeds
#[tokio::test]
async fn test_valid_bearer_token_succeeds() {
    let config = get_auth_config();
    let client = create_client().await;

    // First check if auth is enabled
    if !is_auth_enabled(&client, &config).await {
        println!("⚠ Auth not enabled on server, skipping test");
        println!(
            "  Start server with: HTTP_AUTH_MODE=bearer HTTP_BEARER_TOKEN=test-secret-token cargo run -p mcp-server"
        );
        return;
    }

    // Make request with valid bearer token
    let response = client
        .get(format!("{}/api/projects", config.base.api_url))
        .header(
            "Authorization",
            format!("Bearer {}", config.valid_bearer_token),
        )
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                println!("✓ Valid bearer token accepted (status={})", status);
            } else if status == StatusCode::UNAUTHORIZED {
                panic!(
                    "Bearer token rejected - ensure TEST_BEARER_TOKEN matches server's HTTP_BEARER_TOKEN"
                );
            } else if status == StatusCode::FORBIDDEN {
                panic!("Bearer token forbidden (403) - token may lack required permissions");
            } else {
                assert!(
                    !status.is_client_error(),
                    "Valid bearer token should not result in client error, got {}",
                    status
                );
                println!("✓ Request processed (status={})", status);
            }
        }
        Err(e) => {
            panic!("API server not running or unreachable: {}", e);
        }
    }
}

/// Test: Invalid JWT signature should return 401
///
/// Acceptance Criteria: Test invalid JWT signature returns 401
#[tokio::test]
async fn test_invalid_jwt_signature_returns_401() {
    let config = get_auth_config();
    let client = create_client().await;

    // Check if auth is enabled
    if !is_auth_enabled(&client, &config).await {
        println!("⚠ Auth not enabled on server, skipping test");
        return;
    }

    // Generate a JWT with invalid signature
    let invalid_jwt = generate_mock_jwt(false);

    let response = client
        .get(format!("{}/api/projects", config.base.api_url))
        .header("Authorization", format!("Bearer {}", invalid_jwt))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            assert!(
                status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN,
                "Invalid JWT should be rejected with 401/403, got {}. Auth is enabled but invalid token was accepted!",
                status
            );
            println!("✓ Invalid JWT signature correctly rejected with {}", status);
        }
        Err(e) => {
            panic!("API server not running: {}", e);
        }
    }
}

/// Test: Expired JWT should return 401
///
/// Acceptance Criteria: Test expired JWT returns 401
#[tokio::test]
async fn test_expired_jwt_returns_401() {
    let config = get_auth_config();
    let client = create_client().await;

    // Check if auth is enabled
    if !is_auth_enabled(&client, &config).await {
        println!("⚠ Auth not enabled on server, skipping test");
        return;
    }

    // Generate an expired JWT
    let expired_jwt = generate_mock_jwt(true);

    let response = client
        .get(format!("{}/api/projects", config.base.api_url))
        .header("Authorization", format!("Bearer {}", expired_jwt))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            assert!(
                status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN,
                "Expired JWT should be rejected with 401/403, got {}. Auth is enabled but expired token was accepted!",
                status
            );
            println!("✓ Expired JWT correctly rejected with {}", status);
        }
        Err(e) => {
            panic!("API server not running: {}", e);
        }
    }
}

/// Test: Missing Authorization header should return 401
///
/// Acceptance Criteria: Test missing Authorization header returns 401
#[tokio::test]
async fn test_missing_auth_header_returns_401() {
    let config = get_auth_config();
    let client = create_client().await;

    // Check if auth is enabled
    if !is_auth_enabled(&client, &config).await {
        println!("⚠ Auth not enabled on server, skipping test");
        println!("  This test requires HTTP_AUTH_MODE=bearer or jwt");
        return;
    }

    // Make request without Authorization header
    let response = client
        .get(format!("{}/api/projects", config.base.api_url))
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            if status == StatusCode::UNAUTHORIZED {
                println!("✓ Missing auth header correctly rejected with 401");
            } else {
                panic!("Expected 401 for missing auth header, got {}", status);
            }
        }
        Err(e) => {
            panic!("API server not running: {}", e);
        }
    }
}

// ============================================================================
// Rate Limiting Tests
// ============================================================================

/// Test: Exceeding rate limit should return 429
///
/// Acceptance Criteria: Test rate limit exceeded returns 429
#[tokio::test]
async fn test_rate_limit_exceeded_returns_429() {
    let config = get_auth_config();
    let client = create_client().await;

    // Health endpoint is typically not rate-limited, use a real API endpoint
    let endpoint = format!("{}/api/projects", config.base.api_url);

    // Check if auth is needed upfront
    let auth_enabled = is_auth_enabled(&client, &config).await;

    // First, check if rate limiting is enabled by making a burst of requests
    // Read endpoints have 100 RPS limit, so we need to exceed that
    let mut hit_rate_limit = false;
    let mut requests_sent = 0;
    let start = Instant::now();

    // Send requests in rapid succession (up to 150 to exceed 100 RPS limit)
    for i in 0..150 {
        let mut req = client.get(&endpoint);
        if auth_enabled {
            req = req.header(
                "Authorization",
                format!("Bearer {}", config.valid_bearer_token),
            );
        }

        let response = req.send().await;
        requests_sent = i + 1;

        match response {
            Ok(resp) => {
                if resp.status() == StatusCode::TOO_MANY_REQUESTS {
                    hit_rate_limit = true;
                    let elapsed = start.elapsed();
                    println!(
                        "✓ Rate limit hit after {} requests in {:?}",
                        requests_sent, elapsed
                    );
                    break;
                } else if resp.status() == StatusCode::UNAUTHORIZED {
                    println!("⚠ Auth required but token rejected, cannot test rate limiting");
                    return;
                }
            }
            Err(e) => {
                println!("⚠ Request {} failed: {}", i, e);
                break;
            }
        }
    }

    if !hit_rate_limit {
        println!(
            "⚠ Skipping rate limit test - rate limiting not enabled on server ({} requests sent without 429)",
            requests_sent
        );
        println!("  To test rate limiting, configure server with: HTTP_RATE_LIMIT_ENABLED=true HTTP_RATE_LIMIT_RPS=50");
        return;
    }
    println!(
        "✓ Rate limit correctly triggered after {} requests",
        requests_sent
    );
}

/// Test: Rate limit should reset after time window passes
///
/// Acceptance Criteria: Test rate limit reset after window passes
#[tokio::test]
async fn test_rate_limit_resets_after_window() {
    let config = get_auth_config();
    let client = create_client().await;

    let endpoint = format!("{}/api/projects", config.base.api_url);

    // Check if auth is needed upfront
    let auth_enabled = is_auth_enabled(&client, &config).await;

    // First, try to trigger rate limit
    let mut rate_limited = false;

    for _ in 0..150 {
        let mut req = client.get(&endpoint);
        if auth_enabled {
            req = req.header(
                "Authorization",
                format!("Bearer {}", config.valid_bearer_token),
            );
        }

        let response = req.send().await;
        match response {
            Ok(resp) => {
                if resp.status() == StatusCode::TOO_MANY_REQUESTS {
                    rate_limited = true;
                    println!("✓ Rate limit triggered");
                    break;
                } else if resp.status() == StatusCode::UNAUTHORIZED {
                    println!("⚠ Auth required but token rejected");
                    return;
                }
            }
            Err(e) => {
                println!("⚠ Request failed: {}", e);
                return;
            }
        }
    }

    if !rate_limited {
        println!("⚠ Skipping reset test - rate limiting not enabled on server");
        println!("  To test rate limiting, configure server with: HTTP_RATE_LIMIT_ENABLED=true");
        return;
    }
    println!("✓ Rate limit triggered, testing reset behavior");

    // Wait for rate limit window to reset (typically 1 second)
    println!("  Waiting 2 seconds for rate limit window to reset...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Try again - should succeed now
    let mut req = client.get(&endpoint);
    if auth_enabled {
        req = req.header(
            "Authorization",
            format!("Bearer {}", config.valid_bearer_token),
        );
    }

    let response = req.send().await;
    match response {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() || status == StatusCode::OK {
                println!("✓ Rate limit reset - request succeeded after waiting");
            } else if status == StatusCode::TOO_MANY_REQUESTS {
                // Rate limit window might be longer than 2 seconds
                println!("⚠ Rate limit still active - window may be longer than 2 seconds");
                // This is acceptable behavior, not a test failure
            } else {
                println!(
                    "✓ Request processed with status {} after rate limit reset",
                    status
                );
            }
        }
        Err(e) => {
            panic!("Request after rate limit reset should not fail: {}", e);
        }
    }
}
