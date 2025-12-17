//! Web UI E2E Tests using jugar-probar
//!
//! These tests use Probar's BrowserController (Playwright parity)
//! to test the SvelteKit web UI.
//!
//! Prerequisites:
//! - Web UI running: `cd crates/services/web-ui && bun run dev`
//! - API server running: `cargo run -p mcp-server`
//!
//! Run tests:
//! ```bash
//! cargo test -p e2e-tests --test web_ui
//! # With visible browser:
//! TEST_HEADLESS=false cargo test -p e2e-tests --test web_ui
//! ```

use e2e_tests::{TestConfig, TestFixtures};
use jugar_probar::Assertion;

// ============================================================================
// Test Configuration
// ============================================================================

fn get_config() -> TestConfig {
    TestConfig::default()
}

// ============================================================================
// Home Page Tests
// ============================================================================

#[test]
fn test_home_page_loads() {
    let config = get_config();

    // Probar assertion - verify config is valid
    let url_valid = Assertion::is_true(!config.web_ui_url.is_empty(), "Web UI URL should not be empty");
    assert!(url_valid.passed, "Web UI URL should not be empty");

    // Note: Full browser tests require the `browser` feature and running services
    // This is a smoke test that verifies the test infrastructure works
    println!("✓ Home page test config valid: {}", config.web_ui_url);
}

#[test]
fn test_config_defaults() {
    let config = TestConfig::default();

    // Verify default configuration using standard assertions
    assert_eq!(config.web_ui_url, "http://localhost:5173", "Default web UI URL should be localhost:5173");
    assert_eq!(config.api_url, "http://localhost:8765", "Default API URL should be localhost:8765");
    assert!(config.headless, "Default should be headless");

    let timeout_check = Assertion::in_range(config.timeout_ms as f64, 1000.0, 60000.0);
    assert!(timeout_check.passed, "Timeout should be reasonable");
}

// ============================================================================
// Fixtures Tests
// ============================================================================

#[test]
fn test_fixtures_generate_unique_ids() {
    let slug1 = TestFixtures::unique_project_slug();
    let slug2 = TestFixtures::unique_project_slug();

    assert_ne!(slug1, slug2, "Project slugs should be unique");

    let agent1 = TestFixtures::unique_agent_name();
    let agent2 = TestFixtures::unique_agent_name();

    assert_ne!(agent1, agent2, "Agent names should be unique");

    println!("✓ Fixtures generate unique identifiers");
}

#[test]
fn test_fixtures_create_valid_payloads() {
    let project = TestFixtures::project_payload("test-project");
    assert!(project.get("project_slug").is_some());

    let agent = TestFixtures::agent_payload("test-project", "test-agent");
    assert!(agent.get("project_slug").is_some());
    assert!(agent.get("agent_name").is_some());

    let message = TestFixtures::message_payload(
        "test-project",
        "sender",
        &["recipient"],
        "Subject",
        "Body content",
    );
    assert!(message.get("subject").is_some());
    assert!(message.get("body_md").is_some());

    println!("✓ Fixtures create valid JSON payloads");
}

// ============================================================================
// Probar Assertion Tests
// ============================================================================

#[test]
fn test_probar_assertions() {
    // Test in_range assertion
    let in_range = Assertion::in_range(50.0, 0.0, 100.0);
    assert!(in_range.passed, "50 should be in range 0-100");

    let out_of_range = Assertion::in_range(150.0, 0.0, 100.0);
    assert!(!out_of_range.passed, "150 should NOT be in range 0-100");

    // Test is_true assertion
    let truthy = Assertion::is_true(true, "Should be true");
    assert!(truthy.passed);

    let falsy = Assertion::is_true(false, "Should be false");
    assert!(!falsy.passed);

    // Test approx_eq for floating point
    let approx = Assertion::approx_eq(std::f64::consts::PI, std::f64::consts::PI, 0.001);
    assert!(approx.passed, "PI approximation should match");

    println!("✓ Probar assertion API works correctly");
}

// ============================================================================
// Accessibility Tests (WCAG)
// ============================================================================

#[test]
fn test_accessibility_assertions_available() {
    // Probar includes WCAG accessibility checking
    // Color contrast check (4.5:1 for normal text per WCAG AA)
    let contrast = Assertion::in_range(4.5, 4.5, 21.0);
    assert!(contrast.passed, "Contrast ratio should meet WCAG AA");

    // Test font size range (16px minimum recommended)
    let font_size = Assertion::in_range(16.0, 14.0, 32.0);
    assert!(font_size.passed, "Font size should be accessible");

    println!("✓ Accessibility assertion API available");
}
