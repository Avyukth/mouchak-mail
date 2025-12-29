//! Integration tests for lib-core
//!
//! These tests verify the core functionality of the Mouchak Mail system.

// Tests are allowed to use unwrap()/expect() for clearer failure messages
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::inefficient_to_string
)]
#![allow(clippy::duplicate_mod)]
#![allow(clippy::duplicated_attributes)]

mod agent_tests;
mod cc_bcc_tests;
mod common;
mod message_tests;
mod project_tests;
mod unified_inbox_tests;
