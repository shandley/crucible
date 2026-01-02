# CLAUDE.md - Crucible Development Context

## Overview

Crucible is an LLM-native data curation tool. It infers validation rules from context (rather than requiring humans to specify them) and provides non-destructive curation with full provenance.

## Key Principles

1. **Intent-driven**: AI infers what rules should be, not humans writing rules
2. **Non-destructive**: Original data never modified, curation layer sits alongside
3. **LLM-forward**: LLM integration is core, not an afterthought
4. **Provenance**: Every observation, suggestion, and decision is tracked

## Architecture

```
Input → Inference Engine → Schema Model → Validation → Suggestions → Curation Layer
          │
          ├── Statistical analyzer
          ├── Semantic analyzer
          ├── Contextual analyzer
          └── LLM augmentation
```

## Technology Stack

- **Core**: Rust
- **Python bindings**: PyO3
- **LLM**: Anthropic Claude API (primary)
- **Serialization**: serde + JSON

## Planning Documents

- `README.md` - Project overview
- `ARCHITECTURE.md` - Technical architecture
- `DESIGN.md` - Design decisions and rationale
- `CURATION_LAYER_SPEC.md` - JSON schema specification
- `ROADMAP.md` - Development phases
- `INTEGRATION.md` - BioStack integration

## Development Phases

1. **Foundation**: Core types, basic inference (no LLM)
2. **LLM Integration**: LLM-enhanced inference and suggestions
3. **Curation Layer**: Full spec implementation, persistence
4. **Application**: Export, CLI, audit trail
5. **Python Bindings**: PyO3, pip package
6. **Polish**: Documentation, testing, optimization

## Key Types

```rust
// Inferred schema for a column
struct ColumnSchema {
    name: String,
    inferred_type: ColumnType,
    semantic_role: SemanticRole,
    expected_values: Option<Vec<String>>,
    expected_range: Option<(f64, f64)>,
    constraints: Vec<Constraint>,
    confidence: f64,
}

// An issue detected
struct Observation {
    id: String,
    observation_type: ObservationType,
    severity: Severity,  // Info, Warning, Error
    column: String,
    description: String,
    evidence: serde_json::Value,
    confidence: f64,
}

// A proposed fix
struct Suggestion {
    id: String,
    observation_id: String,
    action: SuggestionAction,  // Standardize, ConvertNA, Flag, etc.
    parameters: serde_json::Value,
    rationale: String,
    confidence: f64,
}

// User decision on a suggestion
struct Decision {
    id: String,
    suggestion_id: String,
    status: DecisionStatus,  // Accepted, Rejected, Modified
    decided_by: String,
    notes: Option<String>,
}
```

## LLM Integration Points

1. **Schema inference**: Enhance inferred schema with domain knowledge
2. **Observation explanation**: Generate natural language descriptions
3. **Suggestion rationale**: Explain why a fix is recommended
4. **Confidence calibration**: Assess suggestion confidence from context

## Related Projects

- **BioStack**: LLM-native bioinformatics platform (parent ecosystem)
- **biostack-curate**: Skill that wraps Crucible with biological context
- **pointblank** (R): Inspiration for validation concepts
- **janitor** (R): Inspiration for cleaning functions

## Commands

```bash
# Build
cargo build --release

# Test
cargo test

# Run CLI (after Phase 4)
cargo run --bin crucible -- analyze metadata.tsv

# Build Python wheel (after Phase 5)
cd crucible-py && maturin build --release
```

## What NOT to Do

- Don't require humans to write validation rules upfront
- Don't modify original data in place
- Don't defer LLM integration to later phases
- Don't make LLM required for basic functionality (degraded mode)
