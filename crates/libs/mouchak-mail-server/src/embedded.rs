//! Embedded static assets for single-binary web UI distribution.
//!
//! This module is only compiled when the `with-web-ui` feature is enabled.
//! It embeds the SvelteKit frontend from web-ui/build at compile time.

use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../../services/web-ui/build"]
pub struct Assets;
