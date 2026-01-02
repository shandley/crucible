//! Integration tests for CurationLayer.

use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

use crucible::curation::{CurationContext, CurationLayer};
use crucible::{Crucible, DecisionStatus, MockProvider};

/// Helper to create a temporary file with given content.
fn create_test_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(content.as_bytes())
        .expect("Failed to write to temp file");
    file
}

/// Create a test dataset that will generate observations and suggestions.
fn create_test_data() -> NamedTempFile {
    let content = "sample_id,diagnosis,status\n\
                   S001,CD,active\n\
                   S002,UC,missing\n\
                   S003,CD,active\n\
                   S004,Control,missing\n\
                   S005,UC,inactive\n";
    create_test_file(content)
}

/// Create analysis result with suggestions for testing.
fn create_analysis_with_suggestions() -> crucible::AnalysisResult {
    let file = create_test_data();
    let crucible = Crucible::new().with_llm(MockProvider::new());
    crucible.analyze(file.path()).expect("Analysis failed")
}

// =============================================================================
// CurationLayer Creation Tests
// =============================================================================

#[test]
fn test_create_curation_layer_from_analysis() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new().with_domain("biomedical");

    let curation = CurationLayer::from_analysis(result.clone(), context);

    // Verify basic structure
    assert_eq!(curation.crucible_version, "1.0.0");
    assert!(!curation.source.file.is_empty());
    assert_eq!(curation.schema.columns.len(), 3);
    assert!(!curation.observations.is_empty());
}

#[test]
fn test_curation_context_builder() {
    use crucible::curation::InferenceConfig;

    let context = CurationContext::new()
        .with_domain("biomedical")
        .with_study_name("IBD Cohort Study")
        .with_inference_config(InferenceConfig::new().with_llm("claude-3-haiku"));

    assert_eq!(context.hints.domain.as_deref(), Some("biomedical"));
    assert_eq!(context.hints.study_name.as_deref(), Some("IBD Cohort Study"));
    assert_eq!(context.inference_config.llm_model.as_deref(), Some("claude-3-haiku"));
}

#[test]
fn test_curation_layer_has_suggestions() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new();

    let curation = CurationLayer::from_analysis(result, context);

    // MockProvider generates suggestions for observations
    assert!(!curation.suggestions.is_empty(), "Should have suggestions");
}

#[test]
fn test_curation_layer_initial_state() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new();

    let curation = CurationLayer::from_analysis(result, context);

    // All suggestions should be pending initially
    assert!(curation.decisions.is_empty());
    assert_eq!(curation.pending_suggestions().len(), curation.suggestions.len());
    assert!(!curation.is_complete());
}

// =============================================================================
// Decision Tests
// =============================================================================

#[test]
fn test_accept_suggestion() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new();
    let mut curation = CurationLayer::from_analysis(result, context);

    // Get first suggestion
    let suggestion_id = curation.suggestions[0].id.clone();

    // Accept it
    let decision = curation.accept(&suggestion_id).expect("Accept failed");

    assert_eq!(decision.status, DecisionStatus::Accepted);
    assert_eq!(decision.suggestion_id, suggestion_id);
    assert!(decision.decided_at.is_some());
}

#[test]
fn test_accept_by_user() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new();
    let mut curation = CurationLayer::from_analysis(result, context);

    let suggestion_id = curation.suggestions[0].id.clone();
    let decision = curation.accept_by(&suggestion_id, "curator@example.com").expect("Accept failed");

    assert_eq!(decision.decided_by.as_deref(), Some("curator@example.com"));
}

#[test]
fn test_reject_suggestion() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new();
    let mut curation = CurationLayer::from_analysis(result, context);

    let suggestion_id = curation.suggestions[0].id.clone();
    let decision = curation
        .reject(&suggestion_id, "Not applicable to our use case")
        .expect("Reject failed");

    assert_eq!(decision.status, DecisionStatus::Rejected);
    assert_eq!(decision.notes.as_deref(), Some("Not applicable to our use case"));
}

#[test]
fn test_modify_suggestion() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new();
    let mut curation = CurationLayer::from_analysis(result, context);

    let suggestion_id = curation.suggestions[0].id.clone();
    let modifications = serde_json::json!({
        "mapping": {
            "missing": "unknown"
        }
    });

    let decision = curation
        .modify(&suggestion_id, modifications.clone(), "Changed null mapping")
        .expect("Modify failed");

    assert_eq!(decision.status, DecisionStatus::Modified);
    assert_eq!(decision.modifications.as_ref(), Some(&modifications));
    assert_eq!(decision.notes.as_deref(), Some("Changed null mapping"));
}

#[test]
fn test_cannot_decide_twice() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new();
    let mut curation = CurationLayer::from_analysis(result, context);

    let suggestion_id = curation.suggestions[0].id.clone();

    // First decision
    curation.accept(&suggestion_id).expect("First accept failed");

    // Second decision should fail
    let result = curation.accept(&suggestion_id);
    assert!(result.is_err());
}

#[test]
fn test_cannot_decide_nonexistent_suggestion() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new();
    let mut curation = CurationLayer::from_analysis(result, context);

    let result = curation.accept("nonexistent_suggestion_id");
    assert!(result.is_err());
}

// =============================================================================
// Query Tests
// =============================================================================

#[test]
fn test_pending_suggestions() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new();
    let mut curation = CurationLayer::from_analysis(result, context);

    let initial_pending = curation.pending_suggestions().len();
    assert!(initial_pending > 0);

    // Accept one
    let suggestion_id = curation.suggestions[0].id.clone();
    curation.accept(&suggestion_id).unwrap();

    // Pending should decrease
    assert_eq!(curation.pending_suggestions().len(), initial_pending - 1);
}

#[test]
fn test_accepted_decisions() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new();
    let mut curation = CurationLayer::from_analysis(result, context);

    // Initially no decisions
    assert!(curation.accepted_decisions().is_empty());

    // Accept one
    let suggestion_id = curation.suggestions[0].id.clone();
    curation.accept(&suggestion_id).unwrap();

    assert_eq!(curation.accepted_decisions().len(), 1);
}

#[test]
fn test_rejected_decisions() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new();
    let mut curation = CurationLayer::from_analysis(result, context);

    let suggestion_id = curation.suggestions[0].id.clone();
    curation.reject(&suggestion_id, "Test rejection").unwrap();

    assert_eq!(curation.rejected_decisions().len(), 1);
}

#[test]
fn test_decision_for_suggestion() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new();
    let mut curation = CurationLayer::from_analysis(result, context);

    let suggestion_id = curation.suggestions[0].id.clone();

    // No decision yet
    assert!(curation.decision_for(&suggestion_id).is_none());

    // Accept
    curation.accept(&suggestion_id).unwrap();

    // Now should have decision
    let decision = curation.decision_for(&suggestion_id);
    assert!(decision.is_some());
    assert_eq!(decision.unwrap().status, DecisionStatus::Accepted);
}

#[test]
fn test_progress_tracking() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new();
    let mut curation = CurationLayer::from_analysis(result, context);

    let total = curation.suggestions.len();
    assert_eq!(curation.progress(), 0.0);

    // Decide first suggestion
    let suggestion_id = curation.suggestions[0].id.clone();
    curation.accept(&suggestion_id).unwrap();

    assert!((curation.progress() - (1.0 / total as f64)).abs() < 0.01);
}

#[test]
fn test_is_complete() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new();
    let mut curation = CurationLayer::from_analysis(result, context);

    assert!(!curation.is_complete());

    // Decide all suggestions
    let suggestion_ids: Vec<_> = curation.suggestions.iter().map(|s| s.id.clone()).collect();
    for id in suggestion_ids {
        curation.accept(&id).unwrap();
    }

    assert!(curation.is_complete());
    assert_eq!(curation.progress(), 1.0);
}

// =============================================================================
// Persistence Tests
// =============================================================================

#[test]
fn test_save_and_load() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new().with_domain("biomedical");
    let mut curation = CurationLayer::from_analysis(result, context);

    // Make some decisions
    let suggestion_id = curation.suggestions[0].id.clone();
    curation.accept(&suggestion_id).unwrap();

    // Save to temp file
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let save_path = temp_dir.path().join("test.curation.json");
    curation.save(&save_path).expect("Save failed");

    // Load back
    let loaded = CurationLayer::load(&save_path).expect("Load failed");

    // Verify
    assert_eq!(loaded.crucible_version, curation.crucible_version);
    assert_eq!(loaded.schema.columns.len(), curation.schema.columns.len());
    assert_eq!(loaded.suggestions.len(), curation.suggestions.len());
    assert_eq!(loaded.decisions.len(), curation.decisions.len());
    assert_eq!(loaded.context.hints.domain, curation.context.hints.domain);
}

#[test]
fn test_save_creates_parent_directory() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new();
    let curation = CurationLayer::from_analysis(result, context);

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let save_path = temp_dir.path().join("subdir").join("test.curation.json");

    // Should create subdir automatically
    curation.save(&save_path).expect("Save failed");

    assert!(save_path.exists());
}

#[test]
fn test_round_trip_serialization() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new()
        .with_domain("biomedical")
        .with_study_name("Test Study");
    let mut curation = CurationLayer::from_analysis(result, context);

    // Make various decisions
    if curation.suggestions.len() >= 3 {
        let id1 = curation.suggestions[0].id.clone();
        let id2 = curation.suggestions[1].id.clone();
        let id3 = curation.suggestions[2].id.clone();

        curation.accept(&id1).unwrap();
        curation.reject(&id2, "Not needed").unwrap();
        curation.modify(&id3, serde_json::json!({"test": true}), "Modified").unwrap();
    }

    // Serialize
    let json = serde_json::to_string_pretty(&curation).expect("Serialization failed");

    // Deserialize
    let loaded: CurationLayer = serde_json::from_str(&json).expect("Deserialization failed");

    // Verify round-trip
    assert_eq!(loaded.crucible_version, curation.crucible_version);
    assert_eq!(loaded.schema.columns.len(), curation.schema.columns.len());
    assert_eq!(loaded.observations.len(), curation.observations.len());
    assert_eq!(loaded.suggestions.len(), curation.suggestions.len());
    assert_eq!(loaded.decisions.len(), curation.decisions.len());
}

#[test]
fn test_load_nonexistent_file() {
    let result = CurationLayer::load("/nonexistent/path/test.curation.json");
    assert!(result.is_err());
}

// =============================================================================
// History Tests
// =============================================================================

#[test]
fn test_save_with_history() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new();
    let mut curation = CurationLayer::from_analysis(result, context);

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let save_path = temp_dir.path().join("test.curation.json");

    // Save initial version
    curation.save(&save_path).expect("Initial save failed");

    // Modify and save with history
    let suggestion_id = curation.suggestions[0].id.clone();
    curation.accept(&suggestion_id).unwrap();
    curation.save_with_history(&save_path).expect("Save with history failed");

    // Check history directory exists
    let history_dir = temp_dir.path().join("test.curation.history");
    assert!(history_dir.exists());

    // List history
    let history = CurationLayer::list_history(&save_path).expect("List history failed");
    assert_eq!(history.len(), 1);
}

#[test]
fn test_list_history_empty() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let save_path = temp_dir.path().join("test.curation.json");

    // No history exists yet
    let history = CurationLayer::list_history(&save_path).expect("List history failed");
    assert!(history.is_empty());
}

// =============================================================================
// Summary Tests
// =============================================================================

#[test]
fn test_summary_updates_on_decision() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new();
    let mut curation = CurationLayer::from_analysis(result, context);

    let initial_pending = curation.summary.suggestions_by_status.pending;

    // Accept a suggestion
    let suggestion_id = curation.suggestions[0].id.clone();
    curation.accept(&suggestion_id).unwrap();

    // Summary should be updated
    assert_eq!(curation.summary.suggestions_by_status.pending, initial_pending - 1);
    assert_eq!(curation.summary.suggestions_by_status.accepted, 1);
}

#[test]
fn test_suggestion_counts() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new();
    let mut curation = CurationLayer::from_analysis(result, context);

    // Make various decisions
    if curation.suggestions.len() >= 3 {
        let id1 = curation.suggestions[0].id.clone();
        let id2 = curation.suggestions[1].id.clone();
        let id3 = curation.suggestions[2].id.clone();

        curation.accept(&id1).unwrap();
        curation.reject(&id2, "Not needed").unwrap();
        curation.modify(&id3, serde_json::json!({}), "Modified").unwrap();
    }

    let counts = &curation.summary.suggestions_by_status;

    // Verify counts add up
    assert_eq!(
        counts.total(),
        counts.pending + counts.accepted + counts.modified + counts.rejected + counts.applied
    );

    // Modified counts as approved
    assert_eq!(counts.approved(), counts.accepted + counts.modified + counts.applied);
}

// =============================================================================
// curation_path Helper Tests
// =============================================================================

#[test]
fn test_curation_path_helper() {
    use crucible::curation::curation_path;

    let path = curation_path("data/metadata.tsv");
    assert_eq!(path.to_string_lossy(), "data/metadata.curation.json");

    let path = curation_path("test.csv");
    assert_eq!(path.to_string_lossy(), "test.curation.json");
}

#[test]
fn test_crucible_curation_path_helper() {
    use crucible::curation::crucible_curation_path;

    let path = crucible_curation_path("data/metadata.tsv");
    assert_eq!(path.to_string_lossy(), "data/.crucible/metadata.curation.json");
}

// =============================================================================
// Updated At Timestamp Tests
// =============================================================================

#[test]
fn test_updated_at_changes_on_decision() {
    let result = create_analysis_with_suggestions();
    let context = CurationContext::new();
    let mut curation = CurationLayer::from_analysis(result, context);

    let initial_updated = curation.updated_at;

    // Small delay to ensure timestamp changes
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Make a decision
    let suggestion_id = curation.suggestions[0].id.clone();
    curation.accept(&suggestion_id).unwrap();

    assert!(curation.updated_at > initial_updated);
}

// =============================================================================
// Decision Status Tests
// =============================================================================

#[test]
fn test_decision_status_labels() {
    assert_eq!(DecisionStatus::Pending.label(), "Pending");
    assert_eq!(DecisionStatus::Accepted.label(), "Accepted");
    assert_eq!(DecisionStatus::Modified.label(), "Modified");
    assert_eq!(DecisionStatus::Rejected.label(), "Rejected");
    assert_eq!(DecisionStatus::Applied.label(), "Applied");
}

#[test]
fn test_decision_status_is_approved() {
    assert!(!DecisionStatus::Pending.is_approved());
    assert!(DecisionStatus::Accepted.is_approved());
    assert!(DecisionStatus::Modified.is_approved());
    assert!(!DecisionStatus::Rejected.is_approved());
    assert!(DecisionStatus::Applied.is_approved());
}

// =============================================================================
// Real-World Workflow Test
// =============================================================================

#[test]
fn test_complete_curation_workflow() {
    // 1. Analyze data
    let content = "sample_id\tdiagnosis\tage\tstatus\n\
                   S001\tCD\t25\tactive\n\
                   S002\tUC\t30\tmissing\n\
                   S003\tCD\t28\tactive\n\
                   S004\tControl\t22\tmissing\n\
                   S005\tUC\t35\tinactive\n";
    let file = create_test_file(content);

    let crucible = Crucible::new().with_llm(MockProvider::new());
    let result = crucible.analyze(file.path()).expect("Analysis failed");

    // 2. Create curation layer with context
    let context = CurationContext::new()
        .with_domain("biomedical")
        .with_study_name("IBD Cohort Study");

    let mut curation = CurationLayer::from_analysis(result, context);

    // Verify initial state
    assert!(!curation.suggestions.is_empty());
    assert!(!curation.is_complete());

    // 3. Review and decide on suggestions
    let suggestion_ids: Vec<_> = curation.suggestions.iter().map(|s| s.id.clone()).collect();

    for (i, id) in suggestion_ids.iter().enumerate() {
        match i % 3 {
            0 => curation.accept(id).unwrap(),
            1 => curation.reject(id, "Not applicable").unwrap(),
            2 => curation.modify(id, serde_json::json!({"custom": true}), "Customized").unwrap(),
            _ => unreachable!(),
        };
    }

    // 4. Save curation layer
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let save_path = temp_dir.path().join("metadata.curation.json");
    curation.save(&save_path).expect("Save failed");

    // 5. Load and verify
    let loaded = CurationLayer::load(&save_path).expect("Load failed");

    assert!(loaded.is_complete());
    assert_eq!(loaded.progress(), 1.0);
    assert_eq!(loaded.context.hints.domain.as_deref(), Some("biomedical"));
    assert_eq!(loaded.context.hints.study_name.as_deref(), Some("IBD Cohort Study"));
    assert_eq!(loaded.decisions.len(), curation.suggestions.len());
}
