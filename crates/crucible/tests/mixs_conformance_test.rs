//! MIxS Schema Conformance Tests
//!
//! These tests verify Crucible's MIxS implementation against the official
//! Genomic Standards Consortium MIxS 6.0 specification.
//!
//! Reference: https://genomicsstandardsconsortium.github.io/mixs/
//!
//! # Test Categories
//!
//! 1. Core mandatory fields (8 fields per MIxS 6.0)
//! 2. Environmental packages (15 packages)
//! 3. Field format validation
//! 4. Alias matching
//! 5. Package-specific requirements

use crucible::bio::{MixsField, MixsFieldRequirement, MixsPackage, MixsSchema, MIXS_CORE_FIELDS};

// =============================================================================
// MIxS 6.0 Reference Data
// =============================================================================

/// Core mandatory fields per MIxS 6.0 specification.
/// Reference: https://genomicsstandardsconsortium.github.io/mixs/0001018/
const MIXS_6_CORE_MANDATORY: &[&str] = &[
    "investigation_type",
    "project_name",
    "lat_lon",
    "geo_loc_name",
    "collection_date",
    "env_broad_scale",
    "env_local_scale",
    "env_medium",
];

/// All MIxS 6.0 environmental packages.
/// Reference: https://genomicsstandardsconsortium.github.io/mixs/
const MIXS_6_PACKAGES: &[&str] = &[
    "air",
    "built environment",
    "host-associated",
    "human-associated",
    "human-gut",
    "human-oral",
    "human-skin",
    "human-vaginal",
    "microbial mat/biofilm",
    "miscellaneous",
    "plant-associated",
    "sediment",
    "soil",
    "wastewater/sludge",
    "water",
];

/// Human-associated package names.
const HUMAN_PACKAGES: &[&str] = &[
    "human-associated",
    "human-gut",
    "human-oral",
    "human-skin",
    "human-vaginal",
];

// =============================================================================
// Core Mandatory Field Tests
// =============================================================================

#[test]
fn test_all_core_mandatory_fields_defined() {
    let schema = MixsSchema::new();

    for field_name in MIXS_6_CORE_MANDATORY {
        let field = schema
            .core_fields
            .iter()
            .find(|f| f.name == *field_name);

        assert!(
            field.is_some(),
            "Core mandatory field '{}' is not defined in schema",
            field_name
        );
    }
}

#[test]
fn test_core_mandatory_fields_have_correct_requirement() {
    let schema = MixsSchema::new();

    for field_name in MIXS_6_CORE_MANDATORY {
        let field = schema
            .core_fields
            .iter()
            .find(|f| f.name == *field_name)
            .expect(&format!("Field {} should exist", field_name));

        assert_eq!(
            field.requirement,
            MixsFieldRequirement::Mandatory,
            "Field '{}' should be Mandatory, found {:?}",
            field_name,
            field.requirement
        );
    }
}

#[test]
fn test_static_core_fields_match_schema() {
    let schema = MixsSchema::new();

    // MIXS_CORE_FIELDS should match what's in the schema
    for field_name in MIXS_CORE_FIELDS {
        let in_schema = schema.core_fields.iter().any(|f| f.name == *field_name);
        assert!(
            in_schema,
            "MIXS_CORE_FIELDS contains '{}' but it's not in schema",
            field_name
        );
    }

    // Count should match
    assert_eq!(
        MIXS_CORE_FIELDS.len(),
        MIXS_6_CORE_MANDATORY.len(),
        "MIXS_CORE_FIELDS count should match MIxS 6.0 spec"
    );
}

#[test]
fn test_core_fields_have_descriptions() {
    let schema = MixsSchema::new();

    for field in &schema.core_fields {
        if field.requirement == MixsFieldRequirement::Mandatory {
            assert!(
                !field.description.is_empty(),
                "Mandatory field '{}' should have a description",
                field.name
            );
        }
    }
}

#[test]
fn test_ontology_fields_have_ontology_reference() {
    let schema = MixsSchema::new();

    // These fields should reference ENVO ontology
    let envo_fields = ["env_broad_scale", "env_local_scale", "env_medium"];

    for field_name in envo_fields {
        let field = schema
            .core_fields
            .iter()
            .find(|f| f.name == field_name)
            .expect(&format!("Field {} should exist", field_name));

        assert_eq!(
            field.ontology.as_deref(),
            Some("ENVO"),
            "Field '{}' should reference ENVO ontology",
            field_name
        );
    }
}

// =============================================================================
// Environmental Package Tests
// =============================================================================

#[test]
fn test_all_15_packages_defined() {
    let all_packages = MixsPackage::all();
    assert_eq!(
        all_packages.len(),
        15,
        "MIxS 6.0 defines 15 environmental packages"
    );
}

#[test]
fn test_package_names_match_spec() {
    for (i, expected_name) in MIXS_6_PACKAGES.iter().enumerate() {
        let package = MixsPackage::all()[i];
        let actual_name = package.name();

        assert_eq!(
            actual_name, *expected_name,
            "Package {} name mismatch: expected '{}', got '{}'",
            i, expected_name, actual_name
        );
    }
}

#[test]
fn test_human_packages_identified_correctly() {
    for package in MixsPackage::all() {
        let name = package.name();
        let is_human = package.is_human_package();

        let should_be_human = HUMAN_PACKAGES.contains(&name);

        assert_eq!(
            is_human, should_be_human,
            "Package '{}' is_human_package() = {}, expected {}",
            name, is_human, should_be_human
        );
    }
}

#[test]
fn test_all_packages_have_descriptions() {
    for package in MixsPackage::all() {
        let desc = package.description();
        assert!(
            !desc.is_empty(),
            "Package {:?} should have a description",
            package
        );
    }
}

#[test]
fn test_package_flexible_parsing() {
    // Test various input formats
    let test_cases = [
        // (input, expected package)
        ("human-gut", MixsPackage::HumanGut),
        ("Human-Gut", MixsPackage::HumanGut),
        ("HUMAN_GUT", MixsPackage::HumanGut),
        ("gut", MixsPackage::HumanGut),
        ("stool", MixsPackage::HumanGut),
        ("fecal", MixsPackage::HumanGut),
        ("soil", MixsPackage::Soil),
        ("SOIL", MixsPackage::Soil),
        ("water", MixsPackage::Water),
        ("marine", MixsPackage::Water),
        ("freshwater", MixsPackage::Water),
        ("air", MixsPackage::Air),
        ("oral", MixsPackage::HumanOral),
        ("saliva", MixsPackage::HumanOral),
        ("skin", MixsPackage::HumanSkin),
        ("sediment", MixsPackage::Sediment),
        ("wastewater", MixsPackage::WastewaterSludge),
        ("sewage", MixsPackage::WastewaterSludge),
    ];

    for (input, expected) in test_cases {
        let result = MixsPackage::from_str_flexible(input);
        assert_eq!(
            result,
            Some(expected),
            "from_str_flexible('{}') should return {:?}",
            input,
            expected
        );
    }
}

#[test]
fn test_invalid_package_names_return_none() {
    let invalid_names = [
        "invalid",
        "not_a_package",
        "random",
        "",
        "12345",
    ];

    for name in invalid_names {
        let result = MixsPackage::from_str_flexible(name);
        assert!(
            result.is_none(),
            "from_str_flexible('{}') should return None",
            name
        );
    }
}

// =============================================================================
// Package-Specific Field Tests
// =============================================================================

#[test]
fn test_human_gut_has_required_fields() {
    let schema = MixsSchema::new();
    let gut_fields = schema.fields_for_package(MixsPackage::HumanGut);

    // Human-gut should have host_subject_id as mandatory
    let subject_id = gut_fields.iter().find(|f| f.name == "host_subject_id");
    assert!(subject_id.is_some(), "HumanGut should have host_subject_id");
    assert_eq!(
        subject_id.unwrap().requirement,
        MixsFieldRequirement::Mandatory,
        "host_subject_id should be Mandatory for HumanGut"
    );

    // Should have host_body_site
    let body_site = gut_fields.iter().find(|f| f.name == "host_body_site");
    assert!(body_site.is_some(), "HumanGut should have host_body_site");
}

#[test]
fn test_soil_has_required_fields() {
    let schema = MixsSchema::new();
    let soil_fields = schema.fields_for_package(MixsPackage::Soil);

    // Soil should have depth as mandatory
    let depth = soil_fields.iter().find(|f| f.name == "depth");
    assert!(depth.is_some(), "Soil should have depth field");
    assert_eq!(
        depth.unwrap().requirement,
        MixsFieldRequirement::Mandatory,
        "depth should be Mandatory for Soil"
    );

    // Should have elevation
    let elev = soil_fields.iter().find(|f| f.name == "elev");
    assert!(elev.is_some(), "Soil should have elev field");
}

#[test]
fn test_water_has_required_fields() {
    let schema = MixsSchema::new();
    let water_fields = schema.fields_for_package(MixsPackage::Water);

    // Water should have depth
    let depth = water_fields.iter().find(|f| f.name == "depth");
    assert!(depth.is_some(), "Water should have depth field");

    // Should have temp (recommended)
    let temp = water_fields.iter().find(|f| f.name == "temp");
    assert!(temp.is_some(), "Water should have temp field");
}

#[test]
fn test_mandatory_fields_for_package() {
    let schema = MixsSchema::new();

    // Every package should have at least the 8 core mandatory fields
    for package in MixsPackage::all() {
        let mandatory = schema.mandatory_fields_for_package(*package);
        assert!(
            mandatory.len() >= 8,
            "Package {:?} should have at least 8 mandatory fields, has {}",
            package,
            mandatory.len()
        );
    }
}

// =============================================================================
// Field Alias Tests
// =============================================================================

#[test]
fn test_collection_date_aliases() {
    let schema = MixsSchema::new();

    let aliases = ["collection_date", "sample_date", "date_collected", "sampling_date"];

    for alias in aliases {
        let field = schema.find_field(alias, None);
        assert!(
            field.is_some(),
            "Should find field via alias '{}'",
            alias
        );
        assert_eq!(
            field.unwrap().name,
            "collection_date",
            "Alias '{}' should resolve to 'collection_date'",
            alias
        );
    }
}

#[test]
fn test_lat_lon_aliases() {
    let schema = MixsSchema::new();

    let aliases = ["lat_lon", "latitude_longitude", "geo_loc", "coordinates"];

    for alias in aliases {
        let field = schema.find_field(alias, None);
        assert!(
            field.is_some(),
            "Should find field via alias '{}'",
            alias
        );
        assert_eq!(
            field.unwrap().name,
            "lat_lon",
            "Alias '{}' should resolve to 'lat_lon'",
            alias
        );
    }
}

#[test]
fn test_env_medium_aliases() {
    let schema = MixsSchema::new();

    let aliases = ["env_medium", "material", "environment_material", "env_material", "sample_material"];

    for alias in aliases {
        let field = schema.find_field(alias, None);
        assert!(
            field.is_some(),
            "Should find field via alias '{}'",
            alias
        );
        assert_eq!(
            field.unwrap().name,
            "env_medium",
            "Alias '{}' should resolve to 'env_medium'",
            alias
        );
    }
}

#[test]
fn test_case_insensitive_field_matching() {
    let schema = MixsSchema::new();

    let test_cases = [
        "collection_date",
        "Collection_Date",
        "COLLECTION_DATE",
        "Collection-Date",
    ];

    for input in test_cases {
        let field = schema.find_field(input, None);
        assert!(
            field.is_some(),
            "Should find field regardless of case: '{}'",
            input
        );
    }
}

// =============================================================================
// Field Format Tests
// =============================================================================

#[test]
fn test_date_fields_have_format() {
    let schema = MixsSchema::new();

    let field = schema.find_field("collection_date", None).unwrap();
    assert!(
        field.format.is_some(),
        "collection_date should specify expected format"
    );
    assert!(
        field.format.as_ref().unwrap().contains("YYYY"),
        "collection_date format should mention YYYY pattern"
    );
}

#[test]
fn test_coordinate_fields_have_format() {
    let schema = MixsSchema::new();

    let field = schema.find_field("lat_lon", None).unwrap();
    assert!(
        field.format.is_some(),
        "lat_lon should specify expected format"
    );
    assert!(
        field.format.as_ref().unwrap().contains("DD"),
        "lat_lon format should mention decimal degrees"
    );
}

#[test]
fn test_measurement_fields_have_format() {
    let schema = MixsSchema::new();

    // Check soil depth field
    let depth = schema.find_field("depth", Some(MixsPackage::Soil)).unwrap();
    assert!(
        depth.format.is_some(),
        "depth should specify expected format"
    );
    assert!(
        depth.format.as_ref().unwrap().contains("unit"),
        "depth format should mention unit requirement"
    );
}

#[test]
fn test_ontology_fields_have_examples() {
    let schema = MixsSchema::new();

    let ontology_fields = ["env_broad_scale", "env_local_scale", "env_medium"];

    for field_name in ontology_fields {
        let field = schema.find_field(field_name, None).unwrap();
        assert!(
            field.example.is_some(),
            "Ontology field '{}' should have an example",
            field_name
        );
        assert!(
            field.example.as_ref().unwrap().contains("ENVO:"),
            "Ontology field '{}' example should contain ENVO ID",
            field_name
        );
    }
}

// =============================================================================
// Requirement Level Tests
// =============================================================================

#[test]
fn test_requirement_labels() {
    assert_eq!(MixsFieldRequirement::Mandatory.label(), "Mandatory");
    assert_eq!(MixsFieldRequirement::Conditional.label(), "Conditional");
    assert_eq!(MixsFieldRequirement::Recommended.label(), "Recommended");
    assert_eq!(MixsFieldRequirement::Optional.label(), "Optional");
}

#[test]
fn test_requirement_codes() {
    assert_eq!(MixsFieldRequirement::Mandatory.code(), "M");
    assert_eq!(MixsFieldRequirement::Conditional.code(), "C");
    assert_eq!(MixsFieldRequirement::Recommended.code(), "X");
    assert_eq!(MixsFieldRequirement::Optional.code(), "-");
}

// =============================================================================
// Schema Coverage Tests
// =============================================================================

#[test]
fn test_schema_has_recommended_fields() {
    let schema = MixsSchema::new();

    let recommended_count = schema
        .core_fields
        .iter()
        .filter(|f| f.requirement == MixsFieldRequirement::Recommended)
        .count();

    assert!(
        recommended_count >= 2,
        "Schema should have some recommended fields, found {}",
        recommended_count
    );
}

#[test]
fn test_package_fields_initialized() {
    let schema = MixsSchema::new();

    // All packages should have an entry in package_fields
    for package in MixsPackage::all() {
        assert!(
            schema.package_fields.contains_key(package),
            "Package {:?} should have an entry in package_fields",
            package
        );
    }
}

#[test]
fn test_fields_for_package_includes_core() {
    let schema = MixsSchema::new();

    for package in MixsPackage::all() {
        let fields = schema.fields_for_package(*package);

        // Should include all core fields
        for core_field in &schema.core_fields {
            let found = fields.iter().any(|f| f.name == core_field.name);
            assert!(
                found,
                "Package {:?} fields should include core field '{}'",
                package,
                core_field.name
            );
        }
    }
}

// =============================================================================
// MixsField Builder Tests
// =============================================================================

#[test]
fn test_mixs_field_builder() {
    let field = MixsField::new("test_field", MixsFieldRequirement::Mandatory)
        .with_label("Test Field")
        .with_description("A test field")
        .with_format("text")
        .with_example("example value")
        .with_ontology("TEST")
        .with_aliases(vec!["alias1", "alias2"]);

    assert_eq!(field.name, "test_field");
    assert_eq!(field.label, "Test Field");
    assert_eq!(field.description, "A test field");
    assert_eq!(field.format, Some("text".to_string()));
    assert_eq!(field.example, Some("example value".to_string()));
    assert_eq!(field.ontology, Some("TEST".to_string()));
    assert_eq!(field.aliases, vec!["alias1", "alias2"]);
}

#[test]
fn test_mixs_field_default_label() {
    let field = MixsField::new("some_field_name", MixsFieldRequirement::Optional);

    // Default label should be derived from name with underscores replaced
    assert_eq!(field.label, "some field name");
}

// =============================================================================
// Summary Test
// =============================================================================

#[test]
fn test_mixs_conformance_summary() {
    let schema = MixsSchema::new();

    println!("\n=== MIxS Schema Conformance Summary ===\n");

    println!("Core Fields: {}", schema.core_fields.len());

    let mandatory_core = schema
        .core_fields
        .iter()
        .filter(|f| f.requirement == MixsFieldRequirement::Mandatory)
        .count();
    println!("  Mandatory: {}", mandatory_core);

    let recommended_core = schema
        .core_fields
        .iter()
        .filter(|f| f.requirement == MixsFieldRequirement::Recommended)
        .count();
    println!("  Recommended: {}", recommended_core);

    println!("\nEnvironmental Packages: {}", MixsPackage::all().len());

    for package in MixsPackage::all() {
        let pkg_fields = schema
            .package_fields
            .get(package)
            .map(|f| f.len())
            .unwrap_or(0);
        let mandatory = schema.mandatory_fields_for_package(*package).len();
        println!(
            "  {:30} - {} fields ({} mandatory)",
            package.name(),
            pkg_fields + schema.core_fields.len(),
            mandatory
        );
    }

    println!("\n=== End Summary ===\n");

    // Assert basic conformance
    assert_eq!(mandatory_core, 8, "Should have 8 core mandatory fields");
    assert_eq!(MixsPackage::all().len(), 15, "Should have 15 packages");
}
