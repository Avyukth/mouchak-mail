use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use governor::{
    clock::DefaultClock,
    state::keyed::DashMapStateStore,
    Quota, RateLimiter,
};
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use axum::extract::ConnectInfo;
use tracing::warn;

// Type alias for our RateLimiter. 
// We use IP address (SocketAddr) as the key.
// Actually, `SocketAddr` might include port, we usually want just IP.
type IpRateLimiter = RateLimiter<std::net::IpAddr, DashMapStateStore<std::net::IpAddr>, DefaultClock>;

#[derive(Clone)]
pub struct RateLimitConfig {
    pub limiter: Arc<IpRateLimiter>,
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimitConfig {
    pub fn new() -> Self {
        let enabled = std::env::var("RATE_LIMIT_ENABLED").unwrap_or_else(|_| "true".into()) == "true";
        
        let rps = std::env::var("RATE_LIMIT_RPS")
            .unwrap_or_else(|_| "100".into())
            .parse::<u32>()
            .unwrap_or(100);
            
        let burst = std::env::var("RATE_LIMIT_BURST")
            .unwrap_or_else(|_| "200".into())
            .parse::<u32>()
            .unwrap_or(200);

        let quota = Quota::per_second(NonZeroU32::new(rps).unwrap_or(NonZeroU32::new(100).unwrap()))
            .allow_burst(NonZeroU32::new(burst).unwrap_or(NonZeroU32::new(200).unwrap()));

        let limiter = Arc::new(RateLimiter::keyed(quota));

        tracing::info!("Rate Limiting: enabled={}, rps={}, burst={}", enabled, rps, burst);

        Self {
            limiter,
            enabled,
        }
    }
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
        forwarded.to_str()
            .ok()
            .and_then(|s| s.split(',').next()) // Take the first IP in the list
            .and_then(|s| s.trim().parse::<std::net::IpAddr>().ok())
            .unwrap_or(peer.ip())
    } else {
        peer.ip()
    };

    match config.limiter.check_key(&ip) {
        Ok(_) => Ok(next.run(req).await),
        Err(_) => {
            warn!("RateLimit: IP {} exceeded quota", ip);
            Err(StatusCode::TOO_MANY_REQUESTS)
        }
    }
}
