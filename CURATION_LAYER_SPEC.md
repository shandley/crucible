# Curation Layer Specification

Version: 1.0.0-draft

The curation layer is a JSON document that captures all inferences, observations, suggestions, and decisions for a dataset. It sits alongside the original data without modifying it.

## File Convention

```
data/
├── metadata.tsv                    # Original data (never modified)
└── metadata.curation.json          # Curation layer
```

Or in a dedicated directory:
```
data/
├── metadata.tsv
└── .crucible/
    ├── metadata.curation.json
    └── metadata.curation.history/  # Historical versions
        ├── 2024-12-30T10:00:00.json
        └── 2024-12-30T12:00:00.json
```

## Schema

### Root Object

```json
{
  "crucible_version": "1.0.0",
  "created_at": "2024-12-30T10:00:00Z",
  "updated_at": "2024-12-30T12:00:00Z",

  "source": { ... },
  "context": { ... },
  "schema": { ... },
  "observations": [ ... ],
  "suggestions": [ ... ],
  "decisions": [ ... ],

  "summary": { ... }
}
```

### Source

Metadata about the source data file.

```json
{
  "source": {
    "file": "metadata.tsv",
    "path": "/Users/researcher/project/data/metadata.tsv",
    "hash": "sha256:abc123...",
    "size_bytes": 1048576,
    "format": "tsv",
    "encoding": "utf-8",
    "row_count": 1359,
    "column_count": 73,
    "analyzed_at": "2024-12-30T10:00:00Z"
  }
}
```

### Context

User-provided and file-derived context hints.

```json
{
  "context": {
    "hints": {
      "study_name": "RISK Pediatric IBD Cohort",
      "domain": "microbiome",
      "expected_sample_count": 1400,
      "identifier_column": "sample_id"
    },
    "file_context": {
      "directory": "RISK_CCFA_analysis",
      "related_files": ["counts.tsv", "taxonomy.tsv"],
      "extraction_source": "RISK_CCFA.rds"
    },
    "inference_config": {
      "confidence_threshold": 0.8,
      "llm_enabled": true,
      "llm_model": "claude-3-opus"
    }
  }
}
```

### Schema

Inferred schema for each column.

```json
{
  "schema": {
    "columns": [
      {
        "name": "sample_id",
        "position": 0,
        "inferred_type": "string",
        "semantic_type": "identifier",
        "semantic_role": "sample_id",
        "nullable": false,
        "unique": true,
        "constraints": [
          {
            "type": "pattern",
            "value": "^[0-9]{4}\\.[A-Z0-9.]+$",
            "confidence": 0.85
          }
        ],
        "statistics": {
          "count": 1359,
          "null_count": 0,
          "unique_count": 1359,
          "sample_values": ["1939.SKBTI.0175", "1939.SKBTI.1068", "1939.SKBTI022"]
        },
        "confidence": 0.95,
        "inference_sources": ["statistical", "semantic"]
      },
      {
        "name": "diagnosis",
        "position": 38,
        "inferred_type": "string",
        "semantic_type": "categorical",
        "semantic_role": "grouping_var",
        "nullable": false,
        "unique": false,
        "expected_values": ["CD", "UC", "IC", "no"],
        "constraints": [
          {
            "type": "set_membership",
            "value": ["CD", "UC", "IC", "no"],
            "confidence": 0.92
          }
        ],
        "statistics": {
          "count": 1359,
          "null_count": 0,
          "unique_count": 4,
          "value_counts": {
            "CD": 543,
            "UC": 287,
            "IC": 156,
            "no": 373
          }
        },
        "confidence": 0.92,
        "inference_sources": ["statistical", "semantic", "llm"],
        "llm_insight": "IBD diagnosis categories: Crohn's Disease (CD), Ulcerative Colitis (UC), Indeterminate Colitis (IC), and healthy controls (no)"
      },
      {
        "name": "age",
        "position": 22,
        "inferred_type": "float",
        "semantic_type": "continuous",
        "semantic_role": "covariate",
        "nullable": false,
        "unique": false,
        "expected_range": [0, 18],
        "constraints": [
          {
            "type": "range",
            "min": 0,
            "max": 18,
            "confidence": 0.88
          }
        ],
        "statistics": {
          "count": 1359,
          "null_count": 0,
          "min": 2.5,
          "max": 17.8,
          "mean": 12.4,
          "std": 3.2,
          "median": 12.8
        },
        "confidence": 0.88,
        "inference_sources": ["statistical", "contextual", "llm"],
        "llm_insight": "Pediatric study (RISK cohort), ages consistent with pediatric range 0-18"
      }
    ],
    "row_constraints": [
      {
        "type": "unique_identifier",
        "columns": ["sample_id"],
        "confidence": 0.95
      }
    ],
    "cross_column_rules": [
      {
        "type": "conditional_presence",
        "description": "If diagnosis is CD/UC/IC, disease_stat should not be empty",
        "condition": "diagnosis IN ('CD', 'UC', 'IC')",
        "expectation": "disease_stat IS NOT NULL",
        "confidence": 0.75
      }
    ]
  }
}
```

### Observations

Issues detected during validation.

```json
{
  "observations": [
    {
      "id": "obs_001",
      "type": "missing_pattern",
      "severity": "warning",
      "column": "disease_stat",
      "description": "String 'missing' appears to represent NA values",
      "evidence": {
        "pattern": "missing",
        "occurrences": 193,
        "percentage": 14.2,
        "sample_rows": [5, 12, 23, 45, 67]
      },
      "confidence": 0.92,
      "detected_at": "2024-12-30T10:00:05Z",
      "detector": "semantic_analyzer"
    },
    {
      "id": "obs_002",
      "type": "inconsistency",
      "severity": "warning",
      "column": "antibiotics",
      "description": "Mixed boolean representations: 'true', 'false', 'TRUE', 'FALSE', and empty strings",
      "evidence": {
        "value_counts": {
          "true": 234,
          "false": 567,
          "TRUE": 12,
          "FALSE": 45,
          "": 193,
          "NA": 308
        }
      },
      "confidence": 0.95,
      "detected_at": "2024-12-30T10:00:06Z",
      "detector": "statistical_analyzer"
    },
    {
      "id": "obs_003",
      "type": "outlier",
      "severity": "info",
      "column": "age",
      "description": "Value 45.2 is outside expected pediatric range (0-18)",
      "evidence": {
        "value": 45.2,
        "row": 892,
        "sample_id": "1939.ADULT.001",
        "z_score": 10.2,
        "expected_range": [0, 18]
      },
      "confidence": 0.85,
      "detected_at": "2024-12-30T10:00:07Z",
      "detector": "llm_analyzer",
      "llm_explanation": "This appears to be an adult sample in a pediatric cohort. The sample ID contains 'ADULT' which confirms this is likely intentionally included (perhaps a parent/control) rather than a data error."
    }
  ]
}
```

### Observation Types

```
missing_pattern     - NA-like values not properly encoded
inconsistency       - Format/case variations of same concept
outlier             - Values outside expected range
duplicate           - Duplicate rows or identifiers
type_mismatch       - Values don't match inferred type
constraint_violation- Violates inferred constraint
completeness        - High missing rate
cardinality         - Unexpected number of unique values
cross_column        - Cross-column rule violation
```

### Suggestions

Proposed fixes for observations.

```json
{
  "suggestions": [
    {
      "id": "sug_001",
      "observation_id": "obs_001",
      "action": "convert_na",
      "priority": 1,
      "parameters": {
        "column": "disease_stat",
        "from_values": ["missing"],
        "to_value": null
      },
      "rationale": "Convert string 'missing' to proper NA for correct statistical handling. Missing data should be explicitly null, not a string category.",
      "affected_rows": 193,
      "confidence": 0.92,
      "reversible": true,
      "suggested_at": "2024-12-30T10:00:10Z",
      "suggester": "na_conversion_suggester"
    },
    {
      "id": "sug_002",
      "observation_id": "obs_002",
      "action": "standardize",
      "priority": 2,
      "parameters": {
        "column": "antibiotics",
        "mapping": {
          "true": "true",
          "TRUE": "true",
          "false": "false",
          "FALSE": "false",
          "": null,
          "NA": null
        }
      },
      "rationale": "Standardize boolean column to lowercase 'true'/'false' with proper NA encoding. This ensures consistent parsing and analysis.",
      "affected_rows": 558,
      "confidence": 0.95,
      "reversible": true,
      "suggested_at": "2024-12-30T10:00:11Z",
      "suggester": "standardization_suggester"
    },
    {
      "id": "sug_003",
      "observation_id": "obs_003",
      "action": "flag",
      "priority": 3,
      "parameters": {
        "column": "age",
        "rows": [892],
        "flag_column": "_age_review",
        "flag_value": "adult_in_pediatric_cohort"
      },
      "rationale": "Flag this sample for review rather than remove. The sample ID suggests this may be an intentionally included adult (parent or control). Researcher should verify.",
      "affected_rows": 1,
      "confidence": 0.70,
      "reversible": true,
      "suggested_at": "2024-12-30T10:00:12Z",
      "suggester": "llm_suggester"
    }
  ]
}
```

### Suggestion Actions

```
standardize    - Normalize format, case, encoding
convert_na     - Convert string values to proper NA
coerce         - Type conversion (string → number)
flag           - Add flag column for human review
remove         - Remove row or column
merge          - Combine duplicate entries
rename         - Rename column
split          - Split compound values into multiple columns
derive         - Create computed column
```

### Decisions

Record of user actions on suggestions.

```json
{
  "decisions": [
    {
      "id": "dec_001",
      "suggestion_id": "sug_001",
      "status": "accepted",
      "decided_by": "user:scott@example.com",
      "decided_at": "2024-12-30T11:30:00Z",
      "notes": "Confirmed these are missing values, not a category"
    },
    {
      "id": "dec_002",
      "suggestion_id": "sug_002",
      "status": "modified",
      "decided_by": "user:scott@example.com",
      "decided_at": "2024-12-30T11:31:00Z",
      "modifications": {
        "mapping": {
          "": "unknown"
        }
      },
      "notes": "Empty strings should be 'unknown', not NA - these are samples where antibiotics status wasn't recorded"
    },
    {
      "id": "dec_003",
      "suggestion_id": "sug_003",
      "status": "rejected",
      "decided_by": "user:scott@example.com",
      "decided_at": "2024-12-30T11:32:00Z",
      "notes": "This is a known adult control sample, no action needed"
    }
  ]
}
```

### Decision Statuses

```
pending   - Not yet reviewed
accepted  - Approved as-is
modified  - Approved with changes
rejected  - Not approved
applied   - Accepted and exported
```

### Summary

High-level summary for quick review.

```json
{
  "summary": {
    "total_columns": 73,
    "columns_with_issues": 12,
    "total_observations": 47,
    "observations_by_severity": {
      "error": 2,
      "warning": 31,
      "info": 14
    },
    "observations_by_type": {
      "missing_pattern": 8,
      "inconsistency": 15,
      "outlier": 6,
      "type_mismatch": 3,
      "completeness": 10,
      "duplicate": 5
    },
    "total_suggestions": 35,
    "suggestions_by_status": {
      "pending": 20,
      "accepted": 10,
      "rejected": 3,
      "modified": 2
    },
    "total_affected_rows": 892,
    "data_quality_score": 0.78,
    "recommendation": "Address high-priority suggestions before analysis. 2 error-level issues require attention."
  }
}
```

## Versioning

When the curation layer is updated:

1. Increment `updated_at`
2. Optionally save previous version to history directory
3. New observations/suggestions get new IDs
4. Decisions are append-only (never deleted)

## Validation

The curation layer itself can be validated against this JSON Schema (to be published separately).

Required fields:
- `crucible_version`
- `source.file`
- `source.hash`
- `schema.columns` (at least one)

## Extensions

Domain-specific extensions can add fields with prefixed keys:

```json
{
  "schema": {
    "columns": [
      {
        "name": "diagnosis",
        "bio:ontology": "MONDO:0005011",
        "bio:variable_type": "phenotype"
      }
    ]
  }
}
```

The `bio:` prefix indicates BioStack-specific extensions.
