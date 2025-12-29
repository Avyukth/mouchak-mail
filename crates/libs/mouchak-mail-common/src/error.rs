use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Core error: {0}")]
    Core(String), // Placeholder for wrapping lib-core errors if needed

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, ServerError>;
