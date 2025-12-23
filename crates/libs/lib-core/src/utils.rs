//! Utility functions and helpers.
//!
//! This module provides common utility functions used throughout lib-core.
//!
//! # Functions
//!
//! - `slugify` - Convert text to URL-safe slugs
//! - `parse_timestamp` - Parse timestamp with warning on failure

use chrono::NaiveDateTime;
use slug;

/// Standard timestamp format used in database storage
pub const TS_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// Parse a timestamp string with logging on failure.
///
/// This function parses a timestamp string in the standard format (`%Y-%m-%d %H:%M:%S`)
/// and logs a warning if parsing fails, returning the epoch time as fallback.
///
/// # Arguments
///
/// * `ts_str` - The timestamp string to parse
/// * `field_name` - Name of the field being parsed (for logging context)
///
/// # Returns
///
/// A `NaiveDateTime` - either the parsed value or epoch time (1970-01-01 00:00:00) on failure.
///
/// # Examples
///
/// ```
/// use lib_core::utils::parse_timestamp;
///
/// let ts = parse_timestamp("2024-12-24 10:30:00", "created_at");
/// assert_eq!(ts.to_string(), "2024-12-24 10:30:00");
///
/// // Invalid string returns epoch with warning logged
/// let ts_invalid = parse_timestamp("invalid", "updated_at");
/// assert_eq!(ts_invalid.to_string(), "1970-01-01 00:00:00");
/// ```
pub fn parse_timestamp(ts_str: &str, field_name: &str) -> NaiveDateTime {
    match NaiveDateTime::parse_from_str(ts_str, TS_FORMAT) {
        Ok(dt) => dt,
        Err(e) => {
            tracing::warn!(
                raw = %ts_str,
                field = %field_name,
                error = %e,
                "Failed to parse timestamp, using epoch time"
            );
            NaiveDateTime::default()
        }
    }
}

/// Parse an optional timestamp string with logging on failure.
///
/// Same as `parse_timestamp` but for optional strings. Returns `None` if input is `None`,
/// otherwise returns `Some(timestamp)`.
pub fn parse_timestamp_opt(ts_str: Option<String>, field_name: &str) -> Option<NaiveDateTime> {
    ts_str.map(|s| parse_timestamp(&s, field_name))
}

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

/// Input validation including names, paths, and TTLs.
pub mod validation;

/// Agent mistake detection helpers (PORT-1.3).
pub mod mistake_detection;

/// Image processing and validation helpers (PORT-7.3).
pub mod image_processing;

/// Git pathspec matching utilities for file reservation conflict detection.
pub mod pathspec;
