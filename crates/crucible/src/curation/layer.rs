//! Curation layer - the main persistence structure for data curation.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::crucible::{AnalysisResult, AnalysisSummary, ObservationCounts};
use crate::error::{CrucibleError, Result};
use crate::input::SourceMetadata;
use crate::schema::TableSchema;
use crate::suggestion::Suggestion;
use crate::validation::Observation;

use super::context::CurationContext;
use super::decision::{Decision, DecisionStatus};

/// Current version of the crucible curation format.
pub const CRUCIBLE_VERSION: &str = "1.0.0";

/// Counts of suggestions by decision status.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SuggestionCounts {
    /// Not yet reviewed.
    pub pending: usize,
    /// Approved as-is.
    pub accepted: usize,
    /// Approved with modifications.
    pub modified: usize,
    /// Not approved.
    pub rejected: usize,
    /// Applied to output.
    pub applied: usize,
}

impl SuggestionCounts {
    /// Create new counts.
    pub fn new() -> Self {
        Self::default()
    }

    /// Total number of suggestions.
    pub fn total(&self) -> usize {
        self.pending + self.accepted + self.modified + self.rejected + self.applied
    }

    /// Number of decided suggestions (not pending).
    pub fn decided(&self) -> usize {
        self.accepted + self.modified + self.rejected + self.applied
    }

    /// Number of approved suggestions (accepted, modified, or applied).
    pub fn approved(&self) -> usize {
        self.accepted + self.modified + self.applied
    }
}

/// Enhanced summary for curation layer with suggestion status tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurationSummary {
    /// Total number of columns.
    pub total_columns: usize,

    /// Number of columns with at least one observation.
    pub columns_with_issues: usize,

    /// Total number of observations.
    pub total_observations: usize,

    /// Observations by severity.
    pub observations_by_severity: ObservationCounts,

    /// Observations by type.
    pub observations_by_type: HashMap<String, usize>,

    /// Data quality score (0.0-1.0).
    pub data_quality_score: f64,

    /// Human-readable recommendation.
    pub recommendation: String,

    /// Total number of suggestions.
    pub total_suggestions: usize,

    /// Suggestions by decision status.
    pub suggestions_by_status: SuggestionCounts,

    /// Total rows affected by suggestions.
    pub total_affected_rows: usize,
}

impl CurationSummary {
    /// Create from analysis summary and suggestions.
    pub fn from_analysis(
        summary: &AnalysisSummary,
        suggestions: &[Suggestion],
        decisions: &[Decision],
    ) -> Self {
        let total_affected_rows: usize = suggestions.iter().map(|s| s.affected_rows).sum();

        let mut suggestions_by_status = SuggestionCounts::new();
        suggestions_by_status.pending = suggestions.len();

        // Count decisions by status
        for decision in decisions {
            match decision.status {
                DecisionStatus::Pending => {} // Already counted
                DecisionStatus::Accepted => {
                    suggestions_by_status.pending =
                        suggestions_by_status.pending.saturating_sub(1);
                    suggestions_by_status.accepted += 1;
                }
                DecisionStatus::Modified => {
                    suggestions_by_status.pending =
                        suggestions_by_status.pending.saturating_sub(1);
                    suggestions_by_status.modified += 1;
                }
                DecisionStatus::Rejected => {
                    suggestions_by_status.pending =
                        suggestions_by_status.pending.saturating_sub(1);
                    suggestions_by_status.rejected += 1;
                }
                DecisionStatus::Applied => {
                    suggestions_by_status.pending =
                        suggestions_by_status.pending.saturating_sub(1);
                    suggestions_by_status.applied += 1;
                }
            }
        }

        Self {
            total_columns: summary.total_columns,
            columns_with_issues: summary.columns_with_issues,
            total_observations: summary.total_observations,
            observations_by_severity: summary.observations_by_severity.clone(),
            observations_by_type: summary.observations_by_type.clone(),
            data_quality_score: summary.data_quality_score,
            recommendation: summary.recommendation.clone(),
            total_suggestions: suggestions.len(),
            suggestions_by_status,
            total_affected_rows,
        }
    }
}

/// The curation layer - captures all inferences, observations, suggestions,
/// and decisions for a dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurationLayer {
    /// Version of the crucible format.
    pub crucible_version: String,

    /// When the curation layer was created.
    pub created_at: DateTime<Utc>,

    /// When the curation layer was last updated.
    pub updated_at: DateTime<Utc>,

    /// Metadata about the source file.
    pub source: SourceMetadata,

    /// Context for the curation.
    pub context: CurationContext,

    /// Inferred schema.
    pub schema: TableSchema,

    /// Detected observations/issues.
    pub observations: Vec<Observation>,

    /// Suggested fixes.
    pub suggestions: Vec<Suggestion>,

    /// Decisions made on suggestions.
    pub decisions: Vec<Decision>,

    /// Summary statistics.
    pub summary: CurationSummary,
}

impl CurationLayer {
    /// Create a new curation layer from analysis results.
    pub fn from_analysis(result: AnalysisResult, context: CurationContext) -> Self {
        let now = Utc::now();

        // Create initial summary with no decisions
        let summary = CurationSummary::from_analysis(&result.summary, &result.suggestions, &[]);

        Self {
            crucible_version: CRUCIBLE_VERSION.to_string(),
            created_at: now,
            updated_at: now,
            source: result.source,
            context,
            schema: result.schema,
            observations: result.observations,
            suggestions: result.suggestions,
            decisions: Vec::new(),
            summary,
        }
    }

    /// Accept a suggestion as-is.
    pub fn accept(&mut self, suggestion_id: &str) -> Result<&Decision> {
        self.validate_suggestion_exists(suggestion_id)?;
        self.validate_no_existing_decision(suggestion_id)?;

        let decision = Decision::accept(suggestion_id);
        self.decisions.push(decision);
        self.touch();

        Ok(self.decisions.last().unwrap())
    }

    /// Accept a suggestion with a user identifier.
    pub fn accept_by(&mut self, suggestion_id: &str, user: &str) -> Result<&Decision> {
        self.validate_suggestion_exists(suggestion_id)?;
        self.validate_no_existing_decision(suggestion_id)?;

        let decision = Decision::accept(suggestion_id).with_decided_by(user);
        self.decisions.push(decision);
        self.touch();

        Ok(self.decisions.last().unwrap())
    }

    /// Reject a suggestion with notes.
    pub fn reject(&mut self, suggestion_id: &str, notes: &str) -> Result<&Decision> {
        self.validate_suggestion_exists(suggestion_id)?;
        self.validate_no_existing_decision(suggestion_id)?;

        let decision = Decision::reject(suggestion_id, notes);
        self.decisions.push(decision);
        self.touch();

        Ok(self.decisions.last().unwrap())
    }

    /// Reject a suggestion with user and notes.
    pub fn reject_by(&mut self, suggestion_id: &str, user: &str, notes: &str) -> Result<&Decision> {
        self.validate_suggestion_exists(suggestion_id)?;
        self.validate_no_existing_decision(suggestion_id)?;

        let decision = Decision::reject(suggestion_id, notes).with_decided_by(user);
        self.decisions.push(decision);
        self.touch();

        Ok(self.decisions.last().unwrap())
    }

    /// Modify a suggestion with changes and notes.
    pub fn modify(
        &mut self,
        suggestion_id: &str,
        modifications: Value,
        notes: &str,
    ) -> Result<&Decision> {
        self.validate_suggestion_exists(suggestion_id)?;
        self.validate_no_existing_decision(suggestion_id)?;

        let decision = Decision::modify(suggestion_id, modifications, notes);
        self.decisions.push(decision);
        self.touch();

        Ok(self.decisions.last().unwrap())
    }

    /// Modify a suggestion with user, changes, and notes.
    pub fn modify_by(
        &mut self,
        suggestion_id: &str,
        user: &str,
        modifications: Value,
        notes: &str,
    ) -> Result<&Decision> {
        self.validate_suggestion_exists(suggestion_id)?;
        self.validate_no_existing_decision(suggestion_id)?;

        let decision = Decision::modify(suggestion_id, modifications, notes).with_decided_by(user);
        self.decisions.push(decision);
        self.touch();

        Ok(self.decisions.last().unwrap())
    }

    /// Get all pending (undecided) suggestions.
    pub fn pending_suggestions(&self) -> Vec<&Suggestion> {
        let decided_ids: std::collections::HashSet<_> =
            self.decisions.iter().map(|d| &d.suggestion_id).collect();

        self.suggestions
            .iter()
            .filter(|s| !decided_ids.contains(&s.id))
            .collect()
    }

    /// Get all accepted decisions.
    pub fn accepted_decisions(&self) -> Vec<&Decision> {
        self.decisions
            .iter()
            .filter(|d| d.status.is_approved())
            .collect()
    }

    /// Get all rejected decisions.
    pub fn rejected_decisions(&self) -> Vec<&Decision> {
        self.decisions
            .iter()
            .filter(|d| d.status == DecisionStatus::Rejected)
            .collect()
    }

    /// Get the decision for a specific suggestion.
    pub fn decision_for(&self, suggestion_id: &str) -> Option<&Decision> {
        self.decisions
            .iter()
            .find(|d| d.suggestion_id == suggestion_id)
    }

    /// Get a suggestion by ID.
    pub fn suggestion(&self, suggestion_id: &str) -> Option<&Suggestion> {
        self.suggestions.iter().find(|s| s.id == suggestion_id)
    }

    /// Get an observation by ID.
    pub fn observation(&self, observation_id: &str) -> Option<&Observation> {
        self.observations.iter().find(|o| o.id == observation_id)
    }

    /// Check if all suggestions have been decided.
    pub fn is_complete(&self) -> bool {
        self.pending_suggestions().is_empty()
    }

    /// Get progress as a fraction (0.0 to 1.0).
    pub fn progress(&self) -> f64 {
        if self.suggestions.is_empty() {
            return 1.0;
        }
        self.decisions.len() as f64 / self.suggestions.len() as f64
    }

    // Helper methods

    fn validate_suggestion_exists(&self, suggestion_id: &str) -> Result<()> {
        if self.suggestion(suggestion_id).is_none() {
            return Err(CrucibleError::Validation(format!(
                "Suggestion '{}' not found",
                suggestion_id
            )));
        }
        Ok(())
    }

    fn validate_no_existing_decision(&self, suggestion_id: &str) -> Result<()> {
        if self.decision_for(suggestion_id).is_some() {
            return Err(CrucibleError::Validation(format!(
                "Decision already exists for suggestion '{}'",
                suggestion_id
            )));
        }
        Ok(())
    }

    fn touch(&mut self) {
        self.updated_at = Utc::now();
        self.update_summary();
    }

    fn update_summary(&mut self) {
        self.summary.suggestions_by_status = SuggestionCounts::new();
        self.summary.suggestions_by_status.pending = self.suggestions.len();

        for decision in &self.decisions {
            match decision.status {
                DecisionStatus::Pending => {}
                DecisionStatus::Accepted => {
                    self.summary.suggestions_by_status.pending =
                        self.summary.suggestions_by_status.pending.saturating_sub(1);
                    self.summary.suggestions_by_status.accepted += 1;
                }
                DecisionStatus::Modified => {
                    self.summary.suggestions_by_status.pending =
                        self.summary.suggestions_by_status.pending.saturating_sub(1);
                    self.summary.suggestions_by_status.modified += 1;
                }
                DecisionStatus::Rejected => {
                    self.summary.suggestions_by_status.pending =
                        self.summary.suggestions_by_status.pending.saturating_sub(1);
                    self.summary.suggestions_by_status.rejected += 1;
                }
                DecisionStatus::Applied => {
                    self.summary.suggestions_by_status.pending =
                        self.summary.suggestions_by_status.pending.saturating_sub(1);
                    self.summary.suggestions_by_status.applied += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggestion_counts() {
        let mut counts = SuggestionCounts::new();
        counts.pending = 5;
        counts.accepted = 3;
        counts.rejected = 1;

        assert_eq!(counts.total(), 9);
        assert_eq!(counts.decided(), 4);
        assert_eq!(counts.approved(), 3);
    }

    #[test]
    fn test_decision_status_labels() {
        assert_eq!(DecisionStatus::Pending.label(), "Pending");
        assert_eq!(DecisionStatus::Accepted.label(), "Accepted");
        assert_eq!(DecisionStatus::Applied.label(), "Applied");
    }
}
