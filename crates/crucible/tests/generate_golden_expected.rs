//! Helper test to generate expected.json files for golden tests.
//!
//! Run with: cargo test generate_expected_json --test generate_golden_expected -- --ignored --nocapture

use std::fs;
use std::path::Path;

use crucible::{ContextHints, Crucible, CurationContext, CurationLayer, MockProvider};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GoldenManifest {
    name: String,
    #[serde(default)]
    domain: Option<String>,
    #[serde(default)]
    mixs_package: Option<String>,
}

fn load_manifest(test_dir: &Path) -> GoldenManifest {
    let manifest_path = test_dir.join("manifest.json");
    let content = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("Failed to read manifest at {:?}: {}", manifest_path, e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse manifest at {:?}: {}", manifest_path, e))
}

fn generate_expected_for_test(test_name: &str) {
    let test_dir = Path::new("../../test_data/golden").join(test_name);

    if !test_dir.exists() {
        eprintln!("Test directory does not exist: {:?}", test_dir);
        return;
    }

    let input_path = test_dir.join("input.tsv");
    if !input_path.exists() {
        eprintln!("Input file does not exist: {:?}", input_path);
        return;
    }

    let manifest = load_manifest(&test_dir);

    let mut crucible = Crucible::new().with_llm(MockProvider::new());

    if let Some(domain) = &manifest.domain {
        crucible = crucible.with_context(ContextHints::new().with_domain(domain));
    }

    let result = crucible.analyze(&input_path)
        .unwrap_or_else(|e| panic!("Analysis failed for {:?}: {}", input_path, e));

    let mut context = CurationContext::new();
    if let Some(domain) = &manifest.domain {
        context = context.with_domain(domain.clone());
    }

    let curation = CurationLayer::from_analysis(result, context);

    let expected_path = test_dir.join("expected.json");
    let json = serde_json::to_string_pretty(&curation)
        .expect("Failed to serialize curation layer");

    fs::write(&expected_path, &json)
        .unwrap_or_else(|e| panic!("Failed to write expected.json at {:?}: {}", expected_path, e));

    println!("Generated: {:?}", expected_path);
    println!("  - {} observations", curation.observations.len());
    println!("  - {} suggestions", curation.suggestions.len());
}

#[test]
#[ignore]
fn generate_expected_json() {
    let new_tests = [
        "coordinate_validation",
        "duplicate_samples",
        "null_value_variants",
        "numeric_range",
        "whitespace_issues",
        "empty_values",
        "identifier_patterns",
    ];

    for test_name in &new_tests {
        println!("\nGenerating expected.json for: {}", test_name);
        generate_expected_for_test(test_name);
    }
}
