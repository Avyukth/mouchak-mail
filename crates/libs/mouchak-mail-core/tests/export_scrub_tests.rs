#![allow(clippy::unwrap_used, clippy::expect_used)]

use mouchak_mail_core::ctx::Ctx;
use mouchak_mail_core::model::ModelManager;
use mouchak_mail_core::model::agent::{AgentBmc, AgentForCreate};
use mouchak_mail_core::model::export::{ExportBmc, ExportFormat, ScrubMode};
use mouchak_mail_core::model::message::{MessageBmc, MessageForCreate};
use mouchak_mail_core::model::project::ProjectBmc;
use uuid::Uuid;

#[tokio::test]
async fn test_export_scrubbing() -> mouchak_mail_core::Result<()> {
    let mm =
        ModelManager::new(std::sync::Arc::new(mouchak_mail_common::config::AppConfig::default())).await?;
    let ctx = Ctx::root_ctx();

    // 1. Setup
    let project_slug = Uuid::new_v4().to_string();
    let project_id = ProjectBmc::create(&ctx, &mm, &project_slug, "Export Test").await?;

    let sender: i64 = AgentBmc::create(
        &ctx,
        &mm,
        AgentForCreate {
            project_id,
            name: "sender".to_string(),
            program: "default".to_string(),
            model: "gpt-4".to_string(),
            task_description: "".to_string(),
        },
    )
    .await?
    .into();

    let recipient: i64 = AgentBmc::create(
        &ctx,
        &mm,
        AgentForCreate {
            project_id,
            name: "recipient".to_string(),
            program: "default".to_string(),
            model: "gpt-4".to_string(),
            task_description: "".to_string(),
        },
    )
    .await?
    .into();

    // Create a message with PII
    let subject = "Sensitive Info: user@example.com (555) 123-4567";
    let body = "Please process credit card 4111-1111-1111-1111 and SSN 123-45-6789. Contact support@company.com.";

    let msg = MessageForCreate {
        project_id: project_id.into(),
        sender_id: sender,
        recipient_ids: vec![recipient],
        cc_ids: None,
        bcc_ids: None,
        subject: subject.to_string(),
        body_md: body.to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    MessageBmc::create(&ctx, &mm, msg).await?;

    // 2. Test ScrubMode::Standard
    let export_std = ExportBmc::export_mailbox(
        &ctx,
        &mm,
        &project_slug,
        ExportFormat::Markdown,
        ScrubMode::Standard,
        false,
    )
    .await?;

    // Check redaction
    assert!(export_std.content.contains("[EMAIL]"));
    assert!(!export_std.content.contains("user@example.com"));
    assert!(export_std.content.contains("[PHONE]"));
    // Standard does NOT scrub Name or CC
    assert!(export_std.content.contains("sender")); // Name
    assert!(export_std.content.contains("4111-1111-1111-1111")); // CC

    // 3. Test ScrubMode::Aggressive
    let export_agg = ExportBmc::export_mailbox(
        &ctx,
        &mm,
        &project_slug,
        ExportFormat::Markdown,
        ScrubMode::Aggressive,
        false,
    )
    .await?;

    // Check redaction
    assert!(export_agg.content.contains("[EMAIL]"));
    assert!(export_agg.content.contains("[PHONE]"));
    assert!(export_agg.content.contains("[CREDIT-CARD]"));
    assert!(!export_agg.content.contains("4111-1111-1111-1111"));
    assert!(export_agg.content.contains("[SSN]"));
    assert!(!export_agg.content.contains("123-45-6789"));
    assert!(export_agg.content.contains("[REDACTED-NAME]"));
    assert!(!export_agg.content.contains("sender")); // Name redacted in From field?
    // Wait, "sender" is the name. "From: sender | ..." -> "From: [REDACTED-NAME] | ..."

    Ok(())
}
