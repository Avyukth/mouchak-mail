//! Product and Sibling tests
//!
//! Tests for product creation, linking, and sibling discovery.

#[path = "common/mod.rs"]
mod common;

use crate::common::TestContext;
use lib_core::model::project::ProjectBmc;
use lib_core::model::product::ProductBmc;

#[tokio::test]
async fn test_sibling_logic() {
    let tc = TestContext::new().await.expect("Failed to create test context");
    
    // 1. Create Projects A, B, C
    let id_a = ProjectBmc::create(&tc.ctx, &tc.mm, "proj-a", "Project A").await.unwrap();
    let id_b = ProjectBmc::create(&tc.ctx, &tc.mm, "proj-b", "Project B").await.unwrap();
    let id_c = ProjectBmc::create(&tc.ctx, &tc.mm, "proj-c", "Project C").await.unwrap();

    // 2. Create Product P
    let product_p = ProductBmc::ensure(&tc.ctx, &tc.mm, "prod-p", "Product P").await.unwrap();

    // 3. Link A and B to P
    ProductBmc::link_project(&tc.ctx, &tc.mm, product_p.id, id_a).await.unwrap();
    ProductBmc::link_project(&tc.ctx, &tc.mm, product_p.id, id_b).await.unwrap();

    // 4. Verify siblings for A -> should be [B]
    let sibs_a = ProjectBmc::list_siblings(&tc.ctx, &tc.mm, id_a).await.unwrap();
    assert_eq!(sibs_a.len(), 1);
    assert_eq!(sibs_a[0].id, id_b);
    assert_eq!(sibs_a[0].slug, "proj-b");

    // 5. Verify siblings for B -> should be [A]
    let sibs_b = ProjectBmc::list_siblings(&tc.ctx, &tc.mm, id_b).await.unwrap();
    assert_eq!(sibs_b.len(), 1);
    assert_eq!(sibs_b[0].id, id_a);

    // 6. Verify siblings for C -> empty
    let sibs_c = ProjectBmc::list_siblings(&tc.ctx, &tc.mm, id_c).await.unwrap();
    assert!(sibs_c.is_empty());
}
