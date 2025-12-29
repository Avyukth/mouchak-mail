use super::*;

// =============================================================================
// Agent Name Scrubbing Tests
// =============================================================================
// Agent names like "BlueMountain", "GreenCastle" are ALREADY meaningless
// pseudonyms by design (random adjective+noun). No need to further obfuscate.

#[test]
fn test_scrubber_standard_mode_preserves_names() {
    let scrubber = Scrubber::new(ScrubMode::Standard);
    // Agent names are already pseudonyms - keep them for readability
    assert_eq!(scrubber.scrub_name("BlueMountain"), "BlueMountain");
    assert_eq!(scrubber.scrub_name("GreenCastle"), "GreenCastle");
    assert_eq!(scrubber.scrub_name("RedFox"), "RedFox");
}

#[test]
fn test_scrubber_aggressive_mode_redacts_names() {
    let scrubber = Scrubber::new(ScrubMode::Aggressive);
    // Only aggressive/strict mode redacts names (for maximum privacy)
    assert_eq!(scrubber.scrub_name("BlueMountain"), "[REDACTED-NAME]");
}

#[test]
fn test_scrubber_none_mode_preserves_everything() {
    let scrubber = Scrubber::new(ScrubMode::None);
    assert_eq!(scrubber.scrub_name("BlueMountain"), "BlueMountain");
    // Even secrets pass through in None mode
    assert_eq!(
        scrubber.scrub("ghp_1234567890abcdef1234567890abcdef123456"),
        "ghp_1234567890abcdef1234567890abcdef123456"
    );
}

// =============================================================================
// Secret Pattern Tests (aligned with Python SECRET_PATTERNS)
// =============================================================================

#[test]
fn test_scrubber_github_tokens() {
    let scrubber = Scrubber::new(ScrubMode::Standard);

    // GitHub personal access tokens (classic)
    let text = "Use token: ghp_1234567890abcdef1234567890abcdef123456";
    assert_eq!(scrubber.scrub(text), "Use token: [GITHUB-TOKEN]");

    // GitHub fine-grained PAT
    let text2 = "New PAT: github_pat_1234567890abcdef1234";
    assert_eq!(scrubber.scrub(text2), "New PAT: [GITHUB-PAT]");
}

#[test]
fn test_scrubber_slack_tokens() {
    let scrubber = Scrubber::new(ScrubMode::Standard);

    // Slack bot token
    let text = "Bot: xoxb-123456789012-1234567890123-abcdefghij";
    assert_eq!(scrubber.scrub(text), "Bot: [SLACK-TOKEN]");

    // Slack app token
    let text2 = "App: xoxa-12345678901-abcdefghijklmnop";
    assert_eq!(scrubber.scrub(text2), "App: [SLACK-TOKEN]");
}

#[test]
fn test_scrubber_openai_keys() {
    let scrubber = Scrubber::new(ScrubMode::Standard);

    let text = "OpenAI key: sk-1234567890abcdef1234567890abcdef";
    assert_eq!(scrubber.scrub(text), "OpenAI key: [OPENAI-KEY]");
}

#[test]
fn test_scrubber_aws_keys() {
    let scrubber = Scrubber::new(ScrubMode::Standard);

    let text = "AWS key: AKIAIOSFODNN7EXAMPLE";
    assert_eq!(scrubber.scrub(text), "AWS key: [AWS-KEY]");
}

#[test]
fn test_scrubber_bearer_tokens() {
    let scrubber = Scrubber::new(ScrubMode::Standard);

    let text = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
    // Bearer token pattern matches first
    assert!(scrubber.scrub(text).contains("[BEARER-TOKEN]"));
}

#[test]
fn test_scrubber_jwt_tokens() {
    let scrubber = Scrubber::new(ScrubMode::Standard);

    // JWT structure: header.payload.signature (all base64url)
    let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
    let text = format!("Token: {}", jwt);
    assert_eq!(scrubber.scrub(&text), "Token: [JWT]");
}

#[test]
fn test_scrubber_generic_hex_tokens() {
    let scrubber = Scrubber::new(ScrubMode::Standard);

    // 32-character hex (common API keys)
    let text = "API key: a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4";
    assert_eq!(scrubber.scrub(text), "API key: [TOKEN]");

    // 64-character hex (SHA-256 hashes used as tokens)
    let text2 = "Hash: a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";
    assert_eq!(scrubber.scrub(text2), "Hash: [TOKEN]");
}

// =============================================================================
// Personal Information Tests
// =============================================================================

#[test]
fn test_scrubber_emails() {
    let scrubber = Scrubber::new(ScrubMode::Standard);

    let text = "Contact: user@example.com for support";
    assert_eq!(scrubber.scrub(text), "Contact: [EMAIL] for support");
}

#[test]
fn test_scrubber_phone_numbers() {
    let scrubber = Scrubber::new(ScrubMode::Standard);

    // Various phone formats
    assert!(scrubber.scrub("Call 123-456-7890").contains("[PHONE]"));
    assert!(scrubber.scrub("Call (123) 456-7890").contains("[PHONE]"));
    assert!(scrubber.scrub("Call 123.456.7890").contains("[PHONE]"));
}

// =============================================================================
// Aggressive Mode Additional Patterns
// =============================================================================

#[test]
fn test_scrubber_aggressive_credit_cards() {
    let scrubber = Scrubber::new(ScrubMode::Aggressive);

    let text = "Card: 4111 1111 1111 1111";
    assert_eq!(scrubber.scrub(text), "Card: [CREDIT-CARD]");
}

#[test]
fn test_scrubber_aggressive_ssn() {
    let scrubber = Scrubber::new(ScrubMode::Aggressive);

    let text = "SSN: 123-45-6789";
    assert_eq!(scrubber.scrub(text), "SSN: [SSN]");
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_scrubber_multiple_secrets_in_text() {
    let scrubber = Scrubber::new(ScrubMode::Standard);

    let text = "Use ghp_abcdefghijklmnopqrstuvwxyz123456789012 with sk-abcdefghijklmnopqrstuvwx";
    let scrubbed = scrubber.scrub(text);

    assert!(scrubbed.contains("[GITHUB-TOKEN]"));
    assert!(scrubbed.contains("[OPENAI-KEY]"));
    assert!(!scrubbed.contains("ghp_"));
    assert!(!scrubbed.contains("sk-"));
}

#[test]
fn test_scrubber_preserves_agent_conversation_context() {
    let scrubber = Scrubber::new(ScrubMode::Standard);

    // Real conversation with secrets embedded - agent names should remain readable
    let conversation = r#"
BlueMountain: Let's deploy with this token: ghp_abcdefghij1234567890abcdefghij123456
GreenCastle: Wait, use this one instead: sk-newsecret1234567890abcdefgh
BlueMountain: Good catch! Updating now.
"#;

    let scrubbed = scrubber.scrub(conversation);

    // Agent names preserved for readability
    assert!(scrubbed.contains("BlueMountain"));
    assert!(scrubbed.contains("GreenCastle"));

    // Secrets scrubbed
    assert!(scrubbed.contains("[GITHUB-TOKEN]"));
    assert!(scrubbed.contains("[OPENAI-KEY]"));
    assert!(!scrubbed.contains("ghp_"));
    assert!(!scrubbed.contains("sk-"));
}
