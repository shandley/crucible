# CLAUDE.md - Crucible Development Context

## Overview

Crucible is an LLM-native data curation tool. It infers validation rules from context (rather than requiring humans to specify them) and provides non-destructive curation with full provenance.

## Key Principles

1. **Intent-driven**: AI infers what rules should be, not humans writing rules
2. **Non-destructive**: Original data never modified, curation layer sits alongside
3. **LLM-forward**: LLM integration is core, not an afterthought
4. **Provenance**: Every observation, suggestion, and decision is tracked

## Current Status

**Phase 4 Complete** - Full CLI with embedded web UI for interactive curation.

```
Phase 1: Foundation        ✅ COMPLETE
Phase 2: LLM Integration   ✅ COMPLETE
Phase 3: Curation Layer    ✅ COMPLETE
Phase 4: Application       ✅ COMPLETE
Phase 5: Python Bindings   ← NEXT
Phase 6: Polish            Planned
```

## Crate Structure

```
crucible/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── crucible/           # Core library
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── analysis/   # Analyzers (statistical, semantic)
│   │       ├── curation/   # CurationLayer, decisions, persistence
│   │       ├── inference/  # Schema inference, type detection
│   │       ├── llm/        # LLM providers (Anthropic, OpenAI, Ollama)
│   │       ├── suggestion/ # SuggestionEngine, SuggestionAction
│   │       ├── transform/  # TransformEngine, apply operations
│   │       └── validation/ # Validators, observations
│   │
│   └── crucible-cli/       # CLI binary with embedded web UI
│       ├── src/
│       │   ├── main.rs
│       │   ├── cli.rs      # Clap definitions
│       │   ├── commands/   # analyze, review, apply, status, diff, batch
│       │   ├── server/     # Axum web server + REST API
│       │   └── web.rs      # rust-embed static file serving
│       └── frontend/       # React + TypeScript + Vite
│           └── src/
│               ├── App.tsx         # Main application component
│               ├── api/client.ts   # REST API client
│               ├── components/     # UI components
│               └── types/          # TypeScript types
```

## Key Commands

```bash
# Development
cargo build                           # Build all crates
cargo build -p crucible-cli           # Build CLI only
cargo test                            # Run all tests
cargo test -p crucible                # Test core library only

# Frontend development
cd crates/crucible-cli/frontend
npm install                           # Install dependencies
npm run dev                           # Development server (hot reload)
npm run build                         # Production build (embeds in binary)

# CLI usage
cargo run --bin crucible -- analyze test_data/ibd_cohort_metadata.tsv
cargo run --bin crucible -- review test_data/ibd_cohort_metadata.tsv
cargo run --bin crucible -- status test_data/ibd_cohort_metadata.curation.json
cargo run --bin crucible -- apply test_data/ibd_cohort_metadata.curation.json -o curated.tsv

# With LLM enhancement
ANTHROPIC_API_KEY=... cargo run --bin crucible -- analyze data.tsv --llm anthropic
```

## Architecture

```
Input → Inference Engine → Schema Model → Validation → Suggestions → Curation Layer
          │
          ├── Statistical analyzer (types, distributions, outliers)
          ├── Semantic analyzer (column names, patterns)
          ├── Contextual analyzer (file metadata, user hints)
          └── LLM augmentation (domain knowledge, explanations)
```

## Key Types

```rust
// Core analysis result
struct AnalysisResult {
    schema: TableSchema,
    observations: Vec<Observation>,
    suggestions: Vec<Suggestion>,
    summary: AnalysisSummary,
}

// Curation layer (persisted as .curation.json)
struct CurationLayer {
    source: SourceMetadata,
    context: CurationContext,
    schema: TableSchema,
    observations: Vec<Observation>,
    suggestions: Vec<Suggestion>,
    decisions: Vec<Decision>,
    summary: CurationSummary,
}

// Suggestion actions
enum SuggestionAction {
    Standardize { mappings, canonical },
    ConvertNA { values, target },
    ConvertDate { format, target_format },
    Coerce { target_type },
    Flag { reason },
}

// Decision status
enum DecisionStatus {
    Pending,
    Accepted,
    Rejected,
    Modified,
}
```

## Web UI Architecture

The frontend is a React SPA embedded in the CLI binary via rust-embed:

```
App.tsx
├── State: React Query for server state, useState for UI state
├── Mutations: accept, reject, reset, batch operations
├── Components:
│   ├── StatusBar (progress, quality score)
│   ├── SuggestionCard (individual suggestion)
│   ├── SuggestionGroup (column-based grouping)
│   ├── DataPreview (tabular view with highlighting)
│   └── Button, Badge, etc.
└── Features:
    ├── Keyboard navigation (j/k, Enter, Escape)
    ├── Undo/redo (Ctrl+Z, Ctrl+Shift+Z)
    ├── Column filtering
    └── Auto-save with progress indicator
```

## REST API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | /api/curation | Load curation layer |
| GET | /api/data-preview | Get data sample |
| POST | /api/decisions/:id/accept | Accept suggestion |
| POST | /api/decisions/:id/reject | Reject suggestion |
| POST | /api/decisions/:id/reset | Reset to pending |
| POST | /api/batch/accept | Batch accept |
| POST | /api/batch/reject | Batch reject |
| POST | /api/save | Force save |

## LLM Providers

```rust
// Provider trait
trait LlmProvider: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<String>;
    async fn complete_with_schema<T: DeserializeOwned>(&self, prompt: &str) -> Result<T>;
}

// Available providers
AnthropicProvider  // ANTHROPIC_API_KEY
OpenAiProvider     // OPENAI_API_KEY
OllamaProvider     // Local, no key required
MockProvider       // For testing
```

## Important Patterns

### 1. Non-destructive Curation
Original data is never modified. The curation layer (.curation.json) stores all decisions, and `crucible apply` generates a new file.

### 2. Suggestion Priority
Suggestions are prioritized by: severity × confidence × action type. Lower priority number = more important.

### 3. TransformEngine
Applies accepted suggestions to generate curated data:
```rust
let engine = TransformEngine::new(&curation_layer);
let (transformed_data, audit) = engine.apply(&original_data)?;
```

### 4. Evidence-based Observations
Each observation includes evidence (value_counts, sample_rows) that the UI uses for highlighting.

## Known Issues / Tech Debt

1. **Unused variants warning**: `ApiError::BadRequest` and `ApiError::Internal` in server/error.rs
2. **Unused variable warning**: `status` in commands/batch.rs
3. **Dead code warning**: `PatternType::Identifier` in validation/validators.rs
4. **Frontend**: No tests yet (add Vitest/React Testing Library)
5. **Performance**: Large files (>100k rows) may be slow in preview

## Testing

```bash
# All tests
cargo test

# Specific crate
cargo test -p crucible
cargo test -p crucible-cli

# With output
cargo test -- --nocapture

# Integration test with test data
cargo run --bin crucible -- analyze test_data/ibd_cohort_metadata.tsv
```

## Future Development (Phase 5+)

### Phase 5: Python Bindings
- PyO3 bindings for core types
- pandas/polars DataFrame integration
- Async LLM calls
- pip package via maturin

### Phase 6: Polish
- rustdoc documentation
- Property-based tests (proptest)
- Streaming for large files
- LLM call batching
- Benchmarks

### Future Features to Consider
- Multi-file curation (related tables)
- Incremental updates
- Custom validator plugins
- Local LLM support (llama.cpp integration)
- biostack-curate skill wrapper

## Related Projects

- **BioStack**: LLM-native bioinformatics platform (parent ecosystem)
- **biostack-curate**: Skill that wraps Crucible with biological context
- **pointblank** (R): Inspiration for validation concepts
- **janitor** (R): Inspiration for cleaning functions

## What NOT to Do

- Don't require humans to write validation rules upfront
- Don't modify original data in place
- Don't defer LLM integration to later phases
- Don't make LLM required for basic functionality (degraded mode works)
- Don't add features without updating tests
- Don't skip the frontend build when modifying UI (npm run build)

## Planning Documents

- `README.md` - Project overview
- `ARCHITECTURE.md` - Technical architecture
- `DESIGN.md` - Design decisions and rationale
- `CURATION_LAYER_SPEC.md` - JSON schema specification
- `ROADMAP.md` - Development phases
- `INTEGRATION.md` - BioStack integration
