//! File reservation model tests
//!
//! Tests for file reservation CRUD operations - critical for multi-agent coordination.

// Tests are allowed to use unwrap()/expect() for clearer failure messages
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::inefficient_to_string
)]

#[path = "common/mod.rs"]
mod common;

use crate::common::TestContext;
use chrono::{Duration, Utc};
use lib_core::model::agent::{AgentBmc, AgentForCreate};
use lib_core::model::file_reservation::{FileReservationBmc, FileReservationForCreate};
use lib_core::model::project::ProjectBmc;
use lib_core::types::{AgentId, ProjectId};
use lib_core::utils::slugify;

/// Helper to set up a project and agent for file reservation tests
async fn setup_project_and_agent(tc: &TestContext) -> (i64, i64) {
    let human_key = "/test/repo";
    let slug = slugify(human_key);

    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");

    let agent = AgentForCreate {
        project_id: ProjectId(project_id),
        name: "test-agent".to_string(),
        program: "claude-code".to_string(),
        model: "claude-3".to_string(),
        task_description: "Testing file reservations".to_string(),
    };

    let agent_id = AgentBmc::create(&tc.ctx, &tc.mm, agent)
        .await
        .expect("Failed to create agent");

    (project_id, agent_id.into())
}

/// Test creating a file reservation
#[tokio::test]
async fn test_create_file_reservation() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc).await;

    let expires_ts = Utc::now().naive_utc() + Duration::hours(1);
    let fr_c = FileReservationForCreate {
        project_id: ProjectId(project_id),
        agent_id: AgentId(agent_id),
        path_pattern: "src/**/*.rs".to_string(),
        exclusive: true,
        reason: "Editing source files".to_string(),
        expires_ts,
    };

    let reservation_id = FileReservationBmc::create(&tc.ctx, &tc.mm, fr_c)
        .await
        .expect("Failed to create file reservation");

    assert!(reservation_id > 0, "Reservation ID should be positive");
}

/// Test getting a file reservation by ID
#[tokio::test]
async fn test_get_file_reservation() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc).await;

    let expires_ts = Utc::now().naive_utc() + Duration::hours(2);
    let fr_c = FileReservationForCreate {
        project_id: ProjectId(project_id),
        agent_id: AgentId(agent_id),
        path_pattern: "Cargo.toml".to_string(),
        exclusive: true,
        reason: "Updating dependencies".to_string(),
        expires_ts,
    };

    let reservation_id = FileReservationBmc::create(&tc.ctx, &tc.mm, fr_c)
        .await
        .expect("Failed to create file reservation");

    let reservation = FileReservationBmc::get(&tc.ctx, &tc.mm, reservation_id)
        .await
        .expect("Failed to get file reservation");

    assert_eq!(reservation.id, reservation_id);
    assert_eq!(reservation.project_id, ProjectId(project_id));
    assert_eq!(reservation.agent_id, AgentId(agent_id));
    assert_eq!(reservation.path_pattern, "Cargo.toml");
    assert!(reservation.exclusive);
    assert_eq!(reservation.reason, "Updating dependencies");
    assert!(reservation.released_ts.is_none());
}

/// Test listing active file reservations for a project
#[tokio::test]
async fn test_list_active_file_reservations() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc).await;

    // Create multiple reservations
    let expires_ts = Utc::now().naive_utc() + Duration::hours(1);
    for pattern in &["src/*.rs", "tests/*.rs", "docs/*.md"] {
        let fr_c = FileReservationForCreate {
            project_id: ProjectId(project_id),
            agent_id: AgentId(agent_id),
            path_pattern: pattern.to_string(),
            exclusive: true,
            reason: "Testing".to_string(),
            expires_ts,
        };
        FileReservationBmc::create(&tc.ctx, &tc.mm, fr_c)
            .await
            .expect("Failed to create reservation");
    }

    let active =
        FileReservationBmc::list_active_for_project(&tc.ctx, &tc.mm, ProjectId(project_id))
            .await
            .expect("Failed to list active reservations");

    assert_eq!(active.len(), 3, "Should have 3 active reservations");
}

/// Test releasing a file reservation
#[tokio::test]
async fn test_release_file_reservation() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc).await;

    let expires_ts = Utc::now().naive_utc() + Duration::hours(1);
    let fr_c = FileReservationForCreate {
        project_id: ProjectId(project_id),
        agent_id: AgentId(agent_id),
        path_pattern: "README.md".to_string(),
        exclusive: false,
        reason: "Updating docs".to_string(),
        expires_ts,
    };

    let reservation_id = FileReservationBmc::create(&tc.ctx, &tc.mm, fr_c)
        .await
        .expect("Failed to create reservation");

    // Release the reservation
    FileReservationBmc::release(&tc.ctx, &tc.mm, reservation_id)
        .await
        .expect("Failed to release reservation");

    // Verify it's released
    let reservation = FileReservationBmc::get(&tc.ctx, &tc.mm, reservation_id)
        .await
        .expect("Failed to get reservation");

    assert!(
        reservation.released_ts.is_some(),
        "Reservation should have released_ts set"
    );

    // Active list should be empty
    let active =
        FileReservationBmc::list_active_for_project(&tc.ctx, &tc.mm, ProjectId(project_id))
            .await
            .expect("Failed to list active reservations");

    assert_eq!(
        active.len(),
        0,
        "Should have no active reservations after release"
    );
}

/// Test releasing a file reservation by path
#[tokio::test]
async fn test_release_by_path() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc).await;

    let expires_ts = Utc::now().naive_utc() + Duration::hours(1);
    let path_pattern = "lib/**/*.rs";
    let fr_c = FileReservationForCreate {
        project_id: ProjectId(project_id),
        agent_id: AgentId(agent_id),
        path_pattern: path_pattern.to_string(),
        exclusive: true,
        reason: "Refactoring".to_string(),
        expires_ts,
    };

    let reservation_id = FileReservationBmc::create(&tc.ctx, &tc.mm, fr_c)
        .await
        .expect("Failed to create reservation");

    // Release by path
    let released_id =
        FileReservationBmc::release_by_path(&tc.ctx, &tc.mm, project_id, agent_id, path_pattern)
            .await
            .expect("Failed to release by path");

    assert_eq!(
        released_id,
        Some(reservation_id),
        "Should return the released reservation ID"
    );
}

/// Test force releasing a file reservation
#[tokio::test]
async fn test_force_release() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc).await;

    let expires_ts = Utc::now().naive_utc() + Duration::hours(1);
    let fr_c = FileReservationForCreate {
        project_id: ProjectId(project_id),
        agent_id: AgentId(agent_id),
        path_pattern: "config/*.yaml".to_string(),
        exclusive: true,
        reason: "Config update".to_string(),
        expires_ts,
    };

    let reservation_id = FileReservationBmc::create(&tc.ctx, &tc.mm, fr_c)
        .await
        .expect("Failed to create reservation");

    // Force release (emergency override)
    FileReservationBmc::force_release(&tc.ctx, &tc.mm, reservation_id)
        .await
        .expect("Failed to force release");

    // Verify it's released
    let reservation = FileReservationBmc::get(&tc.ctx, &tc.mm, reservation_id)
        .await
        .expect("Failed to get reservation");

    assert!(
        reservation.released_ts.is_some(),
        "Should be released after force_release"
    );
}

/// Test renewing a file reservation
#[tokio::test]
async fn test_renew_file_reservation() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc).await;

    let original_expires = Utc::now().naive_utc() + Duration::hours(1);
    let fr_c = FileReservationForCreate {
        project_id: ProjectId(project_id),
        agent_id: AgentId(agent_id),
        path_pattern: "build/**".to_string(),
        exclusive: true,
        reason: "Build process".to_string(),
        expires_ts: original_expires,
    };

    let reservation_id = FileReservationBmc::create(&tc.ctx, &tc.mm, fr_c)
        .await
        .expect("Failed to create reservation");

    // Renew with extended time
    let new_expires = Utc::now().naive_utc() + Duration::hours(3);
    FileReservationBmc::renew(&tc.ctx, &tc.mm, reservation_id, new_expires)
        .await
        .expect("Failed to renew reservation");

    // Verify the new expiry time (note: datetime comparison may have precision issues)
    let reservation = FileReservationBmc::get(&tc.ctx, &tc.mm, reservation_id)
        .await
        .expect("Failed to get reservation");

    // Check that expires_ts was updated (should be later than original)
    assert!(
        reservation.expires_ts > original_expires,
        "Expiry should be extended after renewal"
    );
}

/// Test listing all reservations (including released)
#[tokio::test]
async fn test_list_all_for_project() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc).await;

    let expires_ts = Utc::now().naive_utc() + Duration::hours(1);

    // Create and release one reservation
    let fr_c = FileReservationForCreate {
        project_id: ProjectId(project_id),
        agent_id: AgentId(agent_id),
        path_pattern: "old/*.rs".to_string(),
        exclusive: false,
        reason: "Old work".to_string(),
        expires_ts,
    };
    let released_id = FileReservationBmc::create(&tc.ctx, &tc.mm, fr_c)
        .await
        .expect("Failed to create reservation");
    FileReservationBmc::release(&tc.ctx, &tc.mm, released_id)
        .await
        .expect("Failed to release");

    // Create an active reservation
    let fr_c2 = FileReservationForCreate {
        project_id: ProjectId(project_id),
        agent_id: AgentId(agent_id),
        path_pattern: "new/*.rs".to_string(),
        exclusive: true,
        reason: "New work".to_string(),
        expires_ts,
    };
    FileReservationBmc::create(&tc.ctx, &tc.mm, fr_c2)
        .await
        .expect("Failed to create reservation");

    // list_all_for_project should return both
    let all = FileReservationBmc::list_all_for_project(&tc.ctx, &tc.mm, project_id)
        .await
        .expect("Failed to list all reservations");

    assert_eq!(
        all.len(),
        2,
        "Should have 2 total reservations (1 released, 1 active)"
    );

    // list_active should return only 1
    let active =
        FileReservationBmc::list_active_for_project(&tc.ctx, &tc.mm, ProjectId(project_id))
            .await
            .expect("Failed to list active reservations");

    assert_eq!(active.len(), 1, "Should have 1 active reservation");
}

/// Test file reservation not found error
#[tokio::test]
async fn test_file_reservation_not_found() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let result = FileReservationBmc::get(&tc.ctx, &tc.mm, 99999).await;

    assert!(
        result.is_err(),
        "Should return error for nonexistent reservation"
    );
}

/// Test listing all active reservations across all projects
#[tokio::test]
async fn test_list_all_active() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Create first project with reservations
    let human_key1 = "/test/project1";
    let slug1 = slugify(human_key1);
    let project1_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug1, human_key1)
        .await
        .expect("Failed to create project 1");

    let agent1 = AgentForCreate {
        project_id: ProjectId(project1_id),
        name: "agent-one".to_string(),
        program: "claude".to_string(),
        model: "claude-3".to_string(),
        task_description: "Agent one".to_string(),
    };
    let agent1_id: i64 = AgentBmc::create(&tc.ctx, &tc.mm, agent1)
        .await
        .expect("Failed to create agent 1")
        .into();

    // Create second project with reservations
    let human_key2 = "/test/project2";
    let slug2 = slugify(human_key2);
    let project2_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug2, human_key2)
        .await
        .expect("Failed to create project 2");

    let agent2 = AgentForCreate {
        project_id: ProjectId(project2_id),
        name: "agent-two".to_string(),
        program: "claude".to_string(),
        model: "claude-3".to_string(),
        task_description: "Agent two".to_string(),
    };
    let agent2_id: i64 = AgentBmc::create(&tc.ctx, &tc.mm, agent2)
        .await
        .expect("Failed to create agent 2")
        .into();

    let expires_ts = Utc::now().naive_utc() + Duration::hours(1);

    // Create reservations in project 1
    let fr1 = FileReservationForCreate {
        project_id: ProjectId(project1_id),
        agent_id: AgentId(agent1_id),
        path_pattern: "src/*.rs".to_string(),
        exclusive: true,
        reason: "Project 1 work".to_string(),
        expires_ts,
    };
    FileReservationBmc::create(&tc.ctx, &tc.mm, fr1)
        .await
        .expect("Failed to create reservation 1");

    // Create reservations in project 2
    let fr2 = FileReservationForCreate {
        project_id: ProjectId(project2_id),
        agent_id: AgentId(agent2_id),
        path_pattern: "lib/*.rs".to_string(),
        exclusive: true,
        reason: "Project 2 work".to_string(),
        expires_ts,
    };
    FileReservationBmc::create(&tc.ctx, &tc.mm, fr2)
        .await
        .expect("Failed to create reservation 2");

    // List all active across all projects
    let all_active = FileReservationBmc::list_all_active(&tc.ctx, &tc.mm)
        .await
        .expect("Failed to list all active reservations");

    assert_eq!(
        all_active.len(),
        2,
        "Should have 2 active reservations across projects"
    );

    // Verify they are from different projects
    let project_ids: Vec<i64> = all_active.iter().map(|r| r.project_id.into()).collect();
    assert!(
        project_ids.contains(&project1_id),
        "Should include project 1"
    );
    assert!(
        project_ids.contains(&project2_id),
        "Should include project 2"
    );
}

// ============================================================================
// PRECOMMIT GUARD CONFLICT DETECTION TESTS
// ============================================================================

use lib_core::model::precommit_guard::PrecommitGuardBmc;
use serial_test::serial;
use temp_env::async_with_vars;

/// Helper to create a second agent for conflict testing
async fn create_second_agent(tc: &TestContext, project_id: i64) -> i64 {
    let agent = AgentForCreate {
        project_id: ProjectId(project_id),
        name: "other-agent".to_string(),
        program: "claude-code".to_string(),
        model: "claude-3".to_string(),
        task_description: "Another agent".to_string(),
    };
    AgentBmc::create(&tc.ctx, &tc.mm, agent)
        .await
        .expect("Failed to create second agent")
        .into()
}

/// Test that guard detects conflicts when another agent holds a reservation
#[tokio::test]
#[serial]
async fn test_guard_detects_reservation_conflict() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Create project and two agents
    let (project_id, agent1_id) = setup_project_and_agent(&tc).await;
    let _agent2_id = create_second_agent(&tc, project_id).await;

    // Agent 1 reserves src/**/*.rs
    let expires_ts = Utc::now().naive_utc() + Duration::hours(1);
    let fr_c = FileReservationForCreate {
        project_id: ProjectId(project_id),
        agent_id: AgentId(agent1_id),
        path_pattern: "src/**/*.rs".to_string(),
        exclusive: true,
        reason: "Working on source files".to_string(),
        expires_ts,
    };
    FileReservationBmc::create(&tc.ctx, &tc.mm, fr_c)
        .await
        .expect("Failed to create reservation");

    // Set up env for guard check
    let project_slug = slugify("/test/repo");
    async_with_vars(
        [
            ("WORKTREES_ENABLED", Some("1")),
            ("AGENT_MAIL_PROJECT", Some(project_slug.as_str())),
            ("AGENT_MAIL_BYPASS", None::<&str>),
            ("AGENT_MAIL_GUARD_MODE", None::<&str>),
        ],
        async {
            // Agent 2 tries to commit src/main.rs - should detect conflict
            let result = PrecommitGuardBmc::check_reservations(
                &tc.ctx,
                &tc.mm,
                "other-agent",
                &["src/main.rs".to_string()],
            )
            .await;

            assert!(result.is_ok(), "Should not error");
            let violations = result.unwrap();
            assert!(violations.is_some(), "Should detect violation");
            let violations = violations.unwrap();
            assert_eq!(violations.len(), 1, "Should have exactly one violation");
            assert!(
                violations[0].contains("src/main.rs"),
                "Violation should mention the file"
            );
            assert!(
                violations[0].contains("test-agent"),
                "Violation should mention the holder"
            );
        },
    )
    .await;
}

/// Test that guard allows commits for files owned by same agent
#[tokio::test]
#[serial]
async fn test_guard_allows_own_reservations() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc).await;

    // Agent reserves src/**/*.rs
    let expires_ts = Utc::now().naive_utc() + Duration::hours(1);
    let fr_c = FileReservationForCreate {
        project_id: ProjectId(project_id),
        agent_id: AgentId(agent_id),
        path_pattern: "src/**/*.rs".to_string(),
        exclusive: true,
        reason: "Working on source files".to_string(),
        expires_ts,
    };
    FileReservationBmc::create(&tc.ctx, &tc.mm, fr_c)
        .await
        .expect("Failed to create reservation");

    // Set up env for guard check
    let project_slug = slugify("/test/repo");
    async_with_vars(
        [
            ("WORKTREES_ENABLED", Some("1")),
            ("AGENT_MAIL_PROJECT", Some(project_slug.as_str())),
            ("AGENT_MAIL_BYPASS", None::<&str>),
            ("AGENT_MAIL_GUARD_MODE", None::<&str>),
        ],
        async {
            // Same agent commits src/main.rs - should be allowed (own reservation)
            let result = PrecommitGuardBmc::check_reservations(
                &tc.ctx,
                &tc.mm,
                "test-agent",
                &["src/main.rs".to_string()],
            )
            .await;

            assert!(result.is_ok(), "Should not error");
            assert!(
                result.unwrap().is_none(),
                "Should not detect violation for own reservation"
            );
        },
    )
    .await;
}

/// Test that guard ignores expired reservations
#[tokio::test]
#[serial]
async fn test_guard_ignores_expired_reservations() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent1_id) = setup_project_and_agent(&tc).await;
    let _agent2_id = create_second_agent(&tc, project_id).await;

    // Agent 1 reserves src/**/*.rs with expired time
    let expires_ts = Utc::now().naive_utc() - Duration::hours(1); // Already expired!
    let fr_c = FileReservationForCreate {
        project_id: ProjectId(project_id),
        agent_id: AgentId(agent1_id),
        path_pattern: "src/**/*.rs".to_string(),
        exclusive: true,
        reason: "Working on source files".to_string(),
        expires_ts,
    };
    FileReservationBmc::create(&tc.ctx, &tc.mm, fr_c)
        .await
        .expect("Failed to create reservation");

    // Set up env for guard check
    let project_slug = slugify("/test/repo");
    async_with_vars(
        [
            ("WORKTREES_ENABLED", Some("1")),
            ("AGENT_MAIL_PROJECT", Some(project_slug.as_str())),
            ("AGENT_MAIL_BYPASS", None::<&str>),
            ("AGENT_MAIL_GUARD_MODE", None::<&str>),
        ],
        async {
            // Agent 2 tries to commit - should be allowed (reservation expired)
            let result = PrecommitGuardBmc::check_reservations(
                &tc.ctx,
                &tc.mm,
                "other-agent",
                &["src/main.rs".to_string()],
            )
            .await;

            assert!(result.is_ok(), "Should not error");
            assert!(
                result.unwrap().is_none(),
                "Should not detect violation for expired reservation"
            );
        },
    )
    .await;
}
