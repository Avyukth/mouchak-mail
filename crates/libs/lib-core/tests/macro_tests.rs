// Tests are allowed to use unwrap()/expect() for clearer failure messages
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::inefficient_to_string
)]
#![allow(unused)]

use crate::common::TestContext;
use lib_core::model::macro_def::{MacroDefBmc, MacroDefForCreate};
use lib_core::model::project::ProjectBmc;
use lib_core::types::ProjectId;

mod common;

#[tokio::test]
async fn test_builtin_macros_registration() {
    let ctx = TestContext::new().await.unwrap();
    let mm = &ctx.mm;
    let c = &ctx.ctx;

    // Create a project
    let project_id = ProjectBmc::create(c, mm, "macro-test-project", "/path/to/project")
        .await
        .unwrap();

    // Verify automatic registration via ProjectBmc::create
    let listed = MacroDefBmc::list(c, mm, project_id.get()).await.unwrap();
    assert_eq!(
        listed.len(),
        5,
        "ProjectBmc::create should have registered 5 built-in macros"
    );

    // built-in macros names to check
    let names: Vec<String> = listed.into_iter().map(|m| m.name).collect();
    assert!(names.contains(&"start_session".to_string()));
    assert!(names.contains(&"prepare_thread".to_string()));
    assert!(names.contains(&"file_reservation_cycle".to_string()));
    assert!(names.contains(&"contact_handshake".to_string()));
    assert!(names.contains(&"broadcast_message".to_string()));

    // Verify idempotency
    let created_again = MacroDefBmc::ensure_builtin_macros(c, mm, project_id.get())
        .await
        .unwrap();
    assert_eq!(
        created_again.len(),
        0,
        "Calling ensure_builtin_macros again should create 0 new macros"
    );
}

#[tokio::test]
async fn test_builtin_macro_contents() {
    let ctx = TestContext::new().await.unwrap();
    let mm = &ctx.mm;
    let c = &ctx.ctx;

    let project_id = ProjectBmc::create(c, mm, "macro-content-test", "/path/to/project")
        .await
        .unwrap();

    // Test start_session macro structure
    let start_session = MacroDefBmc::get_by_name(c, mm, project_id.get(), "start_session")
        .await
        .unwrap();
    assert_eq!(start_session.name, "start_session");
    assert!(start_session.description.contains("Register an agent"));
    assert_eq!(start_session.steps.len(), 2);
    assert_eq!(start_session.steps[0]["tool"], "register_agent");
    assert_eq!(start_session.steps[1]["tool"], "check_inbox");

    // Test prepare_thread macro structure
    let prepare_thread = MacroDefBmc::get_by_name(c, mm, project_id.get(), "prepare_thread")
        .await
        .unwrap();
    assert_eq!(prepare_thread.name, "prepare_thread");
    assert!(prepare_thread.description.contains("thread"));
    assert_eq!(prepare_thread.steps.len(), 2);
    assert_eq!(prepare_thread.steps[0]["tool"], "send_message");
    assert_eq!(prepare_thread.steps[1]["tool"], "reserve_file");

    // Test file_reservation_cycle macro structure
    let file_res = MacroDefBmc::get_by_name(c, mm, project_id.get(), "file_reservation_cycle")
        .await
        .unwrap();
    assert_eq!(file_res.name, "file_reservation_cycle");
    assert_eq!(file_res.steps.len(), 3);
    assert_eq!(file_res.steps[0]["tool"], "reserve_file");
    assert_eq!(file_res.steps[1]["action"], "user_work");
    assert_eq!(file_res.steps[2]["tool"], "release_reservation");

    // Test contact_handshake macro structure
    let contact = MacroDefBmc::get_by_name(c, mm, project_id.get(), "contact_handshake")
        .await
        .unwrap();
    assert_eq!(contact.name, "contact_handshake");
    assert_eq!(contact.steps.len(), 2);
    assert_eq!(contact.steps[0]["tool"], "request_contact");
    assert_eq!(contact.steps[1]["tool"], "respond_contact");

    // Test broadcast_message macro structure
    let broadcast = MacroDefBmc::get_by_name(c, mm, project_id.get(), "broadcast_message")
        .await
        .unwrap();
    assert_eq!(broadcast.name, "broadcast_message");
    assert_eq!(broadcast.steps.len(), 2);
    assert_eq!(broadcast.steps[0]["tool"], "list_agents");
    assert_eq!(broadcast.steps[1]["tool"], "send_message");
}

#[tokio::test]
async fn test_builtin_macros_per_project_isolation() {
    let ctx = TestContext::new().await.unwrap();
    let mm = &ctx.mm;
    let c = &ctx.ctx;

    // Create two projects
    let project1_id = ProjectBmc::create(c, mm, "project-1", "/path/1")
        .await
        .unwrap();
    let project2_id = ProjectBmc::create(c, mm, "project-2", "/path/2")
        .await
        .unwrap();

    // Both should have 5 built-in macros
    let p1_macros = MacroDefBmc::list(c, mm, project1_id.get()).await.unwrap();
    let p2_macros = MacroDefBmc::list(c, mm, project2_id.get()).await.unwrap();

    assert_eq!(p1_macros.len(), 5);
    assert_eq!(p2_macros.len(), 5);

    // All macros should be different instances (different IDs)
    let p1_ids: Vec<i64> = p1_macros.iter().map(|m| m.id).collect();
    let p2_ids: Vec<i64> = p2_macros.iter().map(|m| m.id).collect();

    // No ID overlap
    for id in &p1_ids {
        assert!(
            !p2_ids.contains(id),
            "Macro IDs should be unique across projects"
        );
    }

    // Each should have the same macro names
    let p1_names: Vec<String> = p1_macros.iter().map(|m| m.name.clone()).collect();
    let p2_names: Vec<String> = p2_macros.iter().map(|m| m.name.clone()).collect();

    let mut sorted_p1 = p1_names.clone();
    let mut sorted_p2 = p2_names.clone();
    sorted_p1.sort();
    sorted_p2.sort();

    assert_eq!(sorted_p1, sorted_p2);
}

#[tokio::test]
async fn test_builtin_macros_have_timestamps() {
    let ctx = TestContext::new().await.unwrap();
    let mm = &ctx.mm;
    let c = &ctx.ctx;

    let project_id = ProjectBmc::create(c, mm, "timestamp-test", "/path/ts")
        .await
        .unwrap();
    let macros = MacroDefBmc::list(c, mm, project_id.get()).await.unwrap();

    for macro_def in macros {
        // Verify timestamps are not default/zero
        assert!(
            macro_def.created_ts.and_utc().timestamp() > 0,
            "Macro {} should have valid created_ts",
            macro_def.name
        );
        assert!(
            macro_def.updated_ts.and_utc().timestamp() > 0,
            "Macro {} should have valid updated_ts",
            macro_def.name
        );
    }
}

#[tokio::test]
async fn test_macro_registration_order() {
    let ctx = TestContext::new().await.unwrap();
    let mm = &ctx.mm;
    let c = &ctx.ctx;

    let project_id = ProjectBmc::create(c, mm, "order-test", "/path/order")
        .await
        .unwrap();

    // List returns macros ordered by name ASC
    let macros = MacroDefBmc::list(c, mm, project_id.get()).await.unwrap();
    let names: Vec<String> = macros.iter().map(|m| m.name.clone()).collect();

    let mut sorted_names = names.clone();
    sorted_names.sort();

    assert_eq!(
        names, sorted_names,
        "Macros should be returned in alphabetical order"
    );
}

#[tokio::test]
async fn test_macro_crud() {
    let ctx = TestContext::new().await.unwrap();
    let mm = &ctx.mm;
    let c = &ctx.ctx;

    let project_id = ProjectBmc::create(c, mm, "crud-test", "/tmp/crud")
        .await
        .unwrap();

    // Create
    let macro_c = MacroDefForCreate {
        project_id: project_id.get(),
        name: "custom_macro".to_string(),
        description: "A custom test macro".to_string(),
        steps: vec![serde_json::json!({"action": "test"})],
    };
    let mid = MacroDefBmc::create(c, mm, macro_c).await.unwrap();

    // Get
    let m = MacroDefBmc::get_by_name(c, mm, project_id.get(), "custom_macro")
        .await
        .unwrap();
    assert_eq!(m.id, mid);
    assert_eq!(m.description, "A custom test macro");

    // List
    let list = MacroDefBmc::list(c, mm, project_id.get()).await.unwrap();
    assert!(list.iter().any(|x| x.name == "custom_macro"));

    // Delete
    let deleted = MacroDefBmc::delete(c, mm, project_id.get(), "custom_macro")
        .await
        .unwrap();
    assert!(deleted);

    // Verify gone
    let list_after = MacroDefBmc::list(c, mm, project_id.get()).await.unwrap();
    assert!(!list_after.iter().any(|x| x.name == "custom_macro"));
}

#[tokio::test]
async fn test_register_custom_macro() {
    let ctx = TestContext::new().await.unwrap();
    let mm = &ctx.mm;
    let c = &ctx.ctx;

    let project_id = ProjectBmc::create(c, mm, "custom-reg", "/path/custom")
        .await
        .unwrap();

    // Initially has 5 built-in macros
    let initial = MacroDefBmc::list(c, mm, project_id.get()).await.unwrap();
    assert_eq!(initial.len(), 5);

    // Register a custom macro with complex steps
    let custom = MacroDefForCreate {
        project_id: project_id.get(),
        name: "deploy_workflow".to_string(),
        description: "Deploy workflow with testing and notification".to_string(),
        steps: vec![
            serde_json::json!({
                "tool": "run_tests",
                "description": "Run test suite",
                "params": ["project_slug"]
            }),
            serde_json::json!({
                "tool": "build_release",
                "description": "Build release artifacts",
                "params": ["project_slug", "version"]
            }),
            serde_json::json!({
                "tool": "send_message",
                "description": "Notify team of deployment",
                "params": ["project_slug", "sender", "recipients", "subject", "body"]
            }),
        ],
    };

    let custom_id = MacroDefBmc::create(c, mm, custom).await.unwrap();
    assert!(custom_id > 0);

    // Verify it appears in the list (now 6 total)
    let after = MacroDefBmc::list(c, mm, project_id.get()).await.unwrap();
    assert_eq!(after.len(), 6);

    // Verify we can retrieve it
    let retrieved = MacroDefBmc::get_by_name(c, mm, project_id.get(), "deploy_workflow")
        .await
        .unwrap();
    assert_eq!(retrieved.id, custom_id);
    assert_eq!(retrieved.steps.len(), 3);
}

#[tokio::test]
async fn test_list_registered_macros() {
    let ctx = TestContext::new().await.unwrap();
    let mm = &ctx.mm;
    let c = &ctx.ctx;

    let project_id = ProjectBmc::create(c, mm, "list-test", "/path/list")
        .await
        .unwrap();

    // List all macros
    let all_macros = MacroDefBmc::list(c, mm, project_id.get()).await.unwrap();

    // Should have exactly 5 built-in macros
    assert_eq!(all_macros.len(), 5);

    // Verify each macro has required fields
    for macro_def in &all_macros {
        assert!(macro_def.id > 0, "Macro ID should be positive");
        assert_eq!(
            macro_def.project_id,
            project_id.get(),
            "Macro should belong to correct project"
        );
        assert!(!macro_def.name.is_empty(), "Macro name should not be empty");
        assert!(
            !macro_def.description.is_empty(),
            "Macro description should not be empty"
        );
        assert!(
            !macro_def.steps.is_empty(),
            "Macro should have at least one step"
        );
    }

    // Add custom macros
    for i in 1..=3 {
        let custom = MacroDefForCreate {
            project_id: project_id.get(),
            name: format!("custom_{}", i),
            description: format!("Custom macro number {}", i),
            steps: vec![serde_json::json!({"action": format!("step_{}", i)})],
        };
        MacroDefBmc::create(c, mm, custom).await.unwrap();
    }

    // Should now have 8 macros
    let updated = MacroDefBmc::list(c, mm, project_id.get()).await.unwrap();
    assert_eq!(updated.len(), 8);
}

#[tokio::test]
async fn test_unregister_macro() {
    let ctx = TestContext::new().await.unwrap();
    let mm = &ctx.mm;
    let c = &ctx.ctx;

    let project_id = ProjectBmc::create(c, mm, "unreg-test", "/path/unreg")
        .await
        .unwrap();

    // Create a custom macro
    let custom = MacroDefForCreate {
        project_id: project_id.get(),
        name: "temp_macro".to_string(),
        description: "Temporary macro for testing".to_string(),
        steps: vec![serde_json::json!({"action": "temp"})],
    };
    MacroDefBmc::create(c, mm, custom).await.unwrap();

    // Verify it exists
    let before = MacroDefBmc::list(c, mm, project_id.get()).await.unwrap();
    assert_eq!(before.len(), 6); // 5 built-in + 1 custom

    // Delete it
    let deleted = MacroDefBmc::delete(c, mm, project_id.get(), "temp_macro")
        .await
        .unwrap();
    assert!(deleted, "Delete should return true");

    // Verify it's gone
    let after = MacroDefBmc::list(c, mm, project_id.get()).await.unwrap();
    assert_eq!(after.len(), 5); // back to just built-in

    // Deleting again should return false
    let deleted_again = MacroDefBmc::delete(c, mm, project_id.get(), "temp_macro")
        .await
        .unwrap();
    assert!(
        !deleted_again,
        "Deleting non-existent macro should return false"
    );
}

#[tokio::test]
async fn test_macro_get_nonexistent() {
    let ctx = TestContext::new().await.unwrap();
    let mm = &ctx.mm;
    let c = &ctx.ctx;

    let project_id = ProjectBmc::create(c, mm, "nonexist-test", "/path/nonexist")
        .await
        .unwrap();

    // Try to get a macro that doesn't exist
    let result = MacroDefBmc::get_by_name(c, mm, project_id.get(), "does_not_exist").await;
    assert!(
        result.is_err(),
        "Getting non-existent macro should return error"
    );
}

#[tokio::test]
async fn test_builtin_macros_parameter_structure() {
    let ctx = TestContext::new().await.unwrap();
    let mm = &ctx.mm;
    let c = &ctx.ctx;

    let project_id = ProjectBmc::create(c, mm, "param-test", "/path/param")
        .await
        .unwrap();

    // Test that each built-in macro has proper parameter structure
    let start_session = MacroDefBmc::get_by_name(c, mm, project_id.get(), "start_session")
        .await
        .unwrap();
    assert!(
        start_session.steps[0].get("params").is_some(),
        "Steps should have params field"
    );
    let params = start_session.steps[0]["params"].as_array().unwrap();
    assert!(!params.is_empty(), "Params should not be empty");

    let prepare = MacroDefBmc::get_by_name(c, mm, project_id.get(), "prepare_thread")
        .await
        .unwrap();
    for step in &prepare.steps {
        assert!(
            step.get("description").is_some(),
            "Each step should have description"
        );
    }
}

#[tokio::test]
async fn test_macro_steps_serialization() {
    let ctx = TestContext::new().await.unwrap();
    let mm = &ctx.mm;
    let c = &ctx.ctx;

    let project_id = ProjectBmc::create(c, mm, "serial-test", "/path/serial")
        .await
        .unwrap();

    // Create macro with complex nested JSON in steps
    let complex_steps = vec![serde_json::json!({
        "tool": "complex_tool",
        "params": {
            "nested": {
                "deeply": {
                    "nested": "value"
                }
            },
            "array": [1, 2, 3],
            "boolean": true
        }
    })];

    let custom = MacroDefForCreate {
        project_id: project_id.get(),
        name: "complex_macro".to_string(),
        description: "Macro with complex step structure".to_string(),
        steps: complex_steps.clone(),
    };

    MacroDefBmc::create(c, mm, custom).await.unwrap();

    // Retrieve and verify structure is preserved
    let retrieved = MacroDefBmc::get_by_name(c, mm, project_id.get(), "complex_macro")
        .await
        .unwrap();
    assert_eq!(retrieved.steps.len(), 1);
    assert_eq!(retrieved.steps[0]["tool"], "complex_tool");
    assert_eq!(
        retrieved.steps[0]["params"]["nested"]["deeply"]["nested"],
        "value"
    );
    assert_eq!(retrieved.steps[0]["params"]["array"][0], 1);
    assert_eq!(retrieved.steps[0]["params"]["boolean"], true);
}
