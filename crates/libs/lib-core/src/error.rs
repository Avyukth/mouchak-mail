use thiserror::Error;
use strum_macros::{AsRefStr};
// use sqlx::migrate::MigrateError; // Import MigrateError

#[derive(Debug, Error, AsRefStr)]
pub enum Error {
    // -- Externals
    #[error("Libsql Error: {0}")]
    Libsql(#[from] libsql::Error),
    #[error("Git Error: {0}")]
    Git2(#[from] git2::Error),
    // #[error("Migration Error: {0}")]
    // Migrate(#[from] MigrateError),
    #[error("Serde JSON Error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    // -- Internals
    #[error("Entity not found")]
    NotFound,
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Authentication failed")]
    AuthError,

    // -- Model Specific
    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Message not found: {0}")]
    MessageNotFound(i64),

    #[error("FileReservation not found: {0}")]
    FileReservationNotFound(String),

    #[error("Product not found: {0}")]
    ProductNotFound(String),

    #[error("Macro not found: {0}")]
    MacroNotFound(String),

    #[error("Build slot not found: {0}")]
    BuildSlotNotFound(i64),
}

pub type Result<T> = core::result::Result<T, Error>;
