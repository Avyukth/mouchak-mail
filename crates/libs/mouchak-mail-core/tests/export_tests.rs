//! Export model tests
//!
//! Tests for mailbox export functionality in various formats.

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
use mouchak_mail_core::model::export::{ExportBmc, ExportFormat, ScrubMode};
use mouchak_mail_core::model::message::{MessageBmc, MessageForCreate};
use mouchak_mail_core::model::project::ProjectBmc;
use mouchak_mail_core::types::ProjectId;
use mouchak_mail_core::utils::slugify;

/// Helper to set up a project with messages for export tests
async fn setup_project_with_messages(tc: &TestContext, suffix: &str) -> (ProjectId, String) {
    let human_key = format!("/test/export-repo-{}", suffix);
    let slug = slugify(&human_key);

    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, &human_key)
        .await
        .expect("Failed to create project");

    // Create sender agent
    let sender = AgentForCreate {
        project_id,
        name: "sender-agent".to_string(),
        program: "claude-code".to_string(),
        model: "claude-3".to_string(),
        task_description: "Sending messages".to_string(),
    };
    let sender_id = AgentBmc::create(&tc.ctx, &tc.mm, sender)
        .await
        .expect("Failed to create sender");

    // Create recipient agent
    let recipient = AgentForCreate {
        project_id,
        name: "recipient-agent".to_string(),
        program: "cursor".to_string(),
        model: "gpt-4".to_string(),
        task_description: "Receiving messages".to_string(),
    };
    let recipient_id = AgentBmc::create(&tc.ctx, &tc.mm, recipient)
        .await
        .expect("Failed to create recipient");

    // Create some messages
    for i in 1..=3 {
        let msg = MessageForCreate {
            project_id: project_id.get(),
            sender_id: sender_id.into(),
            recipient_ids: vec![recipient_id.into()],
            cc_ids: None,
            bcc_ids: None,
            subject: format!("Test Message {}", i),
            body_md: format!("This is the body of message {}.", i),
            thread_id: None,
            importance: None,
            ack_required: false,
        };
        MessageBmc::create(&tc.ctx, &tc.mm, msg)
            .await
            .expect("Failed to create message");
    }

    (project_id, slug)
}

/// Test exporting mailbox in JSON format
#[tokio::test]
async fn test_export_json() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (_, slug) = setup_project_with_messages(&tc, "json").await;

    let exported = ExportBmc::export_mailbox(
        &tc.ctx,
        &tc.mm,
        &slug,
        ExportFormat::Json,
        ScrubMode::None,
        false,
    )
    .await
    .expect("Failed to export mailbox");

    assert_eq!(exported.project_slug, slug);
    assert_eq!(exported.format, "json");
    assert_eq!(exported.message_count, 3);
    assert!(exported.content.contains("Test Message"));
    assert!(
        exported.content.starts_with('['),
        "JSON should start with array"
    );
}

/// Test exporting mailbox in HTML format
#[tokio::test]
async fn test_export_html() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (_, slug) = setup_project_with_messages(&tc, "html").await;

    let exported = ExportBmc::export_mailbox(
        &tc.ctx,
        &tc.mm,
        &slug,
        ExportFormat::Html,
        ScrubMode::None,
        false,
    )
    .await
    .expect("Failed to export mailbox");

    assert_eq!(exported.format, "html");
    assert!(exported.content.contains("<!DOCTYPE html>"));
    assert!(exported.content.contains("<title>"));
    assert!(exported.content.contains("Test Message"));
}

/// Test exporting mailbox in Markdown format
#[tokio::test]
async fn test_export_markdown() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (_, slug) = setup_project_with_messages(&tc, "md").await;

    let exported = ExportBmc::export_mailbox(
        &tc.ctx,
        &tc.mm,
        &slug,
        ExportFormat::Markdown,
        ScrubMode::None,
        false,
    )
    .await
    .expect("Failed to export mailbox");

    assert_eq!(exported.format, "markdown");
    assert!(exported.content.contains("# Mailbox Export"));
    assert!(exported.content.contains("## Test Message"));
    assert!(exported.content.contains("**From:**"));
}

/// Test exporting mailbox in CSV format
#[tokio::test]
async fn test_export_csv() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (_, slug) = setup_project_with_messages(&tc, "csv").await;

    let exported = ExportBmc::export_mailbox(
        &tc.ctx,
        &tc.mm,
        &slug,
        ExportFormat::Csv,
        ScrubMode::None,
        false,
    )
    .await
    .expect("Failed to export mailbox");

    assert_eq!(exported.format, "csv");
    assert!(
        exported
            .content
            .contains("id,created_at,sender,subject,body")
    );
    assert!(exported.content.contains("Test Message"));
}

/// Test exporting empty mailbox
#[tokio::test]
async fn test_export_empty_mailbox() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let human_key = "/test/empty-export-repo";
    let slug = slugify(human_key);

    ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");

    let exported = ExportBmc::export_mailbox(
        &tc.ctx,
        &tc.mm,
        &slug,
        ExportFormat::Json,
        ScrubMode::None,
        false,
    )
    .await
    .expect("Failed to export mailbox");

    assert_eq!(exported.message_count, 0);
    assert_eq!(exported.content, "[]"); // Empty JSON array
}

/// Test export format parsing
#[tokio::test]
async fn test_export_format_parsing() {
    use std::str::FromStr;

    assert_eq!(ExportFormat::from_str("html").unwrap(), ExportFormat::Html);
    assert_eq!(ExportFormat::from_str("HTML").unwrap(), ExportFormat::Html);
    assert_eq!(ExportFormat::from_str("json").unwrap(), ExportFormat::Json);
    assert_eq!(
        ExportFormat::from_str("md").unwrap(),
        ExportFormat::Markdown
    );
    assert_eq!(
        ExportFormat::from_str("markdown").unwrap(),
        ExportFormat::Markdown
    );
    assert_eq!(ExportFormat::from_str("csv").unwrap(), ExportFormat::Csv);
    // Unknown defaults to JSON
    assert_eq!(
        ExportFormat::from_str("unknown").unwrap(),
        ExportFormat::Json
    );
}

/// Test export for nonexistent project
#[tokio::test]
async fn test_export_nonexistent_project() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let result = ExportBmc::export_mailbox(
        &tc.ctx,
        &tc.mm,
        "nonexistent-slug",
        ExportFormat::Json,
        ScrubMode::None,
        false,
    )
    .await;

    assert!(result.is_err(), "Should fail for nonexistent project");
}

/// Test exported_at timestamp is set
#[tokio::test]
async fn test_export_timestamp() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (_, slug) = setup_project_with_messages(&tc, "timestamp").await;

    let exported = ExportBmc::export_mailbox(
        &tc.ctx,
        &tc.mm,
        &slug,
        ExportFormat::Json,
        ScrubMode::None,
        false,
    )
    .await
    .expect("Failed to export mailbox");

    assert!(!exported.exported_at.is_empty());
    assert!(exported.exported_at.contains("UTC"));
}

/// Test commit_archive creates a git commit with exported mailbox
#[tokio::test]
async fn test_commit_archive() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (_, slug) = setup_project_with_messages(&tc, "archive").await;

    let commit_message = "Archive mailbox for testing";
    let oid = ExportBmc::commit_archive(&tc.ctx, &tc.mm, &slug, commit_message)
        .await
        .expect("Failed to commit archive");

    // Verify OID is a valid git hash (40 hex characters)
    assert_eq!(oid.len(), 40, "Git OID should be 40 characters");
    assert!(
        oid.chars().all(|c| c.is_ascii_hexdigit()),
        "Git OID should be hexadecimal"
    );
}

/// Test commit_archive for empty mailbox
#[tokio::test]
async fn test_commit_archive_empty_mailbox() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let human_key = "/test/empty-archive-repo";
    let slug = slugify(human_key);

    ProjectBmc::create(&tc.ctx, &tc.mm, &slug, human_key)
        .await
        .expect("Failed to create project");

    let oid = ExportBmc::commit_archive(&tc.ctx, &tc.mm, &slug, "Archive empty mailbox")
        .await
        .expect("Failed to commit empty archive");

    // Should still create a valid commit even with no messages
    assert_eq!(oid.len(), 40, "Git OID should be 40 characters");
}

/// Test commit_archive for nonexistent project fails
#[tokio::test]
async fn test_commit_archive_nonexistent_project() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let result =
        ExportBmc::commit_archive(&tc.ctx, &tc.mm, "nonexistent-archive-slug", "Should fail").await;

    assert!(
        result.is_err(),
        "commit_archive should fail for nonexistent project"
    );
}

#[tokio::test]
async fn test_export_signing_keypair_generation() {
    use mouchak_mail_core::model::export::{
        generate_signing_keypair, signing_key_to_base64, verifying_key_to_base64,
    };

    let (signing_key, verifying_key) = generate_signing_keypair();

    let private_b64 = signing_key_to_base64(&signing_key);
    let public_b64 = verifying_key_to_base64(&verifying_key);

    assert!(!private_b64.is_empty());
    assert!(!public_b64.is_empty());
    assert_ne!(private_b64, public_b64);
}

#[tokio::test]
async fn test_export_manifest_signing() {
    use mouchak_mail_core::model::export::{ExportManifest, ExportedMailbox, generate_signing_keypair};

    let exported = ExportedMailbox {
        project_slug: "test-project".to_string(),
        project_name: "Test Project".to_string(),
        message_count: 5,
        exported_at: "2025-12-20T00:00:00Z".to_string(),
        content: "test content".to_string(),
        format: "json".to_string(),
    };

    let (signing_key, _) = generate_signing_keypair();
    let mut manifest = ExportManifest::new(&exported);

    assert!(manifest.signature.is_none());
    assert!(manifest.public_key.is_none());

    manifest.sign(&signing_key);

    assert!(manifest.signature.is_some());
    assert!(manifest.public_key.is_some());
}

#[tokio::test]
async fn test_export_manifest_verification() {
    use mouchak_mail_core::model::export::{ExportManifest, ExportedMailbox, generate_signing_keypair};

    let exported = ExportedMailbox {
        project_slug: "test-project".to_string(),
        project_name: "Test Project".to_string(),
        message_count: 5,
        exported_at: "2025-12-20T00:00:00Z".to_string(),
        content: "test content".to_string(),
        format: "json".to_string(),
    };

    let (signing_key, _) = generate_signing_keypair();
    let mut manifest = ExportManifest::new(&exported);
    manifest.sign(&signing_key);

    let verified = manifest.verify().expect("Verification should succeed");
    assert!(verified, "Signed manifest should verify");
}

#[tokio::test]
async fn test_export_manifest_tamper_detection() {
    use mouchak_mail_core::model::export::{ExportManifest, ExportedMailbox, generate_signing_keypair};

    let exported = ExportedMailbox {
        project_slug: "test-project".to_string(),
        project_name: "Test Project".to_string(),
        message_count: 5,
        exported_at: "2025-12-20T00:00:00Z".to_string(),
        content: "test content".to_string(),
        format: "json".to_string(),
    };

    let (signing_key, _) = generate_signing_keypair();
    let mut manifest = ExportManifest::new(&exported);
    manifest.sign(&signing_key);

    manifest.message_count = 10;

    let verified = manifest.verify().expect("Verification call should succeed");
    assert!(!verified, "Tampered manifest should not verify");
}

#[tokio::test]
async fn test_export_verify_with_external_key() {
    use mouchak_mail_core::model::export::{
        ExportManifest, ExportedMailbox, generate_signing_keypair, verifying_key_to_base64,
    };

    let exported = ExportedMailbox {
        project_slug: "test-project".to_string(),
        project_name: "Test Project".to_string(),
        message_count: 5,
        exported_at: "2025-12-20T00:00:00Z".to_string(),
        content: "test content".to_string(),
        format: "json".to_string(),
    };

    let (signing_key, verifying_key) = generate_signing_keypair();
    let public_b64 = verifying_key_to_base64(&verifying_key);

    let mut manifest = ExportManifest::new(&exported);
    manifest.sign(&signing_key);

    let verified = manifest
        .verify_with_key(&public_b64)
        .expect("Verification should succeed");
    assert!(verified, "Should verify with correct public key");
}

#[tokio::test]
async fn test_export_verify_wrong_key_fails() {
    use mouchak_mail_core::model::export::{
        ExportManifest, ExportedMailbox, generate_signing_keypair, verifying_key_to_base64,
    };

    let exported = ExportedMailbox {
        project_slug: "test-project".to_string(),
        project_name: "Test Project".to_string(),
        message_count: 5,
        exported_at: "2025-12-20T00:00:00Z".to_string(),
        content: "test content".to_string(),
        format: "json".to_string(),
    };

    let (signing_key, _) = generate_signing_keypair();
    let (_, wrong_key) = generate_signing_keypair();
    let wrong_public_b64 = verifying_key_to_base64(&wrong_key);

    let mut manifest = ExportManifest::new(&exported);
    manifest.sign(&signing_key);

    let verified = manifest
        .verify_with_key(&wrong_public_b64)
        .expect("Verification call should succeed");
    assert!(!verified, "Should fail with wrong public key");
}

#[tokio::test]
async fn test_export_mailbox_signed() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    use mouchak_mail_core::model::export::generate_signing_keypair;

    let (_, slug) = setup_project_with_messages(&tc, "signed").await;
    let (signing_key, _) = generate_signing_keypair();

    let (exported, manifest) = ExportBmc::export_mailbox_signed(
        &tc.ctx,
        &tc.mm,
        &slug,
        ExportFormat::Json,
        ScrubMode::None,
        false,
        Some(&signing_key),
    )
    .await
    .expect("Failed to export signed mailbox");

    assert!(manifest.signature.is_some());
    assert!(manifest.public_key.is_some());

    let verified =
        ExportBmc::verify_export(&exported, &manifest).expect("Verification should succeed");
    assert!(verified, "Signed export should verify");
}

#[tokio::test]
async fn test_age_identity_generation() {
    use mouchak_mail_core::model::export::generate_age_identity;

    let (identity, recipient) = generate_age_identity();

    // Identity should start with "AGE-SECRET-KEY-"
    assert!(
        identity.starts_with("AGE-SECRET-KEY-"),
        "Identity should start with AGE-SECRET-KEY-"
    );

    // Recipient should start with "age1"
    assert!(
        recipient.starts_with("age1"),
        "Recipient should start with age1"
    );

    // Both should be valid strings
    assert!(!identity.is_empty(), "Identity should not be empty");
    assert!(!recipient.is_empty(), "Recipient should not be empty");
}

#[tokio::test]
async fn test_age_encrypt_decrypt_with_passphrase() {
    use mouchak_mail_core::model::export::{decrypt_with_passphrase, encrypt_with_passphrase};

    let test_data = b"Hello, world! This is a test message for age encryption.";
    let passphrase = "test-passphrase-123";

    // Encrypt
    let encrypted =
        encrypt_with_passphrase(test_data, passphrase).expect("Encryption should succeed");

    // Should be armored (ASCII)
    let encrypted_str = String::from_utf8(encrypted.clone()).expect("Should be valid UTF-8");
    assert!(
        encrypted_str.contains("-----BEGIN AGE ENCRYPTED FILE-----"),
        "Should contain armor header"
    );

    // Decrypt
    let decrypted =
        decrypt_with_passphrase(&encrypted, passphrase).expect("Decryption should succeed");

    // Should match original
    assert_eq!(decrypted, test_data, "Decrypted data should match original");
}

#[tokio::test]
async fn test_age_encrypt_decrypt_with_keypair() {
    use mouchak_mail_core::model::export::{decrypt_with_identity, encrypt_with_age, generate_age_identity};

    let test_data = b"Hello, world! This is a test message for age key encryption.";
    let (identity, recipient) = generate_age_identity();

    // Encrypt
    let encrypted = encrypt_with_age(test_data, &[recipient]).expect("Encryption should succeed");

    // Should be armored (ASCII)
    let encrypted_str = String::from_utf8(encrypted.clone()).expect("Should be valid UTF-8");
    assert!(
        encrypted_str.contains("-----BEGIN AGE ENCRYPTED FILE-----"),
        "Should contain armor header"
    );

    // Decrypt
    let decrypted =
        decrypt_with_identity(&encrypted, &identity).expect("Decryption should succeed");

    // Should match original
    assert_eq!(decrypted, test_data, "Decrypted data should match original");
}

#[tokio::test]
async fn test_age_wrong_passphrase_fails() {
    use mouchak_mail_core::model::export::{decrypt_with_passphrase, encrypt_with_passphrase};

    let test_data = b"Secret message";
    let passphrase = "correct-passphrase";
    let wrong_passphrase = "wrong-passphrase";

    // Encrypt
    let encrypted =
        encrypt_with_passphrase(test_data, passphrase).expect("Encryption should succeed");

    // Try to decrypt with wrong passphrase - should fail
    let result = decrypt_with_passphrase(&encrypted, wrong_passphrase);
    assert!(
        result.is_err(),
        "Decryption with wrong passphrase should fail"
    );
}

#[tokio::test]
async fn test_age_wrong_identity_fails() {
    use mouchak_mail_core::model::export::{decrypt_with_identity, encrypt_with_age, generate_age_identity};

    let test_data = b"Secret message";
    let (_identity, recipient) = generate_age_identity();
    let (wrong_identity, _) = generate_age_identity();

    // Encrypt
    let encrypted = encrypt_with_age(test_data, &[recipient]).expect("Encryption should succeed");

    // Try to decrypt with wrong identity - should fail
    let result = decrypt_with_identity(&encrypted, &wrong_identity);
    assert!(
        result.is_err(),
        "Decryption with wrong identity should fail"
    );
}

// ============================================================================
// Additional Share/Export Tests for Security & Integrity
// ============================================================================

/// Test content hash computation using SHA-1 (as used by export module)
#[tokio::test]
async fn test_content_hash_computation() {
    use sha1::Digest;

    let content = b"Hello, World!";
    let mut hasher = sha1::Sha1::new();
    hasher.update(content);
    let hash = hex::encode(hasher.finalize());

    // SHA-1 produces 40-character hex strings
    assert_eq!(hash.len(), 40, "SHA-1 hash should be 40 hex characters");
    // Known SHA-1 hash for "Hello, World!"
    assert_eq!(hash, "0a0a9f2a6772942557ab5355d76af442f8f65e01");
}

/// Test content hash in manifest matches actual content
#[tokio::test]
async fn test_manifest_content_hash_integrity() {
    use mouchak_mail_core::model::export::{ExportManifest, ExportedMailbox};
    use sha1::Digest;

    let content = "Test content for hashing";
    let exported = ExportedMailbox {
        project_slug: "test".to_string(),
        project_name: "Test".to_string(),
        message_count: 1,
        exported_at: "2025-12-20T00:00:00Z".to_string(),
        content: content.to_string(),
        format: "json".to_string(),
    };

    let manifest = ExportManifest::new(&exported);

    // Compute hash manually
    let mut hasher = sha1::Sha1::new();
    hasher.update(content.as_bytes());
    let expected_hash = hex::encode(hasher.finalize());

    assert_eq!(
        manifest.content_hash, expected_hash,
        "Manifest hash should match computed hash"
    );
}

/// Test manifest structure has all required fields
#[tokio::test]
async fn test_manifest_structure_completeness() {
    use mouchak_mail_core::model::export::{ExportManifest, ExportedMailbox};

    let exported = ExportedMailbox {
        project_slug: "test-project".to_string(),
        project_name: "Test Project Name".to_string(),
        message_count: 42,
        exported_at: "2025-12-20T12:00:00Z".to_string(),
        content: "content".to_string(),
        format: "markdown".to_string(),
    };

    let manifest = ExportManifest::new(&exported);

    // Verify all fields are set correctly
    assert_eq!(manifest.version, "1.0");
    assert_eq!(manifest.project_slug, "test-project");
    assert_eq!(manifest.message_count, 42);
    assert_eq!(manifest.format, "markdown");
    assert!(!manifest.content_hash.is_empty());
    assert!(
        manifest.signature.is_none(),
        "Unsigned manifest should have no signature"
    );
    assert!(
        manifest.public_key.is_none(),
        "Unsigned manifest should have no public key"
    );
}

/// Test signing key serialization roundtrip
#[tokio::test]
async fn test_signing_key_serialization_roundtrip() {
    use mouchak_mail_core::model::export::{
        generate_signing_keypair, signing_key_from_base64, signing_key_to_base64,
    };

    let (original_key, _) = generate_signing_keypair();
    let serialized = signing_key_to_base64(&original_key);
    let deserialized =
        signing_key_from_base64(&serialized).expect("Deserialization should succeed");

    // Sign same message with both keys - signatures should match
    use ed25519_dalek::Signer;
    let message = b"test message";
    let sig1 = original_key.sign(message);
    let sig2 = deserialized.sign(message);

    assert_eq!(
        sig1.to_bytes(),
        sig2.to_bytes(),
        "Signatures should match after roundtrip"
    );
}

/// Test verifying key serialization roundtrip
#[tokio::test]
async fn test_verifying_key_serialization_roundtrip() {
    use mouchak_mail_core::model::export::{
        generate_signing_keypair, verifying_key_from_base64, verifying_key_to_base64,
    };

    let (signing_key, original_verifying_key) = generate_signing_keypair();
    let serialized = verifying_key_to_base64(&original_verifying_key);
    let deserialized =
        verifying_key_from_base64(&serialized).expect("Deserialization should succeed");

    // Both keys should verify the same signature
    use ed25519_dalek::{Signer, Verifier};
    let message = b"test message";
    let signature = signing_key.sign(message);

    assert!(original_verifying_key.verify(message, &signature).is_ok());
    assert!(deserialized.verify(message, &signature).is_ok());
}

/// Test scrub mode parsing from strings
#[tokio::test]
async fn test_scrub_mode_parsing() {
    use mouchak_mail_core::model::export::ScrubMode;
    use std::str::FromStr;

    assert_eq!(ScrubMode::from_str("none").unwrap(), ScrubMode::None);
    assert_eq!(ScrubMode::from_str("NONE").unwrap(), ScrubMode::None);
    assert_eq!(
        ScrubMode::from_str("standard").unwrap(),
        ScrubMode::Standard
    );
    assert_eq!(
        ScrubMode::from_str("STANDARD").unwrap(),
        ScrubMode::Standard
    );
    assert_eq!(
        ScrubMode::from_str("aggressive").unwrap(),
        ScrubMode::Aggressive
    );
    assert_eq!(
        ScrubMode::from_str("AGGRESSIVE").unwrap(),
        ScrubMode::Aggressive
    );
    // Unknown defaults to None
    assert_eq!(ScrubMode::from_str("unknown").unwrap(), ScrubMode::None);
}

/// Test encrypted export with identity roundtrip
#[tokio::test]
async fn test_encrypted_export_identity_roundtrip() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    use mouchak_mail_core::model::export::{ExportBmc, generate_age_identity, generate_signing_keypair};

    let (_, slug) = setup_project_with_messages(&tc, "enc-roundtrip").await;
    let (identity, recipient) = generate_age_identity();
    let (signing_key, _) = generate_signing_keypair();

    // Export with encryption
    let (encrypted, manifest) = ExportBmc::export_mailbox_encrypted(
        &tc.ctx,
        &tc.mm,
        &slug,
        ExportFormat::Json,
        ScrubMode::None,
        false,
        &[recipient],
        Some(&signing_key),
    )
    .await
    .expect("Encrypted export should succeed");

    assert!(!encrypted.is_empty());
    assert!(manifest.signature.is_some());

    // Decrypt and verify
    let (decrypted, dec_manifest) = ExportBmc::decrypt_export_with_identity(&encrypted, &identity)
        .expect("Decryption should succeed");

    assert_eq!(decrypted.project_slug, slug);
    assert_eq!(decrypted.message_count, 3);
    assert!(dec_manifest.verify().expect("Verification should succeed"));
}

/// Test encrypted export with passphrase roundtrip
#[tokio::test]
async fn test_encrypted_export_passphrase_roundtrip() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    use mouchak_mail_core::model::export::{ExportBmc, generate_signing_keypair};

    let (_, slug) = setup_project_with_messages(&tc, "pass-roundtrip").await;
    let passphrase = "my-secure-passphrase-123";
    let (signing_key, _) = generate_signing_keypair();

    // Export with passphrase encryption
    let (encrypted, _manifest) = ExportBmc::export_mailbox_passphrase(
        &tc.ctx,
        &tc.mm,
        &slug,
        ExportFormat::Markdown,
        ScrubMode::None,
        false,
        passphrase,
        Some(&signing_key),
    )
    .await
    .expect("Passphrase export should succeed");

    // Decrypt
    let (decrypted, dec_manifest) =
        ExportBmc::decrypt_export_with_passphrase(&encrypted, passphrase)
            .expect("Decryption should succeed");

    assert_eq!(decrypted.project_slug, slug);
    assert!(dec_manifest.verify().expect("Verification should succeed"));
}

/// Test bundle without signature verifies by content hash only
#[tokio::test]
async fn test_verify_bundle_without_signature() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    let (_, slug) = setup_project_with_messages(&tc, "no-sig").await;

    // Export without signing key
    let (exported, manifest) = ExportBmc::export_mailbox_signed(
        &tc.ctx,
        &tc.mm,
        &slug,
        ExportFormat::Json,
        ScrubMode::None,
        false,
        None, // No signing key
    )
    .await
    .expect("Export should succeed");

    assert!(manifest.signature.is_none());
    assert!(manifest.public_key.is_none());

    // Verify should still pass based on content hash
    let verified =
        ExportBmc::verify_export(&exported, &manifest).expect("Verification should succeed");
    assert!(verified, "Unsigned export should verify by content hash");
}

/// Test verification fails when content is tampered
#[tokio::test]
async fn test_verify_bundle_content_tampered() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    use mouchak_mail_core::model::export::generate_signing_keypair;

    let (_, slug) = setup_project_with_messages(&tc, "tamper").await;
    let (signing_key, _) = generate_signing_keypair();

    let (mut exported, manifest) = ExportBmc::export_mailbox_signed(
        &tc.ctx,
        &tc.mm,
        &slug,
        ExportFormat::Json,
        ScrubMode::None,
        false,
        Some(&signing_key),
    )
    .await
    .expect("Export should succeed");

    // Tamper with content
    exported.content = "TAMPERED CONTENT".to_string();

    // Verify should fail
    let verified =
        ExportBmc::verify_export(&exported, &manifest).expect("Verification call should succeed");
    assert!(!verified, "Tampered content should not verify");
}

/// Test multi-recipient encryption
#[tokio::test]
async fn test_age_multi_recipient_encryption() {
    use mouchak_mail_core::model::export::{decrypt_with_identity, encrypt_with_age, generate_age_identity};

    let test_data = b"Message for multiple recipients";

    // Generate two recipient identities
    let (identity1, recipient1) = generate_age_identity();
    let (identity2, recipient2) = generate_age_identity();

    // Encrypt for both recipients
    let encrypted = encrypt_with_age(test_data, &[recipient1, recipient2])
        .expect("Multi-recipient encryption should succeed");

    // Both identities should be able to decrypt
    let decrypted1 =
        decrypt_with_identity(&encrypted, &identity1).expect("Identity 1 should decrypt");
    let decrypted2 =
        decrypt_with_identity(&encrypted, &identity2).expect("Identity 2 should decrypt");

    assert_eq!(decrypted1, test_data);
    assert_eq!(decrypted2, test_data);
}

/// Test export includes HTML escaping for XSS prevention
#[tokio::test]
async fn test_export_html_xss_prevention() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    use mouchak_mail_core::model::agent::{AgentBmc, AgentForCreate};
    use mouchak_mail_core::model::message::{MessageBmc, MessageForCreate};
    use mouchak_mail_core::model::project::ProjectBmc;

    let slug = mouchak_mail_core::utils::slugify("/test/xss-export");
    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, "/test/xss-export")
        .await
        .expect("Create project");

    let sender_id = AgentBmc::create(
        &tc.ctx,
        &tc.mm,
        AgentForCreate {
            project_id,
            name: "sender".into(),
            program: "test".into(),
            model: "test".into(),
            task_description: "test".into(),
        },
    )
    .await
    .expect("Create sender");

    // Create message with XSS payload
    let xss_payload = "<script>alert('XSS')</script>";
    MessageBmc::create(
        &tc.ctx,
        &tc.mm,
        MessageForCreate {
            project_id: project_id.get(),
            sender_id: sender_id.into(),
            recipient_ids: vec![],
            cc_ids: None,
            bcc_ids: None,
            subject: xss_payload.to_string(),
            body_md: xss_payload.to_string(),
            thread_id: None,
            importance: None,
            ack_required: false,
        },
    )
    .await
    .expect("Create message");

    // Export as HTML
    let exported = ExportBmc::export_mailbox(
        &tc.ctx,
        &tc.mm,
        &slug,
        ExportFormat::Html,
        ScrubMode::None,
        false,
    )
    .await
    .expect("Export should succeed");

    // Should be escaped
    assert!(
        !exported.content.contains("<script>"),
        "HTML export should escape script tags"
    );
    assert!(
        exported.content.contains("&lt;script&gt;"),
        "Script tags should be HTML escaped"
    );
}

/// Test export with unicode content
#[tokio::test]
async fn test_export_unicode_content() {
    let tc = TestContext::new()
        .await
        .expect("Failed to create test context");

    use mouchak_mail_core::model::agent::{AgentBmc, AgentForCreate};
    use mouchak_mail_core::model::message::{MessageBmc, MessageForCreate};
    use mouchak_mail_core::model::project::ProjectBmc;

    let slug = mouchak_mail_core::utils::slugify("/test/unicode-export");
    let project_id = ProjectBmc::create(&tc.ctx, &tc.mm, &slug, "/test/unicode-export")
        .await
        .expect("Create project");

    let sender_id = AgentBmc::create(
        &tc.ctx,
        &tc.mm,
        AgentForCreate {
            project_id,
            name: "送信者".into(), // Japanese "sender"
            program: "test".into(),
            model: "test".into(),
            task_description: "test".into(),
        },
    )
    .await
    .expect("Create sender");

    // Create message with unicode
    MessageBmc::create(
        &tc.ctx,
        &tc.mm,
        MessageForCreate {
            project_id: project_id.get(),
            sender_id: sender_id.into(),
            recipient_ids: vec![],
            cc_ids: None,
            bcc_ids: None,
            subject: "こんにちは世界".to_string(), // "Hello World" in Japanese
            body_md: "Привет мир 🌍".to_string(),  // Russian + emoji
            thread_id: None,
            importance: None,
            ack_required: false,
        },
    )
    .await
    .expect("Create message");

    // Export as JSON
    let exported = ExportBmc::export_mailbox(
        &tc.ctx,
        &tc.mm,
        &slug,
        ExportFormat::Json,
        ScrubMode::None,
        false,
    )
    .await
    .expect("Export should succeed");

    assert!(exported.content.contains("こんにちは世界"));
    assert!(exported.content.contains("Привет мир"));
    assert!(exported.content.contains("🌍"));
}

/// Test scrubber handles edge cases
#[tokio::test]
async fn test_scrubber_edge_cases() {
    use mouchak_mail_core::model::export::Scrubber;

    let scrubber = Scrubber::new(ScrubMode::Standard);

    // Empty string
    assert_eq!(scrubber.scrub(""), "");

    // No PII
    assert_eq!(scrubber.scrub("Hello World"), "Hello World");

    // Multiple emails in one string
    let multi_email = "Contact alice@example.com or bob@example.org";
    let scrubbed = scrubber.scrub(multi_email);
    assert!(!scrubbed.contains("alice@example.com"));
    assert!(!scrubbed.contains("bob@example.org"));
    assert_eq!(scrubbed.matches("[EMAIL]").count(), 2);

    // Phone number variations
    let phones = "Call (555) 123-4567 or 555.987.6543";
    let scrubbed_phones = scrubber.scrub(phones);
    assert!(scrubbed_phones.contains("[PHONE]"));
}

/// Test aggressive scrubbing of credit cards and SSNs
#[tokio::test]
async fn test_aggressive_scrub_sensitive_numbers() {
    use mouchak_mail_core::model::export::Scrubber;

    let scrubber = Scrubber::new(ScrubMode::Aggressive);

    // Credit card
    let cc_text = "Card: 4111-1111-1111-1111";
    let scrubbed = scrubber.scrub(cc_text);
    assert!(scrubbed.contains("[CREDIT-CARD]"));
    assert!(!scrubbed.contains("4111"));

    // SSN
    let ssn_text = "SSN: 123-45-6789";
    let scrubbed_ssn = scrubber.scrub(ssn_text);
    assert!(scrubbed_ssn.contains("[SSN]"));
    assert!(!scrubbed_ssn.contains("123-45-6789"));
}

/// Test export format default behavior
#[tokio::test]
async fn test_export_format_default() {
    use std::str::FromStr;

    // Unknown format should default to JSON
    let format = ExportFormat::from_str("xml").unwrap();
    assert_eq!(format, ExportFormat::Json);

    let format2 = ExportFormat::from_str("pdf").unwrap();
    assert_eq!(format2, ExportFormat::Json);
}
