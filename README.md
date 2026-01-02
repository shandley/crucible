# Crucible

**LLM-native data curation for the modern era.**

Crucible is a Rust library (with Python bindings) that brings intelligent, non-destructive data curation to tabular datasets. Unlike traditional validation tools where humans write rules, Crucible infers what the rules *should be* from context, then suggests curations with full provenance tracking.

## The Problem

Metadata curation is a massive pain point for researchers:
- Inconsistent formats ("NA", "N/A", "missing", "" all meaning the same thing)
- Case variations ("Control", "control", "CONTROL")
- Embedded special characters breaking parsers
- Type mismatches (numbers stored as strings)
- No provenance when changes are made

Traditional tools like `pointblank` or `janitor` require humans to specify validation rules. But an LLM trained on millions of datasets already *knows* what a "diagnosis" column should contain, what reasonable age ranges look like, and how to standardize categorical variables.

## The Solution

Crucible takes an **intent-driven** approach:

```
Traditional:
  Human writes: col_vals_between(age, 0, 100)
  System checks against that rule

Crucible:
  AI observes: column "age" in study titled "Pediatric IBD Cohort"
  AI infers: ages should be 0-18 (pediatric)
  AI notices: Sample X has age=45
  AI suggests: "Sample X appears adult in pediatric cohort - verify?"
```

## Core Principles

1. **Non-destructive**: Original data is never modified. A curation layer sits alongside.
2. **Provenance**: Every observation, suggestion, and decision is tracked.
3. **LLM-native**: Leverages LLM knowledge for inference and explanation.
4. **Multi-modal inference**: Combines statistical, semantic, contextual, and LLM sources.
5. **User control**: AI suggests, humans decide. Configurable confidence thresholds.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        CRUCIBLE                                 │
├─────────────────────────────────────────────────────────────────┤
│  Inference Engine                                               │
│  ├── Statistical: distributions, types, outliers, correlations │
│  ├── Semantic: column names, value patterns, format hints      │
│  ├── Contextual: file metadata, user-provided hints            │
│  └── LLM: domain knowledge, pattern recognition, explanation   │
├─────────────────────────────────────────────────────────────────┤
│  Schema Model                                                   │
│  ├── Inferred column types and semantics                       │
│  ├── Expected values, ranges, constraints                      │
│  └── Cross-column relationships                                │
├─────────────────────────────────────────────────────────────────┤
│  Curation Layer                                                 │
│  ├── Observations (issues detected)                            │
│  ├── Suggestions (proposed fixes)                              │
│  └── Decisions (accepted/rejected, with provenance)            │
├─────────────────────────────────────────────────────────────────┤
│  Application                                                    │
│  ├── Non-destructive: curated view without modifying source    │
│  └── Export: generate cleaned dataset with full audit trail    │
└─────────────────────────────────────────────────────────────────┘
```

## Usage

### Rust (Phase 1 - Available Now)

```rust
use crucible::Crucible;

// Create analyzer
let crucible = Crucible::new();

// Analyze a CSV/TSV file
let result = crucible.analyze("metadata.tsv")?;

// View inferred schema
for col in &result.schema.columns {
    println!("{}: {:?} ({:?})",
        col.name,
        col.inferred_type,
        col.semantic_role);
}

// View detected issues
for obs in &result.observations {
    println!("[{:?}] {}: {}",
        obs.severity,
        obs.column,
        obs.description);
}

// Check data quality score
println!("Quality: {:.0}%", result.summary.data_quality_score * 100.0);
println!("Recommendation: {}", result.summary.recommendation);

// Serialize to JSON
let json = serde_json::to_string_pretty(&result)?;
```

### Phase 1 Features

- **Type Inference**: Integer, Float, String, Boolean, Date, DateTime
- **Semantic Role Detection**: Identifier, Grouping, Covariate, Outcome, Metadata
- **Delimiter Detection**: Auto-detect CSV, TSV, semicolon, pipe
- **Missing Value Detection**: NA, N/A, null, empty, and custom patterns like "missing"
- **Outlier Detection**: IQR and z-score methods
- **Consistency Checks**: Boolean format variations, case inconsistencies
- **Data Quality Scoring**: Automated quality assessment with recommendations

### Future Phases

**Phase 2**: LLM integration for enhanced inference and explanations
**Phase 3**: Full curation layer with accept/reject workflow
**Phase 4**: CLI and export functionality
**Phase 5**: Python bindings

### CLI (Coming in Phase 4)

```bash
# Analyze and generate curation layer
crucible analyze metadata.tsv -o curation.json

# Interactive review
crucible review curation.json

# Apply and export
crucible apply curation.json --output metadata_curated.tsv
```

## Integration

Crucible is designed to be domain-agnostic. Domain-specific wrappers add context:

- **biostack-curate**: Adds biological/bioinformatics knowledge
- Future: financial data, clinical data, etc.

## Status

**Phase 1 Complete.** Core inference engine with statistical and semantic analysis.

| Phase | Status | Description |
|-------|--------|-------------|
| Phase 1 | **Complete** | Foundation: type inference, schema detection, validation |
| Phase 2 | Planned | LLM integration for enhanced inference |
| Phase 3 | Planned | Full curation layer with persistence |
| Phase 4 | Planned | CLI and export functionality |
| Phase 5 | Planned | Python bindings |
| Phase 6 | Planned | Polish and optimization |

See planning documents:
- [ARCHITECTURE.md](./ARCHITECTURE.md) - Technical architecture
- [DESIGN.md](./DESIGN.md) - Design decisions and rationale
- [CURATION_LAYER_SPEC.md](./CURATION_LAYER_SPEC.md) - JSON schema specification
- [ROADMAP.md](./ROADMAP.md) - Development phases
- [INTEGRATION.md](./INTEGRATION.md) - BioStack integration

## License

TBD
