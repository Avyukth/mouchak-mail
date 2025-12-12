//! Agent Capability Logic Tests

#[path = "common/mod.rs"]
mod common;

use crate::common::TestContext;
use lib_core::model::agent::{AgentBmc, AgentForCreate};
use lib_core::model::agent_capabilities::{AgentCapabilityBmc, AgentCapabilityForCreate};
use lib_core::model::project::ProjectBmc;
use lib_core::utils::slugify;

#[tokio::test]
async fn test_agent_capability_lifecycle() {
    let tc = TestContext::new().await.expect("Failed to create test context");
    
    // Setup project and agent
    let human_key = "/capability/test";
    let slug = slugify(human_key);
    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key).await.unwrap();
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
    let has_cap = AgentCapabilityBmc::check(&tc.ctx, &tc.mm, agent_id, "send_message").await.unwrap();
    assert!(!has_cap, "Agent should not have capability yet");
    
    // 2. Grant capability
    let cap_c = AgentCapabilityForCreate {
        agent_id,
        capability: "send_message".to_string(),
    };
    let cap_id = AgentCapabilityBmc::create(&tc.ctx, &tc.mm, cap_c).await.unwrap();
    assert!(cap_id > 0);
    
    // 3. Check capability again (should be true)
    let has_cap = AgentCapabilityBmc::check(&tc.ctx, &tc.mm, agent_id, "send_message").await.unwrap();
    assert!(has_cap, "Agent should have capability now");
    
    // 4. List capabilities
    let caps = AgentCapabilityBmc::list_for_agent(&tc.ctx, &tc.mm, agent_id).await.unwrap();
    assert_eq!(caps.len(), 1);
    assert_eq!(caps[0].capability, "send_message");
    
    // 5. Revoke capability
    AgentCapabilityBmc::revoke(&tc.ctx, &tc.mm, cap_id).await.unwrap();
    
    // 6. Check capability (should be false)
    let has_cap = AgentCapabilityBmc::check(&tc.ctx, &tc.mm, agent_id, "send_message").await.unwrap();
    assert!(!has_cap, "Agent should not have capability after revoke");
}
