use config::{Config, File};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub mcp: McpConfig,
    #[serde(default)]
    pub escalation: EscalationConfig,
    #[serde(default)]
    pub quota: QuotaConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub auth_hmac: Option<String>,
    /// Enable serving embedded web UI (when compiled with with-web-ui feature)
    #[serde(default = "default_serve_ui")]
    pub serve_ui: bool,
}

fn default_serve_ui() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EscalationMode {
    #[default]
    Log,
    FileReservation,
    Overseer,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EscalationConfig {
    #[serde(default)]
    pub ack_ttl_enabled: bool,
    #[serde(default = "default_ack_ttl_seconds")]
    pub ack_ttl_seconds: u64,
    #[serde(default)]
    pub escalation_enabled: bool,
    #[serde(default)]
    pub escalation_mode: EscalationMode,
    #[serde(default = "default_scan_interval_seconds")]
    pub scan_interval_seconds: u64,
}

fn default_ack_ttl_seconds() -> u64 {
    1800 // 30 minutes
}

fn default_scan_interval_seconds() -> u64 {
    300 // 5 minutes
}

impl Default for EscalationConfig {
    fn default() -> Self {
        Self {
            ack_ttl_enabled: false,
            ack_ttl_seconds: default_ack_ttl_seconds(),
            escalation_enabled: false,
            escalation_mode: EscalationMode::default(),
            scan_interval_seconds: default_scan_interval_seconds(),
        }
    }
}

impl EscalationConfig {
    pub fn from_env() -> Self {
        Self {
            ack_ttl_enabled: parse_bool_env("ACK_TTL_ENABLED"),
            ack_ttl_seconds: std::env::var("ACK_TTL_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_ack_ttl_seconds),
            escalation_enabled: parse_bool_env("ACK_ESCALATION_ENABLED"),
            escalation_mode: std::env::var("ACK_ESCALATION_MODE")
                .ok()
                .and_then(|m| match m.to_lowercase().as_str() {
                    "log" => Some(EscalationMode::Log),
                    "file_reservation" => Some(EscalationMode::FileReservation),
                    "overseer" => Some(EscalationMode::Overseer),
                    _ => None,
                })
                .unwrap_or_default(),
            scan_interval_seconds: std::env::var("ACK_SCAN_INTERVAL_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_scan_interval_seconds),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct McpConfig {
    pub transport: String,
    pub port: u16,
    /// Enable worktree-related features (build slots, pre-commit guard)
    #[serde(default)]
    pub worktrees_enabled: bool,
    /// Enable git identity features
    #[serde(default)]
    pub git_identity_enabled: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct QuotaConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_quota_attachments_limit_bytes")]
    pub attachments_limit_bytes: u64,
    #[serde(default = "default_quota_inbox_limit_count")]
    pub inbox_limit_count: u64,
}

fn default_quota_attachments_limit_bytes() -> u64 {
    100 * 1024 * 1024 // 100 MB
}

fn default_quota_inbox_limit_count() -> u64 {
    1000
}

impl Default for QuotaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            attachments_limit_bytes: default_quota_attachments_limit_bytes(),
            inbox_limit_count: default_quota_inbox_limit_count(),
        }
    }
}

impl McpConfig {
    /// Check if worktree features should be active
    /// Returns true if either WORKTREES_ENABLED or GIT_IDENTITY_ENABLED is set
    pub fn worktrees_active(&self) -> bool {
        self.worktrees_enabled || self.git_identity_enabled
    }

    /// Create config from environment variables (for standalone MCP usage)
    pub fn from_env() -> Self {
        Self {
            transport: std::env::var("MOUCHAK_MCP__TRANSPORT").unwrap_or_else(|_| "stdio".into()),
            port: std::env::var("MOUCHAK_MCP__PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            worktrees_enabled: parse_bool_env("WORKTREES_ENABLED"),
            git_identity_enabled: parse_bool_env("GIT_IDENTITY_ENABLED"),
        }
    }
}

/// Parse boolean environment variable with truthy value detection
fn parse_bool_env(key: &str) -> bool {
    std::env::var(key)
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "t" | "y"))
        .unwrap_or(false)
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8765,
                auth_hmac: None,
                serve_ui: true,
            },
            mcp: McpConfig {
                transport: "stdio".to_string(),
                port: 3000,
                worktrees_enabled: false,
                git_identity_enabled: false,
            },
            escalation: EscalationConfig::default(),
            quota: QuotaConfig::default(),
        }
    }
}

impl AppConfig {
    /// Load configuration with 12-factor app compliant env var support.
    ///
    /// Priority order (highest to lowest):
    /// 1. `PORT` / `HOST` env vars (12-factor standard)
    /// 2. Config files (`config/default.toml`, `config/{run_mode}.toml`)
    /// 3. Hardcoded defaults (port 8765)
    pub fn load() -> Result<Self, config::ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let mut builder = Config::builder()
            // Start with defaults
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8765)?
            .set_default("server.serve_ui", true)?
            .set_default("mcp.transport", "stdio")?
            .set_default("mcp.port", 3000)?
            .set_default("mcp.worktrees_enabled", false)?
            .set_default("mcp.git_identity_enabled", false)?
            .set_default("escalation.ack_ttl_enabled", false)?
            .set_default("escalation.ack_ttl_seconds", 1800_i64)?
            .set_default("escalation.escalation_enabled", false)?
            .set_default("escalation.escalation_mode", "log")?
            .set_default("escalation.scan_interval_seconds", 300_i64)?
            // Merge in config files
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false));

        // Add user config file from ~/.mcp-agent-mail/config.toml
        if let Ok(home) = env::var("HOME") {
            let path = std::path::Path::new(&home)
                .join(".mcp-agent-mail")
                .join("config.toml");
            builder = builder.add_source(File::from(path).required(false));
        }

        // 12-factor app standard: PORT and HOST env vars
        if let Ok(port) = env::var("PORT") {
            if let Ok(p) = port.parse::<i64>() {
                builder = builder.set_override("server.port", p)?;
            }
        }
        if let Ok(host) = env::var("HOST") {
            builder = builder.set_override("server.host", host)?;
        }

        if parse_bool_env("ACK_TTL_ENABLED") {
            builder = builder.set_override("escalation.ack_ttl_enabled", true)?;
        }
        if let Ok(ttl) = env::var("ACK_TTL_SECONDS") {
            if let Ok(secs) = ttl.parse::<i64>() {
                builder = builder.set_override("escalation.ack_ttl_seconds", secs)?;
            }
        }
        if parse_bool_env("ACK_ESCALATION_ENABLED") {
            builder = builder.set_override("escalation.escalation_enabled", true)?;
        }
        if let Ok(mode) = env::var("ACK_ESCALATION_MODE") {
            builder = builder.set_override("escalation.escalation_mode", mode)?;
        }
        if let Ok(interval) = env::var("ACK_SCAN_INTERVAL_SECONDS") {
            if let Ok(secs) = interval.parse::<i64>() {
                builder = builder.set_override("escalation.scan_interval_seconds", secs)?;
            }
        }

        if parse_bool_env("QUOTA_ENABLED") {
            builder = builder.set_override("quota.enabled", true)?;
        }
        if let Ok(bytes) = env::var("QUOTA_ATTACHMENTS_LIMIT_BYTES") {
            if let Ok(limit) = bytes.parse::<u64>() {
                builder = builder.set_override("quota.attachments_limit_bytes", limit)?;
            }
        }
        if let Ok(count) = env::var("QUOTA_INBOX_LIMIT_COUNT") {
            if let Ok(limit) = count.parse::<u64>() {
                builder = builder.set_override("quota.inbox_limit_count", limit)?;
            }
        }

        builder.build()?.try_deserialize()
    }
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bool_env_truthy() {
        // These should all return true
        for (key, val) in [("TEST_1", "1"), ("TEST_T", "true"), ("TEST_Y", "yes")] {
            // SAFETY: Test code only, single-threaded test execution
            unsafe { std::env::set_var(key, val) };
            assert!(parse_bool_env(key), "Expected true for {}={}", key, val);
            unsafe { std::env::remove_var(key) };
        }
    }

    #[test]
    fn test_parse_bool_env_falsy() {
        // These should all return false
        // SAFETY: Test code only, single-threaded test execution
        unsafe {
            std::env::set_var("TEST_F", "0");
        }
        assert!(!parse_bool_env("TEST_F"));
        unsafe {
            std::env::set_var("TEST_F", "false");
        }
        assert!(!parse_bool_env("TEST_F"));
        unsafe {
            std::env::set_var("TEST_F", "no");
        }
        assert!(!parse_bool_env("TEST_F"));
        unsafe {
            std::env::remove_var("TEST_F");
        }

        // Unset should return false
        unsafe {
            std::env::remove_var("NOT_SET_VAR");
        }
        assert!(!parse_bool_env("NOT_SET_VAR"));
    }

    #[test]
    fn test_worktrees_active() {
        let config = McpConfig {
            transport: "stdio".into(),
            port: 3000,
            worktrees_enabled: false,
            git_identity_enabled: false,
        };
        assert!(!config.worktrees_active());

        let config = McpConfig {
            worktrees_enabled: true,
            git_identity_enabled: false,
            ..config.clone()
        };
        assert!(config.worktrees_active());

        let config = McpConfig {
            worktrees_enabled: false,
            git_identity_enabled: true,
            ..config.clone()
        };
        assert!(config.worktrees_active());

        let config = McpConfig {
            worktrees_enabled: true,
            git_identity_enabled: true,
            ..config.clone()
        };
        assert!(config.worktrees_active());
    }

    #[test]
    fn test_escalation_config_defaults() {
        let config = EscalationConfig::default();
        assert!(!config.ack_ttl_enabled);
        assert_eq!(config.ack_ttl_seconds, 1800);
        assert!(!config.escalation_enabled);
        assert_eq!(config.escalation_mode, EscalationMode::Log);
        assert_eq!(config.scan_interval_seconds, 300);
    }

    #[test]
    fn test_escalation_mode_variants() {
        assert_eq!(EscalationMode::default(), EscalationMode::Log);
        assert_ne!(EscalationMode::FileReservation, EscalationMode::Log);
        assert_ne!(EscalationMode::Overseer, EscalationMode::Log);
    }

    #[test]
    fn test_escalation_config_from_env() {
        // Test 1: Full config override
        unsafe {
            std::env::set_var("ACK_TTL_ENABLED", "true");
            std::env::set_var("ACK_TTL_SECONDS", "3600");
            std::env::set_var("ACK_ESCALATION_ENABLED", "1");
            std::env::set_var("ACK_ESCALATION_MODE", "file_reservation");
            std::env::set_var("ACK_SCAN_INTERVAL_SECONDS", "60");
        }

        let config = EscalationConfig::from_env();
        assert!(config.ack_ttl_enabled);
        assert_eq!(config.ack_ttl_seconds, 3600);
        assert!(config.escalation_enabled);
        assert_eq!(config.escalation_mode, EscalationMode::FileReservation);
        assert_eq!(config.scan_interval_seconds, 60);

        unsafe {
            std::env::remove_var("ACK_TTL_ENABLED");
            std::env::remove_var("ACK_TTL_SECONDS");
            std::env::remove_var("ACK_ESCALATION_ENABLED");
            std::env::remove_var("ACK_ESCALATION_MODE");
            std::env::remove_var("ACK_SCAN_INTERVAL_SECONDS");
        }

        // Test 2: Overseer mode
        unsafe {
            std::env::set_var("ACK_ESCALATION_MODE", "overseer");
        }
        let config = EscalationConfig::from_env();
        assert_eq!(config.escalation_mode, EscalationMode::Overseer);
        unsafe {
            std::env::remove_var("ACK_ESCALATION_MODE");
        }

        // Test 3: Invalid fallback
        unsafe {
            std::env::set_var("ACK_ESCALATION_MODE", "invalid_mode");
        }
        let config = EscalationConfig::from_env();
        assert_eq!(config.escalation_mode, EscalationMode::Log);
        unsafe {
            std::env::remove_var("ACK_ESCALATION_MODE");
        }
    }
}
