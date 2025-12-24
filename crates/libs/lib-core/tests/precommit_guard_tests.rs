use lib_core::model::precommit_guard::{GuardMode, PrecommitGuardBmc, render_prepush_script};
use temp_env::with_vars;

#[test]
fn test_guard_mode_from_env() {
    // Default
    with_vars(
        [
            ("AGENT_MAIL_GUARD_MODE", None::<&str>),
            ("AGENT_MAIL_BYPASS", None::<&str>),
        ],
        || {
            assert_eq!(GuardMode::from_env(), GuardMode::Enforce);
        },
    );

    // Bypass
    with_vars([("AGENT_MAIL_BYPASS", Some("1"))], || {
        assert_eq!(GuardMode::from_env(), GuardMode::Bypass);
    });

    // Warn
    with_vars([("AGENT_MAIL_GUARD_MODE", Some("warn"))], || {
        assert_eq!(GuardMode::from_env(), GuardMode::Warn);
    });

    with_vars([("AGENT_MAIL_GUARD_MODE", Some("advisory"))], || {
        assert_eq!(GuardMode::from_env(), GuardMode::Warn);
    });
}

#[test]
fn test_should_check_gate() {
    // Both disabled
    with_vars(
        [
            ("WORKTREES_ENABLED", None::<&str>),
            ("GIT_IDENTITY_ENABLED", None::<&str>),
        ],
        || {
            assert!(!PrecommitGuardBmc::should_check());
        },
    );

    // WORKTREES_ENABLED
    with_vars([("WORKTREES_ENABLED", Some("1"))], || {
        assert!(PrecommitGuardBmc::should_check());
    });

    // GIT_IDENTITY_ENABLED
    with_vars([("GIT_IDENTITY_ENABLED", Some("true"))], || {
        assert!(PrecommitGuardBmc::should_check());
    });
}

#[test]
fn test_render_prepush_script() {
    let script = render_prepush_script("http://localhost:8080");

    assert!(script.contains("SERVER_URL=\"http://localhost:8080\""));
    assert!(script.contains("check_gate() {"));
    assert!(script.contains("WORKTREES_ENABLED"));
    assert!(script.contains("/api/guard/check-push"));

    // Check it's a valid sh script (basic check)
    assert!(script.starts_with("#!/bin/sh"));
}

// Integration tests for DB interaction require TestContext which is in file_reservation_tests common
// We'll reuse the pattern from file_reservation_tests if possible, or just mock expected behaviors here?
// Actually, PrecommitGuardBmc methods are what we test.
// We can test check_reservations logic with warnings here if we import TestContext.
// But TestContext is in common/mod.rs which is not pub.
// We can duplicate the setup or make common pub.
// For now, let's just stick to unit tests of the helper logic available.
// The DB interation parts are partially tested in file_reservation_tests.rs
// I'll add one integration test for "Warn" mode if I can figure out the module path.
// `file_reservation_tests::common` is not accessible here easily unless I mod it.
// I will create a `common` mod here pointing to same file?
#[path = "common/mod.rs"]
mod common;
use crate::common::TestContext;
use chrono::{Duration, Utc};
use lib_core::model::agent::{AgentBmc, AgentForCreate};
use lib_core::model::file_reservation::{FileReservationBmc, FileReservationForCreate};
use lib_core::model::project::ProjectBmc;
use lib_core::types::ProjectId;
use lib_core::utils::slugify;
use serial_test::serial;
use temp_env::async_with_vars;

#[tokio::test]
#[serial]
async fn test_guard_warn_mode() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Setup logic copied from file_reservation_tests helper
    let human_key = "/test/repo_warn";
    let slug = slugify(human_key);
    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .unwrap();
    // Validate project slug found in DB
    let project = ProjectBmc::get(&tc.ctx, &tc.mm, project_id).await.unwrap();
    let actual_slug = project.slug;

    let agent1 = AgentForCreate {
        project_id: ProjectId(project_id),
        name: "agent1".into(),
        program: "c".into(),
        model: "m".into(),
        task_description: "t".into(),
    };
    let agent1_id = AgentBmc::create(&tc.ctx, &tc.mm, agent1).await.unwrap();

    // Create conflicting reservation
    let fr_c = FileReservationForCreate {
        project_id: ProjectId(project_id),
        agent_id: agent1_id,
        path_pattern: "src/**".into(),
        exclusive: true,
        reason: "r".into(),
        expires_ts: Utc::now().naive_utc() + Duration::hours(1),
    };
    FileReservationBmc::create(&tc.ctx, &tc.mm, fr_c)
        .await
        .unwrap();

    // Create agent2 who will conflict
    let agent2 = AgentForCreate {
        project_id: ProjectId(project_id),
        name: "agent2".into(),
        program: "c".into(),
        model: "m".into(),
        task_description: "t".into(),
    };
    AgentBmc::create(&tc.ctx, &tc.mm, agent2).await.unwrap();

    // Warn mode check with different agent
    async_with_vars(
        [
            ("WORKTREES_ENABLED", Some("1")),
            ("AGENT_MAIL_PROJECT", Some(actual_slug.as_str())),
            ("AGENT_MAIL_GUARD_MODE", Some("warn")), // WARN MODE
        ],
        async {
            let result = PrecommitGuardBmc::check_reservations(
                &tc.ctx,
                &tc.mm,
                "agent2",
                &["src/main.rs".to_string()],
            )
            .await;

            assert!(result.is_ok());
            // In warn mode, it returns Ok(Some(violations))
            let violations = result.unwrap();
            assert!(
                violations.is_some(),
                "Should return violations in warn mode"
            );
            assert!(violations.unwrap().len() > 0);
        },
    )
    .await;
}

#[tokio::test]
#[serial]
async fn test_guard_bypass_mode() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Setup
    let human_key = "/test/repo_bypass";
    let slug = slugify(human_key);
    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .unwrap();
    let project = ProjectBmc::get(&tc.ctx, &tc.mm, project_id).await.unwrap();
    let actual_slug = project.slug;

    let agent1 = AgentForCreate {
        project_id: ProjectId(project_id),
        name: "agent1".into(),
        program: "c".into(),
        model: "m".into(),
        task_description: "t".into(),
    };
    let agent1_id = AgentBmc::create(&tc.ctx, &tc.mm, agent1).await.unwrap();

    // Create conflicting reservation
    let fr_c = FileReservationForCreate {
        project_id: ProjectId(project_id),
        agent_id: agent1_id,
        path_pattern: "src/**".into(),
        exclusive: true,
        reason: "r".into(),
        expires_ts: Utc::now().naive_utc() + Duration::hours(1),
    };
    FileReservationBmc::create(&tc.ctx, &tc.mm, fr_c)
        .await
        .unwrap();

    // Create agent2
    let agent2 = AgentForCreate {
        project_id: ProjectId(project_id),
        name: "agent2".into(),
        program: "c".into(),
        model: "m".into(),
        task_description: "t".into(),
    };
    AgentBmc::create(&tc.ctx, &tc.mm, agent2).await.unwrap();

    // Bypass mode check
    async_with_vars(
        [
            ("WORKTREES_ENABLED", Some("1")),
            ("AGENT_MAIL_PROJECT", Some(actual_slug.as_str())),
            ("AGENT_MAIL_BYPASS", Some("1")), // BYPASS MODE
        ],
        async {
            let result = PrecommitGuardBmc::check_reservations(
                &tc.ctx,
                &tc.mm,
                "agent2",
                &["src/main.rs".to_string()],
            )
            .await;

            assert!(result.is_ok());
            // In bypass mode, it returns Ok(None) even if conflicts exist
            let violations = result.unwrap();
            assert!(
                violations.is_none(),
                "Should ignore violations in bypass mode"
            );
        },
    )
    .await;
}

// PORT-3.4: get_hooks_dir verification
use lib_core::model::precommit_guard::get_hooks_dir;
use std::fs;

#[test]
fn test_get_hooks_dir_defaults() {
    let temp = tempfile::tempdir().unwrap();
    let repo_path = temp.path();
    let git_dir = repo_path.join(".git");
    fs::create_dir(&git_dir).unwrap();

    // Default case: .git/hooks (even if not exists, function returns the path)
    let hooks = get_hooks_dir(repo_path);
    assert_eq!(hooks, git_dir.join("hooks"));
}
