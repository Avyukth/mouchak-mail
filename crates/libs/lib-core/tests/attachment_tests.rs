//! Attachment model tests
//!
//! Tests for attachment CRUD operations - file sharing between agents.

#[path = "common/mod.rs"]
mod common;

use crate::common::TestContext;
use lib_core::model::attachment::{AttachmentBmc, AttachmentForCreate};
use lib_core::model::project::ProjectBmc;
use lib_core::utils::slugify;

/// Helper to set up a project for attachment tests
async fn setup_project(tc: &TestContext) -> i64 {
    let human_key = "/test/attachment-repo";
    let slug = slugify(human_key);

    ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project")
}

/// Test creating an attachment
#[tokio::test]
async fn test_create_attachment() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = setup_project(&tc).await;

    let attachment_c = AttachmentForCreate {
        project_id,
        filename: "design.png".to_string(),
        stored_path: "/data/attachments/abc123.png".to_string(),
        media_type: "image/png".to_string(),
        size_bytes: 1024 * 50, // 50KB
    };

    let attachment_id = AttachmentBmc::create(&tc.ctx, &tc.mm, attachment_c)
        .await
        .expect("Failed to create attachment");

    assert!(attachment_id > 0, "Attachment ID should be positive");
}

/// Test getting an attachment by ID
#[tokio::test]
async fn test_get_attachment() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = setup_project(&tc).await;

    let attachment_c = AttachmentForCreate {
        project_id,
        filename: "report.pdf".to_string(),
        stored_path: "/data/attachments/def456.pdf".to_string(),
        media_type: "application/pdf".to_string(),
        size_bytes: 1024 * 1024, // 1MB
    };

    let attachment_id = AttachmentBmc::create(&tc.ctx, &tc.mm, attachment_c)
        .await
        .expect("Failed to create attachment");

    let attachment = AttachmentBmc::get(&tc.ctx, &tc.mm, attachment_id)
        .await
        .expect("Failed to get attachment");

    assert_eq!(attachment.id, attachment_id);
    assert_eq!(attachment.project_id, project_id);
    assert_eq!(attachment.filename, "report.pdf");
    assert_eq!(attachment.stored_path, "/data/attachments/def456.pdf");
    assert_eq!(attachment.media_type, "application/pdf");
    assert_eq!(attachment.size_bytes, 1024 * 1024);
}

/// Test listing attachments by project
#[tokio::test]
async fn test_list_by_project() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = setup_project(&tc).await;

    // Create multiple attachments
    let files = vec![
        ("doc1.txt", "text/plain", 100),
        ("image.jpg", "image/jpeg", 5000),
        ("data.json", "application/json", 250),
    ];

    for (filename, media_type, size) in &files {
        let attachment_c = AttachmentForCreate {
            project_id,
            filename: filename.to_string(),
            stored_path: format!("/data/attachments/{}", filename),
            media_type: media_type.to_string(),
            size_bytes: *size,
        };
        AttachmentBmc::create(&tc.ctx, &tc.mm, attachment_c)
            .await
            .expect("Failed to create attachment");
    }

    let attachments = AttachmentBmc::list_by_project(&tc.ctx, &tc.mm, project_id)
        .await
        .expect("Failed to list attachments");

    assert_eq!(attachments.len(), 3, "Should have 3 attachments");

    // Verify filenames are present
    let filenames: Vec<&str> = attachments.iter().map(|a| a.filename.as_str()).collect();
    assert!(filenames.contains(&"doc1.txt"));
    assert!(filenames.contains(&"image.jpg"));
    assert!(filenames.contains(&"data.json"));
}

/// Test attachment not found error
#[tokio::test]
async fn test_attachment_not_found() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let result = AttachmentBmc::get(&tc.ctx, &tc.mm, 99999).await;

    assert!(
        result.is_err(),
        "Should return error for nonexistent attachment"
    );
}

/// Test empty attachment list for new project
#[tokio::test]
async fn test_empty_attachment_list() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = setup_project(&tc).await;

    let attachments = AttachmentBmc::list_by_project(&tc.ctx, &tc.mm, project_id)
        .await
        .expect("Failed to list attachments");

    assert!(
        attachments.is_empty(),
        "New project should have no attachments"
    );
}

/// Test attachment with various media types
#[tokio::test]
async fn test_various_media_types() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let project_id = setup_project(&tc).await;

    let media_types = vec![
        ("code.rs", "text/x-rust"),
        ("archive.tar.gz", "application/gzip"),
        ("video.mp4", "video/mp4"),
        ("audio.mp3", "audio/mpeg"),
    ];

    for (filename, media_type) in media_types {
        let attachment_c = AttachmentForCreate {
            project_id,
            filename: filename.to_string(),
            stored_path: format!("/data/{}", filename),
            media_type: media_type.to_string(),
            size_bytes: 1000,
        };

        let id = AttachmentBmc::create(&tc.ctx, &tc.mm, attachment_c)
            .await
            .expect("Failed to create attachment");

        let attachment = AttachmentBmc::get(&tc.ctx, &tc.mm, id)
            .await
            .expect("Failed to get attachment");

        assert_eq!(attachment.media_type, media_type);
    }
}
