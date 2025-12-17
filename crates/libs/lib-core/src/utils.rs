//! Utility functions and helpers.
//!
//! This module provides common utility functions used throughout lib-core.
//!
//! # Functions
//!
//! - `slugify` - Convert text to URL-safe slugs

use slug;

/// Converts text to a URL-safe slug.
///
/// This function transforms arbitrary text into a lowercase, hyphenated
/// string suitable for use in URLs and identifiers. Non-ASCII characters
/// are transliterated or removed, and spaces are replaced with hyphens.
///
/// # Arguments
///
/// * `text` - The input text to convert
///
/// # Returns
///
/// A URL-safe slug string.
///
/// # Examples
///
/// ```
/// use lib_core::utils::slugify;
///
/// assert_eq!(slugify("Hello World"), "hello-world");
/// assert_eq!(slugify("My Project Name"), "my-project-name");
/// assert_eq!(slugify("Café & Restaurant"), "cafe-restaurant");
/// ```
pub fn slugify(text: &str) -> String {
    slug::slugify(text)
}
