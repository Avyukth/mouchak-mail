//! Escalation handling for overdue message acknowledgments.
//!
//! When messages requiring acknowledgment go unacknowledged past a threshold,
//! this module provides escalation actions:
//!
//! - **Log mode**: Log a warning for human review
//! - **File reservation mode**: Create a lock to draw attention
//! - **Overseer mode**: Send an urgent message to human oversight
//!
//! # Example
//!
//! ```no_run
//! use lib_core::model::escalation::{EscalationBmc, EscalationMode};
//! use lib_core::model::ModelManager;
//! use lib_core::ctx::Ctx;
//!
//! # async fn example() -> lib_core::Result<()> {
//! # let mm: ModelManager = todo!();
//! let ctx = Ctx::root_ctx();
//!
//! // Find messages overdue by 24+ hours and log warnings
//! let results = EscalationBmc::escalate_overdue(
//!     &ctx, &mm, 24, EscalationMode::Log, false
//! ).await?;
//! # Ok(())
//! # }
//! ```

use crate::Result;
use crate::ctx::Ctx;
use crate::model::ModelManager;
use crate::model::message::{MessageBmc, MessageForCreate, OverdueMessage};
use crate::model::overseer_message::{OverseerMessageBmc, OverseerMessageForCreate};
use serde::Serialize;
use tracing::{info, warn};

pub use lib_common::config::EscalationMode;

/// Result of an escalation action on a single message.
#[derive(Debug, Clone, Serialize)]
pub struct EscalationResult {
    /// The message ID that was escalated.
    pub message_id: i64,
    /// Description of the action taken (e.g., "logged", "overseer_message_sent").
    pub action_taken: String,
    /// Whether the escalation action succeeded.
    pub success: bool,
    /// Additional details about the action or error.
    pub details: Option<String>,
}

/// Backend Model Controller for escalation operations.
pub struct EscalationBmc;

impl EscalationBmc {
    /// Escalates all overdue message acknowledgments.
    ///
    /// Finds messages that require acknowledgment but haven't been acknowledged
    /// within the threshold period, then takes the specified escalation action.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Request context
    /// * `mm` - Model manager
    /// * `threshold_hours` - Hours after which a message is considered overdue
    /// * `mode` - Escalation mode (Log, FileReservation, or Overseer)
    /// * `dry_run` - If true, only report what would be done without taking action
    ///
    /// # Returns
    ///
    /// A vector of results, one per overdue message processed.
    pub async fn escalate_overdue(
        ctx: &Ctx,
        mm: &ModelManager,
        threshold_hours: i64,
        mode: EscalationMode,
        dry_run: bool,
    ) -> Result<Vec<EscalationResult>> {
        let overdue = MessageBmc::list_overdue_acks(ctx, mm, threshold_hours).await?;
        let mut results = Vec::with_capacity(overdue.len());

        info!(
            count = overdue.len(),
            threshold_hours,
            ?mode,
            dry_run,
            "Processing overdue ACKs"
        );

        for msg in overdue {
            let result = match mode {
                EscalationMode::Log => Self::escalate_log(&msg, dry_run),
                EscalationMode::FileReservation => {
                    Self::escalate_file_reservation(ctx, mm, &msg, dry_run).await
                }
                EscalationMode::Overseer => Self::escalate_overseer(ctx, mm, &msg, dry_run).await,
            };
            results.push(result);
        }

        Ok(results)
    }

    fn escalate_log(msg: &OverdueMessage, dry_run: bool) -> EscalationResult {
        let action = if dry_run {
            "log_dry_run"
        } else {
            warn!(
                message_id = msg.message_id,
                subject = %msg.subject,
                sender = %msg.sender_name,
                recipient = %msg.recipient_name,
                created = %msg.created_ts,
                "OVERDUE ACK: Message requires acknowledgment"
            );
            "logged"
        };

        EscalationResult {
            message_id: msg.message_id,
            action_taken: action.to_string(),
            success: true,
            details: Some(format!(
                "From: {}, To: {}, Subject: {}",
                msg.sender_name, msg.recipient_name, msg.subject
            )),
        }
    }

    async fn escalate_file_reservation(
        ctx: &Ctx,
        mm: &ModelManager,
        msg: &OverdueMessage,
        dry_run: bool,
    ) -> EscalationResult {
        if dry_run {
            return EscalationResult {
                message_id: msg.message_id,
                action_taken: "file_reservation_dry_run".to_string(),
                success: true,
                details: Some(format!(
                    "Would reserve files for message {}",
                    msg.message_id
                )),
            };
        }

        let pattern = format!("messages/**/{}*.md", msg.message_id);
        let expires_ts = chrono::Utc::now().naive_utc() + chrono::Duration::hours(1);

        match crate::model::file_reservation::FileReservationBmc::create(
            ctx,
            mm,
            crate::model::file_reservation::FileReservationForCreate {
                project_id: crate::types::ProjectId::new(msg.project_id),
                agent_id: crate::types::AgentId::new(msg.sender_id),
                path_pattern: pattern.clone(),
                exclusive: false,
                reason: format!(
                    "Escalation lock for overdue ACK on message {}",
                    msg.message_id
                ),
                expires_ts,
            },
        )
        .await
        {
            Ok(id) => EscalationResult {
                message_id: msg.message_id,
                action_taken: "file_reservation_created".to_string(),
                success: true,
                details: Some(format!("Reservation ID: {}, Pattern: {}", id, pattern)),
            },
            Err(e) => EscalationResult {
                message_id: msg.message_id,
                action_taken: "file_reservation_failed".to_string(),
                success: false,
                details: Some(format!("Error: {}", e)),
            },
        }
    }

    async fn escalate_overseer(
        ctx: &Ctx,
        mm: &ModelManager,
        msg: &OverdueMessage,
        dry_run: bool,
    ) -> EscalationResult {
        if dry_run {
            return EscalationResult {
                message_id: msg.message_id,
                action_taken: "overseer_dry_run".to_string(),
                success: true,
                details: Some(format!("Would escalate to overseer: {}", msg.subject)),
            };
        }

        let body = format!(
            "**Overdue ACK Escalation**\n\n\
             Message ID: {}\n\
             Subject: {}\n\
             From: {}\n\
             To: {} (awaiting ACK)\n\
             Created: {}\n\n\
             This message has not been acknowledged within the configured TTL.",
            msg.message_id, msg.subject, msg.sender_name, msg.recipient_name, msg.created_ts
        );

        match OverseerMessageBmc::create(
            ctx,
            mm,
            OverseerMessageForCreate {
                project_id: msg.project_id,
                sender_id: msg.sender_id,
                subject: format!("[OVERDUE ACK] {}", msg.subject),
                body_md: body,
                importance: "high".to_string(),
            },
        )
        .await
        {
            Ok(id) => EscalationResult {
                message_id: msg.message_id,
                action_taken: "overseer_message_sent".to_string(),
                success: true,
                details: Some(format!("Overseer message ID: {}", id)),
            },
            Err(e) => EscalationResult {
                message_id: msg.message_id,
                action_taken: "overseer_message_failed".to_string(),
                success: false,
                details: Some(format!("Error: {}", e)),
            },
        }
    }

    /// Sends a reminder message to the recipient of an overdue message.
    ///
    /// Creates a new high-priority message with "REMINDER:" prefix that
    /// references the original overdue message.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Request context
    /// * `mm` - Model manager
    /// * `msg` - The overdue message to send a reminder for
    ///
    /// # Returns
    ///
    /// The ID of the newly created reminder message.
    pub async fn send_reminder(ctx: &Ctx, mm: &ModelManager, msg: &OverdueMessage) -> Result<i64> {
        let reminder = MessageForCreate {
            project_id: msg.project_id,
            sender_id: msg.sender_id,
            recipient_ids: vec![msg.recipient_id],
            cc_ids: None,
            bcc_ids: None,
            subject: format!("REMINDER: {}", msg.subject),
            body_md: format!(
                "[System Escalation] This message requires acknowledgment and is overdue.\n\n\
                 Original message ID: {}",
                msg.message_id
            ),
            thread_id: None,
            importance: Some("high".to_string()),
            ack_required: true,
        };

        MessageBmc::create(ctx, mm, reminder).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escalation_result_serializes() {
        let result = EscalationResult {
            message_id: 123,
            action_taken: "logged".to_string(),
            success: true,
            details: Some("test".to_string()),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("123"));
        assert!(json.contains("logged"));
    }

    #[test]
    fn test_escalation_log_dry_run() {
        let msg = OverdueMessage {
            message_id: 1,
            project_id: 1,
            sender_id: 1,
            subject: "Test".to_string(),
            sender_name: "alice".to_string(),
            recipient_id: 2,
            recipient_name: "bob".to_string(),
            created_ts: chrono::NaiveDateTime::default(),
        };

        let result = EscalationBmc::escalate_log(&msg, true);
        assert_eq!(result.action_taken, "log_dry_run");
        assert!(result.success);
    }

    #[test]
    fn test_escalation_log_real() {
        let msg = OverdueMessage {
            message_id: 2,
            project_id: 1,
            sender_id: 1,
            subject: "Important".to_string(),
            sender_name: "sender".to_string(),
            recipient_id: 3,
            recipient_name: "recipient".to_string(),
            created_ts: chrono::NaiveDateTime::default(),
        };

        let result = EscalationBmc::escalate_log(&msg, false);
        assert_eq!(result.action_taken, "logged");
        assert!(result.success);
        assert!(result.details.unwrap().contains("Important"));
    }
}
