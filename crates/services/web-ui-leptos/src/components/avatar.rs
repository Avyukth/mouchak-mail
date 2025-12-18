//! Agent Avatar component with deterministic color generation.
//!
//! Displays a circular avatar with initials and a background color
//! derived from a hash of the agent's name for consistent coloring.

use leptos::prelude::*;

/// Color palette for agent avatars (from design spec)
const AVATAR_COLORS: &[&str] = &[
    "#6366f1", // Indigo
    "#f97316", // Orange
    "#ec4899", // Pink
    "#14b8a6", // Teal
    "#8b5cf6", // Purple
    "#f59e0b", // Amber
];

/// Generate a deterministic color from a name using a simple hash.
fn hash_to_color(name: &str) -> &'static str {
    let hash: u32 = name.bytes().fold(0u32, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(b as u32)
    });
    let index = (hash as usize) % AVATAR_COLORS.len();
    AVATAR_COLORS[index]
}

/// Extract initials from a name (1-2 characters).
fn get_initials(name: &str) -> String {
    let parts: Vec<&str> = name.split(|c: char| c == '-' || c == '_' || c.is_whitespace())
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
        assert!(unique.len() > 1, "Expected color variety for different names");
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
}
