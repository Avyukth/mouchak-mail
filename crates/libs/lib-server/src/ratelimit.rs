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
    ConnectInfo(ip): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !config.enabled {
        return Ok(next.run(req).await);
    }

    // Extract IP
    // 1. Try X-Forwarded-For if behind proxy (this is rudimentary, assuming trusted proxy for now)
    // 2. Fallback to ConnectInfo
    // 3. Fallback to 127.0.0.1
    
    // Note: For real production behind generic proxies, inspecting headers is needed.
    // Ideally we trust ConnectInfo if Axum is configured correctly (e.g. valid proxy or direct).
    
    // IP is guaranteed by ConnectInfo if setup correctly using into_make_service_with_connect_info
    let ip = ip.ip();

    // Check (allow localhost bypass logic? Assuming auth middleware handles that, or we treat localhost as just another IP)
    
    match config.limiter.check_key(&ip) {
        Ok(_) => Ok(next.run(req).await),
        Err(_) => {
            warn!("RateLimit: IP {} exceeded quota", ip);
            Err(StatusCode::TOO_MANY_REQUESTS)
        }
    }
}
