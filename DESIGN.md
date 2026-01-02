# Crucible Design Decisions

This document captures key design decisions and their rationale.

## Philosophy

### Intent-Driven vs Rule-Driven

**Decision**: Crucible infers validation rules from context rather than requiring humans to specify them.

**Rationale**: Traditional tools like `pointblank` require domain experts to write rules like `col_vals_between(age, 0, 100)`. This has problems:
- Requires upfront knowledge of all constraints
- Rules become stale as data evolves
- Different datasets need different rules
- Tedious for exploratory analysis

An LLM trained on millions of datasets already knows:
- What "age" typically contains
- What "diagnosis" values look like in medical data
- How sample IDs are usually formatted
- Common patterns in scientific metadata

Crucible leverages this knowledge to infer appropriate rules automatically.

### Non-Destructive by Default

**Decision**: Original data is never modified. All curations exist as a layer alongside the source.

**Rationale**:
- **Provenance**: Can always trace back to original values
- **Reversibility**: Bad curations can be reverted without data loss
- **Auditability**: Complete history of what changed, when, and why
- **Collaboration**: Multiple curation layers can coexist
- **Compliance**: Some domains require unmodified source data

### LLM-Forward, Not LLM-Dependent

**Decision**: LLM integration is prioritized but not required. Crucible works in degraded mode without LLM.

**Rationale**:
- Some environments can't access external APIs
- Cost considerations for large-scale processing
- Reproducibility concerns (LLM outputs vary)
- Offline/air-gapped scenarios

The LLM enhances but doesn't gate core functionality.

## Technical Decisions

### Language: Rust with Python Bindings

**Decision**: Core implementation in Rust, Python bindings via PyO3.

**Rationale**:
- **Performance**: Rust handles large datasets efficiently
- **Ecosystem alignment**: BioStack is Rust-based
- **Memory safety**: Critical for data processing
- **Python accessibility**: PyO3 provides seamless Python integration
- **LLM libraries**: Python bindings give access to LLM ecosystem

### Multi-Modal Inference

**Decision**: Combine multiple inference sources with weighted fusion.

**Sources**:

| Source | Strengths | Weaknesses |
|--------|-----------|------------|
| Statistical | Objective, fast, deterministic | No semantic understanding |
| Semantic | Understands naming conventions | Limited to patterns |
| Contextual | Domain-specific knowledge | Requires user input |
| LLM | Broad knowledge, reasoning | Slow, non-deterministic, costly |

**Fusion strategy**:
```
final_confidence = Σ(source_confidence × source_weight)
```

Weights are tunable per source, allowing users to emphasize deterministic or LLM-based inference.

### Confidence Thresholds

**Decision**: User-tunable confidence thresholds control automation.

**Default thresholds**:
- `> 0.9`: Auto-suggest with high confidence
- `0.7 - 0.9`: Suggest with medium confidence, recommend review
- `0.5 - 0.7`: Flag for investigation
- `< 0.5`: Do not suggest, but note observation

**Rationale**: Different users have different risk tolerances. A researcher doing exploratory analysis might accept 0.7 confidence suggestions automatically. A clinical data manager might require manual review of everything.

### Curation Layer Format: JSON

**Decision**: Curation layers are stored as JSON files.

**Rationale**:
- Human-readable for debugging
- Easy to version control (git diff)
- Interoperable across languages
- Serde integration is excellent
- Can be validated against JSON Schema

**Trade-offs considered**:
- Binary format (more compact, less readable)
- SQLite (better for large numbers of observations, but overkill)
- YAML (more readable, but parsing is slower)

### Column Type Taxonomy

**Decision**: Use a semantic type system, not just primitive types.

```rust
pub enum ColumnType {
    // Primitive types
    Integer,
    Float,
    String,
    Boolean,
    DateTime,

    // Semantic types (inferred)
    Identifier,      // Sample IDs, unique keys
    Categorical,     // Finite set of values
    Ordinal,         // Ordered categories
    Continuous,      // Numeric measurements
    FreeText,        // Unstructured text
    Missing,         // All values missing
}

pub enum SemanticRole {
    SampleId,        // Row identifier
    GroupingVar,     // For statistical grouping
    Covariate,       // Potential confounders
    Outcome,         // Dependent variable
    Technical,       // Batch, sequencing info
    Administrative,  // Timestamps, operators
    Unknown,
}
```

**Rationale**: Knowing that a column is an "Identifier" vs just a "String" enables smarter validation:
- Identifiers should be unique
- Grouping variables need multiple levels
- Covariates should vary

### Observation Severity Levels

**Decision**: Three-level severity system.

```rust
pub enum Severity {
    Info,     // Notable but not problematic
    Warning,  // Potential issue, analysis can proceed
    Error,    // Likely problem, should be addressed
}
```

**Rationale**:
- Simpler than pointblank's multi-level action system
- Maps well to user mental models (info/warn/error)
- Easy to filter and prioritize

### Suggestion Actions

**Decision**: Fixed set of suggestion action types.

```rust
pub enum SuggestionAction {
    Standardize,     // Normalize format/case
    ConvertNA,       // Convert string to proper NA
    Coerce,          // Type conversion
    Flag,            // Mark for human review
    Remove,          // Drop row/column
    Merge,           // Combine duplicate entries
    Rename,          // Rename column
    Split,           // Split compound values
}
```

**Rationale**:
- Exhaustive enumeration enables type-safe handling
- Each action has well-defined semantics
- Easy to audit and undo
- LLM can suggest within this vocabulary

## What Crucible Is NOT

### Not a Data Transformation Pipeline

Crucible is about **understanding and curating**, not transforming. For complex transformations, use dedicated tools (dplyr, pandas, polars).

### Not a Schema Definition Language

Crucible **infers** schemas, it doesn't require you to define them upfront. If you have a schema, provide it as context hints.

### Not a Database

Crucible operates on files, not databases. For database validation, use database-native tools or query Crucible via Python bindings.

### Not Domain-Specific

Crucible is domain-agnostic. Domain knowledge (biology, finance, etc.) is added via:
- Context hints
- LLM prompting
- Wrapper libraries (e.g., biostack-curate)

## Open Questions

### Q1: How to handle very large files?

Options:
- Streaming with sampling for inference
- Chunked processing with aggregation
- Approximate algorithms for statistics

### Q2: How to handle nested/hierarchical data?

Options:
- Flatten before processing
- Recursive schema inference
- Punt to JSON-specific tooling

### Q3: How to version curation layers?

Options:
- Embed version in JSON
- Use git for versioning
- Separate version manifest

### Q4: How to handle real-time/streaming data?

Options:
- Out of scope for v1
- Incremental curation updates
- Batch windows

## Inspiration Sources

- **pointblank** (R): Declarative validation, action levels
- **janitor** (R): Practical cleaning functions (clean_names, remove_empty)
- **great_expectations** (Python): Expectation suites, documentation
- **pandera** (Python): Schema validation with pandas
- **dbt** (SQL): Data testing, documentation
