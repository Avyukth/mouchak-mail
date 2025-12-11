//! Test configuration

/// Configuration for E2E tests
#[derive(Debug, Clone)]
pub struct TestConfig {
    /// Base URL for the web UI (default: http://localhost:5173)
    pub web_ui_url: String,
    /// Base URL for the API server (default: http://localhost:8765)
    pub api_url: String,
    /// Browser headless mode (default: true for CI)
    pub headless: bool,
    /// Timeout for page operations in milliseconds
    pub timeout_ms: u64,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            web_ui_url: std::env::var("TEST_WEB_UI_URL")
                .unwrap_or_else(|_| "http://localhost:5173".to_string()),
            api_url: std::env::var("TEST_API_URL")
                .unwrap_or_else(|_| "http://localhost:8765".to_string()),
            headless: std::env::var("TEST_HEADLESS")
                .map(|v| v != "false")
                .unwrap_or(true),
            timeout_ms: std::env::var("TEST_TIMEOUT_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30000),
        }
    }
}

impl TestConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_visible_browser(mut self) -> Self {
        self.headless = false;
        self
    }
}
