mod common;

use common::TestContext;
use lib_core::model::agent::{AgentBmc, AgentForCreate};
use lib_core::model::escalation::{EscalationBmc, EscalationMode};
use lib_core::model::message::{MessageBmc, MessageForCreate};
use lib_core::model::project::ProjectBmc;
use uuid::Uuid;

#[tokio::test]
async fn test_list_overdue_acks() -> lib_core::Result<()> {
    let tc = TestContext::new().await?;
    let mm = &tc.mm;
    let ctx = &tc.ctx;

    // 1. Setup Data
    let project_slug = Uuid::new_v4().to_string();
    let project_id = ProjectBmc::create(&ctx, &mm, &project_slug, "Escalation Test").await?;

    let sender = AgentBmc::create(
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
    .await?;

    let recipient = AgentBmc::create(
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
    .await?;

    // 2. Create Overdue Message (Older than 24h)
    let msg_c = MessageForCreate {
        project_id,
        sender_id: sender,
        recipient_ids: vec![recipient],
        cc_ids: None,
        bcc_ids: None,
        subject: "Overdue Subject".to_string(),
        body_md: "Must ack".to_string(),
        thread_id: None,
        importance: None,
        ack_required: true,
    };
    let overdue_msg_id = MessageBmc::create(&ctx, &mm, msg_c).await?;

    // Backdate it
    let db = mm.db_for_test();
    db.execute(
        "UPDATE messages SET created_ts = datetime('now', '-25 hours') WHERE id = ?",
        [overdue_msg_id],
    )
    .await?;

    // 3. Create Recent Message (Not Overdue)
    let msg_recent = MessageForCreate {
        project_id,
        sender_id: sender,
        recipient_ids: vec![recipient],
        cc_ids: None,
        bcc_ids: None,
        subject: "Recent Subject".to_string(),
        body_md: "Must ack but recent".to_string(),
        thread_id: None,
        importance: None,
        ack_required: true,
    };
    let _recent_msg_id = MessageBmc::create(&ctx, &mm, msg_recent).await?;

    // 4. Create Acked Message (Old but Acked)
    let msg_acked = MessageForCreate {
        project_id,
        sender_id: sender,
        recipient_ids: vec![recipient],
        cc_ids: None,
        bcc_ids: None,
        subject: "Acked Subject".to_string(),
        body_md: "Already acked".to_string(),
        thread_id: None,
        importance: None,
        ack_required: true,
    };
    let acked_msg_id = MessageBmc::create(&ctx, &mm, msg_acked).await?;
    // Backdate
    db.execute(
        "UPDATE messages SET created_ts = datetime('now', '-25 hours') WHERE id = ?",
        [acked_msg_id],
    )
    .await?;
    // Ack it
    MessageBmc::acknowledge(&ctx, &mm, acked_msg_id, recipient).await?;

    // 5. Create Non-Ack-Required Message (Old but no ack required)
    let msg_no_ack = MessageForCreate {
        project_id,
        sender_id: sender,
        recipient_ids: vec![recipient],
        cc_ids: None,
        bcc_ids: None,
        subject: "No Ack Subject".to_string(),
        body_md: "No ack needed".to_string(),
        thread_id: None,
        importance: None,
        ack_required: false,
    };
    let no_ack_msg_id = MessageBmc::create(&ctx, &mm, msg_no_ack).await?;
    db.execute(
        "UPDATE messages SET created_ts = datetime('now', '-25 hours') WHERE id = ?",
        [no_ack_msg_id],
    )
    .await?;

    // Run Query using 24 hour threshold
    let overdue_list = MessageBmc::list_overdue_acks(&ctx, &mm, 24).await?;

    // Verify
    assert_eq!(
        overdue_list.len(),
        1,
        "Should find exactly 1 overdue message, found {}",
        overdue_list.len()
    );
    assert_eq!(overdue_list[0].message_id, overdue_msg_id);
    assert_eq!(overdue_list[0].subject, "Overdue Subject");

    Ok(())
}

#[tokio::test]
async fn test_escalate_overdue_log_mode_dry_run() -> lib_core::Result<()> {
    let tc = TestContext::new().await?;
    let mm = &tc.mm;
    let ctx = &tc.ctx;

    let project_slug = Uuid::new_v4().to_string();
    let project_id = ProjectBmc::create(ctx, mm, &project_slug, "Escalation Log Test").await?;

    let sender = AgentBmc::create(
        ctx,
        mm,
        AgentForCreate {
            project_id,
            name: "sender".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: String::new(),
        },
    )
    .await?;

    let recipient = AgentBmc::create(
        ctx,
        mm,
        AgentForCreate {
            project_id,
            name: "recipient".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: String::new(),
        },
    )
    .await?;

    let msg_c = MessageForCreate {
        project_id,
        sender_id: sender,
        recipient_ids: vec![recipient],
        cc_ids: None,
        bcc_ids: None,
        subject: "Test Escalation".to_string(),
        body_md: "Needs ack".to_string(),
        thread_id: None,
        importance: None,
        ack_required: true,
    };
    let msg_id = MessageBmc::create(ctx, mm, msg_c).await?;

    let db = mm.db_for_test();
    db.execute(
        "UPDATE messages SET created_ts = datetime('now', '-25 hours') WHERE id = ?",
        [msg_id],
    )
    .await?;

    let results = EscalationBmc::escalate_overdue(ctx, mm, 24, EscalationMode::Log, true).await?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].message_id, msg_id);
    assert_eq!(results[0].action_taken, "log_dry_run");
    assert!(results[0].success);

    Ok(())
}

#[tokio::test]
async fn test_escalate_overdue_log_mode_real() -> lib_core::Result<()> {
    let tc = TestContext::new().await?;
    let mm = &tc.mm;
    let ctx = &tc.ctx;

    let project_slug = Uuid::new_v4().to_string();
    let project_id = ProjectBmc::create(ctx, mm, &project_slug, "Escalation Log Real").await?;

    let sender = AgentBmc::create(
        ctx,
        mm,
        AgentForCreate {
            project_id,
            name: "sender".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: String::new(),
        },
    )
    .await?;

    let recipient = AgentBmc::create(
        ctx,
        mm,
        AgentForCreate {
            project_id,
            name: "recipient".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: String::new(),
        },
    )
    .await?;

    let msg_c = MessageForCreate {
        project_id,
        sender_id: sender,
        recipient_ids: vec![recipient],
        cc_ids: None,
        bcc_ids: None,
        subject: "Log Real Test".to_string(),
        body_md: "Needs ack".to_string(),
        thread_id: None,
        importance: None,
        ack_required: true,
    };
    let msg_id = MessageBmc::create(ctx, mm, msg_c).await?;

    let db = mm.db_for_test();
    db.execute(
        "UPDATE messages SET created_ts = datetime('now', '-25 hours') WHERE id = ?",
        [msg_id],
    )
    .await?;

    let results = EscalationBmc::escalate_overdue(ctx, mm, 24, EscalationMode::Log, false).await?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].message_id, msg_id);
    assert_eq!(results[0].action_taken, "logged");
    assert!(results[0].success);
    assert!(
        results[0]
            .details
            .as_ref()
            .unwrap()
            .contains("Log Real Test")
    );

    Ok(())
}

#[tokio::test]
async fn test_escalate_overdue_no_overdue_messages() -> lib_core::Result<()> {
    let tc = TestContext::new().await?;
    let mm = &tc.mm;
    let ctx = &tc.ctx;

    let project_slug = Uuid::new_v4().to_string();
    let project_id = ProjectBmc::create(ctx, mm, &project_slug, "No Overdue Test").await?;

    let sender = AgentBmc::create(
        ctx,
        mm,
        AgentForCreate {
            project_id,
            name: "sender".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: String::new(),
        },
    )
    .await?;

    let recipient = AgentBmc::create(
        ctx,
        mm,
        AgentForCreate {
            project_id,
            name: "recipient".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: String::new(),
        },
    )
    .await?;

    let msg_c = MessageForCreate {
        project_id,
        sender_id: sender,
        recipient_ids: vec![recipient],
        cc_ids: None,
        bcc_ids: None,
        subject: "Recent Message".to_string(),
        body_md: "Fresh message".to_string(),
        thread_id: None,
        importance: None,
        ack_required: true,
    };
    let _msg_id = MessageBmc::create(ctx, mm, msg_c).await?;

    let results = EscalationBmc::escalate_overdue(ctx, mm, 24, EscalationMode::Log, false).await?;

    assert!(results.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_escalate_overdue_overseer_mode_dry_run() -> lib_core::Result<()> {
    let tc = TestContext::new().await?;
    let mm = &tc.mm;
    let ctx = &tc.ctx;

    let project_slug = Uuid::new_v4().to_string();
    let project_id = ProjectBmc::create(ctx, mm, &project_slug, "Overseer Test").await?;

    let sender = AgentBmc::create(
        ctx,
        mm,
        AgentForCreate {
            project_id,
            name: "sender".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: String::new(),
        },
    )
    .await?;

    let recipient = AgentBmc::create(
        ctx,
        mm,
        AgentForCreate {
            project_id,
            name: "recipient".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: String::new(),
        },
    )
    .await?;

    let msg_c = MessageForCreate {
        project_id,
        sender_id: sender,
        recipient_ids: vec![recipient],
        cc_ids: None,
        bcc_ids: None,
        subject: "Overseer Dry Run".to_string(),
        body_md: "Needs ack".to_string(),
        thread_id: None,
        importance: None,
        ack_required: true,
    };
    let msg_id = MessageBmc::create(ctx, mm, msg_c).await?;

    let db = mm.db_for_test();
    db.execute(
        "UPDATE messages SET created_ts = datetime('now', '-25 hours') WHERE id = ?",
        [msg_id],
    )
    .await?;

    let results =
        EscalationBmc::escalate_overdue(ctx, mm, 24, EscalationMode::Overseer, true).await?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].action_taken, "overseer_dry_run");
    assert!(results[0].success);

    Ok(())
}

#[tokio::test]
async fn test_escalate_overdue_overseer_mode_real() -> lib_core::Result<()> {
    let tc = TestContext::new().await?;
    let mm = &tc.mm;
    let ctx = &tc.ctx;

    let project_slug = Uuid::new_v4().to_string();
    let project_id = ProjectBmc::create(ctx, mm, &project_slug, "Overseer Real Test").await?;

    let sender = AgentBmc::create(
        ctx,
        mm,
        AgentForCreate {
            project_id,
            name: "sender".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: String::new(),
        },
    )
    .await?;

    let recipient = AgentBmc::create(
        ctx,
        mm,
        AgentForCreate {
            project_id,
            name: "recipient".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: String::new(),
        },
    )
    .await?;

    let msg_c = MessageForCreate {
        project_id,
        sender_id: sender,
        recipient_ids: vec![recipient],
        cc_ids: None,
        bcc_ids: None,
        subject: "Overseer Real".to_string(),
        body_md: "Needs ack".to_string(),
        thread_id: None,
        importance: None,
        ack_required: true,
    };
    let msg_id = MessageBmc::create(ctx, mm, msg_c).await?;

    let db = mm.db_for_test();
    db.execute(
        "UPDATE messages SET created_ts = datetime('now', '-25 hours') WHERE id = ?",
        [msg_id],
    )
    .await?;

    let results =
        EscalationBmc::escalate_overdue(ctx, mm, 24, EscalationMode::Overseer, false).await?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].action_taken, "overseer_message_sent");
    assert!(results[0].success);
    assert!(
        results[0]
            .details
            .as_ref()
            .unwrap()
            .contains("Overseer message ID")
    );

    Ok(())
}

#[tokio::test]
async fn test_escalate_overdue_file_reservation_dry_run() -> lib_core::Result<()> {
    let tc = TestContext::new().await?;
    let mm = &tc.mm;
    let ctx = &tc.ctx;

    let project_slug = Uuid::new_v4().to_string();
    let project_id = ProjectBmc::create(ctx, mm, &project_slug, "FileRes Dry Test").await?;

    let sender = AgentBmc::create(
        ctx,
        mm,
        AgentForCreate {
            project_id,
            name: "sender".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: String::new(),
        },
    )
    .await?;

    let recipient = AgentBmc::create(
        ctx,
        mm,
        AgentForCreate {
            project_id,
            name: "recipient".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: String::new(),
        },
    )
    .await?;

    let msg_c = MessageForCreate {
        project_id,
        sender_id: sender,
        recipient_ids: vec![recipient],
        cc_ids: None,
        bcc_ids: None,
        subject: "FileRes Dry".to_string(),
        body_md: "Needs ack".to_string(),
        thread_id: None,
        importance: None,
        ack_required: true,
    };
    let msg_id = MessageBmc::create(ctx, mm, msg_c).await?;

    let db = mm.db_for_test();
    db.execute(
        "UPDATE messages SET created_ts = datetime('now', '-25 hours') WHERE id = ?",
        [msg_id],
    )
    .await?;

    let results =
        EscalationBmc::escalate_overdue(ctx, mm, 24, EscalationMode::FileReservation, true).await?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].action_taken, "file_reservation_dry_run");
    assert!(results[0].success);
    assert!(
        results[0]
            .details
            .as_ref()
            .unwrap()
            .contains("Would reserve files")
    );

    Ok(())
}

#[tokio::test]
async fn test_escalate_overdue_file_reservation_real() -> lib_core::Result<()> {
    let tc = TestContext::new().await?;
    let mm = &tc.mm;
    let ctx = &tc.ctx;

    let project_slug = Uuid::new_v4().to_string();
    let project_id = ProjectBmc::create(ctx, mm, &project_slug, "FileRes Real Test").await?;

    let sender = AgentBmc::create(
        ctx,
        mm,
        AgentForCreate {
            project_id,
            name: "sender".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: String::new(),
        },
    )
    .await?;

    let recipient = AgentBmc::create(
        ctx,
        mm,
        AgentForCreate {
            project_id,
            name: "recipient".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: String::new(),
        },
    )
    .await?;

    let msg_c = MessageForCreate {
        project_id,
        sender_id: sender,
        recipient_ids: vec![recipient],
        cc_ids: None,
        bcc_ids: None,
        subject: "FileRes Real".to_string(),
        body_md: "Needs ack".to_string(),
        thread_id: None,
        importance: None,
        ack_required: true,
    };
    let msg_id = MessageBmc::create(ctx, mm, msg_c).await?;

    let db = mm.db_for_test();
    db.execute(
        "UPDATE messages SET created_ts = datetime('now', '-25 hours') WHERE id = ?",
        [msg_id],
    )
    .await?;

    let results =
        EscalationBmc::escalate_overdue(ctx, mm, 24, EscalationMode::FileReservation, false)
            .await?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].action_taken, "file_reservation_created");
    assert!(results[0].success);
    assert!(
        results[0]
            .details
            .as_ref()
            .unwrap()
            .contains("Reservation ID")
    );

    Ok(())
}

#[tokio::test]
async fn test_send_reminder() -> lib_core::Result<()> {
    let tc = TestContext::new().await?;
    let mm = &tc.mm;
    let ctx = &tc.ctx;

    let project_slug = Uuid::new_v4().to_string();
    let project_id = ProjectBmc::create(ctx, mm, &project_slug, "Reminder Test").await?;

    let sender = AgentBmc::create(
        ctx,
        mm,
        AgentForCreate {
            project_id,
            name: "sender".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: String::new(),
        },
    )
    .await?;

    let recipient = AgentBmc::create(
        ctx,
        mm,
        AgentForCreate {
            project_id,
            name: "recipient".to_string(),
            program: "test".to_string(),
            model: "test".to_string(),
            task_description: String::new(),
        },
    )
    .await?;

    // Create overdue message manually
    use lib_core::model::message::OverdueMessage;
    let overdue = OverdueMessage {
        message_id: 999,
        project_id,
        sender_id: sender,
        subject: "Original Subject".to_string(),
        sender_name: "sender".to_string(),
        recipient_id: recipient,
        recipient_name: "recipient".to_string(),
        created_ts: chrono::NaiveDateTime::default(),
    };

    let reminder_id = EscalationBmc::send_reminder(ctx, mm, &overdue).await?;
    assert!(reminder_id > 0);

    // Verify the reminder message was created
    let reminder = MessageBmc::get(ctx, mm, reminder_id).await?;
    assert!(reminder.subject.starts_with("REMINDER:"));
    assert!(reminder.body_md.contains("System Escalation"));
    assert!(reminder.ack_required);

    Ok(())
}
