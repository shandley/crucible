# Crucible Development Roadmap

## Overview

Development is organized into phases, each producing a usable artifact. The LLM integration is prioritized throughout, not deferred.

```
Phase 1: Foundation        [Core types, basic inference, no LLM]        ✅ COMPLETE
Phase 2: LLM Integration   [LLM-enhanced inference and suggestions]     ✅ COMPLETE
Phase 3: Curation Layer    [Full spec implementation, persistence]      ← NEXT
Phase 4: Application       [Export, CLI, audit trail]
Phase 5: Python Bindings   [PyO3, pip package]
Phase 6: Polish            [Documentation, testing, optimization]
```

## Phase 1: Foundation ✅ COMPLETE

**Goal**: Basic inference engine that can analyze a CSV/TSV and produce observations.

### Deliverables

- [x] Project scaffolding (Cargo workspace)
- [x] Input parsing (CSV/TSV with delimiter detection)
- [x] Statistical analyzer
  - [x] Type detection (integer, float, string, boolean, datetime)
  - [x] Distribution analysis (min, max, mean, std, quartiles)
  - [x] Cardinality analysis (unique count, value frequencies)
  - [x] Missing value detection (null, empty string)
  - [x] Outlier detection (IQR, z-score methods)
- [x] Semantic analyzer
  - [x] Column name parsing (extract semantic hints)
  - [x] Value pattern detection (regex inference)
  - [x] Format detection (dates, identifiers, codes)
- [x] Basic schema model
  - [x] ColumnSchema struct
  - [x] TableSchema struct
  - [x] Inference fusion (combine statistical + semantic)
- [x] Basic validators
  - [x] Type validator
  - [x] Range validator
  - [x] Set membership validator
  - [x] Uniqueness validator
- [x] Observation generation
  - [x] Observation struct with severity levels
  - [x] Template-based descriptions (no LLM yet)

### Exit Criteria

```rust
let crucible = Crucible::new();
let result = crucible.analyze("metadata.tsv")?;

// Should produce:
// - Inferred schema for all columns
// - Observations for common issues
// - Template-based descriptions
```

### Estimated Scope

- ~15-20 source files
- ~2000-3000 lines of Rust
- Tests for each analyzer

---

## Phase 2: LLM Integration ✅ COMPLETE

**Goal**: LLM-enhanced inference, explanations, and suggestions.

### Deliverables

- [x] LLM provider trait
  - [x] Anthropic implementation (Claude API)
  - [x] Mock provider for testing
- [x] LLM-enhanced schema inference
  - [x] Send column samples to LLM
  - [x] Parse structured schema response
  - [x] Merge with statistical/semantic inference
- [x] LLM-generated explanations
  - [x] Observation explanations
  - [x] Domain-aware context
- [x] Suggestion generator
  - [x] Basic suggestion types (standardize, convert_na, flag)
  - [x] LLM-generated rationale
  - [x] Confidence calibration
- [x] Context hints
  - [x] User-provided hints (study name, domain)
  - [x] File-derived context (filename, related files)
  - [x] Context passed to LLM

### Exit Criteria

```rust
let crucible = Crucible::new()
    .with_llm(AnthropicProvider::new(api_key)?);

let result = crucible.analyze("metadata.tsv")?;

// Should produce:
// - LLM-enhanced schema with semantic insights
// - Natural language observation explanations
// - Suggestions with rationale
```

### LLM Prompt Design

Key prompts to design:
1. **Schema inference prompt**: Given column samples, infer type/constraints
2. **Observation explanation prompt**: Explain issue in domain context
3. **Suggestion prompt**: Propose fix with rationale
4. **Confidence calibration prompt**: Assess suggestion confidence

### Estimated Scope

- ~10 additional source files
- ~1500-2000 lines of Rust
- LLM integration tests (with mocks)

---

## Phase 3: Curation Layer

**Goal**: Full curation layer spec implementation with persistence.

### Deliverables

- [ ] Full curation layer struct matching spec
  - [ ] Source metadata
  - [ ] Context
  - [ ] Schema
  - [ ] Observations
  - [ ] Suggestions
  - [ ] Decisions
  - [ ] Summary
- [ ] JSON serialization/deserialization
  - [ ] serde integration
  - [ ] Schema validation
- [ ] Decision tracking
  - [ ] Accept/reject/modify suggestions
  - [ ] Decision audit trail
  - [ ] User attribution
- [ ] Curation layer persistence
  - [ ] Save to `.curation.json`
  - [ ] Load and continue curation
  - [ ] History/versioning

### Exit Criteria

```rust
let crucible = Crucible::new().with_llm(provider);
let curation = crucible.analyze("metadata.tsv")?;

// Review and decide
curation.accept("sug_001")?;
curation.reject("sug_002", "Not applicable")?;
curation.modify("sug_003", modifications)?;

// Persist
curation.save("metadata.curation.json")?;

// Later, load and continue
let curation = CurationLayer::load("metadata.curation.json")?;
```

### Estimated Scope

- ~8 additional source files
- ~1000-1500 lines of Rust
- Round-trip serialization tests

---

## Phase 4: Application

**Goal**: Apply curations and export cleaned data.

### Deliverables

- [ ] Curation applicator
  - [ ] Apply accepted suggestions to data
  - [ ] Non-destructive (produces new file)
  - [ ] Audit trail in output
- [ ] Export formats
  - [ ] CSV/TSV export
  - [ ] JSON export
  - [ ] Audit metadata (what was changed)
- [ ] CLI application
  - [ ] `crucible analyze <file>` - Generate curation layer
  - [ ] `crucible review <curation>` - Interactive review (TUI?)
  - [ ] `crucible apply <curation>` - Export curated data
  - [ ] `crucible status <curation>` - Summary report
- [ ] Batch processing
  - [ ] Multiple files
  - [ ] Directory scanning

### Exit Criteria

```bash
# Analyze
crucible analyze metadata.tsv -o metadata.curation.json

# Review (shows summary, pending suggestions)
crucible status metadata.curation.json

# Apply and export
crucible apply metadata.curation.json -o metadata_curated.tsv

# Verify
diff metadata.tsv metadata_curated.tsv
```

### Estimated Scope

- ~10 additional source files
- ~1500-2000 lines of Rust
- CLI integration tests

---

## Phase 5: Python Bindings

**Goal**: Python package for broader accessibility.

### Deliverables

- [ ] PyO3 bindings
  - [ ] Crucible class
  - [ ] CurationLayer class
  - [ ] Observation, Suggestion types
- [ ] Python-native API
  - [ ] DataFrame integration (pandas, polars)
  - [ ] Async support for LLM calls
- [ ] Package distribution
  - [ ] maturin build
  - [ ] PyPI publication
  - [ ] wheels for major platforms

### Exit Criteria

```python
from crucible import Crucible

crucible = Crucible(llm="anthropic", api_key=os.environ["ANTHROPIC_API_KEY"])
curation = crucible.analyze("metadata.tsv")

# Works with pandas
import pandas as pd
df = pd.read_csv("metadata.tsv", sep="\t")
curation = crucible.analyze(df)

# Review
for obs in curation.observations:
    print(f"{obs.severity}: {obs.description}")

# Accept and export
curation.accept("sug_001")
curation.export("metadata_curated.tsv")
```

### Estimated Scope

- ~5 additional source files
- ~1000 lines of Rust (PyO3)
- Python test suite

---

## Phase 6: Polish

**Goal**: Production readiness.

### Deliverables

- [ ] Documentation
  - [ ] API documentation (rustdoc)
  - [ ] User guide
  - [ ] Examples
- [ ] Testing
  - [ ] Property-based tests (proptest)
  - [ ] Fuzzing for parsers
  - [ ] LLM response variation tests
- [ ] Performance
  - [ ] Benchmarks
  - [ ] Streaming for large files
  - [ ] LLM call batching
- [ ] Robustness
  - [ ] Error handling review
  - [ ] Edge case handling
  - [ ] Encoding issues

### Estimated Scope

- Documentation, tests, optimizations
- ~2000+ lines of tests and docs

---

## Future Phases (Post-v1.0)

### Phase 7: Advanced Features

- [ ] Multi-file curation (related tables)
- [ ] Incremental updates (new data arrives)
- [ ] Custom validator plugins
- [ ] Local LLM support (llama.cpp)

### Phase 8: Integrations

- [ ] biostack-curate skill
- [ ] Database connectors
- [ ] Cloud storage (S3, GCS)
- [ ] Workflow integration (Nextflow, Snakemake)

---

## Development Principles

1. **LLM-forward**: Don't defer LLM integration. It's core to the value proposition.
2. **Test continuously**: Each phase has tests before moving on.
3. **Usable artifacts**: Each phase produces something that can be used.
4. **Iterate on spec**: The curation layer spec will evolve based on implementation learnings.

## Success Metrics

- **Phase 1**: Can analyze a file and produce observations
- **Phase 2**: LLM explanations are more useful than templates
- **Phase 3**: Curation layer round-trips correctly
- **Phase 4**: Can export cleaned data with audit trail
- **Phase 5**: Python users can use crucible
- **Phase 6**: Ready for production use

## Timeline

Not specified. Each phase is complete when exit criteria are met, not by date.
