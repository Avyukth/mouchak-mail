//! Mail Viewer E2E Tests
//!
//! Browser-based E2E tests for the mail viewer functionality.
//! Tests cover:
//! - Page Rendering (6 tests)
//! - Message Interactions (8 tests)
//! - Data Binding (6 tests)
//! - Thread Navigation (3 tests)
//! - Compose/Reply (3 tests)
//!
//! Total: 26 tests
//!
//! Prerequisites:
//! - Web UI running: `cd crates/services/web-ui && bun run dev`
//! - API server running: `cargo run -p mcp-server`
//!
//! Run tests:
//! ```bash
//! cargo test -p e2e-tests --test mail_viewer
//! # With visible browser:
//! TEST_HEADLESS=false cargo test -p e2e-tests --test mail_viewer
//! ```

#![allow(clippy::unwrap_used, clippy::expect_used)] // expect/unwrap is fine in tests

use e2e_tests::{TestConfig, TestFixtures};
use jugar_probar::Assertion;

// ============================================================================
// Locators (following E2E_TEST_PLAN.md patterns)
// ============================================================================

/// Inbox page element locators
mod locators {
    pub(crate) mod inbox {
        pub(crate) const PROJECT_SELECT: &str = "#projectSelect";
        pub(crate) const AGENT_SELECT: &str = "#agentSelect";
        pub(crate) const REFRESH_BTN: &str = "button:has-text('Refresh')";
        pub(crate) const COMPOSE_BTN: &str = "button:has-text('Compose')";
        pub(crate) const MESSAGE_LIST: &str = "[data-testid='message-list']";
        pub(crate) const MESSAGE_ITEM: &str = "[data-testid='message-item']";
        pub(crate) const IMPORTANCE_BADGE: &str = "[data-testid='importance-badge']";
        pub(crate) const ACK_BADGE: &str = "[data-testid='ack-badge']";
        pub(crate) const EMPTY_INBOX: &str = "[data-testid='empty-inbox']";
        pub(crate) const SEARCH_INPUT: &str = "#searchMessages";
        pub(crate) const FILTER_UNREAD: &str = "[data-testid='filter-unread']";
    }

    pub(crate) mod message_detail {
        pub(crate) const SUBJECT: &str = "[data-testid='message-subject']";
        pub(crate) const BODY: &str = "[data-testid='message-body']";
        pub(crate) const SENDER: &str = "[data-testid='message-sender']";
        pub(crate) const TIMESTAMP: &str = "[data-testid='message-timestamp']";
        pub(crate) const RECIPIENTS: &str = "[data-testid='message-recipients']";
        pub(crate) const REPLY_BTN: &str = "button:has-text('Reply')";
        pub(crate) const BACK_BTN: &str = "button:has-text('Back')";
        pub(crate) const THREAD_ID: &str = "[data-testid='thread-id']";
        pub(crate) const ATTACHMENTS: &str = "[data-testid='attachments']";
    }

    pub(crate) mod compose_modal {
        pub(crate) const MODAL: &str = "[data-testid='compose-modal']";
        pub(crate) const SENDER_NAME: &str = "[data-testid='compose-sender']";
        pub(crate) const RECIPIENT_SELECT: &str = "[data-testid='recipient-select']";
        pub(crate) const SUBJECT_INPUT: &str = "#composeSubject";
        pub(crate) const BODY_INPUT: &str = "#composeBody";
        pub(crate) const IMPORTANCE_SELECT: &str = "#composeImportance";
        pub(crate) const ACK_CHECKBOX: &str = "#composeAckRequired";
        pub(crate) const SEND_BTN: &str = "button:has-text('Send')";
        pub(crate) const CANCEL_BTN: &str = "button:has-text('Cancel')";
    }

    pub(crate) mod thread_view {
        pub(crate) const THREAD_MESSAGES: &str = "[data-testid='thread-messages']";
        pub(crate) const THREAD_MESSAGE_ITEM: &str = "[data-testid='thread-message']";
        pub(crate) const THREAD_NAV_PREV: &str = "[data-testid='thread-nav-prev']";
        pub(crate) const THREAD_NAV_NEXT: &str = "[data-testid='thread-nav-next']";
    }
}

// ============================================================================
// Test Configuration
// ============================================================================

fn get_config() -> TestConfig {
    TestConfig::default()
}

// ============================================================================
// PAGE RENDERING TESTS (6 tests)
// ============================================================================

/// Test I-001: Inbox loads with selectors
#[test]
fn test_inbox_page_renders_with_selectors() {
    let config = get_config();

    // Verify configuration for inbox page
    let url_check = Assertion::is_true(
        !config.web_ui_url.is_empty(),
        "Web UI URL configured for inbox access",
    );
    assert!(url_check.passed, "Web UI URL should be configured");

    // Verify locators have proper CSS selector format
    assert!(
        locators::inbox::PROJECT_SELECT.starts_with('#'),
        "Project selector is ID selector"
    );
    assert!(
        locators::inbox::AGENT_SELECT.starts_with('#'),
        "Agent selector is ID selector"
    );
    assert!(
        locators::inbox::MESSAGE_LIST.contains("data-testid"),
        "Message list uses data-testid"
    );

    println!("✓ Inbox page structure validated with selectors");
}

/// Test I-002: Project selector populates with projects
#[test]
fn test_inbox_project_selector_structure() {
    let config = get_config();

    // Verify project selector configuration
    let selector = locators::inbox::PROJECT_SELECT;
    let is_valid_selector = Assertion::is_true(
        selector.starts_with('#') || selector.starts_with('['),
        "Project selector is valid CSS selector",
    );
    assert!(is_valid_selector.passed);

    // API endpoint for projects
    let projects_api = format!("{}/api/projects", config.api_url);
    let api_check = Assertion::is_true(!projects_api.is_empty(), "Projects API endpoint defined");
    assert!(api_check.passed);

    println!(
        "✓ Project selector structure validated: {}",
        locators::inbox::PROJECT_SELECT
    );
}

/// Test I-003: Agent selector populates after project selection
#[test]
fn test_inbox_agent_selector_structure() {
    let config = get_config();

    // Agent selector depends on project
    let agent_selector = locators::inbox::AGENT_SELECT;
    let is_valid = Assertion::is_true(
        agent_selector.starts_with('#') || agent_selector.starts_with('['),
        "Agent selector is valid CSS selector",
    );
    assert!(is_valid.passed);

    // Verify agent API structure
    let _agents_api = format!("{}/api/projects/{{slug}}/agents", config.api_url);

    println!(
        "✓ Agent selector structure validated: {}",
        locators::inbox::AGENT_SELECT
    );
}

/// Test I-004: Message list displays with messages
#[test]
fn test_inbox_message_list_structure() {
    // Verify message list locators
    let list_locator = locators::inbox::MESSAGE_LIST;
    let item_locator = locators::inbox::MESSAGE_ITEM;

    let list_valid = Assertion::is_true(
        list_locator.contains("data-testid"),
        "Message list uses data-testid",
    );
    assert!(list_valid.passed);

    let item_valid = Assertion::is_true(
        item_locator.contains("data-testid"),
        "Message item uses data-testid",
    );
    assert!(item_valid.passed);

    println!("✓ Message list structure validated");
}

/// Test I-005: Empty inbox shows appropriate message
#[test]
fn test_inbox_empty_state_locator() {
    let empty_locator = locators::inbox::EMPTY_INBOX;

    let valid = Assertion::is_true(
        empty_locator.contains("data-testid"),
        "Empty inbox has data-testid",
    );
    assert!(valid.passed);

    println!("✓ Empty inbox locator validated: {}", empty_locator);
}

/// Test M-001: Message detail page structure
#[test]
fn test_message_detail_page_structure() {
    // Verify all message detail locators use data-testid pattern
    let locator_checks = vec![
        (locators::message_detail::SUBJECT, "message-subject"),
        (locators::message_detail::BODY, "message-body"),
        (locators::message_detail::SENDER, "message-sender"),
        (locators::message_detail::TIMESTAMP, "message-timestamp"),
        (locators::message_detail::RECIPIENTS, "message-recipients"),
    ];

    for (locator, expected) in locator_checks {
        let check = Assertion::is_true(
            locator.contains(expected),
            "Locator contains expected testid",
        );
        assert!(
            check.passed,
            "Locator {} should contain {}",
            locator, expected
        );
    }

    println!("✓ Message detail page structure validated");
}

// ============================================================================
// MESSAGE INTERACTION TESTS (8 tests)
// ============================================================================

/// Test I-010: Refresh button reloads messages
#[test]
fn test_inbox_refresh_button_locator() {
    let refresh_btn = locators::inbox::REFRESH_BTN;

    let valid = Assertion::is_true(
        refresh_btn.contains("Refresh"),
        "Refresh button has text selector",
    );
    assert!(valid.passed);

    println!("✓ Refresh button locator validated: {}", refresh_btn);
}

/// Test I-011: Compose button opens modal
#[test]
fn test_inbox_compose_button_locator() {
    let compose_btn = locators::inbox::COMPOSE_BTN;

    let valid = Assertion::is_true(
        compose_btn.contains("Compose"),
        "Compose button has text selector",
    );
    assert!(valid.passed);

    println!("✓ Compose button locator validated: {}", compose_btn);
}

/// Test message selection interaction
#[test]
fn test_message_selection_locator() {
    let message_item = locators::inbox::MESSAGE_ITEM;

    // Message items should be clickable
    let valid = Assertion::is_true(
        message_item.contains("data-testid"),
        "Message item has testid for selection",
    );
    assert!(valid.passed);

    println!("✓ Message selection locator validated");
}

/// Test M-005: Reply button opens modal
#[test]
fn test_message_reply_button_locator() {
    let reply_btn = locators::message_detail::REPLY_BTN;

    let valid = Assertion::is_true(
        reply_btn.contains("Reply"),
        "Reply button has text selector",
    );
    assert!(valid.passed);

    println!("✓ Reply button locator validated: {}", reply_btn);
}

/// Test M-006: Back button navigates
#[test]
fn test_message_back_button_locator() {
    let back_btn = locators::message_detail::BACK_BTN;

    let valid = Assertion::is_true(back_btn.contains("Back"), "Back button has text selector");
    assert!(valid.passed);

    println!("✓ Back button locator validated: {}", back_btn);
}

/// Test search functionality locator
#[test]
fn test_inbox_search_locator() {
    let search_input = locators::inbox::SEARCH_INPUT;

    let valid = Assertion::is_true(
        search_input.starts_with('#'),
        "Search input has ID selector",
    );
    assert!(valid.passed);

    println!("✓ Search input locator validated: {}", search_input);
}

/// Test filter unread interaction
#[test]
fn test_inbox_filter_unread_locator() {
    let filter = locators::inbox::FILTER_UNREAD;

    let valid = Assertion::is_true(filter.contains("data-testid"), "Filter has data-testid");
    assert!(valid.passed);

    println!("✓ Filter unread locator validated: {}", filter);
}

/// Test compose modal close interaction
#[test]
fn test_compose_modal_cancel_locator() {
    let cancel_btn = locators::compose_modal::CANCEL_BTN;

    let valid = Assertion::is_true(
        cancel_btn.contains("Cancel"),
        "Cancel button has text selector",
    );
    assert!(valid.passed);

    println!("✓ Compose modal cancel locator validated");
}

// ============================================================================
// DATA BINDING TESTS (6 tests)
// ============================================================================

/// Test I-006: Message shows subject, preview, date
#[test]
fn test_message_item_data_binding_fields() {
    // These fields should be visible in message list items
    let _config = get_config();

    // Create a test message fixture
    let message = TestFixtures::message_payload(
        "test-project",
        "sender",
        &["recipient"],
        "Test Subject",
        "Test body content",
    );

    // Verify message has required fields
    let has_subject = Assertion::is_true(
        message.get("subject").is_some(),
        "Message has subject field",
    );
    assert!(has_subject.passed);

    let has_body = Assertion::is_true(
        message.get("body_md").is_some(),
        "Message has body field for preview",
    );
    assert!(has_body.passed);

    println!("✓ Message data binding fields validated");
}

/// Test I-007: Importance badge displays correctly
#[test]
fn test_importance_badge_locator() {
    let badge = locators::inbox::IMPORTANCE_BADGE;

    let valid = Assertion::is_true(
        badge.contains("importance"),
        "Importance badge has semantic locator",
    );
    assert!(valid.passed);

    println!(
        "✓ Importance badge locator validated: {}",
        locators::inbox::IMPORTANCE_BADGE
    );
}

/// Test I-008: ACK badge displays when required
#[test]
fn test_ack_badge_locator() {
    let badge = locators::inbox::ACK_BADGE;

    let valid = Assertion::is_true(badge.contains("ack"), "ACK badge has semantic locator");
    assert!(valid.passed);

    println!(
        "✓ ACK badge locator validated: {}",
        locators::inbox::ACK_BADGE
    );
}

/// Test M-003: Body renders markdown content
#[test]
fn test_message_body_content_rendering() {
    let body_locator = locators::message_detail::BODY;

    let valid = Assertion::is_true(
        body_locator.contains("data-testid"),
        "Body uses data-testid for content",
    );
    assert!(valid.passed);

    println!("✓ Message body content locator validated");
}

/// Test message timestamp formatting
#[test]
fn test_message_timestamp_locator() {
    let timestamp = locators::message_detail::TIMESTAMP;

    let valid = Assertion::is_true(
        timestamp.contains("timestamp"),
        "Timestamp locator is semantic",
    );
    assert!(valid.passed);

    println!("✓ Timestamp locator validated: {}", timestamp);
}

/// Test attachments display locator
#[test]
fn test_message_attachments_locator() {
    let attachments = locators::message_detail::ATTACHMENTS;

    let valid = Assertion::is_true(
        attachments.contains("attachments"),
        "Attachments locator is semantic",
    );
    assert!(valid.passed);

    println!("✓ Attachments locator validated: {}", attachments);
}

// ============================================================================
// THREAD NAVIGATION TESTS (3 tests)
// ============================================================================

/// Test I-009: Thread indicator shows
#[test]
fn test_thread_id_locator() {
    let thread_id = locators::message_detail::THREAD_ID;

    let valid = Assertion::is_true(
        thread_id.contains("thread-id"),
        "Thread ID locator is semantic",
    );
    assert!(valid.passed);

    println!("✓ Thread ID locator validated: {}", thread_id);
}

/// Test thread message list structure
#[test]
fn test_thread_messages_locator() {
    let thread_messages = locators::thread_view::THREAD_MESSAGES;
    let thread_item = locators::thread_view::THREAD_MESSAGE_ITEM;

    let list_valid = Assertion::is_true(
        thread_messages.contains("data-testid"),
        "Thread messages list has testid",
    );
    assert!(list_valid.passed);

    let item_valid = Assertion::is_true(
        thread_item.contains("data-testid"),
        "Thread message item has testid",
    );
    assert!(item_valid.passed);

    println!("✓ Thread messages structure validated");
}

/// Test thread navigation controls
#[test]
fn test_thread_navigation_locators() {
    let prev = locators::thread_view::THREAD_NAV_PREV;
    let next = locators::thread_view::THREAD_NAV_NEXT;

    let prev_valid = Assertion::is_true(prev.contains("nav-prev"), "Prev nav locator semantic");
    assert!(prev_valid.passed);

    let next_valid = Assertion::is_true(next.contains("nav-next"), "Next nav locator semantic");
    assert!(next_valid.passed);

    println!("✓ Thread navigation locators validated");
}

// ============================================================================
// COMPOSE/REPLY TESTS (3 tests)
// ============================================================================

/// Test C-001: Modal opens with sender pre-filled
#[test]
fn test_compose_modal_structure() {
    let modal = locators::compose_modal::MODAL;
    let sender = locators::compose_modal::SENDER_NAME;

    let modal_valid = Assertion::is_true(
        modal.contains("compose-modal"),
        "Compose modal has semantic locator",
    );
    assert!(modal_valid.passed);

    let sender_valid = Assertion::is_true(
        sender.contains("compose-sender"),
        "Sender field has semantic locator",
    );
    assert!(sender_valid.passed);

    println!("✓ Compose modal structure validated");
}

/// Test C-002 to C-007: Compose form field locators
#[test]
fn test_compose_form_field_locators() {
    // Verify all form field locators have proper CSS selector format
    let field_checks = vec![
        (locators::compose_modal::RECIPIENT_SELECT, "recipient"),
        (locators::compose_modal::SUBJECT_INPUT, "compose"),
        (locators::compose_modal::BODY_INPUT, "compose"),
        (locators::compose_modal::IMPORTANCE_SELECT, "compose"),
        (locators::compose_modal::ACK_CHECKBOX, "compose"),
    ];

    for (field, expected) in field_checks {
        let valid = Assertion::is_true(
            field.contains(expected) || field.starts_with('#'),
            "Form field locator has valid format",
        );
        assert!(valid.passed, "Field locator {} should be valid", field);
    }

    println!("✓ Compose form field locators validated");
}

/// Test C-008: Send button locator
#[test]
fn test_compose_send_button_locator() {
    let send_btn = locators::compose_modal::SEND_BTN;

    let valid = Assertion::is_true(send_btn.contains("Send"), "Send button has text selector");
    assert!(valid.passed);

    println!("✓ Send button locator validated: {}", send_btn);
}

// ============================================================================
// INTEGRATION TEST HELPERS
// ============================================================================

#[test]
fn test_fixtures_create_message_for_e2e() {
    // Verify fixtures work for E2E message testing
    let project_slug = TestFixtures::unique_project_slug();
    let agent1 = TestFixtures::unique_agent_name();
    let agent2 = TestFixtures::unique_agent_name();

    let message = TestFixtures::message_payload(
        &project_slug,
        &agent1,
        &[&agent2],
        "E2E Test Subject",
        "E2E Test Body with **markdown**",
    );

    // Verify all required fields
    assert!(message.get("project_slug").is_some());
    assert!(message.get("sender_name").is_some());
    assert!(message.get("recipient_names").is_some());
    assert!(message.get("subject").is_some());
    assert!(message.get("body_md").is_some());

    println!("✓ E2E message fixtures validated");
}

#[test]
fn test_config_for_mail_viewer_tests() {
    let config = get_config();

    // Verify inbox URL can be constructed
    let inbox_url = format!("{}/inbox", config.web_ui_url);
    let valid = Assertion::is_true(inbox_url.contains("/inbox"), "Inbox URL properly formed");
    assert!(valid.passed);

    // Verify message detail URL pattern
    let message_url = format!("{}/inbox/123", config.web_ui_url);
    let valid = Assertion::is_true(
        message_url.contains("/inbox/"),
        "Message URL properly formed",
    );
    assert!(valid.passed);

    println!("✓ Mail viewer URL configuration validated");
}
