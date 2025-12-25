//! Export functionality for mailbox data
//!
//! Supports exporting messages in HTML, JSON, and Markdown formats.

use crate::Result;
use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::model::message::MessageBmc;
use crate::model::project::ProjectBmc;
use serde::{Deserialize, Serialize};

/// Export format options
/// Export format options.
///
/// Supported formats for mailbox export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Render as a standalone HTML page
    Html,
    /// Raw message data in JSON
    Json,
    /// Markdown document (suitable for LLM contexts)
    Markdown,
    /// Comma-separated values
    Csv,
}

impl std::str::FromStr for ExportFormat {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "html" => Self::Html,
            "md" | "markdown" => Self::Markdown,
            "csv" => Self::Csv,
            _ => Self::Json, // default
        })
    }
}

/// Exported mailbox data
/// Exported mailbox data container.
///
/// Contains the rendered content and metadata about the export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedMailbox {
    pub project_slug: String,
    pub project_name: String,
    pub message_count: usize,
    pub exported_at: String,
    pub content: String,
    pub format: String,
}

use lazy_static::lazy_static;
use regex::Regex;

/// Scrubbing mode for privacy protection
/// Scrubbing mode for privacy protection.
///
/// Controls how sensitive data is redacted from exports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrubMode {
    /// No scrubbing (full fidelity)
    #[default]
    None,
    /// Scrub PII (email, phone) and secrets (API keys)
    Standard,
    /// Scrub PII, secrets, and financial info (CC, SSN)
    Aggressive,
}

impl std::str::FromStr for ScrubMode {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "standard" => Self::Standard,
            "aggressive" | "strict" => Self::Aggressive,
            _ => Self::None,
        })
    }
}

/// Service for redacting sensitive information from text.
///
/// Uses regex patterns to identify and replace PII and secrets.
pub struct Scrubber {
    mode: ScrubMode,
}

impl Scrubber {
    pub fn new(mode: ScrubMode) -> Self {
        Self { mode }
    }

    #[allow(clippy::expect_used)]
    pub fn scrub(&self, text: &str) -> String {
        if self.mode == ScrubMode::None {
            return text.to_string();
        }

        let mut cleaned = text.to_string();

        lazy_static! {
            // Personal information patterns
            static ref EMAIL_RE: Regex =
                Regex::new(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b")
                    .expect("valid email regex");
            static ref PHONE_RE: Regex =
                Regex::new(r"\b\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b")
                    .expect("valid phone regex");

            // API keys and tokens (aligned with Python SECRET_PATTERNS)
            static ref GITHUB_TOKEN_RE: Regex =
                Regex::new(r"(?i)\bghp_[A-Za-z0-9]{36,}\b")
                    .expect("valid github token regex");
            static ref GITHUB_PAT_RE: Regex =
                Regex::new(r"(?i)\bgithub_pat_[A-Za-z0-9_]{20,}\b")
                    .expect("valid github pat regex");
            static ref SLACK_TOKEN_RE: Regex =
                Regex::new(r"(?i)\bxox[baprs]-[A-Za-z0-9-]{10,}\b")
                    .expect("valid slack token regex");
            static ref OPENAI_KEY_RE: Regex =
                Regex::new(r"(?i)\bsk-[a-zA-Z0-9]{20,}\b")
                    .expect("valid openai key regex");
            static ref AWS_KEY_RE: Regex =
                Regex::new(r"\bAKIA[A-Z0-9]{16}\b")
                    .expect("valid aws key regex");
            static ref BEARER_RE: Regex =
                Regex::new(r"(?i)bearer\s+[a-zA-Z0-9._-]{20,}")
                    .expect("valid bearer regex");
            static ref JWT_RE: Regex =
                Regex::new(r"\beyJ[0-9A-Za-z_-]+\.[0-9A-Za-z_-]+\.[0-9A-Za-z_-]+\b")
                    .expect("valid jwt regex");
            static ref GENERIC_TOKEN_RE: Regex =
                Regex::new(r"\b[a-f0-9]{32,64}\b")
                    .expect("valid token regex");
        }

        // Personal information
        cleaned = EMAIL_RE.replace_all(&cleaned, "[EMAIL]").to_string();
        cleaned = PHONE_RE.replace_all(&cleaned, "[PHONE]").to_string();

        // API keys and tokens - order matters for specificity
        cleaned = GITHUB_TOKEN_RE
            .replace_all(&cleaned, "[GITHUB-TOKEN]")
            .to_string();
        cleaned = GITHUB_PAT_RE
            .replace_all(&cleaned, "[GITHUB-PAT]")
            .to_string();
        cleaned = SLACK_TOKEN_RE
            .replace_all(&cleaned, "[SLACK-TOKEN]")
            .to_string();
        cleaned = OPENAI_KEY_RE
            .replace_all(&cleaned, "[OPENAI-KEY]")
            .to_string();
        cleaned = AWS_KEY_RE.replace_all(&cleaned, "[AWS-KEY]").to_string();
        cleaned = BEARER_RE
            .replace_all(&cleaned, "[BEARER-TOKEN]")
            .to_string();
        cleaned = JWT_RE.replace_all(&cleaned, "[JWT]").to_string();
        cleaned = GENERIC_TOKEN_RE
            .replace_all(&cleaned, "[TOKEN]")
            .to_string();

        if self.mode == ScrubMode::Aggressive {
            lazy_static! {
                static ref CC_RE: Regex =
                    Regex::new(r"\b(?:\d[ -]*?){13,16}\b").expect("valid credit card regex");
                static ref SSN_RE: Regex =
                    Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").expect("valid SSN regex");
            }
            cleaned = CC_RE.replace_all(&cleaned, "[CREDIT-CARD]").to_string();
            cleaned = SSN_RE.replace_all(&cleaned, "[SSN]").to_string();
        }

        cleaned
    }

    pub fn scrub_body(&self, body: &str) -> String {
        self.scrub(body)
    }

    pub fn scrub_name(&self, name: &str) -> String {
        match self.mode {
            ScrubMode::Aggressive => "[REDACTED-NAME]".to_string(),
            _ => name.to_string(),
        }
    }
}

pub struct ExportBmc;

impl ExportBmc {
    /// Export a project's mailbox to the specified format
    pub async fn export_mailbox(
        ctx: &Ctx,
        mm: &ModelManager,
        project_slug: &str,
        format: ExportFormat,
        scrub_mode: ScrubMode,
        _include_attachments: bool,
    ) -> Result<ExportedMailbox> {
        // Get project
        let project = ProjectBmc::get_by_slug(ctx, mm, project_slug).await?;

        // Get recent messages (limit to 100 for export)
        let messages = MessageBmc::list_recent(ctx, mm, project.id, 100).await?;

        let exported_at = chrono::Utc::now()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string();
        let message_count = messages.len();

        let scrubber = Scrubber::new(scrub_mode);

        let content = match format {
            ExportFormat::Html => Self::render_html(&project.slug, &messages, &scrubber),
            ExportFormat::Json => Self::render_json(&messages, &scrubber)?,
            ExportFormat::Markdown => Self::render_markdown(&project.slug, &messages, &scrubber),
            ExportFormat::Csv => Self::render_csv(&messages, &scrubber)?,
        };

        let format_str = match format {
            ExportFormat::Html => "html",
            ExportFormat::Json => "json",
            ExportFormat::Markdown => "markdown",
            ExportFormat::Csv => "csv",
        };

        Ok(ExportedMailbox {
            project_slug: project.slug.clone(),
            project_name: project.human_key.clone(),
            message_count,
            exported_at,
            content,
            format: format_str.to_string(),
        })
    }

    fn render_html(
        project_slug: &str,
        messages: &[crate::model::message::Message],
        scrubber: &Scrubber,
    ) -> String {
        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        html.push_str(&format!(
            "<title>Mailbox Export - {}</title>\n",
            project_slug
        ));
        html.push_str(
            "<style>
body { font-family: system-ui, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
.message { border: 1px solid #ddd; padding: 15px; margin: 10px 0; border-radius: 8px; }
.subject { font-weight: bold; font-size: 1.1em; }
.meta { color: #666; font-size: 0.9em; margin: 5px 0; }
.body { margin-top: 10px; white-space: pre-wrap; }
</style>\n</head>\n<body>\n",
        );
        html.push_str(&format!("<h1>Mailbox Export: {}</h1>\n", project_slug));
        html.push_str(&format!("<p>Total messages: {}</p>\n", messages.len()));

        for msg in messages {
            let scrubbed_subject = scrubber.scrub(&msg.subject);
            let scrubbed_body = scrubber.scrub_body(&msg.body_md);
            let scrubbed_sender = scrubber.scrub_name(&msg.sender_name);

            html.push_str("<div class=\"message\">\n");
            html.push_str(&format!(
                "<div class=\"subject\">{}</div>\n",
                html_escape(&scrubbed_subject)
            ));
            html.push_str(&format!(
                "<div class=\"meta\">From: {} | {}</div>\n",
                html_escape(&scrubbed_sender),
                msg.created_ts.format("%Y-%m-%d %H:%M")
            ));
            html.push_str(&format!(
                "<div class=\"body\">{}</div>\n",
                html_escape(&scrubbed_body)
            ));
            html.push_str("</div>\n");
        }

        html.push_str("</body>\n</html>");
        html
    }

    fn render_json(
        messages: &[crate::model::message::Message],
        scrubber: &Scrubber,
    ) -> Result<String> {
        // For JSON, we might want to clone and scrub fields.
        // Or create a scrubbed struct.
        // Easiest is to convert to Value, walk it? Or just map to a new Vec.
        // Let's use a temporary struct or just modify if we can.
        // Messy to redefine struct. Let's use serde_json::Value
        let mut vals = Vec::new();
        for msg in messages {
            let mut val = serde_json::to_value(msg)?;
            if let Some(obj) = val.as_object_mut() {
                if let Some(s) = obj.get("subject").and_then(|v| v.as_str()) {
                    obj.insert(
                        "subject".to_string(),
                        serde_json::Value::String(scrubber.scrub(s)),
                    );
                }
                if let Some(s) = obj.get("body_md").and_then(|v| v.as_str()) {
                    obj.insert(
                        "body_md".to_string(),
                        serde_json::Value::String(scrubber.scrub_body(s)),
                    );
                }
                if let Some(s) = obj.get("sender_name").and_then(|v| v.as_str()) {
                    obj.insert(
                        "sender_name".to_string(),
                        serde_json::Value::String(scrubber.scrub_name(s)),
                    );
                }
                // Scrub recipient names if available in future, but Message struct currently doesn't inline them nicely in JSON without extra work?
                // `Message` struct has `sender_name`.
            }
            vals.push(val);
        }
        Ok(serde_json::to_string_pretty(&vals)?)
    }

    fn render_markdown(
        project_slug: &str,
        messages: &[crate::model::message::Message],
        scrubber: &Scrubber,
    ) -> String {
        let mut md = String::new();
        md.push_str(&format!("# Mailbox Export: {}\n\n", project_slug));
        md.push_str(&format!("Total messages: {}\n\n---\n\n", messages.len()));

        for msg in messages {
            let scrubbed_subject = scrubber.scrub(&msg.subject);
            let scrubbed_body = scrubber.scrub_body(&msg.body_md);
            let scrubbed_sender = scrubber.scrub_name(&msg.sender_name);

            md.push_str(&format!("## {}\n\n", scrubbed_subject));
            md.push_str(&format!(
                "**From:** {} | **Date:** {}\n\n",
                scrubbed_sender,
                msg.created_ts.format("%Y-%m-%d %H:%M")
            ));
            md.push_str(&format!("{}\n\n---\n\n", scrubbed_body));
        }

        md
    }

    fn render_csv(
        messages: &[crate::model::message::Message],
        scrubber: &Scrubber,
    ) -> Result<String> {
        let mut wtr = csv::Writer::from_writer(vec![]);

        // Header
        wtr.write_record(["id", "created_at", "sender", "subject", "body"])
            .map_err(|e| crate::Error::InvalidInput(format!("CSV Error: {}", e)))?;

        // Rows
        for msg in messages {
            wtr.write_record(&[
                msg.id.to_string(),
                msg.created_ts.format("%Y-%m-%d %H:%M:%S").to_string(),
                scrubber.scrub_name(&msg.sender_name),
                scrubber.scrub(&msg.subject),
                scrubber.scrub_body(&msg.body_md),
            ])
            .map_err(|e| crate::Error::InvalidInput(format!("CSV Error: {}", e)))?;
        }

        let data = wtr
            .into_inner()
            .map_err(|e| crate::Error::InvalidInput(format!("CSV Error: {}", e)))?;
        Ok(String::from_utf8(data).unwrap_or_default())
    }
}

impl ExportBmc {
    pub async fn commit_archive(
        ctx: &Ctx,
        mm: &ModelManager,
        project_slug: &str,
        message: &str,
    ) -> Result<String> {
        // 1. Export in Markdown (default for archive)
        let exported = Self::export_mailbox(
            ctx,
            mm,
            project_slug,
            ExportFormat::Markdown,
            ScrubMode::None,
            true,
        )
        .await?;

        // 2. Determine file path in repo
        let now = chrono::Utc::now();
        let filename = format!("{}_{}.md", project_slug, now.format("%Y%m%d_%H%M%S"));
        let rel_path = std::path::Path::new("mailboxes")
            .join(project_slug)
            .join(&filename);

        // 3. Git Operations - serialized to prevent lock contention
        let _git_guard = mm.git_lock.lock().await;

        // Use cached repository to prevent FD exhaustion
        let repo_arc = mm.get_repo().await?;
        let repo = repo_arc.lock().await;

        // 4. Commit
        let oid = crate::store::git_store::commit_file(
            &repo,
            &rel_path,
            &exported.content,
            message,
            "MCP Agent Mail",       // Committer name
            "mcp@generic-agent.ai", // Committer email
        )?;

        Ok(oid.to_string())
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// --- Ed25519 Signing Support ---

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::OsRng;

/// Export manifest with optional signature for integrity verification
/// Export manifest with optional signature for integrity verification.
///
/// Used to verify that an export hasn't been tampered with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportManifest {
    /// Version of the manifest format
    pub version: String,
    /// Project slug
    pub project_slug: String,
    /// Export timestamp (ISO 8601)
    pub exported_at: String,
    /// Number of messages exported
    pub message_count: usize,
    /// SHA-256 hash of the content
    pub content_hash: String,
    /// Export format used
    pub format: String,
    /// Ed25519 signature (base64, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Public key used for signing (base64, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
}

impl ExportManifest {
    /// Create a new unsigned manifest
    pub fn new(exported: &ExportedMailbox) -> Self {
        // Compute SHA-256 hash of content
        let content_hash = {
            use sha1::Digest;
            let mut hasher = sha1::Sha1::new();
            hasher.update(exported.content.as_bytes());
            hex::encode(hasher.finalize())
        };

        Self {
            version: "1.0".to_string(),
            project_slug: exported.project_slug.clone(),
            exported_at: exported.exported_at.clone(),
            message_count: exported.message_count,
            content_hash,
            format: exported.format.clone(),
            signature: None,
            public_key: None,
        }
    }

    /// Get the bytes to be signed (everything except signature and public_key)
    fn signing_payload(&self) -> Vec<u8> {
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.version,
            self.project_slug,
            self.exported_at,
            self.message_count,
            self.content_hash,
            self.format
        )
        .into_bytes()
    }

    /// Sign the manifest with an Ed25519 signing key
    pub fn sign(&mut self, signing_key: &SigningKey) {
        let payload = self.signing_payload();
        let signature = signing_key.sign(&payload);
        self.signature = Some(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            signature.to_bytes(),
        ));
        self.public_key = Some(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            signing_key.verifying_key().to_bytes(),
        ));
    }

    /// Verify the manifest signature
    pub fn verify(&self) -> Result<bool> {
        let signature_b64 = self
            .signature
            .as_ref()
            .ok_or_else(|| crate::Error::InvalidInput("No signature present".to_string()))?;
        let public_key_b64 = self
            .public_key
            .as_ref()
            .ok_or_else(|| crate::Error::InvalidInput("No public key present".to_string()))?;

        // Decode signature
        let signature_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, signature_b64)
                .map_err(|e| {
                    crate::Error::InvalidInput(format!("Invalid signature base64: {}", e))
                })?;

        let signature = Signature::from_slice(&signature_bytes)
            .map_err(|e| crate::Error::InvalidInput(format!("Invalid signature format: {}", e)))?;

        // Decode public key
        let public_key_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, public_key_b64)
                .map_err(|e| {
                    crate::Error::InvalidInput(format!("Invalid public key base64: {}", e))
                })?;

        let public_key_array: [u8; 32] = public_key_bytes
            .try_into()
            .map_err(|_| crate::Error::InvalidInput("Public key must be 32 bytes".to_string()))?;

        let verifying_key = VerifyingKey::from_bytes(&public_key_array)
            .map_err(|e| crate::Error::InvalidInput(format!("Invalid public key: {}", e)))?;

        // Verify
        let payload = self.signing_payload();
        Ok(verifying_key.verify(&payload, &signature).is_ok())
    }

    /// Verify with a specific public key (for external verification)
    pub fn verify_with_key(&self, public_key_b64: &str) -> Result<bool> {
        let signature_b64 = self
            .signature
            .as_ref()
            .ok_or_else(|| crate::Error::InvalidInput("No signature present".to_string()))?;

        // Decode signature
        let signature_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, signature_b64)
                .map_err(|e| {
                    crate::Error::InvalidInput(format!("Invalid signature base64: {}", e))
                })?;

        let signature = Signature::from_slice(&signature_bytes)
            .map_err(|e| crate::Error::InvalidInput(format!("Invalid signature format: {}", e)))?;

        // Decode provided public key
        let public_key_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, public_key_b64)
                .map_err(|e| {
                    crate::Error::InvalidInput(format!("Invalid public key base64: {}", e))
                })?;

        let public_key_array: [u8; 32] = public_key_bytes
            .try_into()
            .map_err(|_| crate::Error::InvalidInput("Public key must be 32 bytes".to_string()))?;

        let verifying_key = VerifyingKey::from_bytes(&public_key_array)
            .map_err(|e| crate::Error::InvalidInput(format!("Invalid public key: {}", e)))?;

        // Verify
        let payload = self.signing_payload();
        Ok(verifying_key.verify(&payload, &signature).is_ok())
    }
}

/// Generate a new Ed25519 signing keypair
pub fn generate_signing_keypair() -> (SigningKey, VerifyingKey) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

/// Export signing key to base64 (for storage/CLI output)
pub fn signing_key_to_base64(key: &SigningKey) -> String {
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, key.to_bytes())
}

/// Import signing key from base64
pub fn signing_key_from_base64(b64: &str) -> Result<SigningKey> {
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
        .map_err(|e| crate::Error::InvalidInput(format!("Invalid signing key base64: {}", e)))?;

    let key_array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| crate::Error::InvalidInput("Signing key must be 32 bytes".to_string()))?;

    Ok(SigningKey::from_bytes(&key_array))
}

/// Export verifying (public) key to base64
pub fn verifying_key_to_base64(key: &VerifyingKey) -> String {
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, key.to_bytes())
}

/// Import verifying key from base64
pub fn verifying_key_from_base64(b64: &str) -> Result<VerifyingKey> {
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
        .map_err(|e| crate::Error::InvalidInput(format!("Invalid public key base64: {}", e)))?;

    let key_array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| crate::Error::InvalidInput("Public key must be 32 bytes".to_string()))?;

    VerifyingKey::from_bytes(&key_array)
        .map_err(|e| crate::Error::InvalidInput(format!("Invalid public key: {}", e)))
}

impl ExportBmc {
    /// Export a project's mailbox with optional signing
    pub async fn export_mailbox_signed(
        ctx: &Ctx,
        mm: &ModelManager,
        project_slug: &str,
        format: ExportFormat,
        scrub_mode: ScrubMode,
        include_attachments: bool,
        signing_key: Option<&SigningKey>,
    ) -> Result<(ExportedMailbox, ExportManifest)> {
        // Export the mailbox
        let exported = Self::export_mailbox(
            ctx,
            mm,
            project_slug,
            format,
            scrub_mode,
            include_attachments,
        )
        .await?;

        // Create manifest
        let mut manifest = ExportManifest::new(&exported);

        // Sign if key provided
        if let Some(key) = signing_key {
            manifest.sign(key);
        }

        Ok((exported, manifest))
    }

    /// Verify an export against its manifest
    pub fn verify_export(exported: &ExportedMailbox, manifest: &ExportManifest) -> Result<bool> {
        // First verify content hash matches
        let content_hash = {
            use sha1::Digest;
            let mut hasher = sha1::Sha1::new();
            hasher.update(exported.content.as_bytes());
            hex::encode(hasher.finalize())
        };

        if content_hash != manifest.content_hash {
            return Ok(false);
        }

        // Then verify signature if present
        if manifest.signature.is_some() {
            manifest.verify()
        } else {
            // No signature, but content hash matches
            Ok(true)
        }
    }
}

// =============================================================================
// Age Encryption Support
// =============================================================================

use std::io::{Read, Write};

/// Generate a new age identity (private key) and recipient (public key)
pub fn generate_age_identity() -> (String, String) {
    use age::secrecy::ExposeSecret;
    let identity = age::x25519::Identity::generate();
    let recipient = identity.to_public();
    (
        identity.to_string().expose_secret().to_string(),
        recipient.to_string(),
    )
}

/// Encrypt data using age with one or more recipients
///
/// Recipients are age public keys (bech32-encoded strings starting with "age1...")
pub fn encrypt_with_age(data: &[u8], recipients: &[String]) -> Result<Vec<u8>> {
    if recipients.is_empty() {
        return Err(crate::Error::InvalidInput(
            "At least one recipient required for encryption".to_string(),
        ));
    }

    // Parse recipients and box them as trait objects
    let parsed_recipients: Vec<Box<dyn age::Recipient + Send>> = recipients
        .iter()
        .map(|r| {
            r.parse::<age::x25519::Recipient>()
                .map(|rec| Box::new(rec) as Box<dyn age::Recipient + Send>)
                .map_err(|e| {
                    crate::Error::InvalidInput(format!("Invalid age recipient '{}': {}", r, e))
                })
        })
        .collect::<Result<Vec<_>>>()?;

    // Create encryptor
    let encryptor = age::Encryptor::with_recipients(
        parsed_recipients
            .iter()
            .map(|r| r.as_ref() as &dyn age::Recipient),
    )
    .map_err(|e| crate::Error::EncryptionError(format!("Failed to create encryptor: {}", e)))?;

    // Encrypt to armored output
    let mut encrypted = Vec::new();
    let armor =
        age::armor::ArmoredWriter::wrap_output(&mut encrypted, age::armor::Format::AsciiArmor)
            .map_err(|e| {
                crate::Error::EncryptionError(format!("Failed to create armored writer: {}", e))
            })?;

    let mut writer = encryptor
        .wrap_output(armor)
        .map_err(|e| crate::Error::EncryptionError(format!("Failed to wrap output: {}", e)))?;

    writer
        .write_all(data)
        .map_err(|e| crate::Error::EncryptionError(format!("Failed to write data: {}", e)))?;

    writer
        .finish()
        .and_then(|armor| armor.finish())
        .map_err(|e| {
            crate::Error::EncryptionError(format!("Failed to finish encryption: {}", e))
        })?;

    Ok(encrypted)
}

/// Encrypt data using age with a passphrase
pub fn encrypt_with_passphrase(data: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    use age::secrecy::SecretString;

    let encryptor =
        age::Encryptor::with_user_passphrase(SecretString::from(passphrase.to_string()));

    // Encrypt to armored output
    let mut encrypted = Vec::new();
    let armor =
        age::armor::ArmoredWriter::wrap_output(&mut encrypted, age::armor::Format::AsciiArmor)
            .map_err(|e| {
                crate::Error::EncryptionError(format!("Failed to create armored writer: {}", e))
            })?;

    let mut writer = encryptor
        .wrap_output(armor)
        .map_err(|e| crate::Error::EncryptionError(format!("Failed to wrap output: {}", e)))?;

    writer
        .write_all(data)
        .map_err(|e| crate::Error::EncryptionError(format!("Failed to write data: {}", e)))?;

    writer
        .finish()
        .and_then(|armor| armor.finish())
        .map_err(|e| {
            crate::Error::EncryptionError(format!("Failed to finish encryption: {}", e))
        })?;

    Ok(encrypted)
}

/// Decrypt age-encrypted data using an identity (private key)
pub fn decrypt_with_identity(encrypted: &[u8], identity_str: &str) -> Result<Vec<u8>> {
    // Parse identity
    let identity: age::x25519::Identity = identity_str
        .parse()
        .map_err(|e| crate::Error::InvalidInput(format!("Invalid age identity: {}", e)))?;

    // Create decryptor
    let armor = age::armor::ArmoredReader::new(encrypted);
    let decryptor = age::Decryptor::new(armor)
        .map_err(|e| crate::Error::DecryptionError(format!("Failed to create decryptor: {}", e)))?;

    let mut decrypted = Vec::new();
    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|e| crate::Error::DecryptionError(format!("Decryption failed: {}", e)))?;

    reader.read_to_end(&mut decrypted).map_err(|e| {
        crate::Error::DecryptionError(format!("Failed to read decrypted data: {}", e))
    })?;

    Ok(decrypted)
}

/// Decrypt age-encrypted data using a passphrase
pub fn decrypt_with_passphrase(encrypted: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    use age::scrypt;
    use age::secrecy::SecretString;

    // Create identity from passphrase
    let identity = scrypt::Identity::new(SecretString::from(passphrase.to_string()));

    // Create decryptor
    let armor = age::armor::ArmoredReader::new(encrypted);
    let decryptor = age::Decryptor::new(armor)
        .map_err(|e| crate::Error::DecryptionError(format!("Failed to create decryptor: {}", e)))?;

    let mut decrypted = Vec::new();
    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|e| crate::Error::DecryptionError(format!("Decryption failed: {}", e)))?;

    reader.read_to_end(&mut decrypted).map_err(|e| {
        crate::Error::DecryptionError(format!("Failed to read decrypted data: {}", e))
    })?;

    Ok(decrypted)
}

/// Encrypt an exported mailbox for secure sharing
impl ExportBmc {
    /// Export and encrypt a mailbox for one or more age recipients
    pub async fn export_mailbox_encrypted(
        ctx: &Ctx,
        mm: &ModelManager,
        project_slug: &str,
        format: ExportFormat,
        scrub_mode: ScrubMode,
        include_attachments: bool,
        recipients: &[String],
        signing_key: Option<&SigningKey>,
    ) -> Result<(Vec<u8>, ExportManifest)> {
        // Export and optionally sign
        let (exported, manifest) = Self::export_mailbox_signed(
            ctx,
            mm,
            project_slug,
            format,
            scrub_mode,
            include_attachments,
            signing_key,
        )
        .await?;

        // Serialize manifest, exported data, and project info together
        let bundle = serde_json::json!({
            "manifest": manifest,
            "content": exported.content,
            "format": exported.format.as_str(),
            "project_slug": exported.project_slug,
            "project_name": exported.project_name,
        });
        let bundle_bytes = serde_json::to_vec_pretty(&bundle).map_err(|e| {
            crate::Error::InvalidInput(format!("Failed to serialize bundle: {}", e))
        })?;

        // Encrypt the bundle
        let encrypted = encrypt_with_age(&bundle_bytes, recipients)?;

        Ok((encrypted, manifest))
    }

    /// Export and encrypt a mailbox with a passphrase
    pub async fn export_mailbox_passphrase(
        ctx: &Ctx,
        mm: &ModelManager,
        project_slug: &str,
        format: ExportFormat,
        scrub_mode: ScrubMode,
        include_attachments: bool,
        passphrase: &str,
        signing_key: Option<&SigningKey>,
    ) -> Result<(Vec<u8>, ExportManifest)> {
        // Export and optionally sign
        let (exported, manifest) = Self::export_mailbox_signed(
            ctx,
            mm,
            project_slug,
            format,
            scrub_mode,
            include_attachments,
            signing_key,
        )
        .await?;

        // Serialize manifest, exported data, and project info together
        let bundle = serde_json::json!({
            "manifest": manifest,
            "content": exported.content,
            "format": exported.format.as_str(),
            "project_slug": exported.project_slug,
            "project_name": exported.project_name,
        });
        let bundle_bytes = serde_json::to_vec_pretty(&bundle).map_err(|e| {
            crate::Error::InvalidInput(format!("Failed to serialize bundle: {}", e))
        })?;

        // Encrypt the bundle
        let encrypted = encrypt_with_passphrase(&bundle_bytes, passphrase)?;

        Ok((encrypted, manifest))
    }

    /// Decrypt and verify an encrypted export
    pub fn decrypt_export_with_identity(
        encrypted: &[u8],
        identity: &str,
    ) -> Result<(ExportedMailbox, ExportManifest)> {
        let decrypted = decrypt_with_identity(encrypted, identity)?;

        // Parse the bundle
        let bundle: serde_json::Value = serde_json::from_slice(&decrypted)
            .map_err(|e| crate::Error::InvalidInput(format!("Failed to parse bundle: {}", e)))?;

        let manifest: ExportManifest = serde_json::from_value(bundle["manifest"].clone())
            .map_err(|e| crate::Error::InvalidInput(format!("Failed to parse manifest: {}", e)))?;

        let content = bundle["content"]
            .as_str()
            .ok_or_else(|| crate::Error::InvalidInput("Missing content in bundle".to_string()))?
            .to_string();

        let format_str = bundle["format"]
            .as_str()
            .ok_or_else(|| crate::Error::InvalidInput("Missing format in bundle".to_string()))?;

        let format: ExportFormat = format_str
            .parse()
            .map_err(|e| crate::Error::InvalidInput(format!("Invalid format: {}", e)))?;

        let project_slug = bundle["project_slug"]
            .as_str()
            .ok_or_else(|| {
                crate::Error::InvalidInput("Missing project_slug in bundle".to_string())
            })?
            .to_string();

        let project_name = bundle["project_name"]
            .as_str()
            .ok_or_else(|| {
                crate::Error::InvalidInput("Missing project_name in bundle".to_string())
            })?
            .to_string();

        let format_str = match format {
            ExportFormat::Html => "html",
            ExportFormat::Json => "json",
            ExportFormat::Markdown => "markdown",
            ExportFormat::Csv => "csv",
        };

        let exported = ExportedMailbox {
            project_slug,
            project_name,
            content,
            format: format_str.to_string(),
            message_count: manifest.message_count,
            exported_at: manifest.exported_at.clone(),
        };

        Ok((exported, manifest))
    }

    /// Decrypt and verify an encrypted export with passphrase
    pub fn decrypt_export_with_passphrase(
        encrypted: &[u8],
        passphrase: &str,
    ) -> Result<(ExportedMailbox, ExportManifest)> {
        let decrypted = decrypt_with_passphrase(encrypted, passphrase)?;

        // Parse the bundle
        let bundle: serde_json::Value = serde_json::from_slice(&decrypted)
            .map_err(|e| crate::Error::InvalidInput(format!("Failed to parse bundle: {}", e)))?;

        let manifest: ExportManifest = serde_json::from_value(bundle["manifest"].clone())
            .map_err(|e| crate::Error::InvalidInput(format!("Failed to parse manifest: {}", e)))?;

        let content = bundle["content"]
            .as_str()
            .ok_or_else(|| crate::Error::InvalidInput("Missing content in bundle".to_string()))?
            .to_string();

        let format_str = bundle["format"]
            .as_str()
            .ok_or_else(|| crate::Error::InvalidInput("Missing format in bundle".to_string()))?;

        let format: ExportFormat = format_str
            .parse()
            .map_err(|e| crate::Error::InvalidInput(format!("Invalid format: {}", e)))?;

        let project_slug = bundle["project_slug"]
            .as_str()
            .ok_or_else(|| {
                crate::Error::InvalidInput("Missing project_slug in bundle".to_string())
            })?
            .to_string();

        let project_name = bundle["project_name"]
            .as_str()
            .ok_or_else(|| {
                crate::Error::InvalidInput("Missing project_name in bundle".to_string())
            })?
            .to_string();

        let format_str = match format {
            ExportFormat::Html => "html",
            ExportFormat::Json => "json",
            ExportFormat::Markdown => "markdown",
            ExportFormat::Csv => "csv",
        };

        let exported = ExportedMailbox {
            project_slug,
            project_name,
            content,
            format: format_str.to_string(),
            message_count: manifest.message_count,
            exported_at: manifest.exported_at.clone(),
        };

        Ok((exported, manifest))
    }
}
#[cfg(test)]
mod export_scrub_tests;
