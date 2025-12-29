//! Image processing and validation utilities (PORT-7.3).
//!
//! Provides functions for validating and processing image attachments.
//!
//! # Supported Formats
//!
//! - PNG, JPEG, GIF, WebP, BMP
//! - Data URIs with base64 encoding
//!
//! # Size Limits
//!
//! - Maximum dimensions: 7680x4320 (8K)
//! - No minimum dimension requirement

use crate::Result;
use base64::{Engine as _, engine::general_purpose};
use image::{GenericImageView, ImageFormat};

/// Errors that can occur during image processing.
#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    /// Image data is corrupted or unreadable.
    #[error("Invalid image data: {0}")]
    InvalidData(String),
    /// Image format is not supported.
    #[error("Unsupported image format")]
    UnsupportedFormat,
    /// Image dimensions are below minimum requirements.
    #[error("Image too small: {width}x{height}")]
    TooSmall {
        /// Image width in pixels.
        width: u32,
        /// Image height in pixels.
        height: u32,
    },
    /// Image dimensions exceed maximum limits (8K).
    #[error("Image too large: {width}x{height}")]
    TooLarge {
        /// Image width in pixels.
        width: u32,
        /// Image height in pixels.
        height: u32,
    },
    /// Data URI is malformed or not base64 encoded.
    #[error("Invalid data URI")]
    InvalidDataUri,
}

/// Validates image data and returns format and dimensions.
///
/// # Arguments
///
/// * `data` - Raw image bytes
///
/// # Returns
///
/// A tuple of (format, width, height) if valid.
///
/// # Errors
///
/// - `ImageError::InvalidData` - Image is corrupted
/// - `ImageError::TooLarge` - Exceeds 7680x4320 pixels
pub fn validate_image(data: &[u8]) -> Result<(ImageFormat, u32, u32)> {
    // 1. Guess format
    let format = image::guess_format(data).map_err(|e| ImageError::InvalidData(e.to_string()))?;

    // 2. Load image to verify integrity and get dimensions
    // limit size to prevent bombs? image crate has some limits but we should be careful.
    let img = image::load_from_memory_with_format(data, format)
        .map_err(|e| ImageError::InvalidData(e.to_string()))?;

    let (width, height) = img.dimensions();

    // 3. Check dimensions (Edge case: 1x1 might be tracking pixel, but let's allow it for now
    // unless strictly forbidden. The test will verify behavior.
    // If input is malformed, load_from_memory would have failed.

    // Check max dimensions (e.g. 8k)
    if width > 7680 || height > 4320 {
        return Err(ImageError::TooLarge { width, height }.into());
    }

    Ok((format, width, height))
}

/// Decodes a base64 data URI to raw bytes.
///
/// # Arguments
///
/// * `uri` - Data URI string (e.g., `data:image/png;base64,iVBOR...`)
///
/// # Returns
///
/// A tuple of (bytes, media_type) where media_type is e.g. "image/png".
///
/// # Errors
///
/// Returns `ImageError::InvalidDataUri` if the URI is malformed or not base64.
pub fn decode_data_uri(uri: &str) -> Result<(Vec<u8>, String)> {
    if !uri.starts_with("data:") {
        return Err(ImageError::InvalidDataUri.into());
    }

    let parts: Vec<&str> = uri.splitn(2, ',').collect();
    if parts.len() != 2 {
        return Err(ImageError::InvalidDataUri.into());
    }

    let metadata = parts[0];
    let data_part = parts[1];

    // Extract media type e.g. "image/png"
    // metadata is like "data:image/png;base64"
    let media_type = metadata
        .strip_prefix("data:")
        .and_then(|s| s.split(';').next())
        .unwrap_or("");

    if !metadata.contains(";base64") {
        // We only support base64 for now
        return Err(ImageError::InvalidDataUri.into());
    }

    let data = general_purpose::STANDARD
        .decode(data_part)
        .map_err(|_| ImageError::InvalidDataUri)?;

    Ok((data, media_type.to_string()))
}

/// Checks if image data is valid without returning details.
///
/// A convenience wrapper around [`validate_image`] for simple boolean checks.
pub fn is_valid_image(data: &[u8]) -> bool {
    validate_image(data).is_ok()
}
