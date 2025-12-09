//! Project model tests
//!
//! Tests for project creation, retrieval, and management.

use crate::common::TestContext;
use lib_core::model::project::ProjectBmc;
use lib_core::utils::slugify;

/// Test creating a new project
#[tokio::test]
async fn test_create_project() {
    let tc = TestContext::new().await.expect("Failed to create test context");
    
    let human_key = "/test/project/path";
    let slug = slugify(human_key);
    
    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");
    
    assert!(project_id > 0, "Project ID should be positive");
}

/// Test getting project by slug
#[tokio::test]
async fn test_get_project_by_slug() {
    let tc = TestContext::new().await.expect("Failed to create test context");
    
    let human_key = "/my/cool/project";
    let slug = slugify(human_key);
    
    let _id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");
    
    let retrieved = ProjectBmc::get_by_slug(&tc.ctx, &tc.mm, &slug)
        .await
        .expect("Failed to get project by slug");
    
    assert_eq!(retrieved.human_key, human_key);
    assert_eq!(retrieved.slug, slug);
}

/// Test getting project by human key
#[tokio::test]
async fn test_get_project_by_human_key() {
    let tc = TestContext::new().await.expect("Failed to create test context");
    
    let human_key = "/project/for/lookup";
    let slug = slugify(human_key);
    
    let _id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");
    
    let retrieved = ProjectBmc::get_by_human_key(&tc.ctx, &tc.mm, human_key)
        .await
        .expect("Failed to get project by human key");
    
    assert_eq!(retrieved.human_key, human_key);
}

/// Test listing all projects
#[tokio::test]
async fn test_list_all_projects() {
    let tc = TestContext::new().await.expect("Failed to create test context");
    
    // Create multiple projects
    for name in &["alpha", "beta", "gamma"] {
        let human_key = format!("/project/{}", name);
        let slug = slugify(&human_key);
        ProjectBmc::create(&tc.ctx, &tc.mm, &slug, &human_key).await.unwrap();
    }
    
    let projects = ProjectBmc::list_all(&tc.ctx, &tc.mm)
        .await
        .expect("Failed to list projects");
    
    assert_eq!(projects.len(), 3, "Should have 3 projects");
}

/// Test project not found error
#[tokio::test]
async fn test_project_not_found() {
    let tc = TestContext::new().await.expect("Failed to create test context");
    
    let result = ProjectBmc::get_by_slug(&tc.ctx, &tc.mm, "nonexistent-slug").await;
    
    assert!(result.is_err(), "Should return error for nonexistent project");
}
