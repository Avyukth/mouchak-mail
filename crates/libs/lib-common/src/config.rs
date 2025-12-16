use serde::Deserialize;
use config::{Config, File};
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
}

#[derive(Debug, Deserialize, Clone)]
pub struct McpConfig {
    pub transport: String,
    pub port: u16,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8765,
                auth_hmac: None,
            },
            mcp: McpConfig {
                transport: "stdio".to_string(),
                port: 3000,
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
            .set_default("mcp.transport", "stdio")?
            .set_default("mcp.port", 3000)?
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
