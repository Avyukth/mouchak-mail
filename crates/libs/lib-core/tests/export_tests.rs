//! Export model tests
//!
//! Tests for mailbox export functionality in various formats.

#[path = "common/mod.rs"]
mod common;

use crate::common::TestContext;
use lib_core::model::agent::{AgentBmc, AgentForCreate};
use lib_core::model::export::{ExportBmc, ExportFormat};
use lib_core::model::message::{MessageBmc, MessageForCreate};
use lib_core::model::project::ProjectBmc;
use lib_core::utils::slugify;

/// Helper to set up a project with messages for export tests
async fn setup_project_with_messages(tc: &TestContext, suffix: &str) -> (i64, String) {
    let human_key = format!("/test/export-repo-{}", suffix);
    let slug = slugify(&human_key);

    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, &human_key)
        .await
        .expect("Failed to create project");

    // Create sender agent
    let sender = AgentForCreate {
        project_id,
        name: "sender-agent".to_string(),
        program: "claude-code".to_string(),
        model: "claude-3".to_string(),
        task_description: "Sending messages".to_string(),
    };
    let sender_id = AgentBmc::create(&tc.ctx, &tc.mm, sender)
        .await
        .expect("Failed to create sender");

    // Create recipient agent
    let recipient = AgentForCreate {
        project_id,
        name: "recipient-agent".to_string(),
        program: "cursor".to_string(),
        model: "gpt-4".to_string(),
        task_description: "Receiving messages".to_string(),
    };
    let recipient_id = AgentBmc::create(&tc.ctx, &tc.mm, recipient)
        .await
        .expect("Failed to create recipient");

    // Create some messages
    for i in 1..=3 {
        let msg = MessageForCreate {
            project_id,
            sender_id,
            recipient_ids: vec![recipient_id],
            cc_ids: None,
            bcc_ids: None,
            subject: format!("Test Message {}", i),
            body_md: format!("This is the body of message {}.", i),
            thread_id: None,
            importance: None,
        };
        MessageBmc::create(&tc.ctx, &tc.mm, msg)
            .await
            .expect("Failed to create message");
    }

    (project_id, slug)
}

/// Test exporting mailbox in JSON format
#[tokio::test]
async fn test_export_json() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (_, slug) = setup_project_with_messages(&tc, "json").await;

    let exported = ExportBmc::export_mailbox(&tc.ctx, &tc.mm, &slug, ExportFormat::Json, false)
        .await
        .expect("Failed to export mailbox");

    assert_eq!(exported.project_slug, slug);
    assert_eq!(exported.format, "json");
    assert_eq!(exported.message_count, 3);
    assert!(exported.content.contains("Test Message"));
    assert!(
        exported.content.starts_with('['),
        "JSON should start with array"
    );
}

/// Test exporting mailbox in HTML format
#[tokio::test]
async fn test_export_html() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (_, slug) = setup_project_with_messages(&tc, "html").await;

    let exported = ExportBmc::export_mailbox(&tc.ctx, &tc.mm, &slug, ExportFormat::Html, false)
        .await
        .expect("Failed to export mailbox");

    assert_eq!(exported.format, "html");
    assert!(exported.content.contains("<!DOCTYPE html>"));
    assert!(exported.content.contains("<title>"));
    assert!(exported.content.contains("Test Message"));
}

/// Test exporting mailbox in Markdown format
#[tokio::test]
async fn test_export_markdown() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (_, slug) = setup_project_with_messages(&tc, "md").await;

    let exported = ExportBmc::export_mailbox(&tc.ctx, &tc.mm, &slug, ExportFormat::Markdown, false)
        .await
        .expect("Failed to export mailbox");

    assert_eq!(exported.format, "markdown");
    assert!(exported.content.contains("# Mailbox Export"));
    assert!(exported.content.contains("## Test Message"));
    assert!(exported.content.contains("**From:**"));
}

/// Test exporting mailbox in CSV format
#[tokio::test]
async fn test_export_csv() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (_, slug) = setup_project_with_messages(&tc, "csv").await;

    let exported = ExportBmc::export_mailbox(&tc.ctx, &tc.mm, &slug, ExportFormat::Csv, false)
        .await
        .expect("Failed to export mailbox");

    assert_eq!(exported.format, "csv");
    assert!(
        exported
            .content
            .contains("id,created_at,sender,subject,body")
    );
    assert!(exported.content.contains("Test Message"));
}

/// Test exporting empty mailbox
#[tokio::test]
async fn test_export_empty_mailbox() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let human_key = "/test/empty-export-repo";
    let slug = slugify(human_key);

    ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");

    let exported = ExportBmc::export_mailbox(&tc.ctx, &tc.mm, &slug, ExportFormat::Json, false)
        .await
        .expect("Failed to export mailbox");

    assert_eq!(exported.message_count, 0);
    assert_eq!(exported.content, "[]"); // Empty JSON array
}

/// Test export format parsing
#[tokio::test]
async fn test_export_format_parsing() {
    use std::str::FromStr;

    assert_eq!(ExportFormat::from_str("html").unwrap(), ExportFormat::Html);
    assert_eq!(ExportFormat::from_str("HTML").unwrap(), ExportFormat::Html);
    assert_eq!(ExportFormat::from_str("json").unwrap(), ExportFormat::Json);
    assert_eq!(
        ExportFormat::from_str("md").unwrap(),
        ExportFormat::Markdown
    );
    assert_eq!(
        ExportFormat::from_str("markdown").unwrap(),
        ExportFormat::Markdown
    );
    assert_eq!(ExportFormat::from_str("csv").unwrap(), ExportFormat::Csv);
    // Unknown defaults to JSON
    assert_eq!(
        ExportFormat::from_str("unknown").unwrap(),
        ExportFormat::Json
    );
}

/// Test export for nonexistent project
#[tokio::test]
async fn test_export_nonexistent_project() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let result = ExportBmc::export_mailbox(
        &tc.ctx,
        &tc.mm,
        "nonexistent-slug",
        ExportFormat::Json,
        false,
    )
    .await;

    assert!(result.is_err(), "Should fail for nonexistent project");
}

/// Test exported_at timestamp is set
#[tokio::test]
async fn test_export_timestamp() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (_, slug) = setup_project_with_messages(&tc, "timestamp").await;

    let exported = ExportBmc::export_mailbox(&tc.ctx, &tc.mm, &slug, ExportFormat::Json, false)
        .await
        .expect("Failed to export mailbox");

    assert!(!exported.exported_at.is_empty());
    assert!(exported.exported_at.contains("UTC"));
}
