#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Integration tests for multi-agent orchestration workflows.
//!
//! Tests the complete lifecycle of tasks through the Worker → Reviewer → Human pipeline.
//! Uses real MCP tools and database operations (not mocks).

mod common;

use common::TestContext;
use lib_core::model::agent::{AgentBmc, AgentForCreate};
use lib_core::model::message::{MessageBmc, MessageForCreate};
use lib_core::model::orchestration::{OrchestrationBmc, OrchestrationState, parse_thread_state};
use lib_core::model::project::ProjectBmc;
use lib_core::types::ProjectId;

async fn setup_project(tc: &TestContext, name: &str) -> (ProjectId, String) {
    let slug = format!("test-project-{}", name);
    let human_key = format!("Test Project {}", name);
    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, &human_key)
        .await
        .expect("Failed to create project");
    (project_id, slug)
}

async fn create_agent(tc: &TestContext, project_id: ProjectId, name: &str, role: &str) -> i64 {
    let agent = AgentForCreate {
        project_id,
        name: name.to_string(),
        program: "claude-code".to_string(),
        model: "claude-sonnet-4".to_string(),
        task_description: format!("{} agent for testing", role),
    };
    AgentBmc::create(&tc.ctx, &tc.mm, agent)
        .await
        .expect("Failed to create agent")
        .into()
}

async fn send_message(
    tc: &TestContext,
    project_id: ProjectId,
    sender_id: i64,
    recipient_ids: Vec<i64>,
    cc_ids: Option<Vec<i64>>,
    subject: &str,
    body: &str,
    thread_id: Option<String>,
    ack_required: bool,
) -> i64 {
    let msg = MessageForCreate {
        project_id: project_id.get(),
        sender_id,
        recipient_ids,
        cc_ids,
        bcc_ids: None,
        subject: subject.to_string(),
        body_md: body.to_string(),
        thread_id,
        importance: Some("high".to_string()),
        ack_required,
    };
    MessageBmc::create(&tc.ctx, &tc.mm, msg)
        .await
        .expect("Failed to create message")
}

#[tokio::test]
async fn test_happy_path_worker_reviewer_approved() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, _slug) = setup_project(&tc, "happy-path").await;

    let worker_id = create_agent(&tc, project_id, "worker-1", "worker").await;
    let reviewer_id = create_agent(&tc, project_id, "reviewer", "reviewer").await;
    let human_id = create_agent(&tc, project_id, "human", "human").await;

    let thread_id = "TASK-happy-001".to_string();

    let _completion_msg = send_message(
        &tc,
        project_id,
        worker_id,
        vec![reviewer_id],
        Some(vec![human_id]),
        "[COMPLETION] Implement feature X",
        "Task completed. All tests passing.",
        Some(thread_id.clone()),
        true,
    )
    .await;

    let messages = MessageBmc::list_by_thread(&tc.ctx, &tc.mm, project_id.get(), &thread_id)
        .await
        .expect("Failed to list thread messages");
    assert_eq!(parse_thread_state(&messages), OrchestrationState::Completed);

    let _reviewing_msg = send_message(
        &tc,
        project_id,
        reviewer_id,
        vec![worker_id],
        Some(vec![human_id]),
        "[REVIEWING] Implement feature X",
        "Claiming review. Starting validation...",
        Some(thread_id.clone()),
        false,
    )
    .await;

    let messages = MessageBmc::list_by_thread(&tc.ctx, &tc.mm, project_id.get(), &thread_id)
        .await
        .expect("Failed to list thread messages");
    assert_eq!(parse_thread_state(&messages), OrchestrationState::Reviewing);

    let _approved_msg = send_message(
        &tc,
        project_id,
        reviewer_id,
        vec![worker_id],
        Some(vec![human_id]),
        "[APPROVED] Implement feature X",
        "Review complete. Code looks good. Ready for production.",
        Some(thread_id.clone()),
        false,
    )
    .await;

    let messages = MessageBmc::list_by_thread(&tc.ctx, &tc.mm, project_id.get(), &thread_id)
        .await
        .expect("Failed to list thread messages");
    assert_eq!(parse_thread_state(&messages), OrchestrationState::Approved);

    assert_eq!(messages.len(), 3);

    let completion = &messages[0];
    let approved = &messages[2];
    assert!(completion.subject.starts_with("[COMPLETION]"));
    assert!(approved.subject.starts_with("[APPROVED]"));
}

#[tokio::test]
async fn test_fix_flow_reviewer_applies_fixes() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, _slug) = setup_project(&tc, "fix-flow").await;

    let worker_id = create_agent(&tc, project_id, "worker-fix", "worker").await;
    let reviewer_id = create_agent(&tc, project_id, "reviewer-fix", "reviewer").await;
    let human_id = create_agent(&tc, project_id, "human-fix", "human").await;

    let thread_id = "TASK-fix-001".to_string();

    send_message(
        &tc,
        project_id,
        worker_id,
        vec![reviewer_id],
        Some(vec![human_id]),
        "[COMPLETION] Add validation logic",
        "Task completed.",
        Some(thread_id.clone()),
        true,
    )
    .await;

    send_message(
        &tc,
        project_id,
        reviewer_id,
        vec![worker_id],
        Some(vec![human_id]),
        "[REVIEWING] Add validation logic",
        "Claiming review...",
        Some(thread_id.clone()),
        false,
    )
    .await;

    send_message(
        &tc,
        project_id,
        reviewer_id,
        vec![worker_id],
        Some(vec![human_id]),
        "[FIXED] Add validation logic",
        "Found missing edge case. Applied fix in commit abc123.",
        Some(thread_id.clone()),
        false,
    )
    .await;

    let messages = MessageBmc::list_by_thread(&tc.ctx, &tc.mm, project_id.get(), &thread_id)
        .await
        .expect("Failed to list thread messages");

    assert_eq!(parse_thread_state(&messages), OrchestrationState::Fixed);
    assert_eq!(messages.len(), 3);
}

#[tokio::test]
async fn test_single_agent_fallback_no_reviewer() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, _slug) = setup_project(&tc, "single-agent").await;

    let worker_id = create_agent(&tc, project_id, "worker-solo", "worker").await;
    let human_id = create_agent(&tc, project_id, "human-solo", "human").await;

    let reviewer_exists = AgentBmc::get_by_name(&tc.ctx, &tc.mm, project_id, "reviewer")
        .await
        .is_ok();
    assert!(!reviewer_exists, "Reviewer should not exist in this test");

    let thread_id = "TASK-solo-001".to_string();

    send_message(
        &tc,
        project_id,
        worker_id,
        vec![human_id],
        None,
        "[COMPLETION] Solo task (Self-Reviewed)",
        "No reviewer present. Worker performed self-review.",
        Some(thread_id.clone()),
        true,
    )
    .await;

    let messages = MessageBmc::list_by_thread(&tc.ctx, &tc.mm, project_id.get(), &thread_id)
        .await
        .expect("Failed to list thread messages");

    assert_eq!(messages.len(), 1);
    assert_eq!(parse_thread_state(&messages), OrchestrationState::Completed);

    let inbox = MessageBmc::list_inbox_for_agent(&tc.ctx, &tc.mm, project_id.get(), human_id, 10)
        .await
        .expect("Failed to list human inbox");
    assert_eq!(
        inbox.len(),
        1,
        "Human should receive the completion message"
    );
}

#[tokio::test]
async fn test_review_claim_prevents_duplicates() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, _slug) = setup_project(&tc, "conflict").await;

    let worker_id = create_agent(&tc, project_id, "worker-conflict", "worker").await;
    let reviewer1_id = create_agent(&tc, project_id, "reviewer-1", "reviewer").await;
    let reviewer2_id = create_agent(&tc, project_id, "reviewer-2", "reviewer").await;

    let thread_id = "TASK-conflict-001".to_string();

    send_message(
        &tc,
        project_id,
        worker_id,
        vec![reviewer1_id, reviewer2_id],
        None,
        "[COMPLETION] Contested task",
        "Task ready for review.",
        Some(thread_id.clone()),
        true,
    )
    .await;

    send_message(
        &tc,
        project_id,
        reviewer1_id,
        vec![worker_id],
        None,
        "[REVIEWING] Contested task",
        "Reviewer-1 claiming review.",
        Some(thread_id.clone()),
        false,
    )
    .await;

    let messages = MessageBmc::list_by_thread(&tc.ctx, &tc.mm, project_id.get(), &thread_id)
        .await
        .expect("Failed to list thread messages");

    assert_eq!(parse_thread_state(&messages), OrchestrationState::Reviewing);

    let reviewing_msg = messages
        .iter()
        .find(|m| m.subject.to_uppercase().starts_with("[REVIEWING]"));
    assert!(reviewing_msg.is_some(), "Should have a REVIEWING message");

    let claimer = reviewing_msg.unwrap();
    assert_eq!(claimer.sender_name, "reviewer-1");
}

#[tokio::test]
async fn test_abandoned_task_detection() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, _slug) = setup_project(&tc, "abandoned").await;

    let worker_id = create_agent(&tc, project_id, "worker-abandoned", "worker").await;
    let reviewer_id = create_agent(&tc, project_id, "reviewer-abandoned", "reviewer").await;

    let thread_id = "TASK-abandoned-001".to_string();

    send_message(
        &tc,
        project_id,
        worker_id,
        vec![reviewer_id],
        None,
        "[TASK_STARTED] Abandoned task",
        "Task starting but will be abandoned.",
        Some(thread_id.clone()),
        true,
    )
    .await;

    let messages = MessageBmc::list_by_thread(&tc.ctx, &tc.mm, project_id.get(), &thread_id)
        .await
        .expect("Failed to list thread messages");

    assert_eq!(parse_thread_state(&messages), OrchestrationState::Started);

    let abandoned = OrchestrationBmc::find_abandoned_tasks(
        &tc.ctx,
        &tc.mm,
        project_id.get(),
        std::time::Duration::from_secs(0),
    )
    .await
    .expect("Failed to find abandoned tasks");

    assert!(
        abandoned.iter().any(|t| t.thread_id == thread_id),
        "Should detect the task as abandoned (no REVIEWING received)"
    );
}

#[tokio::test]
async fn test_full_state_machine_transitions() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, _slug) = setup_project(&tc, "state-machine").await;

    let worker_id = create_agent(&tc, project_id, "worker-sm", "worker").await;
    let reviewer_id = create_agent(&tc, project_id, "reviewer-sm", "reviewer").await;
    let human_id = create_agent(&tc, project_id, "human-sm", "human").await;

    let thread_id = "TASK-sm-001".to_string();

    send_message(
        &tc,
        project_id,
        worker_id,
        vec![reviewer_id],
        None,
        "[TASK_STARTED] Complex task",
        "Starting work on complex task.",
        Some(thread_id.clone()),
        false,
    )
    .await;

    let messages = MessageBmc::list_by_thread(&tc.ctx, &tc.mm, project_id.get(), &thread_id)
        .await
        .unwrap();
    assert_eq!(parse_thread_state(&messages), OrchestrationState::Started);

    send_message(
        &tc,
        project_id,
        worker_id,
        vec![reviewer_id],
        Some(vec![human_id]),
        "[COMPLETION] Complex task",
        "Work complete.",
        Some(thread_id.clone()),
        true,
    )
    .await;

    let messages = MessageBmc::list_by_thread(&tc.ctx, &tc.mm, project_id.get(), &thread_id)
        .await
        .unwrap();
    assert_eq!(parse_thread_state(&messages), OrchestrationState::Completed);

    send_message(
        &tc,
        project_id,
        reviewer_id,
        vec![worker_id],
        Some(vec![human_id]),
        "[REVIEWING] Complex task",
        "Starting review.",
        Some(thread_id.clone()),
        false,
    )
    .await;

    let messages = MessageBmc::list_by_thread(&tc.ctx, &tc.mm, project_id.get(), &thread_id)
        .await
        .unwrap();
    assert_eq!(parse_thread_state(&messages), OrchestrationState::Reviewing);

    send_message(
        &tc,
        project_id,
        reviewer_id,
        vec![worker_id],
        Some(vec![human_id]),
        "[REJECTED] Complex task",
        "Found issues. Please fix.",
        Some(thread_id.clone()),
        false,
    )
    .await;

    let messages = MessageBmc::list_by_thread(&tc.ctx, &tc.mm, project_id.get(), &thread_id)
        .await
        .unwrap();
    assert_eq!(parse_thread_state(&messages), OrchestrationState::Rejected);

    send_message(
        &tc,
        project_id,
        reviewer_id,
        vec![worker_id],
        Some(vec![human_id]),
        "[FIXED] Complex task",
        "Applied fixes.",
        Some(thread_id.clone()),
        false,
    )
    .await;

    let messages = MessageBmc::list_by_thread(&tc.ctx, &tc.mm, project_id.get(), &thread_id)
        .await
        .unwrap();
    assert_eq!(parse_thread_state(&messages), OrchestrationState::Fixed);

    send_message(
        &tc,
        project_id,
        reviewer_id,
        vec![worker_id],
        Some(vec![human_id]),
        "[APPROVED] Complex task",
        "All good now.",
        Some(thread_id.clone()),
        false,
    )
    .await;

    let messages = MessageBmc::list_by_thread(&tc.ctx, &tc.mm, project_id.get(), &thread_id)
        .await
        .unwrap();
    assert_eq!(parse_thread_state(&messages), OrchestrationState::Approved);

    send_message(
        &tc,
        project_id,
        human_id,
        vec![reviewer_id],
        Some(vec![worker_id]),
        "[ACK] Complex task",
        "Acknowledged. Task complete.",
        Some(thread_id.clone()),
        false,
    )
    .await;

    let messages = MessageBmc::list_by_thread(&tc.ctx, &tc.mm, project_id.get(), &thread_id)
        .await
        .unwrap();
    assert_eq!(
        parse_thread_state(&messages),
        OrchestrationState::Acknowledged
    );

    assert_eq!(messages.len(), 7, "Should have 7 messages in full cycle");
}

#[tokio::test]
async fn test_cc_audit_trail_preserved() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, _slug) = setup_project(&tc, "audit-trail").await;

    let worker_id = create_agent(&tc, project_id, "worker-audit", "worker").await;
    let reviewer_id = create_agent(&tc, project_id, "reviewer-audit", "reviewer").await;
    let human_id = create_agent(&tc, project_id, "human-audit", "human").await;

    let thread_id = "TASK-audit-001".to_string();

    let _completion_id = send_message(
        &tc,
        project_id,
        worker_id,
        vec![reviewer_id],
        Some(vec![human_id]),
        "[COMPLETION] Audit test",
        "Task done.",
        Some(thread_id.clone()),
        true,
    )
    .await;

    let human_inbox =
        MessageBmc::list_inbox_for_agent(&tc.ctx, &tc.mm, project_id.get(), human_id, 10)
            .await
            .expect("Failed to list human inbox");

    assert!(
        !human_inbox.is_empty(),
        "Human should see CC'd message in inbox"
    );
}
