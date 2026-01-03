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

### Core Features

- **Type Inference**: Integer, Float, String, Boolean, Date, DateTime
- **Semantic Role Detection**: Identifier, Grouping, Covariate, Outcome, Metadata
- **Delimiter Detection**: Auto-detect CSV, TSV, semicolon, pipe
- **Missing Value Detection**: NA, N/A, null, empty, and custom patterns
- **Outlier Detection**: IQR and z-score methods
- **Consistency Checks**: Boolean format, case variations, typos
- **Date Format Standardization**: ISO 8601 conversion
- **LLM-Enhanced Analysis**: Anthropic, OpenAI, Ollama support
- **Data Quality Scoring**: Automated assessment with recommendations

### Coming Next

**Phase 5**: Python bindings (PyO3, pip package)
**Phase 6**: Polish (documentation, benchmarks, streaming)

### CLI

```bash
# Analyze and generate curation layer
crucible analyze metadata.tsv
# Creates metadata.curation.json

# Analyze with LLM enhancement
crucible analyze metadata.tsv --llm anthropic

# Interactive web review (opens browser)
crucible review metadata.tsv
# Opens http://localhost:3141 with React UI

# Check curation progress
crucible status metadata.curation.json

# Preview changes before applying
crucible diff metadata.curation.json

# Batch accept/reject by type or column
crucible batch metadata.curation.json --accept --action-type standardize
crucible batch metadata.curation.json --reject --column diagnosis

# Apply and export curated data
crucible apply metadata.curation.json -o curated.tsv --format tsv

# Export formats: tsv, csv, json, parquet (with --features parquet)
crucible apply metadata.curation.json -o curated.json --format json
```

### Web UI Features

The interactive review UI (`crucible review`) provides:
- **Suggestion cards** with accept/reject/modify buttons
- **Data preview** with affected row highlighting
- **Column-based grouping** with collapsible sections
- **Keyboard navigation** (j/k, Enter, Escape, Ctrl+Z)
- **Batch operations** (Accept All, Reject All per column)
- **Auto-save** with progress indicator

## Integration

Crucible is designed to be domain-agnostic. Domain-specific wrappers add context:

- **biostack-curate**: Adds biological/bioinformatics knowledge
- Future: financial data, clinical data, etc.

## Status

**Phase 4 Complete.** Full CLI with web UI for interactive curation.

| Phase | Status | Description |
|-------|--------|-------------|
| Phase 1 | **Complete** | Foundation: type inference, schema detection, validation |
| Phase 2 | **Complete** | LLM integration (Anthropic, OpenAI, Ollama) |
| Phase 3 | **Complete** | Curation layer with persistence |
| Phase 4 | **Complete** | CLI + Web UI + export functionality |
| Phase 5 | Next | Python bindings |
| Phase 6 | Planned | Polish and optimization |

See planning documents:
- [ARCHITECTURE.md](./ARCHITECTURE.md) - Technical architecture
- [DESIGN.md](./DESIGN.md) - Design decisions and rationale
- [CURATION_LAYER_SPEC.md](./CURATION_LAYER_SPEC.md) - JSON schema specification
- [ROADMAP.md](./ROADMAP.md) - Development phases
- [INTEGRATION.md](./INTEGRATION.md) - BioStack integration

## License

TBD
