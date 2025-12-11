//! E2E Test Utilities for MCP Agent Mail
//!
//! This crate provides end-to-end testing using jugar-probar's
//! Playwright-compatible browser automation.

pub mod config;
pub mod fixtures;

pub use config::TestConfig;
pub use fixtures::TestFixtures;
