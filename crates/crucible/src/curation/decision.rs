//! Decision tracking for curation suggestions.

use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Status of a decision on a suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DecisionStatus {
    /// Not yet reviewed.
    Pending,
    /// Approved as-is.
    Accepted,
    /// Approved with changes.
    Modified,
    /// Not approved.
    Rejected,
    /// Accepted and exported/applied.
    Applied,
}

impl DecisionStatus {
    /// Get a human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            DecisionStatus::Pending => "Pending",
            DecisionStatus::Accepted => "Accepted",
            DecisionStatus::Modified => "Modified",
            DecisionStatus::Rejected => "Rejected",
            DecisionStatus::Applied => "Applied",
        }
    }

    /// Check if this is a terminal decision (not pending).
    pub fn is_decided(&self) -> bool {
        !matches!(self, DecisionStatus::Pending)
    }

    /// Check if this is an approval (accepted, modified, or applied).
    pub fn is_approved(&self) -> bool {
        matches!(
            self,
            DecisionStatus::Accepted | DecisionStatus::Modified | DecisionStatus::Applied
        )
    }
}

/// A decision made on a suggestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    /// Unique identifier for this decision.
    pub id: String,

    /// ID of the suggestion this decision addresses.
    pub suggestion_id: String,

    /// Current status of the decision.
    pub status: DecisionStatus,

    /// Who made the decision (e.g., "user:email@example.com").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decided_by: Option<String>,

    /// When the decision was made.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decided_at: Option<DateTime<Utc>>,

    /// Modifications to the suggestion (for Modified status).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifications: Option<Value>,

    /// Optional notes explaining the decision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl Decision {
    /// Create a new pending decision for a suggestion.
    pub fn pending(suggestion_id: impl Into<String>) -> Self {
        Self {
            id: generate_decision_id(),
            suggestion_id: suggestion_id.into(),
            status: DecisionStatus::Pending,
            decided_by: None,
            decided_at: None,
            modifications: None,
            notes: None,
        }
    }

    /// Create an acceptance decision.
    pub fn accept(suggestion_id: impl Into<String>) -> Self {
        Self {
            id: generate_decision_id(),
            suggestion_id: suggestion_id.into(),
            status: DecisionStatus::Accepted,
            decided_by: None,
            decided_at: Some(Utc::now()),
            modifications: None,
            notes: None,
        }
    }

    /// Create a rejection decision.
    pub fn reject(suggestion_id: impl Into<String>, notes: impl Into<String>) -> Self {
        Self {
            id: generate_decision_id(),
            suggestion_id: suggestion_id.into(),
            status: DecisionStatus::Rejected,
            decided_by: None,
            decided_at: Some(Utc::now()),
            modifications: None,
            notes: Some(notes.into()),
        }
    }

    /// Create a modification decision.
    pub fn modify(
        suggestion_id: impl Into<String>,
        modifications: Value,
        notes: impl Into<String>,
    ) -> Self {
        Self {
            id: generate_decision_id(),
            suggestion_id: suggestion_id.into(),
            status: DecisionStatus::Modified,
            decided_by: None,
            decided_at: Some(Utc::now()),
            modifications: Some(modifications),
            notes: Some(notes.into()),
        }
    }

    /// Set who made the decision.
    pub fn with_decided_by(mut self, by: impl Into<String>) -> Self {
        self.decided_by = Some(by.into());
        self
    }

    /// Set the decision notes.
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    /// Mark as applied (after export).
    pub fn mark_applied(&mut self) {
        if self.status == DecisionStatus::Accepted || self.status == DecisionStatus::Modified {
            self.status = DecisionStatus::Applied;
        }
    }
}

/// Generate a unique decision ID.
fn generate_decision_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("dec_{:03}", COUNTER.fetch_add(1, Ordering::SeqCst))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_pending_decision() {
        let dec = Decision::pending("sug_001");

        assert!(dec.id.starts_with("dec_"));
        assert_eq!(dec.suggestion_id, "sug_001");
        assert_eq!(dec.status, DecisionStatus::Pending);
        assert!(dec.decided_at.is_none());
    }

    #[test]
    fn test_accept_decision() {
        let dec = Decision::accept("sug_001").with_decided_by("user:test@example.com");

        assert_eq!(dec.status, DecisionStatus::Accepted);
        assert!(dec.decided_at.is_some());
        assert_eq!(dec.decided_by, Some("user:test@example.com".to_string()));
    }

    #[test]
    fn test_reject_decision() {
        let dec = Decision::reject("sug_002", "Not applicable to this dataset");

        assert_eq!(dec.status, DecisionStatus::Rejected);
        assert_eq!(dec.notes, Some("Not applicable to this dataset".to_string()));
    }

    #[test]
    fn test_modify_decision() {
        let mods = serde_json::json!({"mapping": {"": "unknown"}});
        let dec = Decision::modify("sug_003", mods.clone(), "Changed null handling");

        assert_eq!(dec.status, DecisionStatus::Modified);
        assert_eq!(dec.modifications, Some(mods));
    }

    #[test]
    fn test_decision_status_checks() {
        assert!(!DecisionStatus::Pending.is_decided());
        assert!(DecisionStatus::Accepted.is_decided());
        assert!(DecisionStatus::Rejected.is_decided());

        assert!(DecisionStatus::Accepted.is_approved());
        assert!(DecisionStatus::Modified.is_approved());
        assert!(!DecisionStatus::Rejected.is_approved());
        assert!(!DecisionStatus::Pending.is_approved());
    }

    #[test]
    fn test_mark_applied() {
        let mut dec = Decision::accept("sug_001");
        dec.mark_applied();
        assert_eq!(dec.status, DecisionStatus::Applied);

        // Rejected decisions should not become applied
        let mut rejected = Decision::reject("sug_002", "No");
        rejected.mark_applied();
        assert_eq!(rejected.status, DecisionStatus::Rejected);
    }
}
