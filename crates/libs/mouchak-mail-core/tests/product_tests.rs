//! Product and Sibling tests
//!
//! Tests for product creation, linking, and sibling discovery.

// Tests are allowed to use unwrap()/expect() for clearer failure messages
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::inefficient_to_string
)]

#[path = "common/mod.rs"]
mod common;

use crate::common::TestContext;
use mouchak_mail_core::model::product::ProductBmc;
use mouchak_mail_core::model::project::ProjectBmc;

/// Test creating a new product via ensure
#[tokio::test]
async fn test_ensure_product_creates_new() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let product = ProductBmc::ensure(&tc.ctx, &tc.mm, "prod-new-123", "New Product 123")
        .await
        .expect("Failed to create product");

    assert!(product.id > 0);
    assert_eq!(product.product_uid, "prod-new-123");
    assert_eq!(product.name, "New Product 123");
}

/// Test ensure_product returns existing product
#[tokio::test]
async fn test_ensure_product_returns_existing() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Create first
    let product1 = ProductBmc::ensure(&tc.ctx, &tc.mm, "prod-existing", "Existing Product")
        .await
        .expect("Failed to create product");

    // Ensure again - should return same
    let product2 = ProductBmc::ensure(&tc.ctx, &tc.mm, "prod-existing", "Different Name")
        .await
        .expect("Failed to ensure product");

    assert_eq!(product1.id, product2.id);
    assert_eq!(product2.product_uid, "prod-existing");
    // Name should be the original, not the new one
    assert_eq!(product2.name, "Existing Product");
}

/// Test get_by_uid for existing product
#[tokio::test]
async fn test_get_by_uid_success() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Create product first
    let created = ProductBmc::ensure(&tc.ctx, &tc.mm, "prod-get-uid", "Get By UID Product")
        .await
        .expect("Failed to create product");

    // Get by UID
    let found = ProductBmc::get_by_uid(&tc.ctx, &tc.mm, "prod-get-uid")
        .await
        .expect("Failed to get product by UID");

    assert_eq!(found.id, created.id);
    assert_eq!(found.product_uid, "prod-get-uid");
}

/// Test get_by_uid for non-existent product
#[tokio::test]
async fn test_get_by_uid_not_found() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let result = ProductBmc::get_by_uid(&tc.ctx, &tc.mm, "prod-nonexistent-uid").await;

    assert!(
        result.is_err(),
        "Should return error for non-existent product"
    );
}

/// Test list_all returns all products with their linked projects
#[tokio::test]
async fn test_list_all_products() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Create multiple products
    ProductBmc::ensure(&tc.ctx, &tc.mm, "prod-list-1", "List Product 1")
        .await
        .unwrap();
    ProductBmc::ensure(&tc.ctx, &tc.mm, "prod-list-2", "List Product 2")
        .await
        .unwrap();

    let products = ProductBmc::list_all(&tc.ctx, &tc.mm)
        .await
        .expect("Failed to list products");

    // Should have at least our 2 products
    assert!(products.len() >= 2);

    // Find our products
    let p1 = products.iter().find(|p| p.product_uid == "prod-list-1");
    let p2 = products.iter().find(|p| p.product_uid == "prod-list-2");
    assert!(p1.is_some(), "Should find prod-list-1");
    assert!(p2.is_some(), "Should find prod-list-2");
}

/// Test linking and unlinking projects
#[tokio::test]
async fn test_link_unlink_project() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Create product
    let product = ProductBmc::ensure(&tc.ctx, &tc.mm, "prod-link-test", "Link Test Product")
        .await
        .unwrap();

    // Create project
    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, "proj-for-linking", "Project For Linking")
        .await
        .unwrap();

    // Link project to product
    let link_id = ProductBmc::link_project(&tc.ctx, &tc.mm, product.id, project_id.get())
        .await
        .expect("Failed to link project");
    assert!(link_id > 0);

    // Verify link exists
    let linked_projects = ProductBmc::get_linked_projects(&tc.ctx, &tc.mm, product.id)
        .await
        .expect("Failed to get linked projects");
    assert!(linked_projects.contains(&project_id.get()));

    // Unlink project
    let unlinked = ProductBmc::unlink_project(&tc.ctx, &tc.mm, product.id, project_id.get())
        .await
        .expect("Failed to unlink project");
    assert!(unlinked, "Should return true for successful unlink");

    // Verify unlink
    let linked_projects = ProductBmc::get_linked_projects(&tc.ctx, &tc.mm, product.id)
        .await
        .expect("Failed to get linked projects");
    assert!(!linked_projects.contains(&project_id.get()));
}

/// Test unlink_project returns false for non-existent link
#[tokio::test]
async fn test_unlink_nonexistent_returns_false() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let product = ProductBmc::ensure(&tc.ctx, &tc.mm, "prod-unlink-none", "Unlink None Product")
        .await
        .unwrap();

    // Try to unlink a project that was never linked
    let result = ProductBmc::unlink_project(&tc.ctx, &tc.mm, product.id, 99999)
        .await
        .expect("Unlink should not error");

    assert!(!result, "Should return false for non-existent link");
}

/// Test list_for_project returns products a project belongs to
#[tokio::test]
async fn test_list_for_project() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // Create project
    let project_id =
        ProjectBmc::create(&tc.ctx, &tc.mm, "proj-list-prods", "List Products Project")
            .await
            .unwrap();

    // Create two products and link project to both
    let product1 = ProductBmc::ensure(&tc.ctx, &tc.mm, "prod-list-for-1", "List For 1")
        .await
        .unwrap();
    let product2 = ProductBmc::ensure(&tc.ctx, &tc.mm, "prod-list-for-2", "List For 2")
        .await
        .unwrap();

    ProductBmc::link_project(&tc.ctx, &tc.mm, product1.id, project_id.get())
        .await
        .unwrap();
    ProductBmc::link_project(&tc.ctx, &tc.mm, product2.id, project_id.get())
        .await
        .unwrap();

    // List products for this project
    let products = ProductBmc::list_for_project(&tc.ctx, &tc.mm, project_id.get())
        .await
        .expect("Failed to list products for project");

    assert_eq!(products.len(), 2);
    assert!(products.iter().any(|p| p.product_uid == "prod-list-for-1"));
    assert!(products.iter().any(|p| p.product_uid == "prod-list-for-2"));
}

/// Test list_for_project returns empty for unlinked project
#[tokio::test]
async fn test_list_for_project_empty() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, "proj-no-products", "No Products Project")
        .await
        .unwrap();

    let products = ProductBmc::list_for_project(&tc.ctx, &tc.mm, project_id.get())
        .await
        .expect("Failed to list products for project");

    assert!(products.is_empty());
}

/// Test linking same project twice (idempotent)
#[tokio::test]
async fn test_link_project_idempotent() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let product = ProductBmc::ensure(&tc.ctx, &tc.mm, "prod-idem", "Idempotent Product")
        .await
        .unwrap();
    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, "proj-idem", "Idempotent Project")
        .await
        .unwrap();

    // Link twice
    let link1 = ProductBmc::link_project(&tc.ctx, &tc.mm, product.id, project_id.get())
        .await
        .unwrap();
    let link2 = ProductBmc::link_project(&tc.ctx, &tc.mm, product.id, project_id.get())
        .await
        .unwrap();

    // Should return same link ID (or at least not fail)
    assert_eq!(link1, link2, "Linking twice should be idempotent");

    // Should only have one link
    let linked = ProductBmc::get_linked_projects(&tc.ctx, &tc.mm, product.id)
        .await
        .unwrap();
    assert_eq!(
        linked.iter().filter(|&&id| id == project_id.get()).count(),
        1
    );
}

#[tokio::test]
async fn test_sibling_logic() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    // 1. Create Projects A, B, C
    let id_a = ProjectBmc::create(&tc.ctx, &tc.mm, "proj-a", "Project A")
        .await
        .unwrap();
    let id_b = ProjectBmc::create(&tc.ctx, &tc.mm, "proj-b", "Project B")
        .await
        .unwrap();
    let id_c = ProjectBmc::create(&tc.ctx, &tc.mm, "proj-c", "Project C")
        .await
        .unwrap();

    // 2. Create Product P
    let product_p = ProductBmc::ensure(&tc.ctx, &tc.mm, "prod-p", "Product P")
        .await
        .unwrap();

    // 3. Link A and B to P
    ProductBmc::link_project(&tc.ctx, &tc.mm, product_p.id, id_a.get())
        .await
        .unwrap();
    ProductBmc::link_project(&tc.ctx, &tc.mm, product_p.id, id_b.get())
        .await
        .unwrap();

    // 4. Verify siblings for A -> should be [B]
    let sibs_a = ProjectBmc::list_siblings(&tc.ctx, &tc.mm, id_a)
        .await
        .unwrap();
    assert_eq!(sibs_a.len(), 1);
    assert_eq!(sibs_a[0].id, id_b);
    assert_eq!(sibs_a[0].slug, "proj-b");

    // 5. Verify siblings for B -> should be [A]
    let sibs_b = ProjectBmc::list_siblings(&tc.ctx, &tc.mm, id_b)
        .await
        .unwrap();
    assert_eq!(sibs_b.len(), 1);
    assert_eq!(sibs_b[0].id, id_a);

    // 6. Verify siblings for C -> empty
    let sibs_c = ProjectBmc::list_siblings(&tc.ctx, &tc.mm, id_c)
        .await
        .unwrap();
    assert!(sibs_c.is_empty());
}
