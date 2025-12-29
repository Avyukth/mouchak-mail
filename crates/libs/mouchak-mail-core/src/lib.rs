//! # lib-core: Core Business Logic for Mouchak Mail
//!
//! `lib-core` contains the core domain logic and data access for the Mouchak Mail application.
//!
//! This crate provides the core business logic layer for the Mouchak Mail system,
//! following the BMC (Backend Model Controller) pattern for consistent data access.
//!
//! ## Architecture
//!
//! - **BMC Layer**: Stateless controllers for all database operations
//! - **Model Structs**: Data transfer objects (DTOs) for all entities
//! - **ModelManager**: Central manager for database connections and Git operations
//! - **Git Integration**: All entities archived to Git for auditability
//!
//! ## Key Modules
//!
//! - [`model`]: All BMC controllers and data models
//! - [`store`]: Low-level database and Git operations
//! - [`ctx`]: Request context for RBAC
//!
//! ## Example
//!
//! ```no_run
//! use mouchak_mail_core::model::{ModelManager, agent::AgentBmc};
//! use mouchak_mail_core::ctx::Ctx;
//! use mouchak_mail_core::types::ProjectId;
//!
//! async fn example() -> mouchak_mail_core::Result<()> {
//!     let mm = ModelManager::new(std::sync::Arc::new(mouchak_mail_common::config::AppConfig::default())).await?;
//!     let ctx = Ctx::root_ctx();
//!
//!     // List all agents in a project
//!     let agents = AgentBmc::list_all_for_project(&ctx, &mm, ProjectId::new(1)).await?;
//!     println!("Found {} agents", agents.len());
//!     Ok(())
//! }
//! ```

/// Request context for authentication and authorization.
pub mod ctx;

/// Error types and Result alias for lib-core operations.
pub mod error;

/// Backend Model Controllers (BMC) and data models for all entities.
pub mod model;

/// Low-level storage operations for database and Git.
pub mod store;

/// Strong newtypes for domain identifiers.
pub mod types;

/// Utility functions and helpers.
pub mod utils;

// Re-export core types
pub use ctx::Ctx;
pub use error::{Error, Result};
pub use model::ModelManager;
pub use types::{AgentId, AgentName, MessageId, ProjectId, ProjectSlug, ThreadId};
