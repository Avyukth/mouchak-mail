//! Git pathspec matching utilities for file reservation conflict detection.
//!
//! This module provides functions to determine if two path patterns could
//! match overlapping files, used for detecting reservation conflicts.

use glob::Pattern;

/// Check if two path patterns could match overlapping files.
///
/// This function determines whether reservations for `pattern_a` and `pattern_b`
/// could conflict by matching the same files.
///
/// # Arguments
///
/// * `pattern_a` - First path pattern (glob syntax)
/// * `pattern_b` - Second path pattern (glob syntax)
///
/// # Returns
///
/// `true` if the patterns could match overlapping files, `false` otherwise.
///
/// # Examples
///
/// ```
/// use mouchak_mail_core::utils::pathspec::paths_conflict;
///
/// // Exact match
/// assert!(paths_conflict("src/main.rs", "src/main.rs"));
///
/// // Glob matches literal
/// assert!(paths_conflict("src/**/*.rs", "src/main.rs"));
/// assert!(paths_conflict("src/main.rs", "src/**/*.rs"));
///
/// // Overlapping globs
/// assert!(paths_conflict("src/**/*.rs", "src/api/**"));
///
/// // Non-overlapping paths
/// assert!(!paths_conflict("src/**", "tests/**"));
/// ```
pub fn paths_conflict(pattern_a: &str, pattern_b: &str) -> bool {
    // Case 1: Exact match
    if pattern_a == pattern_b {
        return true;
    }

    // Case 2: Pattern A matches pattern B as a literal path
    if let Ok(pat) = Pattern::new(pattern_a) {
        if pat.matches(pattern_b) {
            return true;
        }
    }

    // Case 3: Pattern B matches pattern A as a literal path
    if let Ok(pat) = Pattern::new(pattern_b) {
        if pat.matches(pattern_a) {
            return true;
        }
    }

    // Case 4: Check for prefix overlap (both are patterns that could match same files)
    // "src/**/*.rs" and "src/api/**" could both match "src/api/foo.rs"
    patterns_have_common_prefix(pattern_a, pattern_b)
}

/// Check if two patterns share a common directory prefix before wildcards.
///
/// This catches cases like:
/// - `src/**/*.rs` and `src/api/**` (common prefix: `src/`)
/// - `src/api/**` and `src/**` (one is prefix of other)
fn patterns_have_common_prefix(a: &str, b: &str) -> bool {
    let a_parts: Vec<&str> = a.split('/').filter(|s| !s.is_empty()).collect();
    let b_parts: Vec<&str> = b.split('/').filter(|s| !s.is_empty()).collect();

    // Find length of common non-wildcard prefix
    let mut common_prefix_len = 0;

    for (pa, pb) in a_parts.iter().zip(b_parts.iter()) {
        // If either part contains a wildcard, stop comparing
        if pa.contains('*') || pb.contains('*') {
            // We've reached a wildcard with some common prefix
            break;
        }
        if pa != pb {
            // Paths diverge before any wildcards - no overlap possible
            return false;
        }
        common_prefix_len += 1;
    }

    // If we found at least one common directory, patterns could overlap
    // Also handles cases where one pattern is entirely a prefix of the other
    if common_prefix_len > 0 {
        return true;
    }

    // Edge case: one or both patterns start with wildcards
    // e.g., "**/*.rs" and "*.rs" - these could overlap with anything
    let a_starts_wild = a_parts.first().is_some_and(|p| p.contains('*'));
    let b_starts_wild = b_parts.first().is_some_and(|p| p.contains('*'));

    a_starts_wild || b_starts_wild
}

// ============================================================================
// Kani Formal Verification Proofs
// ============================================================================
//
// Run with: cargo kani --package lib-core
// These proofs mathematically verify safety properties of path conflict detection.

#[cfg(kani)]
mod verification {
    use super::*;

    /// Proof: paths_conflict is reflexive (a pattern always conflicts with itself)
    #[kani::proof]
    fn proof_paths_conflict_reflexive() {
        let pattern: &str = kani::any();
        kani::assume(pattern.len() < 64); // Bound input size for verification
        kani::assume(!pattern.is_empty());

        // A pattern must always conflict with itself
        kani::assert(
            paths_conflict(pattern, pattern),
            "paths_conflict must be reflexive",
        );
    }

    /// Proof: paths_conflict is symmetric (if a conflicts with b, then b conflicts with a)
    #[kani::proof]
    fn proof_paths_conflict_symmetric() {
        let pattern_a: &str = kani::any();
        let pattern_b: &str = kani::any();

        kani::assume(pattern_a.len() < 32);
        kani::assume(pattern_b.len() < 32);

        let ab = paths_conflict(pattern_a, pattern_b);
        let ba = paths_conflict(pattern_b, pattern_a);

        kani::assert(ab == ba, "paths_conflict must be symmetric");
    }

    /// Proof: paths_conflict never panics on any input
    #[kani::proof]
    fn proof_paths_conflict_no_panic() {
        let pattern_a: &str = kani::any();
        let pattern_b: &str = kani::any();

        kani::assume(pattern_a.len() < 64);
        kani::assume(pattern_b.len() < 64);

        // This should complete without panicking
        let _ = paths_conflict(pattern_a, pattern_b);
    }

    /// Proof: patterns_have_common_prefix never panics
    #[kani::proof]
    fn proof_common_prefix_no_panic() {
        let a: &str = kani::any();
        let b: &str = kani::any();

        kani::assume(a.len() < 32);
        kani::assume(b.len() < 32);

        let _ = patterns_have_common_prefix(a, b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        assert!(paths_conflict("src/main.rs", "src/main.rs"));
        assert!(paths_conflict("Cargo.toml", "Cargo.toml"));
    }

    #[test]
    fn test_glob_matches_literal() {
        assert!(paths_conflict("src/**/*.rs", "src/main.rs"));
        assert!(paths_conflict("src/main.rs", "src/**/*.rs"));
        assert!(paths_conflict("src/**", "src/api/auth.rs"));
        assert!(paths_conflict("*.rs", "main.rs"));
    }

    #[test]
    fn test_overlapping_globs() {
        assert!(paths_conflict("src/**/*.rs", "src/api/**"));
        assert!(paths_conflict("src/api/**", "src/**/*.rs"));
        assert!(paths_conflict("src/**", "src/api/**"));
    }

    #[test]
    fn test_non_overlapping_paths() {
        assert!(!paths_conflict("src/**", "tests/**"));
        assert!(!paths_conflict("docs/**", "src/**"));
        assert!(!paths_conflict("src/api/**", "src/auth/**"));
    }

    #[test]
    fn test_nested_directories() {
        assert!(paths_conflict("src/**", "src/api/v1/**"));
        assert!(paths_conflict("src/api/**", "src/api/v1/handlers/**"));
    }

    #[test]
    fn test_wildcard_at_root() {
        // Patterns starting with wildcards could match anything
        assert!(paths_conflict("**/*.rs", "src/main.rs"));
        assert!(paths_conflict("*.toml", "Cargo.toml"));
    }

    #[test]
    fn test_single_files_different() {
        assert!(!paths_conflict("src/main.rs", "src/lib.rs"));
        assert!(!paths_conflict("Cargo.toml", "README.md"));
    }

    #[test]
    fn test_invalid_patterns_handled() {
        // Invalid glob patterns should not panic, just not match
        assert!(!paths_conflict("[invalid", "src/main.rs"));
        assert!(!paths_conflict("src/main.rs", "[invalid"));
    }
}
