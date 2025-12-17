//! Project sibling suggestion model tests
//!
//! Tests for project similarity suggestions and acceptance/dismissal workflow.

#[path = "common/mod.rs"]
mod common;

use crate::common::TestContext;
use lib_core::model::project::ProjectBmc;
use lib_core::model::project_sibling_suggestion::{
    ProjectSiblingSuggestionBmc, ProjectSiblingSuggestionForCreate,
};
use lib_core::utils::slugify;

/// Helper to create two projects for suggestion tests
async fn setup_two_projects(tc: &TestContext, suffix: &str) -> (i64, i64) {
    let human_key_a = format!("/test/sibling-repo-a-{}", suffix);
    let slug_a = slugify(&human_key_a);

    let project_a_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug_a, &human_key_a)
        .await
        .expect("Failed to create project A");

    let human_key_b = format!("/test/sibling-repo-b-{}", suffix);
    let slug_b = slugify(&human_key_b);

    let project_b_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug_b, &human_key_b)
        .await
        .expect("Failed to create project B");

    (project_a_id, project_b_id)
}

/// Test creating a sibling suggestion
#[tokio::test]
async fn test_create_sibling_suggestion() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_a_id, project_b_id) = setup_two_projects(&tc, "create").await;

    let suggestion_c = ProjectSiblingSuggestionForCreate {
        project_a_id,
        project_b_id,
        score: 0.85,
        rationale: "Both projects use similar tech stack".to_string(),
    };

    let suggestion_id = ProjectSiblingSuggestionBmc::create(&tc.ctx, &tc.mm, suggestion_c)
        .await
        .expect("Failed to create sibling suggestion");

    assert!(suggestion_id > 0, "Suggestion ID should be positive");
}

/// Test listing sibling suggestions for a project
#[tokio::test]
async fn test_list_sibling_suggestions() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_a_id, project_b_id) = setup_two_projects(&tc, "list").await;

    // Create a suggestion
    let suggestion_c = ProjectSiblingSuggestionForCreate {
        project_a_id,
        project_b_id,
        score: 0.75,
        rationale: "Similar directory structure".to_string(),
    };

    ProjectSiblingSuggestionBmc::create(&tc.ctx, &tc.mm, suggestion_c)
        .await
        .expect("Failed to create suggestion");

    // List from project A's perspective
    let suggestions = ProjectSiblingSuggestionBmc::list(&tc.ctx, &tc.mm, project_a_id)
        .await
        .expect("Failed to list suggestions");

    assert_eq!(suggestions.len(), 1, "Should have 1 suggestion");
    assert_eq!(suggestions[0].project_a_id, project_a_id);
    assert_eq!(suggestions[0].project_b_id, project_b_id);
    assert_eq!(suggestions[0].status, "pending");

    // List from project B's perspective (should find same suggestion)
    let suggestions_b = ProjectSiblingSuggestionBmc::list(&tc.ctx, &tc.mm, project_b_id)
        .await
        .expect("Failed to list suggestions for B");

    assert_eq!(
        suggestions_b.len(),
        1,
        "Project B should also see the suggestion"
    );
}

/// Test accepting a sibling suggestion
#[tokio::test]
async fn test_accept_sibling_suggestion() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_a_id, project_b_id) = setup_two_projects(&tc, "accept").await;

    let suggestion_c = ProjectSiblingSuggestionForCreate {
        project_a_id,
        project_b_id,
        score: 0.90,
        rationale: "High confidence match".to_string(),
    };

    let suggestion_id = ProjectSiblingSuggestionBmc::create(&tc.ctx, &tc.mm, suggestion_c)
        .await
        .expect("Failed to create suggestion");

    // Accept the suggestion
    ProjectSiblingSuggestionBmc::update_status(&tc.ctx, &tc.mm, suggestion_id, "accepted")
        .await
        .expect("Failed to accept suggestion");

    // Verify it's no longer in pending list
    let suggestions = ProjectSiblingSuggestionBmc::list(&tc.ctx, &tc.mm, project_a_id)
        .await
        .expect("Failed to list suggestions");

    assert!(
        suggestions.is_empty(),
        "Accepted suggestion should not appear in pending list"
    );
}

/// Test dismissing a sibling suggestion
#[tokio::test]
async fn test_dismiss_sibling_suggestion() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_a_id, project_b_id) = setup_two_projects(&tc, "dismiss").await;

    let suggestion_c = ProjectSiblingSuggestionForCreate {
        project_a_id,
        project_b_id,
        score: 0.50,
        rationale: "Low confidence match".to_string(),
    };

    let suggestion_id = ProjectSiblingSuggestionBmc::create(&tc.ctx, &tc.mm, suggestion_c)
        .await
        .expect("Failed to create suggestion");

    // Dismiss the suggestion
    ProjectSiblingSuggestionBmc::update_status(&tc.ctx, &tc.mm, suggestion_id, "dismissed")
        .await
        .expect("Failed to dismiss suggestion");

    // Verify it's no longer in pending list
    let suggestions = ProjectSiblingSuggestionBmc::list(&tc.ctx, &tc.mm, project_a_id)
        .await
        .expect("Failed to list suggestions");

    assert!(
        suggestions.is_empty(),
        "Dismissed suggestion should not appear in pending list"
    );
}

/// Test empty suggestions for new project
#[tokio::test]
async fn test_empty_suggestions() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let human_key = "/test/lonely-repo";
    let slug = slugify(human_key);

    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");

    let suggestions = ProjectSiblingSuggestionBmc::list(&tc.ctx, &tc.mm, project_id)
        .await
        .expect("Failed to list suggestions");

    assert!(
        suggestions.is_empty(),
        "New project should have no suggestions"
    );
}

/// Test multiple suggestions sorted by score
#[tokio::test]
async fn test_suggestions_sorted_by_score() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Create 3 projects
    let human_key_a = "/test/main-repo-score";
    let slug_a = slugify(human_key_a);
    let project_a_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug_a, human_key_a)
        .await
        .expect("Failed to create project A");

    let human_key_b = "/test/sibling-repo-high";
    let slug_b = slugify(human_key_b);
    let project_b_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug_b, human_key_b)
        .await
        .expect("Failed to create project B");

    let human_key_c = "/test/sibling-repo-low";
    let slug_c = slugify(human_key_c);
    let project_c_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug_c, human_key_c)
        .await
        .expect("Failed to create project C");

    // Create suggestions with different scores (insert low score first)
    let low_suggestion = ProjectSiblingSuggestionForCreate {
        project_a_id,
        project_b_id: project_c_id,
        score: 0.40,
        rationale: "Low score suggestion".to_string(),
    };
    ProjectSiblingSuggestionBmc::create(&tc.ctx, &tc.mm, low_suggestion)
        .await
        .expect("Failed to create low suggestion");

    let high_suggestion = ProjectSiblingSuggestionForCreate {
        project_a_id,
        project_b_id,
        score: 0.95,
        rationale: "High score suggestion".to_string(),
    };
    ProjectSiblingSuggestionBmc::create(&tc.ctx, &tc.mm, high_suggestion)
        .await
        .expect("Failed to create high suggestion");

    // List and verify sorting (highest score first)
    let suggestions = ProjectSiblingSuggestionBmc::list(&tc.ctx, &tc.mm, project_a_id)
        .await
        .expect("Failed to list suggestions");

    assert_eq!(suggestions.len(), 2, "Should have 2 suggestions");
    assert!(
        suggestions[0].score > suggestions[1].score,
        "Suggestions should be sorted by score DESC"
    );
    assert_eq!(suggestions[0].score, 0.95);
    assert_eq!(suggestions[1].score, 0.40);
}

/// Test suggestion structure and fields
#[tokio::test]
async fn test_suggestion_structure() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (project_a_id, project_b_id) = setup_two_projects(&tc, "structure").await;

    let suggestion_c = ProjectSiblingSuggestionForCreate {
        project_a_id,
        project_b_id,
        score: 0.77,
        rationale: "Both projects are Rust web services".to_string(),
    };

    let suggestion_id = ProjectSiblingSuggestionBmc::create(&tc.ctx, &tc.mm, suggestion_c)
        .await
        .expect("Failed to create suggestion");

    let suggestions = ProjectSiblingSuggestionBmc::list(&tc.ctx, &tc.mm, project_a_id)
        .await
        .expect("Failed to list suggestions");

    let suggestion = &suggestions[0];
    assert_eq!(suggestion.id, suggestion_id);
    assert_eq!(suggestion.project_a_id, project_a_id);
    assert_eq!(suggestion.project_b_id, project_b_id);
    assert_eq!(suggestion.score, 0.77);
    assert_eq!(suggestion.status, "pending");
    assert_eq!(suggestion.rationale, "Both projects are Rust web services");
    assert!(
        suggestion.confirmed_ts.is_none(),
        "New suggestion should not be confirmed"
    );
    assert!(
        suggestion.dismissed_ts.is_none(),
        "New suggestion should not be dismissed"
    );
}
