# Crucible Architecture

## Technology Stack

| Layer | Technology | Rationale |
|-------|------------|-----------|
| Core | Rust | Performance, safety, aligns with BioStack ecosystem |
| Python bindings | PyO3 | Broad accessibility, LLM library ecosystem |
| LLM integration | Anthropic API (primary) | Claude's strength in structured reasoning |
| Serialization | serde + JSON | Interoperability, human-readable curation layers |

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              CRUCIBLE                                   │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                     INPUT LAYER                                  │   │
│  ├─────────────────────────────────────────────────────────────────┤   │
│  │  DataSource                                                      │   │
│  │  ├── CSV/TSV parser (with delimiter detection)                  │   │
│  │  ├── JSON/JSONL support                                         │   │
│  │  ├── Parquet support (via arrow)                                │   │
│  │  └── Streaming for large files                                  │   │
│  │                                                                  │   │
│  │  ContextHints                                                    │   │
│  │  ├── User-provided metadata (study name, domain, etc.)          │   │
│  │  ├── File-derived context (filename, path, timestamps)          │   │
│  │  └── Schema hints (expected columns, types)                     │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                    │                                    │
│                                    ▼                                    │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                   INFERENCE ENGINE                               │   │
│  ├─────────────────────────────────────────────────────────────────┤   │
│  │                                                                  │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │   │
│  │  │ Statistical  │  │   Semantic   │  │  Contextual  │           │   │
│  │  │   Analyzer   │  │   Analyzer   │  │   Analyzer   │           │   │
│  │  ├──────────────┤  ├──────────────┤  ├──────────────┤           │   │
│  │  │ Type detect  │  │ Name parsing │  │ File context │           │   │
│  │  │ Distribution │  │ Value regex  │  │ User hints   │           │   │
│  │  │ Cardinality  │  │ Unit extract │  │ Domain rules │           │   │
│  │  │ Outliers     │  │ Format codes │  │              │           │   │
│  │  │ Correlation  │  │              │  │              │           │   │
│  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘           │   │
│  │         │                 │                 │                    │   │
│  │         └─────────────────┼─────────────────┘                    │   │
│  │                           │                                      │   │
│  │                           ▼                                      │   │
│  │              ┌─────────────────────────┐                         │   │
│  │              │    Inference Fusion     │                         │   │
│  │              │  (weighted combination) │                         │   │
│  │              └────────────┬────────────┘                         │   │
│  │                           │                                      │   │
│  │                           ▼                                      │   │
│  │              ┌─────────────────────────┐                         │   │
│  │              │     LLM Augmentation    │                         │   │
│  │              │  ├── Schema refinement  │                         │   │
│  │              │  ├── Issue explanation  │                         │   │
│  │              │  └── Fix suggestions    │                         │   │
│  │              └────────────┬────────────┘                         │   │
│  │                           │                                      │   │
│  └───────────────────────────┼──────────────────────────────────────┘   │
│                              │                                          │
│                              ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      SCHEMA MODEL                                │   │
│  ├─────────────────────────────────────────────────────────────────┤   │
│  │  ColumnSchema                                                    │   │
│  │  ├── name: String                                               │   │
│  │  ├── inferred_type: ColumnType                                  │   │
│  │  ├── semantic_role: SemanticRole (identifier, grouping, etc.)   │   │
│  │  ├── expected_values: Option<Vec<String>>                       │   │
│  │  ├── expected_range: Option<(f64, f64)>                         │   │
│  │  ├── constraints: Vec<Constraint>                               │   │
│  │  └── confidence: f64                                            │   │
│  │                                                                  │   │
│  │  TableSchema                                                     │   │
│  │  ├── columns: Vec<ColumnSchema>                                 │   │
│  │  ├── row_constraints: Vec<RowConstraint>                        │   │
│  │  └── cross_column_rules: Vec<CrossColumnRule>                   │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                              │                                          │
│                              ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    VALIDATION ENGINE                             │   │
│  ├─────────────────────────────────────────────────────────────────┤   │
│  │  Validators                                                      │   │
│  │  ├── TypeValidator (values match inferred type)                 │   │
│  │  ├── RangeValidator (values within expected range)              │   │
│  │  ├── SetValidator (values in expected set)                      │   │
│  │  ├── PatternValidator (values match expected pattern)           │   │
│  │  ├── CompletenessValidator (missing value analysis)             │   │
│  │  ├── UniquenessValidator (duplicate detection)                  │   │
│  │  └── ConsistencyValidator (cross-column rules)                  │   │
│  │                                                                  │   │
│  │  Output: Vec<Observation>                                        │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                              │                                          │
│                              ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                   SUGGESTION ENGINE                              │   │
│  ├─────────────────────────────────────────────────────────────────┤   │
│  │  SuggestionGenerator                                             │   │
│  │  ├── StandardizationSuggester (case, format normalization)      │   │
│  │  ├── NAConversionSuggester ("missing" → NA)                     │   │
│  │  ├── TypeCoercionSuggester (string → numeric)                   │   │
│  │  ├── OutlierFlaggingSuggester (mark for review)                 │   │
│  │  └── DeduplicationSuggester (handle duplicates)                 │   │
│  │                                                                  │   │
│  │  LLM Enhancement                                                 │   │
│  │  ├── Generate natural language rationale                        │   │
│  │  ├── Assess confidence based on context                         │   │
│  │  └── Suggest domain-appropriate fixes                           │   │
│  │                                                                  │   │
│  │  Output: Vec<Suggestion>                                         │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                              │                                          │
│                              ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    CURATION LAYER                                │   │
│  ├─────────────────────────────────────────────────────────────────┤   │
│  │  CurationLayer                                                   │   │
│  │  ├── source: SourceMetadata                                     │   │
│  │  ├── schema: TableSchema                                        │   │
│  │  ├── observations: Vec<Observation>                             │   │
│  │  ├── suggestions: Vec<Suggestion>                               │   │
│  │  └── decisions: Vec<Decision>                                   │   │
│  │                                                                  │   │
│  │  Persisted as JSON alongside original data                       │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                              │                                          │
│                              ▼                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                   APPLICATION LAYER                              │   │
│  ├─────────────────────────────────────────────────────────────────┤   │
│  │  CurationApplicator                                              │   │
│  │  ├── apply_view(): Read-through with transformations            │   │
│  │  ├── export(): Generate new file with applied curations         │   │
│  │  └── audit_trail(): Full history of changes                     │   │
│  │                                                                  │   │
│  │  Original data NEVER modified                                    │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

## Module Structure

```
crucible/
├── Cargo.toml
├── src/
│   ├── lib.rs                 # Public API
│   ├── input/
│   │   ├── mod.rs
│   │   ├── parser.rs          # CSV/TSV/JSON parsing
│   │   ├── context.rs         # Context hints
│   │   └── source.rs          # DataSource abstraction
│   ├── inference/
│   │   ├── mod.rs
│   │   ├── statistical.rs     # Distribution, type detection
│   │   ├── semantic.rs        # Name parsing, patterns
│   │   ├── contextual.rs      # External hints
│   │   ├── llm.rs             # LLM integration
│   │   └── fusion.rs          # Combine inference sources
│   ├── schema/
│   │   ├── mod.rs
│   │   ├── column.rs          # ColumnSchema
│   │   ├── table.rs           # TableSchema
│   │   └── types.rs           # ColumnType, SemanticRole
│   ├── validation/
│   │   ├── mod.rs
│   │   ├── validators.rs      # Individual validators
│   │   └── observation.rs     # Observation type
│   ├── suggestion/
│   │   ├── mod.rs
│   │   ├── generators.rs      # Suggestion generators
│   │   └── suggestion.rs      # Suggestion type
│   ├── curation/
│   │   ├── mod.rs
│   │   ├── layer.rs           # CurationLayer
│   │   ├── decision.rs        # Decision tracking
│   │   └── persistence.rs     # JSON serialization
│   ├── application/
│   │   ├── mod.rs
│   │   ├── applicator.rs      # Apply curations
│   │   └── export.rs          # Export curated data
│   └── bindings/
│       └── python.rs          # PyO3 bindings
├── crucible-cli/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs            # CLI application
└── crucible-py/
    ├── pyproject.toml
    └── src/
        └── lib.rs             # Python module
```

## Data Flow

```
1. INPUT
   ├── Data file (CSV/TSV/JSON)
   └── Context hints (optional)
           │
           ▼
2. INFERENCE (parallelizable)
   ├── Statistical analysis ──┐
   ├── Semantic analysis ─────┼──► Fusion ──► LLM augmentation
   └── Contextual analysis ───┘
           │
           ▼
3. SCHEMA MODEL
   └── Inferred expectations for each column
           │
           ▼
4. VALIDATION
   └── Compare data against schema, generate observations
           │
           ▼
5. SUGGESTION
   └── Generate fix proposals with rationale
           │
           ▼
6. CURATION LAYER (persisted)
   └── Schema + Observations + Suggestions
           │
           ▼
7. USER REVIEW
   └── Accept/reject suggestions (decisions tracked)
           │
           ▼
8. APPLICATION (optional)
   └── Export curated data with audit trail
```

## LLM Integration Points

The LLM is integrated at strategic points, not pervasively:

| Point | Purpose | Latency Impact |
|-------|---------|----------------|
| Schema refinement | Enhance inferred schema with domain knowledge | Medium |
| Issue explanation | Generate natural language descriptions | Low |
| Suggestion rationale | Explain why a fix is recommended | Low |
| Confidence calibration | Adjust confidence based on context | Low |
| Complex pattern detection | Identify issues statistical methods miss | Medium |

### LLM API Design

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn infer_schema(&self, sample: &DataSample, context: &ContextHints)
        -> Result<SchemaInference>;

    async fn explain_observation(&self, obs: &Observation, schema: &TableSchema)
        -> Result<String>;

    async fn suggest_fix(&self, obs: &Observation, schema: &TableSchema)
        -> Result<Option<Suggestion>>;

    async fn calibrate_confidence(&self, suggestion: &Suggestion, context: &ContextHints)
        -> Result<f64>;
}
```

### Degraded Mode (No LLM)

Crucible should work without an LLM, with reduced capabilities:

| Feature | With LLM | Without LLM |
|---------|----------|-------------|
| Type inference | Enhanced | Statistical only |
| Issue detection | Domain-aware | Pattern-based |
| Explanations | Natural language | Template-based |
| Confidence | Calibrated | Heuristic |
| Fix suggestions | Context-aware | Rule-based |

## Performance Considerations

1. **Streaming**: Large files processed in chunks, not loaded entirely
2. **Parallelism**: Statistical, semantic, contextual analyzers run in parallel
3. **LLM batching**: Multiple LLM calls batched where possible
4. **Caching**: LLM responses cached for similar patterns
5. **Lazy evaluation**: Suggestions generated on-demand, not upfront

## Security Considerations

1. **Data privacy**: Option to not send data to LLM, only metadata/schema
2. **Local LLM**: Support for local models (llama.cpp, etc.)
3. **Audit trail**: All operations logged for compliance
4. **No mutation**: Original data never modified, eliminating data loss risk
