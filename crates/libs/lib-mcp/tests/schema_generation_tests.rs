//! Tests for schema auto-generation from JsonSchema derives
//!
//! These tests verify that `schema_from_params` correctly:
//! - Extracts parameter names from struct fields
//! - Marks Option<T> fields as optional
//! - Extracts descriptions from doc comments
//! - Generates schemas for all tools
//!
//! This is the TDD test suite for task mcpmail-woia.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::redundant_field_names
)]

use lib_mcp::tools::{RegisterAgentParams, SendMessageParams, schema_from_params};

/// Test that generated schema has correct structure matching what we expect
#[test]
fn test_generated_schema_matches_manual_schema() {
    let schema = schema_from_params::<SendMessageParams>(
        "send_message",
        "Send a message from one agent to others.",
    );

    // Verify name and description
    assert_eq!(schema.name, "send_message");
    assert!(
        schema.description.contains("Send a message"),
        "Expected description to contain 'Send a message', got: {}",
        schema.description
    );

    // Verify all expected parameters are present
    let param_names: Vec<&str> = schema.parameters.iter().map(|p| p.name.as_str()).collect();

    assert!(
        param_names.contains(&"project_slug"),
        "Missing project_slug parameter"
    );
    assert!(
        param_names.contains(&"sender_name"),
        "Missing sender_name parameter"
    );
    assert!(param_names.contains(&"to"), "Missing to parameter");
    assert!(
        param_names.contains(&"subject"),
        "Missing subject parameter"
    );
    assert!(
        param_names.contains(&"body_md"),
        "Missing body_md parameter"
    );
}

/// Test that required/optional fields are correctly detected from Option<T>
#[test]
fn test_required_fields_marked_correctly() {
    let schema = schema_from_params::<SendMessageParams>("send_message", "test");

    // project_slug is NOT Option<T>, should be required
    let project_slug = schema
        .parameters
        .iter()
        .find(|p| p.name == "project_slug")
        .expect("project_slug parameter not found");
    assert!(
        project_slug.required,
        "project_slug should be required (non-Option field)"
    );

    // sender_name is NOT Option<T>, should be required
    let sender_name = schema
        .parameters
        .iter()
        .find(|p| p.name == "sender_name")
        .expect("sender_name parameter not found");
    assert!(
        sender_name.required,
        "sender_name should be required (non-Option field)"
    );

    // importance is Option<String>, should be optional
    let importance = schema
        .parameters
        .iter()
        .find(|p| p.name == "importance")
        .expect("importance parameter not found");
    assert!(
        !importance.required,
        "importance should be optional (Option field)"
    );

    // cc is Option<String>, should be optional
    let cc = schema
        .parameters
        .iter()
        .find(|p| p.name == "cc")
        .expect("cc parameter not found");
    assert!(!cc.required, "cc should be optional (Option field)");
}

/// Test that descriptions are extracted from doc comments
#[test]
fn test_descriptions_from_doc_comments() {
    let schema = schema_from_params::<RegisterAgentParams>("register_agent", "test");

    // project_slug has doc comment: "Project slug the agent belongs to"
    let project_slug = schema
        .parameters
        .iter()
        .find(|p| p.name == "project_slug")
        .expect("project_slug parameter not found");

    assert!(
        !project_slug.description.is_empty(),
        "project_slug should have a description from doc comment"
    );
    // The description should contain something meaningful about project
    assert!(
        project_slug.description.to_lowercase().contains("project"),
        "project_slug description should mention 'project', got: {}",
        project_slug.description
    );
}

/// Test that schema count matches expected number of tools (50+)
#[test]
fn test_schema_count_matches_expected() {
    let schemas = lib_mcp::get_tool_schemas(true);

    // Should have a reasonable number of tools (50+)
    assert!(
        schemas.len() >= 50,
        "Expected at least 50 tool schemas, got {}",
        schemas.len()
    );

    // Verify some known tools exist
    let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"send_message"), "Missing send_message tool");
    assert!(
        names.contains(&"register_agent"),
        "Missing register_agent tool"
    );
    assert!(
        names.contains(&"list_projects"),
        "Missing list_projects tool"
    );
    assert!(
        names.contains(&"ensure_project"),
        "Missing ensure_project tool"
    );
    assert!(names.contains(&"check_inbox"), "Missing check_inbox tool");
    assert!(
        names.contains(&"reply_message"),
        "Missing reply_message tool"
    );
}

/// Test that build slot tools are filtered when worktrees_enabled is false
#[test]
fn test_build_slot_tools_filtered_when_disabled() {
    let schemas_with_build = lib_mcp::get_tool_schemas(true);
    let schemas_without_build = lib_mcp::get_tool_schemas(false);

    // With worktrees enabled, should have build slot tools
    let names_with: Vec<&str> = schemas_with_build.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names_with.contains(&"acquire_build_slot"),
        "acquire_build_slot should be present when worktrees_enabled=true"
    );

    // Without worktrees, should NOT have build slot tools
    let names_without: Vec<&str> = schemas_without_build
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert!(
        !names_without.contains(&"acquire_build_slot"),
        "acquire_build_slot should NOT be present when worktrees_enabled=false"
    );
    assert!(
        !names_without.contains(&"release_build_slot"),
        "release_build_slot should NOT be present when worktrees_enabled=false"
    );
    assert!(
        !names_without.contains(&"renew_build_slot"),
        "renew_build_slot should NOT be present when worktrees_enabled=false"
    );

    // Filtered list should have fewer tools
    assert!(
        schemas_without_build.len() < schemas_with_build.len(),
        "Filtered schema count should be less than full count"
    );
}

/// Test that parameter types are correctly extracted
#[test]
fn test_parameter_types_extracted() {
    let schema = schema_from_params::<SendMessageParams>("send_message", "test");

    // String fields should have type "string"
    let project_slug = schema
        .parameters
        .iter()
        .find(|p| p.name == "project_slug")
        .expect("project_slug not found");
    assert_eq!(
        project_slug.param_type, "string",
        "project_slug should be type 'string'"
    );

    let subject = schema
        .parameters
        .iter()
        .find(|p| p.name == "subject")
        .expect("subject not found");
    assert_eq!(
        subject.param_type, "string",
        "subject should be type 'string'"
    );
}

/// Test that parameters are sorted alphabetically (consistent ordering)
#[test]
fn test_parameters_sorted_alphabetically() {
    let schema = schema_from_params::<SendMessageParams>("send_message", "test");

    let names: Vec<&str> = schema.parameters.iter().map(|p| p.name.as_str()).collect();
    let mut sorted_names = names.clone();
    sorted_names.sort();

    assert_eq!(
        names, sorted_names,
        "Parameters should be sorted alphabetically for consistent output"
    );
}
