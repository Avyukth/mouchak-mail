use config::{Config, File};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub mcp: McpConfig,
}

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
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
            // Merge in config files
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false));

        // 12-factor app standard: PORT and HOST env vars
        if let Ok(port) = env::var("PORT") {
            if let Ok(p) = port.parse::<i64>() {
                builder = builder.set_override("server.port", p)?;
            }
        }
        if let Ok(host) = env::var("HOST") {
            builder = builder.set_override("server.host", host)?;
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
        // Neither enabled
        let config = McpConfig {
            transport: "stdio".into(),
            port: 3000,
            worktrees_enabled: false,
            git_identity_enabled: false,
        };
        assert!(!config.worktrees_active());

        // Only worktrees
        let config = McpConfig {
            worktrees_enabled: true,
            git_identity_enabled: false,
            ..config.clone()
        };
        assert!(config.worktrees_active());

        // Only git_identity
        let config = McpConfig {
            worktrees_enabled: false,
            git_identity_enabled: true,
            ..config.clone()
        };
        assert!(config.worktrees_active());

        // Both enabled
        let config = McpConfig {
            worktrees_enabled: true,
            git_identity_enabled: true,
            ..config.clone()
        };
        assert!(config.worktrees_active());
    }
}
