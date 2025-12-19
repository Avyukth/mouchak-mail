//! Error types for lib-core operations.
//!
//! This module defines the error types used throughout the lib-core crate.
//! Errors are categorized into:
//!
//! - **External errors**: Wrapped errors from dependencies (libsql, git2, serde_json, io)
//! - **Internal errors**: Generic errors for common failure modes
//! - **Model-specific errors**: Entity-specific not-found errors
//!
//! # Example
//!
//! ```
//! use lib_core::error::{Error, Result};
//!
//! fn find_project(slug: &str) -> Result<()> {
//!     if slug.is_empty() {
//!         return Err(Error::InvalidInput("slug cannot be empty".to_string()));
//!     }
//!     // ... lookup logic
//!     Err(Error::ProjectNotFound(slug.to_string()))
//! }
//!
//! match find_project("") {
//!     Ok(_) => println!("Found"),
//!     Err(Error::InvalidInput(msg)) => println!("Invalid: {}", msg),
//!     Err(Error::ProjectNotFound(slug)) => println!("Not found: {}", slug),
//!     Err(e) => println!("Other error: {}", e),
//! }
//! ```

use strum_macros::AsRefStr;
use thiserror::Error;

/// The error type for lib-core operations.
///
/// This enum represents all possible errors that can occur in the lib-core crate.
/// It implements [`std::error::Error`] via `thiserror` and provides `From` implementations
/// for automatic conversion from underlying error types.
///
/// # Categories
///
/// ## External Errors
/// Errors from external dependencies are automatically converted using `#[from]`:
/// - [`Error::Libsql`] - Database errors from libsql
/// - [`Error::Git2`] - Git repository errors
/// - [`Error::SerdeJson`] - JSON serialization/deserialization errors
/// - [`Error::Io`] - Standard I/O errors
///
/// ## Internal Errors
/// Generic errors for common failure scenarios:
/// - [`Error::NotFound`] - Generic entity not found
/// - [`Error::InvalidInput`] - Validation failures
/// - [`Error::AuthError`] - Authentication failures
///
/// ## Model-Specific Errors
/// Entity-specific not-found errors with identifiers:
/// - [`Error::ProjectNotFound`] - Project lookup failed
/// - [`Error::AgentNotFound`] - Agent lookup failed
/// - [`Error::MessageNotFound`] - Message lookup failed
/// - [`Error::FileReservationNotFound`] - File reservation lookup failed
/// - [`Error::ProductNotFound`] - Product lookup failed
/// - [`Error::MacroNotFound`] - Macro lookup failed
/// - [`Error::BuildSlotNotFound`] - Build slot lookup failed
#[derive(Debug, Error, AsRefStr)]
pub enum Error {
    // -- External errors from dependencies
    /// Database error from libsql.
    ///
    /// Automatically converted from [`libsql::Error`] via `From`.
    #[error("Libsql Error: {0}")]
    Libsql(#[from] libsql::Error),

    /// Git repository error.
    ///
    /// Automatically converted from [`git2::Error`] via `From`.
    #[error("Git Error: {0}")]
    Git2(#[from] git2::Error),

    /// JSON serialization/deserialization error.
    ///
    /// Automatically converted from [`serde_json::Error`] via `From`.
    #[error("Serde JSON Error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    /// Standard I/O error.
    ///
    /// Automatically converted from [`std::io::Error`] via `From`.
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    // -- Internal errors for common failure modes
    /// Generic entity not found error.
    ///
    /// Use this when an entity lookup fails and there's no
    /// more specific error variant available.
    #[error("Entity not found")]
    NotFound,

    /// Input validation error.
    ///
    /// Contains a message describing why the input was invalid.
    ///
    /// # Example
    ///
    /// ```
    /// use lib_core::Error;
    ///
    /// let err = Error::InvalidInput("email format invalid".to_string());
    /// assert!(err.to_string().contains("email format invalid"));
    /// ```
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Authentication failure.
    ///
    /// Returned when authentication credentials are invalid
    /// or the user is not authorized to perform an action.
    #[error("Authentication failed")]
    AuthError,

    // -- Model-specific not-found errors
    /// Project not found by slug.
    ///
    /// The contained string is the project slug that was not found.
    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    /// Agent not found by name.
    ///
    /// The contained string is the agent name that was not found.
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    /// Message not found by ID.
    ///
    /// The contained i64 is the message ID that was not found.
    #[error("Message not found: {0}")]
    MessageNotFound(i64),

    /// File reservation not found.
    ///
    /// The contained string is the file path that was not found.
    #[error("FileReservation not found: {0}")]
    FileReservationNotFound(String),

    /// Product not found by slug.
    ///
    /// The contained string is the product slug that was not found.
    #[error("Product not found: {0}")]
    ProductNotFound(String),

    /// Macro not found by name.
    ///
    /// The contained string is the macro name that was not found.
    #[error("Macro not found: {0}")]
    MacroNotFound(String),

    /// Build slot not found by ID.
    ///
    /// The contained i64 is the build slot ID that was not found.
    #[error("Build slot not found: {0}")]
    BuildSlotNotFound(i64),

    /// Lock acquisition timeout.
    ///
    /// Returned when a file lock cannot be acquired within the timeout period.
    #[error("Lock timeout on {path}, held by PID {owner_pid}")]
    LockTimeout { path: String, owner_pid: u32 },

    /// Structured validation error with actionable suggestion.
    ///
    /// Wraps [`crate::utils::validation::ValidationError`] to provide
    /// detailed error information including field, provided value,
    /// reason, and suggestion for recovery.
    ///
    /// # Example
    ///
    /// ```
    /// use lib_core::Error;
    /// use lib_core::utils::validation::validate_agent_name;
    ///
    /// let result = validate_agent_name("invalid-name");
    /// if let Err(ve) = result {
    ///     let err: Error = ve.into();
    ///     assert!(err.to_string().contains("agent name"));
    /// }
    /// ```
    #[error("Validation error: {0}")]
    Validation(#[from] crate::utils::validation::ValidationError),
}

/// A specialized [`Result`] type for lib-core operations.
///
/// This type alias is used throughout lib-core to avoid
/// having to specify the error type repeatedly.
///
/// # Example
///
/// ```
/// use lib_core::Result;
///
/// fn do_something() -> Result<String> {
///     Ok("success".to_string())
/// }
/// ```
pub type Result<T> = core::result::Result<T, Error>;
