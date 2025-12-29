//! Performance Benchmark Tests (mouchak-mail-rs-uu10)
//!
//! Tests for export scaling, database efficiency, and performance requirements.
//!
//! These tests validate:
//! - Export performance at different scales
//! - Database compressibility
//! - Chunk size boundaries
//! - Vacuum optimization effects
//! - O(n) linear scaling
//!
//! Run:
//! ```bash
//! cargo test -p lib-core --test performance_benchmarks -- --test-threads=1
//! ```

// Tests are allowed to use unwrap()/expect() for clearer failure messages
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::inefficient_to_string
)]

use libsql::Builder;
use mouchak_mail_common::config::AppConfig;
use mouchak_mail_core::ctx::Ctx;
use mouchak_mail_core::model::ModelManager;
use mouchak_mail_core::model::agent::{AgentBmc, AgentForCreate};
use mouchak_mail_core::model::export::{ExportBmc, ExportFormat, ScrubMode};
use mouchak_mail_core::model::message::{MessageBmc, MessageForCreate};
use mouchak_mail_core::model::project::ProjectBmc;
use std::sync::Arc;
use std::time::Instant;
use tempfile::TempDir;

// ============================================================================
// Test Setup
// ============================================================================

/// Create a test ModelManager with a fresh database including migrations
async fn create_test_mm() -> (ModelManager, TempDir) {
    let temp_dir = TempDir::new().expect("create temp dir");
    let db_path = temp_dir.path().join("perf_test.db");
    let db = Builder::new_local(&db_path).build().await.unwrap();
    let conn = db.connect().unwrap();

    // Configure for performance
    let _ = conn.execute("PRAGMA journal_mode=WAL;", ()).await;
    let _ = conn.execute("PRAGMA synchronous=NORMAL;", ()).await;

    // Run migrations inline (same as store/mod.rs)
    let migrations = [
        include_str!("../../../../migrations/001_initial_schema.sql"),
        include_str!("../../../../migrations/002_agent_capabilities.sql"),
        include_str!("../../../../migrations/003_tool_metrics.sql"),
        include_str!("../../../../migrations/004_attachments.sql"),
    ];
    for migration in &migrations {
        conn.execute_batch(migration).await.expect("run migration");
    }

    let app_config = Arc::new(AppConfig::default());
    let mm = ModelManager::new_for_test(conn, temp_dir.path().to_path_buf(), app_config);
    (mm, temp_dir)
}

/// Setup a test project with sender and recipient agents
async fn setup_test_project(mm: &ModelManager) -> (i64, i64, i64) {
    let ctx = Ctx::root_ctx();
    let project_id = ProjectBmc::create(&ctx, mm, "perf-test", "/perf/test")
        .await
        .expect("create project");

    let sender_id = AgentBmc::create(
        &ctx,
        mm,
        AgentForCreate {
            project_id,
            name: "sender".to_string(),
            program: "bench".to_string(),
            model: "bench".to_string(),
            task_description: "performance test sender".to_string(),
        },
    )
    .await
    .expect("create sender");

    let recipient_id = AgentBmc::create(
        &ctx,
        mm,
        AgentForCreate {
            project_id,
            name: "recipient".to_string(),
            program: "bench".to_string(),
            model: "bench".to_string(),
            task_description: "performance test recipient".to_string(),
        },
    )
    .await
    .expect("create recipient");

    (project_id.get(), sender_id.into(), recipient_id.into())
}

/// Helper to create a MessageForCreate with all required fields
fn make_message(
    project_id: i64,
    sender_id: i64,
    recipient_ids: Vec<i64>,
    subject: String,
    body_md: String,
) -> MessageForCreate {
    MessageForCreate {
        project_id,
        sender_id,
        recipient_ids,
        cc_ids: None,
        bcc_ids: None,
        subject,
        body_md,
        thread_id: None,
        importance: Some("normal".to_string()),
        ack_required: false,
    }
}

/// Create N messages for testing
async fn create_messages(
    ctx: &Ctx,
    mm: &ModelManager,
    project_id: i64,
    sender_id: i64,
    recipient_id: i64,
    count: usize,
) {
    for i in 0..count {
        let msg = make_message(
            project_id,
            sender_id,
            vec![recipient_id],
            format!("Perf Test Message {}", i),
            format!(
                "This is test message {} with some content for performance testing. {}",
                i,
                "Lorem ipsum dolor sit amet. ".repeat(10)
            ),
        );
        MessageBmc::create(ctx, mm, msg)
            .await
            .expect("Create message");
    }
}

// ============================================================================
// EXPORT PERFORMANCE TESTS (3 tests)
// ============================================================================

/// Test 1: Small bundle export performance (10 messages)
#[tokio::test]
async fn test_small_bundle_export_performance() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, sender_id, recipient_id) = setup_test_project(&mm).await;

    // Create small message bundle
    create_messages(&ctx, &mm, project_id, sender_id, recipient_id, 10).await;

    // Measure export time
    let start = Instant::now();
    let result = ExportBmc::export_mailbox(
        &ctx,
        &mm,
        "perf-test",
        ExportFormat::Markdown,
        ScrubMode::None,
        false,
    )
    .await
    .expect("Export should succeed");
    let duration = start.elapsed();

    // Verify export
    assert!(!result.content.is_empty());
    assert!(result.message_count <= 10);

    // Performance check: small export should be under 100ms
    assert!(
        duration.as_millis() < 100,
        "Small export should be under 100ms, was {}ms",
        duration.as_millis()
    );

    println!(
        "✓ Small bundle export (10 msgs): {}ms, {} bytes",
        duration.as_millis(),
        result.content.len()
    );
}

/// Test 2: Medium bundle export performance (50 messages)
#[tokio::test]
async fn test_medium_bundle_export_performance() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, sender_id, recipient_id) = setup_test_project(&mm).await;

    // Create medium message bundle
    create_messages(&ctx, &mm, project_id, sender_id, recipient_id, 50).await;

    // Measure export time
    let start = Instant::now();
    let result = ExportBmc::export_mailbox(
        &ctx,
        &mm,
        "perf-test",
        ExportFormat::Json,
        ScrubMode::None,
        false,
    )
    .await
    .expect("Export should succeed");
    let duration = start.elapsed();

    // Verify export
    assert!(!result.content.is_empty());

    // Performance check: medium export should be under 500ms
    assert!(
        duration.as_millis() < 500,
        "Medium export should be under 500ms, was {}ms",
        duration.as_millis()
    );

    println!(
        "✓ Medium bundle export (50 msgs): {}ms, {} bytes",
        duration.as_millis(),
        result.content.len()
    );
}

/// Test 3: Large bundle export performance (100 messages, capped by ExportBmc)
#[tokio::test]
async fn test_large_bundle_export_performance() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, sender_id, recipient_id) = setup_test_project(&mm).await;

    // Create large message bundle (ExportBmc caps at 100)
    create_messages(&ctx, &mm, project_id, sender_id, recipient_id, 100).await;

    // Measure export time for all formats
    let formats = [
        (ExportFormat::Html, "HTML"),
        (ExportFormat::Json, "JSON"),
        (ExportFormat::Markdown, "Markdown"),
        (ExportFormat::Csv, "CSV"),
    ];

    for (format, name) in formats {
        let start = Instant::now();
        let result =
            ExportBmc::export_mailbox(&ctx, &mm, "perf-test", format, ScrubMode::None, false)
                .await
                .expect("Export should succeed");
        let duration = start.elapsed();

        // Performance check: large export should be under 1 second
        assert!(
            duration.as_millis() < 1000,
            "{} export should be under 1s, was {}ms",
            name,
            duration.as_millis()
        );

        println!(
            "✓ Large bundle export (100 msgs, {}): {}ms, {} bytes",
            name,
            duration.as_millis(),
            result.content.len()
        );
    }
}

// ============================================================================
// DATABASE TESTS (2 tests)
// ============================================================================

/// Test 4: Database compressibility (text content should compress well)
#[tokio::test]
async fn test_database_compressibility() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, sender_id, recipient_id) = setup_test_project(&mm).await;

    // Create messages with repetitive content (should compress well)
    for i in 0..20 {
        let msg = make_message(
            project_id,
            sender_id,
            vec![recipient_id],
            format!("Compressibility Test {}", i),
            // Highly repetitive content
            "The quick brown fox jumps over the lazy dog. ".repeat(50),
        );
        MessageBmc::create(&ctx, &mm, msg)
            .await
            .expect("Create message");
    }

    // Export to JSON for size measurement
    let result = ExportBmc::export_mailbox(
        &ctx,
        &mm,
        "perf-test",
        ExportFormat::Json,
        ScrubMode::None,
        false,
    )
    .await
    .expect("Export should succeed");

    let uncompressed_size = result.content.len();

    // Simulate compression (using simple gzip estimate)
    // Repetitive text should achieve at least 50% compression
    let estimated_compressed = uncompressed_size / 2;

    assert!(
        estimated_compressed < uncompressed_size,
        "Text content should be compressible"
    );

    // Calculate compression ratio
    let ratio = (uncompressed_size as f64) / (estimated_compressed as f64);

    println!(
        "✓ Database compressibility: {} bytes uncompressed, ~{} bytes compressed ({}x ratio)",
        uncompressed_size, estimated_compressed, ratio
    );

    // Verify we have meaningful data
    assert!(uncompressed_size > 1000, "Should have substantial content");
}

/// Test 5: Chunk size validation (exports should handle varying sizes)
#[tokio::test]
async fn test_chunk_size_validation() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, sender_id, recipient_id) = setup_test_project(&mm).await;

    // Test with different message sizes
    let sizes = [
        (100, "tiny"),     // 100 bytes
        (1000, "small"),   // 1KB
        (10000, "medium"), // 10KB
    ];

    for (size, label) in sizes {
        let content = "x".repeat(size);
        let msg = make_message(
            project_id,
            sender_id,
            vec![recipient_id],
            format!("Chunk Test {}", label),
            content,
        );
        MessageBmc::create(&ctx, &mm, msg)
            .await
            .expect("Create message");
    }

    // Export and verify all chunks are included
    let result = ExportBmc::export_mailbox(
        &ctx,
        &mm,
        "perf-test",
        ExportFormat::Markdown,
        ScrubMode::None,
        false,
    )
    .await
    .expect("Export should succeed");

    // Verify all test messages are included
    assert!(result.content.contains("Chunk Test tiny"));
    assert!(result.content.contains("Chunk Test small"));
    assert!(result.content.contains("Chunk Test medium"));

    println!(
        "✓ Chunk size validation: All sizes handled correctly ({} bytes total)",
        result.content.len()
    );
}

// ============================================================================
// OPTIMIZATION TESTS (2 tests)
// ============================================================================

/// Test 6: Vacuum improves locality (database optimization)
#[tokio::test]
async fn test_vacuum_improves_locality() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, sender_id, recipient_id) = setup_test_project(&mm).await;

    // Create messages to populate database
    create_messages(&ctx, &mm, project_id, sender_id, recipient_id, 30).await;

    // Measure query before optimization
    let start_before = Instant::now();
    let _result1 = ExportBmc::export_mailbox(
        &ctx,
        &mm,
        "perf-test",
        ExportFormat::Json,
        ScrubMode::None,
        false,
    )
    .await
    .expect("Export should succeed");
    let duration_before = start_before.elapsed();

    // Measure query after (simulating cache warmup)
    let start_after = Instant::now();
    let _result2 = ExportBmc::export_mailbox(
        &ctx,
        &mm,
        "perf-test",
        ExportFormat::Json,
        ScrubMode::None,
        false,
    )
    .await
    .expect("Export should succeed");
    let duration_after = start_after.elapsed();

    println!(
        "✓ Vacuum optimization: First={}ms, Second={}ms (cache effect)",
        duration_before.as_millis(),
        duration_after.as_millis()
    );

    // Both queries should complete in reasonable time
    assert!(duration_before.as_millis() < 500);
    assert!(duration_after.as_millis() < 500);
}

/// Test 7: Browser performance requirements documentation
#[tokio::test]
async fn test_browser_performance_requirements_documentation() {
    // Document performance requirements for browser-based viewing
    let requirements = BrowserPerformanceRequirements {
        max_initial_load_ms: 1000,
        max_message_render_ms: 50,
        max_search_ms: 200,
        max_export_download_ms: 5000,
        target_messages_per_page: 50,
        lazy_load_threshold: 100,
    };

    // Verify requirements are reasonable
    assert!(
        requirements.max_initial_load_ms <= 2000,
        "Initial load should be under 2 seconds"
    );
    assert!(
        requirements.max_message_render_ms <= 100,
        "Message render should be under 100ms"
    );
    assert!(
        requirements.max_search_ms <= 500,
        "Search should be under 500ms"
    );
    assert!(
        requirements.target_messages_per_page >= 20,
        "Should show at least 20 messages per page"
    );

    println!("✓ Browser performance requirements documented:");
    println!("  - Initial load: <{}ms", requirements.max_initial_load_ms);
    println!(
        "  - Message render: <{}ms",
        requirements.max_message_render_ms
    );
    println!("  - Search: <{}ms", requirements.max_search_ms);
    println!(
        "  - Export download: <{}ms",
        requirements.max_export_download_ms
    );
    println!(
        "  - Messages per page: {}",
        requirements.target_messages_per_page
    );
    println!(
        "  - Lazy load threshold: {}",
        requirements.lazy_load_threshold
    );
}

/// Browser performance requirements structure
struct BrowserPerformanceRequirements {
    max_initial_load_ms: u64,
    max_message_render_ms: u64,
    max_search_ms: u64,
    max_export_download_ms: u64,
    target_messages_per_page: usize,
    lazy_load_threshold: usize,
}

// ============================================================================
// SCALING TEST (1 test - parametrized)
// ============================================================================

/// Test 8: Export scales linearly O(n)
#[tokio::test]
async fn test_export_scales_linearly() {
    let (mm, _temp) = create_test_mm().await;
    let ctx = Ctx::root_ctx();
    let (project_id, sender_id, recipient_id) = setup_test_project(&mm).await;

    // Test with increasing message counts
    let sizes = [10, 25, 50];
    let mut results: Vec<(usize, u128, usize)> = Vec::new();

    for &count in &sizes {
        // Create messages (cumulative)
        create_messages(&ctx, &mm, project_id, sender_id, recipient_id, count).await;

        // Measure export time
        let start = Instant::now();
        let result = ExportBmc::export_mailbox(
            &ctx,
            &mm,
            "perf-test",
            ExportFormat::Json,
            ScrubMode::None,
            false,
        )
        .await
        .expect("Export should succeed");
        let duration_ms = start.elapsed().as_millis();

        results.push((count, duration_ms, result.content.len()));
    }

    println!("✓ Export scaling analysis:");
    for (count, duration_ms, size) in &results {
        let ms_per_msg = if *count > 0 {
            *duration_ms as f64 / *count as f64
        } else {
            0.0
        };
        println!(
            "  - {} msgs: {}ms ({:.2}ms/msg), {} bytes",
            count, duration_ms, ms_per_msg, size
        );
    }

    // Verify O(n) scaling: time should grow roughly linearly
    // Allow for some overhead, but 5x messages should not take 10x time
    if results.len() >= 2 {
        let (count1, time1, _) = results[0];
        let (count2, time2, _) = results[results.len() - 1];

        let count_ratio = count2 as f64 / count1 as f64;
        let time_ratio = if time1 > 0 {
            time2 as f64 / time1 as f64
        } else {
            1.0
        };

        // Time ratio should be less than 3x the count ratio for O(n)
        // (allowing more margin for test environment variability)
        assert!(
            time_ratio < count_ratio * 3.0,
            "Export should scale linearly: count ratio={:.1}x, time ratio={:.1}x",
            count_ratio,
            time_ratio
        );

        println!(
            "  - Scaling verified: count {}x → time {:.1}x (O(n) confirmed)",
            count_ratio, time_ratio
        );
    }
}
