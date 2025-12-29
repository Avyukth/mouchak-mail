//! Build slot model tests
//!
//! Tests for build slot acquisition, renewal, and release - critical for CI/CD isolation.

// Tests are allowed to use unwrap()/expect() for clearer failure messages
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::inefficient_to_string
)]

#[path = "common/mod.rs"]
mod common;

use crate::common::TestContext;
use mouchak_mail_core::model::agent::{AgentBmc, AgentForCreate};
use mouchak_mail_core::model::build_slot::{BuildSlotBmc, BuildSlotForCreate};
use mouchak_mail_core::model::project::ProjectBmc;
use mouchak_mail_core::utils::slugify;

/// Helper to set up a project and agent for build slot tests
async fn setup_project_and_agent(tc: &TestContext, suffix: &str) -> (i64, i64) {
    let human_key = format!("/test/build-repo-{}", suffix);
    let slug = slugify(&human_key);

    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, &human_key)
        .await
        .expect("Failed to create project");

    let agent = AgentForCreate {
        project_id,
        name: format!("build-agent-{}", suffix),
        program: "claude-code".to_string(),
        model: "claude-3".to_string(),
        task_description: "Testing build slots".to_string(),
    };

    let agent_id = AgentBmc::create(&tc.ctx, &tc.mm, agent)
        .await
        .expect("Failed to create agent");

    (project_id.get(), agent_id.get())
}

/// Test acquiring a build slot
#[tokio::test]
async fn test_acquire_build_slot() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc, "acquire").await;

    let slot_c = BuildSlotForCreate {
        project_id,
        agent_id,
        slot_name: "default".to_string(),
        ttl_seconds: 3600, // 1 hour
    };

    let slot_id = BuildSlotBmc::acquire(&tc.ctx, &tc.mm, slot_c)
        .await
        .expect("Failed to acquire build slot");

    assert!(slot_id > 0, "Slot ID should be positive");
}

/// Test acquiring the same slot twice fails
#[tokio::test]
async fn test_acquire_slot_conflict() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc, "conflict").await;

    let slot_c = BuildSlotForCreate {
        project_id,
        agent_id,
        slot_name: "ci-slot".to_string(),
        ttl_seconds: 3600,
    };

    // First acquisition should succeed
    let _slot_id = BuildSlotBmc::acquire(&tc.ctx, &tc.mm, slot_c.clone())
        .await
        .expect("First acquire should succeed");

    // Second acquisition should fail (slot already held)
    let result = BuildSlotBmc::acquire(&tc.ctx, &tc.mm, slot_c).await;

    assert!(
        result.is_err(),
        "Second acquire should fail - slot already held"
    );
}

/// Test releasing a build slot
#[tokio::test]
async fn test_release_build_slot() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc, "release").await;

    let slot_c = BuildSlotForCreate {
        project_id,
        agent_id,
        slot_name: "release-test".to_string(),
        ttl_seconds: 3600,
    };

    let slot_id = BuildSlotBmc::acquire(&tc.ctx, &tc.mm, slot_c.clone())
        .await
        .expect("Failed to acquire slot");

    // Release the slot
    BuildSlotBmc::release(&tc.ctx, &tc.mm, slot_id)
        .await
        .expect("Failed to release slot");

    // Now we should be able to acquire again
    let slot_c2 = BuildSlotForCreate {
        project_id,
        agent_id,
        slot_name: "release-test".to_string(),
        ttl_seconds: 3600,
    };

    let new_slot_id = BuildSlotBmc::acquire(&tc.ctx, &tc.mm, slot_c2)
        .await
        .expect("Should be able to acquire after release");

    assert!(new_slot_id > slot_id, "New slot ID should be different");
}

/// Test renewing a build slot
#[tokio::test]
async fn test_renew_build_slot() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc, "renew").await;

    let slot_c = BuildSlotForCreate {
        project_id,
        agent_id,
        slot_name: "renew-test".to_string(),
        ttl_seconds: 60, // Short TTL
    };

    let slot_id = BuildSlotBmc::acquire(&tc.ctx, &tc.mm, slot_c)
        .await
        .expect("Failed to acquire slot");

    // Renew with longer TTL
    let new_expires = BuildSlotBmc::renew(&tc.ctx, &tc.mm, slot_id, 7200) // 2 hours
        .await
        .expect("Failed to renew slot");

    // Verify the new expiry is in the future (roughly 2 hours from now)
    let now = chrono::Utc::now().naive_utc();
    let diff = new_expires - now;

    assert!(
        diff.num_seconds() > 7000 && diff.num_seconds() < 7300,
        "New expiry should be approximately 2 hours from now"
    );
}

/// Test listing active build slots
#[tokio::test]
async fn test_list_active_slots() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc, "list").await;

    // Acquire multiple slots
    for slot_name in &["slot-a", "slot-b", "slot-c"] {
        let slot_c = BuildSlotForCreate {
            project_id,
            agent_id,
            slot_name: slot_name.to_string(),
            ttl_seconds: 3600,
        };
        BuildSlotBmc::acquire(&tc.ctx, &tc.mm, slot_c)
            .await
            .expect("Failed to acquire slot");
    }

    let active_slots = BuildSlotBmc::list_active(&tc.ctx, &tc.mm, project_id)
        .await
        .expect("Failed to list active slots");

    assert_eq!(active_slots.len(), 3, "Should have 3 active slots");

    // Verify slot names
    let names: Vec<&str> = active_slots.iter().map(|s| s.slot_name.as_str()).collect();
    assert!(names.contains(&"slot-a"));
    assert!(names.contains(&"slot-b"));
    assert!(names.contains(&"slot-c"));
}

/// Test that released slots don't appear in active list
#[tokio::test]
async fn test_released_slots_not_in_active() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_id, agent_id) = setup_project_and_agent(&tc, "released").await;

    // Acquire and release one slot
    let slot_c = BuildSlotForCreate {
        project_id,
        agent_id,
        slot_name: "temp-slot".to_string(),
        ttl_seconds: 3600,
    };
    let slot_id = BuildSlotBmc::acquire(&tc.ctx, &tc.mm, slot_c)
        .await
        .expect("Failed to acquire slot");

    BuildSlotBmc::release(&tc.ctx, &tc.mm, slot_id)
        .await
        .expect("Failed to release slot");

    // Acquire another slot (keep active)
    let slot_c2 = BuildSlotForCreate {
        project_id,
        agent_id,
        slot_name: "active-slot".to_string(),
        ttl_seconds: 3600,
    };
    BuildSlotBmc::acquire(&tc.ctx, &tc.mm, slot_c2)
        .await
        .expect("Failed to acquire second slot");

    // List active should only show 1
    let active_slots = BuildSlotBmc::list_active(&tc.ctx, &tc.mm, project_id)
        .await
        .expect("Failed to list active slots");

    assert_eq!(active_slots.len(), 1, "Should have 1 active slot");
    assert_eq!(active_slots[0].slot_name, "active-slot");
}

/// Test different agents can hold different slots
#[tokio::test]
async fn test_different_agents_different_slots() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let human_key = "/test/multi-agent-slots";
    let slug = slugify(human_key);

    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");

    // Create two agents
    let agent1 = AgentForCreate {
        project_id,
        name: "agent-1".to_string(),
        program: "claude-code".to_string(),
        model: "claude-3".to_string(),
        task_description: "Agent 1 for slot tests".to_string(),
    };
    let agent1_id = AgentBmc::create(&tc.ctx, &tc.mm, agent1)
        .await
        .expect("Failed to create agent 1");

    let agent2 = AgentForCreate {
        project_id,
        name: "agent-2".to_string(),
        program: "cursor".to_string(),
        model: "gpt-4".to_string(),
        task_description: "Agent 2 for slot tests".to_string(),
    };
    let agent2_id = AgentBmc::create(&tc.ctx, &tc.mm, agent2)
        .await
        .expect("Failed to create agent 2");

    // Agent 1 acquires slot-x
    let slot_c1 = BuildSlotForCreate {
        project_id: project_id.get(),
        agent_id: agent1_id.get(),
        slot_name: "slot-x".to_string(),
        ttl_seconds: 3600,
    };
    BuildSlotBmc::acquire(&tc.ctx, &tc.mm, slot_c1)
        .await
        .expect("Agent 1 should acquire slot-x");

    // Agent 2 acquires slot-y (different slot, should work)
    let slot_c2 = BuildSlotForCreate {
        project_id: project_id.get(),
        agent_id: agent2_id.get(),
        slot_name: "slot-y".to_string(),
        ttl_seconds: 3600,
    };
    BuildSlotBmc::acquire(&tc.ctx, &tc.mm, slot_c2)
        .await
        .expect("Agent 2 should acquire slot-y");

    // Agent 2 tries to acquire slot-x (same slot as agent 1, should fail)
    let slot_c3 = BuildSlotForCreate {
        project_id: project_id.get(),
        agent_id: agent2_id.get(),
        slot_name: "slot-x".to_string(),
        ttl_seconds: 3600,
    };
    let result = BuildSlotBmc::acquire(&tc.ctx, &tc.mm, slot_c3).await;

    assert!(
        result.is_err(),
        "Agent 2 should not be able to acquire slot-x"
    );
}
