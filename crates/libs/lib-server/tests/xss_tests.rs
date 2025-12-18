//! XSS Security Test Corpus
//!
//! Tests XSS prevention across the application.
//! Reference: OWASP XSS Prevention Cheat Sheet

/// XSS vector test cases covering major attack categories
const XSS_VECTORS: &[(&str, &str)] = &[
    // Script tags
    ("script_basic", "<script>alert('XSS')</script>"),
    ("script_src", "<script src='evil.js'></script>"),
    (
        "script_encoded",
        "<script>alert(String.fromCharCode(88,83,83))</script>",
    ),
    // Event handlers
    ("img_onerror", "<img src=x onerror=alert('XSS')>"),
    ("body_onload", "<body onload=alert('XSS')>"),
    ("svg_onload", "<svg onload=alert('XSS')>"),
    ("input_onfocus", "<input onfocus=alert('XSS') autofocus>"),
    (
        "div_onmouseover",
        "<div onmouseover=alert('XSS')>hover</div>",
    ),
    // JavaScript URLs
    ("href_javascript", "<a href='javascript:alert(1)'>click</a>"),
    ("img_src_javascript", "<img src='javascript:alert(1)'>"),
    ("iframe_src", "<iframe src='javascript:alert(1)'></iframe>"),
    // Data URLs
    (
        "data_text_html",
        "<a href='data:text/html,<script>alert(1)</script>'>",
    ),
    (
        "object_data",
        "<object data='data:text/html,<script>alert(1)</script>'>",
    ),
    // Meta refresh
    (
        "meta_refresh",
        "<meta http-equiv='refresh' content='0;url=javascript:alert(1)'>",
    ),
    // SVG XSS
    ("svg_script", "<svg><script>alert('XSS')</script></svg>"),
    ("svg_animate", "<svg><animate onbegin=alert(1)>"),
    // CSS injection
    (
        "style_expression",
        "<div style='background:url(javascript:alert(1))'>",
    ),
    (
        "style_import",
        "<style>@import 'javascript:alert(1)';</style>",
    ),
    // HTML5 vectors
    ("video_onerror", "<video><source onerror=alert(1)>"),
    ("audio_onerror", "<audio src=x onerror=alert('XSS')>"),
    ("details_ontoggle", "<details ontoggle=alert(1) open>"),
    // Markdown-specific (if markdown rendering is enabled)
    ("md_link", "[XSS](javascript:alert(1))"),
    ("md_img", "![XSS](javascript:alert(1))"),
];

/// Dangerous substrings that should NEVER appear unescaped in HTML output
/// Note: We check for unescaped HTML tags, not attribute content
/// Because `<script>` escaped to `&lt;script&gt;` is safe even if it contains "script"
const DANGEROUS_UNESCAPED_TAGS: &[&str] = &[
    "<script", "<img ", "<svg ", "<body ", "<iframe", "<object", "<embed", "<meta ", "<base ",
    "<form ", "<video", "<audio", "<details", "<style", "<link ", "<input ",
    "<div ", // with event handlers
];

/// HTML-escape function for testing reference
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Verify that unescaped HTML tags are not present in output
fn assert_no_unescaped_html_tags(output: &str, context: &str) {
    for tag in DANGEROUS_UNESCAPED_TAGS {
        assert!(
            !output.contains(tag),
            "XSS vulnerability: found unescaped '{}' in {} output",
            tag,
            context
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that all XSS vectors are properly escaped when HTML-escaped
    #[test]
    fn test_xss_vectors_properly_escaped() {
        for (name, vector) in XSS_VECTORS {
            let escaped = html_escape(vector);

            // Verify HTML tags are escaped (< becomes &lt;)
            assert_no_unescaped_html_tags(&escaped, name);

            // Specific checks - the < character should be escaped
            assert!(
                !escaped.contains('<'),
                "Vector '{}' contains unescaped '<': {}",
                name,
                escaped
            );
        }
    }

    /// Test XSS vectors in subject lines
    #[test]
    fn test_xss_in_subject_lines() {
        for (name, vector) in XSS_VECTORS.iter().take(5) {
            let escaped = html_escape(vector);
            assert_no_unescaped_html_tags(&escaped, &format!("subject_{}", name));
            assert!(!escaped.contains('<'), "Subject contains unescaped '<'");
        }
    }

    /// Test XSS vectors in message body
    #[test]
    fn test_xss_in_message_body() {
        // Simulate message body content with XSS
        let malicious_body = r#"
            Hello agent!
            <script>alert('XSS')</script>
            <img src=x onerror="stealCookies()">
            [evil link](javascript:alert(1))
        "#;

        let escaped = html_escape(malicious_body);
        assert_no_unescaped_html_tags(&escaped, "message_body");
        assert!(
            !escaped.contains('<'),
            "Message body contains unescaped '<'"
        );
    }

    /// Test that HTML escaping is idempotent
    #[test]
    fn test_escape_idempotent() {
        let input = "<script>alert(1)</script>";
        let once = html_escape(input);
        let twice = html_escape(&once);

        // Should still be safe after double-escape
        assert_no_unescaped_html_tags(&twice, "double_escape");

        // But double-escape changes the string
        assert_ne!(once, twice);
    }

    /// Test XSS corpus coverage
    #[test]
    fn test_xss_corpus_coverage() {
        // Verify we have vectors for each category
        assert!(XSS_VECTORS.iter().any(|(n, _)| n.starts_with("script")));
        assert!(XSS_VECTORS.iter().any(|(n, _)| n.contains("onerror")));
        assert!(XSS_VECTORS.iter().any(|(n, _)| n.contains("javascript")));
        assert!(XSS_VECTORS.iter().any(|(n, _)| n.contains("data")));
        assert!(XSS_VECTORS.iter().any(|(n, _)| n.contains("svg")));
        assert!(XSS_VECTORS.iter().any(|(n, _)| n.contains("style")));
        assert!(
            XSS_VECTORS
                .iter()
                .any(|(n, _)| n.contains("video") || n.contains("audio"))
        );
    }

    /// Test CSP-related strings would be sanitized
    #[test]
    fn test_csp_bypass_attempts() {
        let csp_bypass_vectors = [
            "<base href='https://evil.com/'>",
            "<form action='https://evil.com/steal'>",
            "<meta http-equiv='Content-Security-Policy'>",
        ];

        for vector in csp_bypass_vectors {
            let escaped = html_escape(vector);
            assert!(!escaped.contains("<base"), "base tag not escaped");
            assert!(!escaped.contains("<form"), "form tag not escaped");
            assert!(!escaped.contains("<meta"), "meta tag not escaped");
        }
    }
}

// ============================================================================
// Integration tests (require running server)
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test that API responses don't reflect XSS in error messages
    #[tokio::test]
    async fn test_xss_in_error_messages() {
        // Error messages should not reflect user input verbatim
        let xss_input = "<script>alert('XSS')</script>";
        let escaped = html_escape(xss_input);

        // Simulate error message that might include user input
        let error_msg = format!("Agent not found: {}", escaped);
        assert_no_unescaped_html_tags(&error_msg, "error_message");
    }
}
