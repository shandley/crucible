# Changelog

All notable changes to Crucible are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-01-05

### Added

#### Core Analysis Engine
- Automatic type inference (Integer, Float, String, Boolean, Date, DateTime)
- Semantic role detection (Identifier, Grouping, Covariate, Outcome, Metadata)
- Delimiter auto-detection (CSV, TSV, semicolon, pipe-separated)
- Missing value detection (NA, N/A, null, empty, custom patterns)
- Data quality scoring with recommendations

#### Validators
- Outlier detection (IQR and z-score methods)
- Case variation detection and standardization
- Typo detection with edit distance matching
- Duplicate row detection
- Boolean format consistency checking
- Date format detection and ISO 8601 standardization
- Regex pattern validation
- Cross-column relationship validation
- Coordinate validation (latitude/longitude)

#### Bioinformatics Features
- MIxS 6.0 schema compliance validation
- NCBI taxonomy validation with 2.5M+ species
- Ontology term mapping (ENVO, UBERON, MONDO, etc.)
- BioSample attribute validation
- Database accession validation (GenBank, SRA, BioProject, etc.)

#### LLM Integration
- Anthropic Claude support
- OpenAI GPT support
- Ollama (local) support
- Interactive Q&A about observations
- AI-powered confidence calibration
- Domain-aware explanations

#### CLI Commands
- `crucible analyze` - Analyze data and create curation layer
- `crucible review` - Interactive web UI for reviewing suggestions
- `crucible apply` - Apply accepted changes and export
- `crucible status` - View curation progress
- `crucible diff` - Preview changes before applying
- `crucible batch` - Bulk accept/reject operations

#### Web UI
- Suggestion cards with Accept/Reject/Modify buttons
- Data preview with affected row highlighting
- Column-based grouping with collapsible sections
- Keyboard navigation (j/k, a, r, Enter, Escape, Ctrl+Z)
- Batch operations per column
- Undo/redo support
- Auto-save with progress indicator
- AI-powered Ask dialog for explanations

#### Export Formats
- TSV (tab-separated)
- CSV (comma-separated)
- JSON (array of objects with audit trail)
- Parquet (optional feature)

#### Testing Infrastructure
- Golden file tests for regression testing
- Property-based testing with proptest
- Real-world dataset validation tests
- Fuzz testing with cargo-fuzz
- Criterion benchmarks for performance
- MIxS schema conformance tests

### Performance
- Lazy regex compilation with once_cell (67,000x faster instance creation)
- Streaming statistics with Welford's algorithm (O(N) single-pass, O(1) memory)
- Reservoir sampling for approximate percentiles (avoids O(N log N) sort)
- Pagination API for data preview (offset/limit with max 500 rows)
- Virtual scrolling with @tanstack/react-virtual (smooth 500K+ row navigation)
- Large file benchmarks (10K, 100K rows) for regression testing
- 100K rows analyzed in ~1.9 seconds, 100MB files supported

### Fixed
- Async runtime panic when using LLM providers (spawn_blocking fix)
- Frontend not loading in development mode
- Ask dialog follow-up question submission
- Mock responses no longer reach users - real LLM required
- Z-index overlap between left panel and data preview sticky header

### Documentation
- User-focused README with installation and usage guide
- Architecture documentation
- Curation layer JSON schema specification
- Testing documentation

[0.1.0]: https://github.com/shandley/crucible/releases/tag/v0.1.0
