use lib_core::ctx::Ctx;
use lib_core::model::ModelManager;
use lib_core::model::agent::{AgentBmc, AgentForCreate};
use lib_core::model::message::{MessageBmc, MessageForCreate};
use lib_core::model::project::ProjectBmc;
use uuid::Uuid;

#[tokio::test]
async fn test_list_overdue_acks() -> lib_core::Result<()> {
    let mm = ModelManager::new().await?;
    let ctx = Ctx::root_ctx();

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
