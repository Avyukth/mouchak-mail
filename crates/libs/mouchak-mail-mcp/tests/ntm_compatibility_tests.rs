//! Tests for NTM (ntm Go client) compatibility layer
//!
//! These tests verify that tool aliases and schema definitions are correctly
//! configured for compatibility with the ntm Go client.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use mouchak_mail_mcp::tools::get_tool_schemas;

#[test]
fn test_whois_schema_exists() {
    let schemas = get_tool_schemas(true);
    let whois = schemas.iter().find(|s| s.name == "whois");

    assert!(whois.is_some(), "whois schema should exist");

    let schema = whois.unwrap();
    assert!(
        schema.description.contains("agent"),
        "whois should describe agent lookup"
    );

    let param_names: Vec<&str> = schema.parameters.iter().map(|p| p.name.as_str()).collect();
    assert!(
        param_names.contains(&"project_slug"),
        "whois should have project_slug param"
    );
    assert!(
        param_names.contains(&"agent_name"),
        "whois should have agent_name param"
    );
}

#[test]
fn test_fetch_inbox_alias_schema_exists() {
    let schemas = get_tool_schemas(true);
    let fetch_inbox = schemas.iter().find(|s| s.name == "fetch_inbox");

    assert!(
        fetch_inbox.is_some(),
        "fetch_inbox alias schema should exist for NTM compatibility"
    );

    let schema = fetch_inbox.unwrap();
    assert!(
        schema.description.contains("inbox") || schema.description.contains("Alias"),
        "fetch_inbox should describe inbox functionality"
    );

    let param_names: Vec<&str> = schema.parameters.iter().map(|p| p.name.as_str()).collect();
    assert!(param_names.contains(&"project_slug"));
    assert!(param_names.contains(&"agent_name"));
}

#[test]
fn test_release_file_reservations_alias_schema_exists() {
    let schemas = get_tool_schemas(true);
    let release = schemas
        .iter()
        .find(|s| s.name == "release_file_reservations");

    assert!(
        release.is_some(),
        "release_file_reservations alias schema should exist for NTM compatibility"
    );

    let schema = release.unwrap();
    let param_names: Vec<&str> = schema.parameters.iter().map(|p| p.name.as_str()).collect();
    assert!(param_names.contains(&"project_slug"));
    assert!(param_names.contains(&"agent_name"));
}

#[test]
fn test_renew_file_reservations_alias_schema_exists() {
    let schemas = get_tool_schemas(true);
    let renew = schemas.iter().find(|s| s.name == "renew_file_reservations");

    assert!(
        renew.is_some(),
        "renew_file_reservations alias schema should exist for NTM compatibility"
    );

    let schema = renew.unwrap();
    let param_names: Vec<&str> = schema.parameters.iter().map(|p| p.name.as_str()).collect();
    assert!(param_names.contains(&"project_slug"));
    assert!(param_names.contains(&"agent_name"));
}

#[test]
fn test_check_inbox_schema_exists() {
    let schemas = get_tool_schemas(true);
    let check_inbox = schemas.iter().find(|s| s.name == "check_inbox");

    assert!(check_inbox.is_some(), "check_inbox schema should exist");
}

#[test]
fn test_total_tool_count_includes_aliases() {
    let schemas = get_tool_schemas(true);

    let has_whois = schemas.iter().any(|s| s.name == "whois");
    let has_fetch_inbox = schemas.iter().any(|s| s.name == "fetch_inbox");
    let has_release_alias = schemas
        .iter()
        .any(|s| s.name == "release_file_reservations");
    let has_renew_alias = schemas.iter().any(|s| s.name == "renew_file_reservations");

    assert!(has_whois, "whois tool should be in schema");
    assert!(has_fetch_inbox, "fetch_inbox alias should be in schema");
    assert!(
        has_release_alias,
        "release_file_reservations alias should be in schema"
    );
    assert!(
        has_renew_alias,
        "renew_file_reservations alias should be in schema"
    );
}

#[test]
fn test_list_project_agents_alias_schema_exists() {
    let schemas = get_tool_schemas(true);
    let list_project_agents = schemas.iter().find(|s| s.name == "list_project_agents");

    assert!(
        list_project_agents.is_some(),
        "list_project_agents alias schema should exist for NTM compatibility"
    );

    let schema = list_project_agents.unwrap();
    assert!(
        schema.description.contains("agents") || schema.description.contains("Alias"),
        "list_project_agents should describe agent listing"
    );

    let param_names: Vec<&str> = schema.parameters.iter().map(|p| p.name.as_str()).collect();
    assert!(param_names.contains(&"project_slug"));
}

#[test]
fn test_create_agent_identity_schema_exists() {
    let schemas = get_tool_schemas(true);
    let create_identity = schemas.iter().find(|s| s.name == "create_agent_identity");

    assert!(
        create_identity.is_some(),
        "create_agent_identity schema should exist for NTM compatibility"
    );

    let schema = create_identity.unwrap();
    let param_names: Vec<&str> = schema.parameters.iter().map(|p| p.name.as_str()).collect();
    assert!(param_names.contains(&"project_slug"));
}

#[test]
fn test_macro_start_session_schema_exists() {
    let schemas = get_tool_schemas(true);
    let macro_session = schemas.iter().find(|s| s.name == "macro_start_session");

    assert!(
        macro_session.is_some(),
        "macro_start_session schema should exist for NTM compatibility"
    );

    let schema = macro_session.unwrap();
    let param_names: Vec<&str> = schema.parameters.iter().map(|p| p.name.as_str()).collect();
    assert!(param_names.contains(&"human_key"));
    assert!(param_names.contains(&"program"));
    assert!(param_names.contains(&"model"));
}
