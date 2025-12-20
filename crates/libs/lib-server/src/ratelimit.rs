use axum::extract::ConnectInfo;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use governor::{Quota, RateLimiter, clock::DefaultClock, state::keyed::DashMapStateStore};
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use tracing::{debug, warn};

/// Rate limiter keyed by composite identity string.
///
/// Key format: `{jwt_subject}:{ip}` for authenticated requests,
/// or just `{ip}` for unauthenticated requests.
///
/// NIST Control: SC-5 (DoS Protection)
type KeyedRateLimiter = RateLimiter<String, DashMapStateStore<String>, DefaultClock>;

#[derive(Clone)]
pub struct RateLimitConfig {
    pub limiter: Arc<KeyedRateLimiter>,
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimitConfig {
    #[allow(clippy::expect_used)] // NonZeroU32 from parsed u32 with fallback defaults; always valid
    pub fn new() -> Self {
        let enabled =
            std::env::var("RATE_LIMIT_ENABLED").unwrap_or_else(|_| "true".into()) == "true";

        // Defaults sized for 100 concurrent agents:
        // - 1000 RPS allows 10 requests/second per agent
        // - 2000 burst handles initial connection spikes
        let rps = std::env::var("RATE_LIMIT_RPS")
            .unwrap_or_else(|_| "1000".into())
            .parse::<u32>()
            .unwrap_or(1000);

        let burst = std::env::var("RATE_LIMIT_BURST")
            .unwrap_or_else(|_| "2000".into())
            .parse::<u32>()
            .unwrap_or(2000);

        let quota = Quota::per_second(NonZeroU32::new(rps).expect("RPS should be non-zero"))
            .allow_burst(NonZeroU32::new(burst).expect("Burst should be non-zero"));

        let limiter = Arc::new(RateLimiter::keyed(quota));

        tracing::info!(
            "Rate Limiting: enabled={}, rps={}, burst={}",
            enabled,
            rps,
            burst
        );

        Self { limiter, enabled }
    }
}

// ============================================================================
// Per-Tool Rate Limiting (PORT-4.2)
// ============================================================================

/// Tool category for rate limiting purposes.
///
/// Different categories have different rate limits:
/// - Write: 10 RPS (mutating operations)
/// - Read: 100 RPS (query operations)
/// - Default: 50 RPS (unknown operations)
///
/// NIST Control: SC-5 (DoS Protection)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCategory {
    /// Mutating operations (send_message, reserve_file, etc.) - 10 RPS
    Write,
    /// Query operations (fetch_inbox, list_agents, etc.) - 100 RPS
    Read,
    /// Unknown operations - 50 RPS
    Default,
}

impl ToolCategory {
    /// Classify a tool name into a category.
    ///
    /// Write tools (10 RPS): Operations that modify state
    /// Read tools (100 RPS): Operations that only read state
    /// Default (50 RPS): Unknown or mixed operations
    pub fn from_tool_name(tool_name: &str) -> Self {
        // Write tools - lower limits (10 rps)
        const WRITE_TOOLS: &[&str] = &[
            "send_message",
            "reply_message",
            "file_reservation_paths",
            "reserve_file",
            "release_reservation",
            "force_release_reservation",
            "renew_file_reservation",
            "acquire_build_slot",
            "release_build_slot",
            "renew_build_slot",
            "register_agent",
            "update_agent_profile",
            "create_agent_identity",
            "mark_message_read",
            "acknowledge_message",
            "request_contact",
            "respond_contact",
            "set_contact_policy",
            "register_macro",
            "unregister_macro",
            "invoke_macro",
            "ensure_project",
            "ensure_product",
            "link_project_to_product",
            "unlink_project_from_product",
            "install_precommit_guard",
            "uninstall_precommit_guard",
            "add_attachment",
        ];

        // Read tools - higher limits (100 rps)
        const READ_TOOLS: &[&str] = &[
            "fetch_inbox",
            "check_inbox",
            "list_outbox",
            "get_message",
            "search_messages",
            "list_agents",
            "get_agent_profile",
            "whois",
            "list_threads",
            "get_thread",
            "summarize_thread",
            "summarize_threads",
            "list_file_reservations",
            "list_contacts",
            "list_macros",
            "list_projects",
            "get_project_info",
            "list_project_siblings",
            "list_products",
            "product_inbox",
            "get_attachment",
            "export_mailbox",
            "list_tool_metrics",
            "get_tool_stats",
            "list_activity",
        ];

        if WRITE_TOOLS.contains(&tool_name) {
            ToolCategory::Write
        } else if READ_TOOLS.contains(&tool_name) {
            ToolCategory::Read
        } else {
            ToolCategory::Default
        }
    }
}

/// Per-tool rate limiter with separate buckets for each category.
///
/// Environment variables:
/// - `RATE_LIMIT_WRITE_RPS`: Write operations limit (default: 10)
/// - `RATE_LIMIT_READ_RPS`: Read operations limit (default: 100)
/// - `RATE_LIMIT_DEFAULT_RPS`: Default limit for unknown tools (default: 50)
///
/// NIST Control: SC-5 (DoS Protection)
#[derive(Clone)]
pub struct ToolRateLimits {
    write_limiter: Arc<KeyedRateLimiter>,
    read_limiter: Arc<KeyedRateLimiter>,
    default_limiter: Arc<KeyedRateLimiter>,
    pub write_rps: u32,
    pub read_rps: u32,
    pub default_rps: u32,
    pub enabled: bool,
}

impl Default for ToolRateLimits {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRateLimits {
    /// Create per-tool rate limiters with defaults or environment overrides.
    #[allow(clippy::expect_used)] // NonZeroU32 from parsed u32 with fallback defaults; always valid
    pub fn new() -> Self {
        let enabled =
            std::env::var("RATE_LIMIT_ENABLED").unwrap_or_else(|_| "true".into()) == "true";

        // Parse RPS values from environment with defaults per specification
        let write_rps = std::env::var("RATE_LIMIT_WRITE_RPS")
            .unwrap_or_else(|_| "10".into())
            .parse::<u32>()
            .unwrap_or(10);

        let read_rps = std::env::var("RATE_LIMIT_READ_RPS")
            .unwrap_or_else(|_| "100".into())
            .parse::<u32>()
            .unwrap_or(100);

        let default_rps = std::env::var("RATE_LIMIT_DEFAULT_RPS")
            .unwrap_or_else(|_| "50".into())
            .parse::<u32>()
            .unwrap_or(50);

        // Create limiters with burst = rps (no burst allowance for tool limits)
        let write_quota =
            Quota::per_second(NonZeroU32::new(write_rps).expect("Write RPS should be non-zero"));
        let read_quota =
            Quota::per_second(NonZeroU32::new(read_rps).expect("Read RPS should be non-zero"));
        let default_quota = Quota::per_second(
            NonZeroU32::new(default_rps).expect("Default RPS should be non-zero"),
        );

        tracing::info!(
            "Per-Tool Rate Limiting: enabled={}, write_rps={}, read_rps={}, default_rps={}",
            enabled,
            write_rps,
            read_rps,
            default_rps
        );

        Self {
            write_limiter: Arc::new(RateLimiter::keyed(write_quota)),
            read_limiter: Arc::new(RateLimiter::keyed(read_quota)),
            default_limiter: Arc::new(RateLimiter::keyed(default_quota)),
            write_rps,
            read_rps,
            default_rps,
            enabled,
        }
    }

    /// Check if a tool request is within rate limits.
    ///
    /// # Arguments
    /// * `tool_name` - The MCP tool name (e.g., "send_message")
    /// * `bucket_key` - The identity key (e.g., "user:ip")
    ///
    /// # Returns
    /// * `Ok(())` if within limits
    /// * `Err(ToolCategory)` if rate limited, with the category that was exceeded
    pub fn check_tool(&self, tool_name: &str, bucket_key: &str) -> Result<(), ToolCategory> {
        if !self.enabled {
            return Ok(());
        }

        let category = ToolCategory::from_tool_name(tool_name);
        let limiter = match category {
            ToolCategory::Write => &self.write_limiter,
            ToolCategory::Read => &self.read_limiter,
            ToolCategory::Default => &self.default_limiter,
        };

        match limiter.check_key(&bucket_key.to_string()) {
            Ok(_) => Ok(()),
            Err(_) => {
                warn!(
                    tool = %tool_name,
                    category = ?category,
                    bucket_key = %bucket_key,
                    "Per-tool rate limit exceeded"
                );
                Err(category)
            }
        }
    }

    /// Get the RPS limit for a tool category.
    pub fn rps_for_category(&self, category: ToolCategory) -> u32 {
        match category {
            ToolCategory::Write => self.write_rps,
            ToolCategory::Read => self.read_rps,
            ToolCategory::Default => self.default_rps,
        }
    }
}

/// Extract JWT subject from Authorization header without full verification.
///
/// This only decodes the JWT payload to extract the `sub` claim for rate limiting.
/// Authentication and signature verification should be done by the auth middleware.
///
/// # Arguments
/// * `auth_header` - The Authorization header value (e.g., "Bearer eyJ...")
///
/// # Returns
/// The `sub` claim if present and valid, or `None`
fn extract_jwt_subject(auth_header: &str) -> Option<String> {
    // Extract token from "Bearer <token>"
    let token = auth_header.strip_prefix("Bearer ")?.trim();

    // JWT has 3 parts: header.payload.signature
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }

    // Decode the payload (second part)
    let payload = URL_SAFE_NO_PAD.decode(parts[1]).ok().or_else(|| {
        // Try with padding
        let padded = format!("{}{}", parts[1], "=".repeat((4 - parts[1].len() % 4) % 4));
        base64::engine::general_purpose::URL_SAFE
            .decode(&padded)
            .ok()
    })?;

    // Parse as JSON and extract "sub"
    let claims: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    claims.get("sub")?.as_str().map(|s| s.to_string())
}

/// Construct the rate limit bucket key.
///
/// Key format:
/// - `{jwt_subject}:{ip}` for authenticated requests
/// - `{ip}` for unauthenticated requests
///
/// NIST Control: SC-5 (DoS Protection)
pub fn get_bucket_key(req: &Request, client_ip: std::net::IpAddr) -> String {
    // Try to extract JWT subject from Authorization header
    if let Some(auth_header) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(subject) = extract_jwt_subject(auth_str) {
                debug!(
                    subject = %subject,
                    ip = %client_ip,
                    "Rate limit key includes JWT subject"
                );
                return format!("{}:{}", subject, client_ip);
            }
        }
    }

    // Fallback to IP-only for unauthenticated requests
    client_ip.to_string()
}

pub async fn rate_limit_middleware(
    State(config): State<RateLimitConfig>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !config.enabled {
        return Ok(next.run(req).await);
    }

    // Determine Client IP
    // Prefer X-Forwarded-For header if present (standard for reverse proxies)
    // Fallback to direct peer address (ConnectInfo)
    let ip = if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        forwarded
            .to_str()
            .ok()
            .and_then(|s| s.split(',').next()) // Take the first IP in the list
            .and_then(|s| s.trim().parse::<std::net::IpAddr>().ok())
            .unwrap_or(peer.ip())
    } else {
        peer.ip()
    };

    // Get bucket key (includes JWT subject if present)
    let bucket_key = get_bucket_key(&req, ip);

    match config.limiter.check_key(&bucket_key) {
        Ok(_) => Ok(next.run(req).await),
        Err(_) => {
            warn!(bucket_key = %bucket_key, "RateLimit: exceeded quota");
            Err(StatusCode::TOO_MANY_REQUESTS)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use axum::http::Request as HttpRequest;
    use std::net::IpAddr;

    /// Create a test JWT token with given subject.
    /// Format: header.payload.signature (signature is fake for testing)
    fn create_test_jwt(subject: &str) -> String {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;

        // Minimal header
        let header = r#"{"alg":"HS256","typ":"JWT"}"#;
        let header_b64 = URL_SAFE_NO_PAD.encode(header);

        // Payload with sub claim
        let payload = format!(r#"{{"sub":"{}","iat":1234567890}}"#, subject);
        let payload_b64 = URL_SAFE_NO_PAD.encode(payload);

        // Fake signature (not verified in rate limiting)
        let signature = "fake_signature_for_testing";

        format!("{}.{}.{}", header_b64, payload_b64, signature)
    }

    #[test]
    fn test_extract_jwt_subject_valid() {
        let token = create_test_jwt("user123");
        let auth_header = format!("Bearer {}", token);

        let subject = extract_jwt_subject(&auth_header);
        assert_eq!(subject, Some("user123".to_string()));
    }

    #[test]
    fn test_extract_jwt_subject_with_email() {
        let token = create_test_jwt("alice@example.com");
        let auth_header = format!("Bearer {}", token);

        let subject = extract_jwt_subject(&auth_header);
        assert_eq!(subject, Some("alice@example.com".to_string()));
    }

    #[test]
    fn test_extract_jwt_subject_missing_bearer() {
        let token = create_test_jwt("user123");
        // No "Bearer " prefix
        let subject = extract_jwt_subject(&token);
        assert_eq!(subject, None);
    }

    #[test]
    fn test_extract_jwt_subject_invalid_token_format() {
        // Only 2 parts instead of 3
        let subject = extract_jwt_subject("Bearer header.payload");
        assert_eq!(subject, None);
    }

    #[test]
    fn test_extract_jwt_subject_invalid_base64() {
        let subject = extract_jwt_subject("Bearer !!invalid!!.!!base64!!.signature");
        assert_eq!(subject, None);
    }

    #[test]
    fn test_extract_jwt_subject_no_sub_claim() {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;

        // JWT without sub claim
        let header = r#"{"alg":"HS256"}"#;
        let payload = r#"{"iat":1234567890}"#;
        let token = format!(
            "{}.{}.sig",
            URL_SAFE_NO_PAD.encode(header),
            URL_SAFE_NO_PAD.encode(payload)
        );

        let subject = extract_jwt_subject(&format!("Bearer {}", token));
        assert_eq!(subject, None);
    }

    #[test]
    fn test_get_bucket_key_with_jwt() {
        let token = create_test_jwt("agent-001");
        let ip: IpAddr = "192.168.1.100".parse().unwrap();

        let req = HttpRequest::builder()
            .header("authorization", format!("Bearer {}", token))
            .body(())
            .unwrap();

        // Convert to axum Request type
        let axum_req: Request = req.map(|_| axum::body::Body::empty());

        let key = get_bucket_key(&axum_req, ip);
        assert_eq!(key, "agent-001:192.168.1.100");
    }

    #[test]
    fn test_get_bucket_key_without_jwt() {
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        let req = HttpRequest::builder().body(()).unwrap();
        let axum_req: Request = req.map(|_| axum::body::Body::empty());

        let key = get_bucket_key(&axum_req, ip);
        assert_eq!(key, "10.0.0.1");
    }

    #[test]
    fn test_get_bucket_key_with_invalid_jwt_falls_back_to_ip() {
        let ip: IpAddr = "172.16.0.1".parse().unwrap();

        let req = HttpRequest::builder()
            .header("authorization", "Bearer invalid.token")
            .body(())
            .unwrap();
        let axum_req: Request = req.map(|_| axum::body::Body::empty());

        let key = get_bucket_key(&axum_req, ip);
        assert_eq!(key, "172.16.0.1");
    }

    #[test]
    fn test_get_bucket_key_ipv6() {
        let token = create_test_jwt("ipv6-user");
        let ip: IpAddr = "2001:db8::1".parse().unwrap();

        let req = HttpRequest::builder()
            .header("authorization", format!("Bearer {}", token))
            .body(())
            .unwrap();
        let axum_req: Request = req.map(|_| axum::body::Body::empty());

        let key = get_bucket_key(&axum_req, ip);
        assert_eq!(key, "ipv6-user:2001:db8::1");
    }

    #[test]
    fn test_rate_limit_config_defaults() {
        // Test that RateLimitConfig can be created with defaults
        let config = RateLimitConfig::new();
        assert!(config.enabled);
    }

    // ========================================================================
    // Per-Tool Rate Limiting Tests (PORT-4.2)
    // ========================================================================

    #[test]
    fn test_tool_category_from_tool_name_write_tools() {
        // Write tools should have lower limits (10 rps)
        assert_eq!(
            ToolCategory::from_tool_name("send_message"),
            ToolCategory::Write
        );
        assert_eq!(
            ToolCategory::from_tool_name("reply_message"),
            ToolCategory::Write
        );
        assert_eq!(
            ToolCategory::from_tool_name("file_reservation_paths"),
            ToolCategory::Write
        );
        assert_eq!(
            ToolCategory::from_tool_name("reserve_file"),
            ToolCategory::Write
        );
        assert_eq!(
            ToolCategory::from_tool_name("acquire_build_slot"),
            ToolCategory::Write
        );
        assert_eq!(
            ToolCategory::from_tool_name("register_agent"),
            ToolCategory::Write
        );
    }

    #[test]
    fn test_tool_category_from_tool_name_read_tools() {
        // Read tools should have higher limits (100 rps)
        assert_eq!(
            ToolCategory::from_tool_name("fetch_inbox"),
            ToolCategory::Read
        );
        assert_eq!(
            ToolCategory::from_tool_name("check_inbox"),
            ToolCategory::Read
        );
        assert_eq!(
            ToolCategory::from_tool_name("list_outbox"),
            ToolCategory::Read
        );
        assert_eq!(
            ToolCategory::from_tool_name("get_message"),
            ToolCategory::Read
        );
        assert_eq!(
            ToolCategory::from_tool_name("search_messages"),
            ToolCategory::Read
        );
        assert_eq!(
            ToolCategory::from_tool_name("list_agents"),
            ToolCategory::Read
        );
        assert_eq!(
            ToolCategory::from_tool_name("list_threads"),
            ToolCategory::Read
        );
        assert_eq!(
            ToolCategory::from_tool_name("get_project_info"),
            ToolCategory::Read
        );
    }

    #[test]
    fn test_tool_category_from_tool_name_default() {
        // Unknown tools should use default limits (50 rps)
        assert_eq!(
            ToolCategory::from_tool_name("unknown_tool"),
            ToolCategory::Default
        );
        assert_eq!(
            ToolCategory::from_tool_name("custom_operation"),
            ToolCategory::Default
        );
    }

    #[test]
    fn test_tool_rate_limits_default_values() {
        let limits = ToolRateLimits::new();

        // Verify default RPS values
        assert_eq!(limits.write_rps, 10);
        assert_eq!(limits.read_rps, 100);
        assert_eq!(limits.default_rps, 50);
    }

    #[test]
    fn test_tool_rate_limits_get_limiter_returns_correct_category() {
        let limits = ToolRateLimits::new();

        // Write tool should use write limiter
        let key = "test-user:127.0.0.1";

        // First, exhaust the write limiter (10 requests)
        for _ in 0..10 {
            assert!(limits.check_tool("send_message", key).is_ok());
        }
        // 11th request should fail for write tools
        assert!(limits.check_tool("send_message", key).is_err());

        // Read limiter should still work (separate bucket)
        assert!(limits.check_tool("fetch_inbox", key).is_ok());
    }

    #[test]
    fn test_tool_rate_limits_write_lower_than_read() {
        let limits = ToolRateLimits::new();
        let key = "agent-1:192.168.1.1";

        // Write limit is 10 rps - exhaust it
        for i in 0..10 {
            let result = limits.check_tool("send_message", key);
            assert!(result.is_ok(), "Write request {} should succeed", i);
        }

        // Next write should fail
        assert!(
            limits.check_tool("send_message", key).is_err(),
            "Write request 11 should be rate limited"
        );

        // But read should still work (has 100 rps limit)
        for i in 0..50 {
            let result = limits.check_tool("fetch_inbox", key);
            assert!(result.is_ok(), "Read request {} should succeed", i);
        }
    }

    #[test]
    fn test_tool_rate_limits_different_keys_independent() {
        let limits = ToolRateLimits::new();

        // Exhaust limit for user1
        for _ in 0..10 {
            assert!(limits.check_tool("send_message", "user1:1.1.1.1").is_ok());
        }
        assert!(limits.check_tool("send_message", "user1:1.1.1.1").is_err());

        // user2 should still have full quota
        for _ in 0..10 {
            assert!(limits.check_tool("send_message", "user2:2.2.2.2").is_ok());
        }
    }
}
