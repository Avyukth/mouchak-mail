//! Message recipient join model.
//!
//! Tracks message delivery status for each recipient (To/CC/BCC).
//! Handles read receipts and acknowledgments.

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// Access record for a message recipient.
///
/// # Fields
///
/// - `message_id` - The message being received
/// - `agent_id` - The receiving agent
/// - `recipient_type` - "to", "cc", or "bcc"
/// - `read_ts` - When the agent read the message
/// - `ack_ts` - When the agent acknowledged the message (if required)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecipient {
    pub message_id: i64,
    pub agent_id: i64,
    pub recipient_type: String, // e.g., "to", "cc", "bcc"
    pub read_ts: Option<NaiveDateTime>,
    pub ack_ts: Option<NaiveDateTime>,
}
