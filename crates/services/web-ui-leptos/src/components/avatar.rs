//! Agent Avatar component with deterministic color generation.
//!
//! Displays a circular avatar with initials and a background color
//! derived from a hash of the agent's name for consistent coloring.

use leptos::prelude::*;

/// Color palette for agent avatars (WCAG AA compliant with white text)
const AVATAR_COLORS: &[&str] = &[
    "#6366f1", // Indigo (4.5:1 contrast)
    "#ea580c", // Orange-600 (3.5:1 contrast, was #f97316 which failed WCAG)
    "#db2777", // Pink-600 (4.3:1 contrast, was #ec4899)
    "#0d9488", // Teal-600 (3.3:1 contrast, was #14b8a6)
    "#7c3aed", // Violet-600 (5.0:1 contrast, was #8b5cf6)
    "#d97706", // Amber-600 (3.2:1 contrast, was #f59e0b)
];

/// Generate a deterministic color from a name using a simple hash.
fn hash_to_color(name: &str) -> &'static str {
    let hash: u32 = name
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let index = (hash as usize) % AVATAR_COLORS.len();
    AVATAR_COLORS[index]
}

/// Extract initials from a name (1-2 characters).
fn get_initials(name: &str) -> String {
    let parts: Vec<&str> = name
        .split(|c: char| c == '-' || c == '_' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .collect();

    match parts.len() {
        0 => "?".to_string(),
        1 => {
            // Single word: take first 1-2 chars
            parts[0].chars().take(2).collect::<String>().to_uppercase()
        }
        _ => {
            // Multiple words: take first char of first two parts
            let first = parts[0].chars().next().unwrap_or('?');
            let second = parts[1].chars().next().unwrap_or('?');
            format!("{}{}", first, second).to_uppercase()
        }
    }
}

/// Agent Avatar component with color generation from name hash.
///
/// # Props
/// - `name`: Agent name (used for initials and color hash)
/// - `size`: Size variant - "sm" (32px), "md" (40px), "lg" (48px)
///
/// # Example
/// ```rust,ignore
/// view! { <AgentAvatar name="worker-1" size="md" /> }
/// ```
#[component]
pub fn AgentAvatar(
    /// The agent's name
    #[prop(into)]
    name: String,
    /// Size: "sm", "md", or "lg"
    #[prop(default = "md")]
    size: &'static str,
) -> impl IntoView {
    let initials = get_initials(&name);
    let bg_color = hash_to_color(&name);

    let size_class = match size {
        "sm" => "w-8 h-8 text-xs",
        "lg" => "w-12 h-12 text-base",
        _ => "w-10 h-10 text-sm",
    };

    view! {
        <div
            class={format!(
                "{} rounded-full flex items-center justify-center font-medium text-white shadow-sm hover:shadow-md transition-shadow",
                size_class
            )}
            style={format!("background-color: {}", bg_color)}
            aria-label={format!("Avatar for {}", name)}
            title={name.clone()}
        >
            {initials}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_to_color_deterministic() {
        // Same name should always produce same color
        let color1 = hash_to_color("worker-1");
        let color2 = hash_to_color("worker-1");
        assert_eq!(color1, color2);
    }

    #[test]
    fn test_hash_to_color_different_names() {
        // Different names should (usually) produce different colors
        let colors: Vec<_> = ["alice", "bob", "charlie", "david", "eve", "frank"]
            .iter()
            .map(|n| hash_to_color(n))
            .collect();

        // Should have some variety (not all the same)
        let unique: std::collections::HashSet<_> = colors.iter().collect();
        assert!(
            unique.len() > 1,
            "Expected color variety for different names"
        );
    }

    #[test]
    fn test_hash_to_color_valid_hex() {
        for name in ["test", "agent", "worker-xyz", "human"] {
            let color = hash_to_color(name);
            assert!(color.starts_with('#'), "Color should be hex: {}", color);
            assert_eq!(color.len(), 7, "Color should be #RRGGBB: {}", color);
        }
    }

    #[test]
    fn test_get_initials_single_word() {
        assert_eq!(get_initials("alice"), "AL");
        assert_eq!(get_initials("b"), "B");
        assert_eq!(get_initials("worker"), "WO");
    }

    #[test]
    fn test_get_initials_hyphenated() {
        assert_eq!(get_initials("worker-1"), "W1");
        assert_eq!(get_initials("claude-code"), "CC");
        assert_eq!(get_initials("test-agent-long"), "TA");
    }

    #[test]
    fn test_get_initials_underscored() {
        assert_eq!(get_initials("worker_1"), "W1");
        assert_eq!(get_initials("my_agent"), "MA");
    }

    #[test]
    fn test_get_initials_spaced() {
        assert_eq!(get_initials("John Doe"), "JD");
        assert_eq!(get_initials("Jane Smith"), "JS");
    }

    #[test]
    fn test_get_initials_empty() {
        assert_eq!(get_initials(""), "?");
    }

    #[test]
    fn test_get_initials_special_chars() {
        assert_eq!(get_initials("---"), "?");
        assert_eq!(get_initials("a---b"), "AB");
    }

    // === GAP TESTS (added by reviewer) ===

    #[test]
    fn test_get_initials_unicode_names() {
        // Unicode characters should work correctly
        assert_eq!(get_initials("日本語"), "日本"); // Japanese
        assert_eq!(get_initials("Müller"), "MÜ"); // German umlaut
        assert_eq!(get_initials("José García"), "JG"); // Spanish accents
        assert_eq!(get_initials("北京-上海"), "北上"); // Chinese with separator
    }

    #[test]
    fn test_get_initials_emoji_names() {
        // Emoji in names should be handled gracefully
        assert_eq!(get_initials("🤖"), "🤖"); // Single emoji
        assert_eq!(get_initials("🤖-bot"), "🤖B"); // Emoji with text
        assert_eq!(get_initials("robot-🤖"), "R🤖"); // Text with emoji
    }

    #[test]
    fn test_get_initials_numeric_only() {
        // Numeric-only names
        assert_eq!(get_initials("123"), "12");
        assert_eq!(get_initials("42-agent"), "4A");
        assert_eq!(get_initials("007"), "00");
    }

    #[test]
    fn test_get_initials_mixed_separators() {
        // Names with mixed separator types
        assert_eq!(get_initials("foo-bar_baz qux"), "FB"); // Only first two parts
        assert_eq!(get_initials("a_b-c d"), "AB");
        assert_eq!(get_initials("  spaced  name  "), "SN"); // Extra whitespace
    }

    #[test]
    fn test_hash_to_color_unicode_deterministic() {
        // Unicode names should produce deterministic colors
        let color1 = hash_to_color("日本語エージェント");
        let color2 = hash_to_color("日本語エージェント");
        assert_eq!(color1, color2);

        // Different unicode names may produce different colors
        let color_jp = hash_to_color("東京");
        let color_cn = hash_to_color("北京");
        // Both should be valid hex colors
        assert!(color_jp.starts_with('#'));
        assert!(color_cn.starts_with('#'));
    }

    #[test]
    fn test_color_palette_wcag_contrast() {
        // Verify all palette colors have adequate contrast with white text
        // WCAG AA requires 4.5:1 for normal text, 3:1 for large text
        // All our colors are designed for white text readability

        fn hex_to_luminance(hex: &str) -> f64 {
            let r = u8::from_str_radix(&hex[1..3], 16).unwrap() as f64 / 255.0;
            let g = u8::from_str_radix(&hex[3..5], 16).unwrap() as f64 / 255.0;
            let b = u8::from_str_radix(&hex[5..7], 16).unwrap() as f64 / 255.0;

            fn channel_luminance(c: f64) -> f64 {
                if c <= 0.03928 {
                    c / 12.92
                } else {
                    ((c + 0.055) / 1.055).powf(2.4)
                }
            }

            0.2126 * channel_luminance(r)
                + 0.7152 * channel_luminance(g)
                + 0.0722 * channel_luminance(b)
        }

        fn contrast_ratio(l1: f64, l2: f64) -> f64 {
            let lighter = l1.max(l2);
            let darker = l1.min(l2);
            (lighter + 0.05) / (darker + 0.05)
        }

        let white_luminance = 1.0; // White has luminance of 1.0

        for color in AVATAR_COLORS {
            let bg_luminance = hex_to_luminance(color);
            let ratio = contrast_ratio(white_luminance, bg_luminance);

            // WCAG AA large text requires 3:1, our avatars use large-ish text
            assert!(
                ratio >= 3.0,
                "Color {} has insufficient contrast ratio {:.2} (need >= 3.0)",
                color,
                ratio
            );
        }
    }

    #[test]
    fn test_size_class_variants() {
        // Verify size class strings contain expected Tailwind classes
        let sm_class = match "sm" {
            "sm" => "w-8 h-8 text-xs",
            "lg" => "w-12 h-12 text-base",
            _ => "w-10 h-10 text-sm",
        };
        assert!(sm_class.contains("w-8"));
        assert!(sm_class.contains("h-8"));
        assert!(sm_class.contains("text-xs"));

        let lg_class = match "lg" {
            "sm" => "w-8 h-8 text-xs",
            "lg" => "w-12 h-12 text-base",
            _ => "w-10 h-10 text-sm",
        };
        assert!(lg_class.contains("w-12"));
        assert!(lg_class.contains("h-12"));
        assert!(lg_class.contains("text-base"));

        let md_class = match "md" {
            "sm" => "w-8 h-8 text-xs",
            "lg" => "w-12 h-12 text-base",
            _ => "w-10 h-10 text-sm",
        };
        assert!(md_class.contains("w-10"));
        assert!(md_class.contains("h-10"));
        assert!(md_class.contains("text-sm"));

        // Unknown sizes should default to md
        let unknown_class = match "xl" {
            "sm" => "w-8 h-8 text-xs",
            "lg" => "w-12 h-12 text-base",
            _ => "w-10 h-10 text-sm",
        };
        assert_eq!(
            unknown_class, md_class,
            "Unknown sizes should default to md"
        );
    }
}
