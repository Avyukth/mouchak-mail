//! Agent Capability Logic Tests

// Tests are allowed to use unwrap()/expect() for clearer failure messages
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::inefficient_to_string
)]

#[path = "common/mod.rs"]
mod common;

use crate::common::TestContext;
use lib_core::model::agent::{AgentBmc, AgentForCreate};
use lib_core::model::agent_capabilities::{
    AgentCapabilityBmc, AgentCapabilityForCreate, CAP_ACKNOWLEDGE_MESSAGE, CAP_FETCH_INBOX,
    CAP_FILE_RESERVATION, CAP_SEND_MESSAGE, DEFAULT_CAPABILITIES,
};
use lib_core::model::project::ProjectBmc;
use lib_core::utils::slugify;

#[tokio::test]
async fn test_agent_capability_lifecycle() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Setup project and agent
    let human_key = "/capability/test";
    let slug = slugify(human_key);
    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .unwrap();
    let project = ProjectBmc::get(&tc.ctx, &tc.mm, project_id).await.unwrap();

    let agent_c = AgentForCreate {
        project_id: project.id,
        name: "TestAgent".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Testing capabilities".to_string(),
    };
    let agent_id = AgentBmc::create(&tc.ctx, &tc.mm, agent_c).await.unwrap();

    // 1. Check capability (should be false)
    let has_cap = AgentCapabilityBmc::check(&tc.ctx, &tc.mm, agent_id, "send_message")
        .await
        .unwrap();
    assert!(!has_cap, "Agent should not have capability yet");

    // 2. Grant capability
    let cap_c = AgentCapabilityForCreate {
        agent_id,
        capability: "send_message".to_string(),
        granted_by: None,
        expires_at: None,
    };
    let cap_id = AgentCapabilityBmc::create(&tc.ctx, &tc.mm, cap_c)
        .await
        .unwrap();
    assert!(cap_id > 0);

    // 3. Check capability again (should be true)
    let has_cap = AgentCapabilityBmc::check(&tc.ctx, &tc.mm, agent_id, "send_message")
        .await
        .unwrap();
    assert!(has_cap, "Agent should have capability now");

    // 4. List capabilities
    let caps = AgentCapabilityBmc::list_for_agent(&tc.ctx, &tc.mm, agent_id)
        .await
        .unwrap();
    assert_eq!(caps.len(), 1);
    assert_eq!(caps[0].capability, "send_message");

    // 5. Revoke capability
    AgentCapabilityBmc::revoke(&tc.ctx, &tc.mm, cap_id)
        .await
        .unwrap();

    // 6. Check capability (should be false)
    let has_cap = AgentCapabilityBmc::check(&tc.ctx, &tc.mm, agent_id, "send_message")
        .await
        .unwrap();
    assert!(!has_cap, "Agent should not have capability after revoke");
}

#[tokio::test]
async fn test_grant_defaults_new_agent() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Setup project and agent
    let human_key = "/capability/grant-defaults";
    let slug = slugify(human_key);
    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .unwrap();
    let project = ProjectBmc::get(&tc.ctx, &tc.mm, project_id).await.unwrap();

    let agent_c = AgentForCreate {
        project_id: project.id,
        name: "DefaultsAgent".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Testing grant_defaults".to_string(),
    };
    let agent_id = AgentBmc::create(&tc.ctx, &tc.mm, agent_c).await.unwrap();

    // Verify agent has no capabilities initially
    let caps_before = AgentCapabilityBmc::list_for_agent(&tc.ctx, &tc.mm, agent_id)
        .await
        .unwrap();
    assert_eq!(
        caps_before.len(),
        0,
        "New agent should have no capabilities"
    );

    // Grant default capabilities
    let granted = AgentCapabilityBmc::grant_defaults(&tc.ctx, &tc.mm, agent_id)
        .await
        .unwrap();
    assert_eq!(granted, 4, "Should grant exactly 4 default capabilities");

    // Verify all default capabilities were granted
    let caps_after = AgentCapabilityBmc::list_for_agent(&tc.ctx, &tc.mm, agent_id)
        .await
        .unwrap();
    assert_eq!(caps_after.len(), 4, "Agent should have 4 capabilities");

    // Verify each specific capability using constants
    let has_send = AgentCapabilityBmc::check(&tc.ctx, &tc.mm, agent_id, CAP_SEND_MESSAGE)
        .await
        .unwrap();
    assert!(has_send, "Agent should have send_message capability");

    let has_fetch = AgentCapabilityBmc::check(&tc.ctx, &tc.mm, agent_id, CAP_FETCH_INBOX)
        .await
        .unwrap();
    assert!(has_fetch, "Agent should have fetch_inbox capability");

    let has_file = AgentCapabilityBmc::check(&tc.ctx, &tc.mm, agent_id, CAP_FILE_RESERVATION)
        .await
        .unwrap();
    assert!(
        has_file,
        "Agent should have file_reservation_paths capability"
    );

    let has_ack = AgentCapabilityBmc::check(&tc.ctx, &tc.mm, agent_id, CAP_ACKNOWLEDGE_MESSAGE)
        .await
        .unwrap();
    assert!(has_ack, "Agent should have acknowledge_message capability");
}

#[test]
fn test_default_capability_constants() {
    // Verify constants have correct values
    assert_eq!(CAP_SEND_MESSAGE, "send_message");
    assert_eq!(CAP_FETCH_INBOX, "fetch_inbox");
    assert_eq!(CAP_FILE_RESERVATION, "file_reservation_paths");
    assert_eq!(CAP_ACKNOWLEDGE_MESSAGE, "acknowledge_message");

    // Verify DEFAULT_CAPABILITIES array
    assert_eq!(DEFAULT_CAPABILITIES.len(), 4);
    assert!(DEFAULT_CAPABILITIES.contains(&CAP_SEND_MESSAGE));
    assert!(DEFAULT_CAPABILITIES.contains(&CAP_FETCH_INBOX));
    assert!(DEFAULT_CAPABILITIES.contains(&CAP_FILE_RESERVATION));
    assert!(DEFAULT_CAPABILITIES.contains(&CAP_ACKNOWLEDGE_MESSAGE));
}

#[tokio::test]
async fn test_expired_capability_rejected() {
    use chrono::{Duration, Utc};

    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Setup project and agent
    let human_key = "/capability/expired-test";
    let slug = slugify(human_key);
    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .unwrap();
    let project = ProjectBmc::get(&tc.ctx, &tc.mm, project_id).await.unwrap();

    let agent_c = AgentForCreate {
        project_id: project.id,
        name: "ExpiredAgent".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Testing expired capabilities".to_string(),
    };
    let agent_id = AgentBmc::create(&tc.ctx, &tc.mm, agent_c).await.unwrap();

    // Grant capability that expires in the past
    let past_time = (Utc::now() - Duration::hours(1)).naive_utc();
    let cap_c = AgentCapabilityForCreate {
        agent_id,
        capability: "send_message".to_string(),
        granted_by: None,
        expires_at: Some(past_time),
    };
    let _cap_id = AgentCapabilityBmc::create(&tc.ctx, &tc.mm, cap_c)
        .await
        .unwrap();

    // Expired capability should NOT pass check
    let has_cap = AgentCapabilityBmc::check(&tc.ctx, &tc.mm, agent_id, "send_message")
        .await
        .unwrap();
    assert!(!has_cap, "Expired capability should be rejected");

    // Grant capability that expires in the future
    let future_time = (Utc::now() + Duration::hours(1)).naive_utc();
    let cap_c2 = AgentCapabilityForCreate {
        agent_id,
        capability: "fetch_inbox".to_string(),
        granted_by: None,
        expires_at: Some(future_time),
    };
    let _cap_id2 = AgentCapabilityBmc::create(&tc.ctx, &tc.mm, cap_c2)
        .await
        .unwrap();

    // Non-expired capability should pass check
    let has_cap2 = AgentCapabilityBmc::check(&tc.ctx, &tc.mm, agent_id, "fetch_inbox")
        .await
        .unwrap();
    assert!(has_cap2, "Non-expired capability should pass");
}

#[tokio::test]
async fn test_duplicate_grant_defaults_fails() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Setup project and agent
    let human_key = "/capability/duplicate-test";
    let slug = slugify(human_key);
    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .unwrap();
    let project = ProjectBmc::get(&tc.ctx, &tc.mm, project_id).await.unwrap();

    let agent_c = AgentForCreate {
        project_id: project.id,
        name: "DuplicateAgent".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Testing duplicate grant".to_string(),
    };
    let agent_id = AgentBmc::create(&tc.ctx, &tc.mm, agent_c).await.unwrap();

    // First grant should succeed
    let granted = AgentCapabilityBmc::grant_defaults(&tc.ctx, &tc.mm, agent_id)
        .await
        .unwrap();
    assert_eq!(granted, 4);

    // Second grant should fail (UNIQUE constraint)
    let result = AgentCapabilityBmc::grant_defaults(&tc.ctx, &tc.mm, agent_id).await;
    assert!(
        result.is_err(),
        "Duplicate grant_defaults should fail due to UNIQUE constraint"
    );
}

#[tokio::test]
async fn test_list_for_agent_filters_expired() {
    use chrono::{Duration, Utc};

    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Setup project and agent
    let human_key = "/capability/list-expired-test";
    let slug = slugify(human_key);
    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .unwrap();
    let project = ProjectBmc::get(&tc.ctx, &tc.mm, project_id).await.unwrap();

    let agent_c = AgentForCreate {
        project_id: project.id,
        name: "ListExpiredAgent".to_string(),
        program: "test".to_string(),
        model: "test".to_string(),
        task_description: "Testing list_for_agent filters expired".to_string(),
    };
    let agent_id = AgentBmc::create(&tc.ctx, &tc.mm, agent_c).await.unwrap();

    // Grant expired capability
    let past_time = (Utc::now() - Duration::hours(1)).naive_utc();
    let expired_cap = AgentCapabilityForCreate {
        agent_id,
        capability: "expired_cap".to_string(),
        granted_by: None,
        expires_at: Some(past_time),
    };
    AgentCapabilityBmc::create(&tc.ctx, &tc.mm, expired_cap)
        .await
        .unwrap();

    // Grant valid capability (no expiry)
    let valid_cap = AgentCapabilityForCreate {
        agent_id,
        capability: "valid_cap".to_string(),
        granted_by: None,
        expires_at: None,
    };
    AgentCapabilityBmc::create(&tc.ctx, &tc.mm, valid_cap)
        .await
        .unwrap();

    // Grant future-expiring capability
    let future_time = (Utc::now() + Duration::hours(1)).naive_utc();
    let future_cap = AgentCapabilityForCreate {
        agent_id,
        capability: "future_cap".to_string(),
        granted_by: None,
        expires_at: Some(future_time),
    };
    AgentCapabilityBmc::create(&tc.ctx, &tc.mm, future_cap)
        .await
        .unwrap();

    // list_for_agent should only return non-expired capabilities
    let caps = AgentCapabilityBmc::list_for_agent(&tc.ctx, &tc.mm, agent_id)
        .await
        .unwrap();

    assert_eq!(
        caps.len(),
        2,
        "Should only return 2 non-expired capabilities, got {}",
        caps.len()
    );

    let cap_names: Vec<&str> = caps.iter().map(|c| c.capability.as_str()).collect();
    assert!(
        cap_names.contains(&"valid_cap"),
        "Should contain valid_cap"
    );
    assert!(
        cap_names.contains(&"future_cap"),
        "Should contain future_cap"
    );
    assert!(
        !cap_names.contains(&"expired_cap"),
        "Should NOT contain expired_cap"
    );
}
